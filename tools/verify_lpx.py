#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
# Copyright (C) 2026 Simon Peter
"""Gate 0 check for a Launchpad X firmware build. Fails hard on any doubt.

usage: verify_lpx.py <core.elf> <fw.bin> <fw.syx> <syxtool.py>
Checks: ELF load segments inside the app region, size margin, vector table,
syx product id (0x0C = Launchpad X) and that the syx decodes back to fw.bin.
"""
import re, struct, subprocess, sys, tempfile, os

RAM_END = 0x20010000      # 64 KB of RAM, the stack starts here
MIN_STACK_MARGIN = 4096
ISR_RESERVE = 2048
APP_START = 0x0800C000
APP_END = 0x08020000      # 0x0800C000 + 0x14000
MIN_MARGIN = 1024


def fail(msg):
    print("FAIL:", msg)
    sys.exit(1)


def call_graph(elf):
    """Functions of the image: name, own stack frame (pushes plus `sub sp`) and the
    functions they call directly. Indirect calls (trait objects) are not visible."""
    dis = subprocess.run(["arm-none-eabi-objdump", "-d", "--no-show-raw-insn", elf], capture_output=True, text=True, check=True).stdout
    funcs, cur = {}, None
    for line in dis.splitlines():
        h = re.match(r"^([0-9a-f]+) <(.*)>:$", line)
        if h:
            cur = int(h.group(1), 16)
            funcs[cur] = {"name": h.group(2), "frame": 0, "calls": set()}
            continue
        if cur is None:
            continue
        m = re.search(r"\bpush(?:\.w)?\s+\{([^}]*)\}", line)
        if m:
            funcs[cur]["frame"] += 4 * len([r for r in m.group(1).split(",") if r.strip()])
        m = re.search(r"\bsub(?:w|\.w)?\s+sp, sp, #(\d+)", line)
        if m:
            funcs[cur]["frame"] += int(m.group(1))
        m = re.search(r"\b(?:bl|b\.w)\s+([0-9a-f]+) <([^>+]*)", line)
        if m and int(m.group(1), 16) != cur:
            funcs[cur]["calls"].add(int(m.group(1), 16))
    return funcs


def stack_depths(funcs):
    sys.setrecursionlimit(100000)
    memo = {}

    def depth(a, seen):
        if a in memo:
            return memo[a]
        f = funcs.get(a)
        if f is None:
            return 0
        best = 0
        for c in f["calls"]:
            if c not in seen:
                best = max(best, depth(c, seen | {a}))
        memo[a] = f["frame"] + best
        return memo[a]

    return {a: depth(a, frozenset()) for a in funcs}


def check_stack(elf):
    """The stack is what is left of the 64 KB of RAM above the static data. The main
    task is polled by the executor; at startup it calls the functions that build the
    big statics, and later it calls the apps. A build whose deepest call chain does not
    fit crashes without a trace (this happened twice: the pads stay dark and USB never
    comes up). Fails hard if the worst-case depth plus a margin exceeds the free RAM.
    Indirect calls are not visible in the call graph, so the app handlers are added as
    extra roots on top of the main task's frame, plus a reserve for interrupts."""
    sections = subprocess.run(["arm-none-eabi-readelf", "-SW", elf], capture_output=True, text=True, check=True).stdout
    m = re.search(r"\.bss\s+NOBITS\s+([0-9a-f]+)\s+[0-9a-f]+\s+([0-9a-f]+)", sections)
    if not m:
        fail("no .bss section found")
    bss_end = int(m.group(1), 16) + int(m.group(2), 16)
    free = RAM_END - bss_end
    funcs = call_graph(elf)
    depths = stack_depths(funcs)
    polls = [a for a, f in funcs.items() if "embassy_main_task" in f["name"] and ("TaskStorage" in f["name"] or "poll" in f["name"])]
    if not polls:
        fail("could not find the main task in the disassembly")
    poll = max(polls, key=lambda a: depths[a])
    startup = depths[poll]
    handlers = ("on_surface", "on_tick", "on_enter", "on_exit", "on_midi", "on_aftertouch", "route_", "receive_sysex", "background_tick")
    run = max((depths[a] for a, f in funcs.items() if any(h in f["name"] for h in handlers)), default=0)
    runtime = funcs[poll]["frame"] + run + ISR_RESERVE
    need = max(startup, runtime)
    print(f"RAM: static data ends {bss_end:#x}, {free} bytes free for the stack; "
          f"startup needs {startup}, running needs {runtime} (worst-case call depth, ISR reserve {ISR_RESERVE})")
    if free - need < MIN_STACK_MARGIN:
        fail(f"stack margin {free - need} bytes is below {MIN_STACK_MARGIN}: the firmware would overflow its stack")


def main():
    if len(sys.argv) != 5:
        fail("usage: verify_lpx.py <core.elf> <fw.bin> <fw.syx> <syxtool.py>")
    elf, binf, syx, tool = sys.argv[1:]

    out = subprocess.run(["arm-none-eabi-readelf", "-lW", elf], capture_output=True, text=True, check=True).stdout
    top = 0
    for line in out.splitlines():
        f = line.split()
        if f and f[0] == "LOAD":
            paddr, filesz = int(f[3], 16), int(f[4], 16)
            vaddr = int(f[2], 16)
            if filesz == 0:
                continue
            # Flash content is placed at its load address (paddr).
            if paddr < APP_START or paddr + filesz > APP_END:
                fail(f"segment {paddr:#x}+{filesz:#x} outside app region")
            top = max(top, paddr + filesz)
    if top == 0:
        fail("no flash segments found")
    margin = APP_END - top
    print(f"image ends at {top:#x}, margin {margin} bytes")
    if margin < MIN_MARGIN:
        fail(f"margin below {MIN_MARGIN}")

    check_stack(elf)

    data = open(binf, "rb").read()
    if len(data) > APP_END - APP_START:
        fail("bin larger than app region")
    sp, reset = struct.unpack_from("<II", data, 0)
    if not (0x20000000 <= sp <= 0x20010000):
        fail(f"initial SP {sp:#x} not in 64K RAM")
    for i in range(1, 16):
        v = struct.unpack_from("<I", data, 4 * i)[0]
        if v and not (APP_START <= (v & ~1) < APP_END):
            fail(f"vector {i} = {v:#x} outside app region")
    if not (reset & 1):
        fail("reset vector lacks thumb bit")
    print(f"vectors ok, SP {sp:#x}, reset {reset:#x}")

    s = open(syx, "rb").read()
    if s[0] != 0xF0 or s[-1] != 0xF7:
        fail("syx not framed F0..F7")
    msgs = s.split(b"\xf7")[:-1]
    for m in msgs:
        if m[:5] != b"\xf0\x00\x20\x29\x00":
            fail("message without Novation header")
    first = msgs[0]
    if first[5] != 0x71 or first[6] != 0x02 or first[7] != 0x0C:
        fail(f"init message family/product {first[6]:#x}/{first[7]:#x}, expected 02/0C")
    print(f"syx: {len(msgs)} messages, product 0x0C (Launchpad X)")

    with tempfile.TemporaryDirectory() as d:
        back = os.path.join(d, "back.bin")
        subprocess.run([sys.executable, "-I", tool, "--to-bin", syx, back], check=True, capture_output=True)
        b = open(back, "rb").read()
        if b[:len(data)] != data or any(x not in (0, 0xFF) for x in b[len(data):]):
            fail("syx does not decode back to fw.bin")
    print("syx round-trips to fw.bin")
    print("PASS")


main()
