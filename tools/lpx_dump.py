#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
# Copyright (C) 2026 Simon Peter
"""Reads memory dumps from a Launchpad X running the flash diagnostic build
(`cargo lpxdiag`, src/sys/diag.rs). Read-only: it only sends read requests.

  lpx_dump.py info
  lpx_dump.py dump <region> <out.bin> [--offset N] [--length N]
  lpx_dump.py all <outdir> [--compare fw.bin]

Regions: 0 internal flash (incl. bootloader), 1 external SPI flash, 2 OTP/UID,
3 option bytes, 4 ST system ROM. Uses ALSA rawmidi through ctypes, no extra packages.
Every chunk is verified by a checksum and retried; any mismatch that survives
the retries stops the run (nothing is guessed or filled in).
"""
import argparse, ctypes, os, re, subprocess, sys, time

PREFIX = bytes([0xF0, 0x7D, 0x4C, 0x50, 0x58, 0x44])
CHUNK = 56
REGIONS = {0: "internal", 1: "external", 2: "otp", 3: "option-bytes", 4: "system-rom"}
SIZES = {2: 0x230, 3: 16, 4: 0x7800}
# STM32F401xE sector map: (start, size)
F401_SECTORS = [(i * 0x4000, 0x4000) for i in range(4)] + [(0x10000, 0x10000)] + [(0x20000 + i * 0x20000, 0x20000) for i in range(3)]


def find_port():
    out = subprocess.run(["amidi", "-l"], capture_output=True, text=True, check=True).stdout
    for line in out.splitlines()[1:]:
        m = re.match(r"\S+\s+(hw:\S+)\s+(.*)", line)
        if m and "Launchpad X" in m.group(2) and "MIDI" in m.group(2) and "DAW" not in m.group(2) and "Bootloader" not in m.group(2):
            return m.group(1)
    sys.exit("no Launchpad X MIDI port found (is the diagnostic build running?)")


class Port:
    def __init__(self, name):
        self.lib = ctypes.CDLL("libasound.so.2")
        self.inp, self.out = ctypes.c_void_p(), ctypes.c_void_p()
        rc = self.lib.snd_rawmidi_open(ctypes.byref(self.inp), ctypes.byref(self.out), name.encode(), 2)
        if rc < 0:
            sys.exit(f"cannot open {name} (busy? close other MIDI programs): error {rc}")

    def drain_input(self):
        buf = ctypes.create_string_buffer(256)
        while self.lib.snd_rawmidi_read(self.inp, buf, 256) > 0:
            pass

    def exchange(self, request, timeout=1.0):
        self.drain_input()
        data = bytes(request)
        written = self.lib.snd_rawmidi_write(self.out, data, len(data))
        if written != len(data):
            raise IOError(f"short write {written}")
        self.lib.snd_rawmidi_drain(self.out)
        buf = ctypes.create_string_buffer(256)
        got = b""
        end = time.time() + timeout
        while time.time() < end:
            n = self.lib.snd_rawmidi_read(self.inp, buf, 256)
            if n > 0:
                got += buf.raw[:n]
                start = got.find(PREFIX)
                if start >= 0 and got.find(b"\xf7", start) >= 0:
                    return got[start:got.find(b"\xf7", start) + 1]
            else:
                time.sleep(0.0005)
        raise TimeoutError("no reply")


def unpack(data):
    out = bytearray()
    for i in range(0, len(data), 8):
        group = data[i:i + 8]
        msbs, rest = group[0], group[1:]
        for j, b in enumerate(rest):
            out.append(b | (((msbs >> j) & 1) << 7))
    return bytes(out)


def request_read(port, region, offset, length, retries=6):
    req = PREFIX + bytes([0x02, region]) + bytes([(offset >> s) & 0x7F for s in (21, 14, 7, 0)]) + bytes([length, 0xF7])
    last = None
    for attempt in range(retries):
        try:
            r = port.exchange(req)
        except (TimeoutError, IOError) as e:
            last = e
            continue
        body = r[len(PREFIX):-1]
        if body[0] == 0x7F:
            sys.exit(f"device error {body[1]} for region {region} offset {offset:#x} length {length}")
        if body[0] != 0x42 or body[1] != region:
            last = ValueError("unexpected reply")
            continue
        off = (body[2] << 21) | (body[3] << 14) | (body[4] << 7) | body[5]
        if off != offset or body[6] != length:
            last = ValueError("reply for another request")
            continue
        raw = unpack(body[7:-2])[:length]
        csum = ((body[-2] << 7) | body[-1])
        if len(raw) != length or csum != (sum(raw) & 0x3FFF):
            last = ValueError("checksum mismatch")
            continue
        return raw
    sys.exit(f"giving up at region {region} offset {offset:#x}: {last}")


def info(port):
    r = port.exchange(PREFIX + bytes([0x01, 0xF7]))
    body = r[len(PREFIX):-1]
    if body[0] != 0x41:
        sys.exit(f"unexpected reply {body.hex()}")
    raw = unpack(body[1:])
    kb = raw[0] | (raw[1] << 8)
    print(f"internal flash size register : {kb} KB")
    print(f"unique id                    : {raw[2:14].hex()}")
    print(f"option bytes (16)            : {raw[14:30].hex()}")
    if raw[30]:
        print(f"external flash JEDEC id      : {raw[31]:02x} {raw[32]:02x} {raw[33]:02x}")
    else:
        print("external flash               : no answer")
    print(f"external size used by firmware: {int.from_bytes(raw[34:38], 'little')} bytes")
    return kb * 1024, int.from_bytes(raw[34:38], "little")


def dump(port, region, offset, length, label):
    data = bytearray()
    start = time.time()
    while len(data) < length:
        n = min(CHUNK, length - len(data))
        data += request_read(port, region, offset + len(data), n)
        if len(data) % 0x4000 == 0 or len(data) == length:
            rate = len(data) / max(time.time() - start, 0.001) / 1024
            print(f"\r{label}: {len(data):>8}/{length} bytes  {rate:6.1f} KB/s", end="", flush=True)
    print()
    return bytes(data)


def classify(block):
    if all(b == 0xFF for b in block):
        return "FF"
    if all(b == 0x00 for b in block):
        return "00"
    return "used"


def high_water(block):
    for i in range(len(block) - 1, -1, -1):
        if block[i] != 0xFF:
            return i + 1
    return 0


def internal_report(data):
    lines = ["Internal flash sectors (STM32F401, base 0x08000000):"]
    for start, size in F401_SECTORS:
        if start >= len(data):
            continue
        block = data[start:start + size]
        kind = classify(block)
        extra = ""
        if kind == "used":
            hw = high_water(block)
            extra = f"  last non-FF byte at +{hw:#x} ({size - hw} bytes of FF at the end)"
        lines.append(f"  {0x08000000 + start:#010x} +{size // 1024:>3} KB  {kind}{extra}")
    return lines


def external_report(data):
    sector = 4096
    runs, cur = [], None
    for s in range(0, len(data), sector):
        kind = classify(data[s:s + sector])
        if cur and cur[2] == kind:
            cur[1] = s + sector
        else:
            cur = [s, s + sector, kind]
            runs.append(cur)
    lines = [f"External SPI flash ({len(data) // 1024} KB), 4 KB sectors, runs of equal kind:"]
    for a, b, kind in runs:
        lines.append(f"  {a:#08x}-{b - 1:#08x}  {(b - a) // 1024:>5} KB  {kind}")
    lines.append("  (kind: FF = erased, 00 = zeros, used = other data)")
    return lines


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--port")
    sub = ap.add_subparsers(dest="cmd", required=True)
    sub.add_parser("info")
    d = sub.add_parser("dump")
    d.add_argument("region", type=int)
    d.add_argument("out")
    d.add_argument("--offset", type=lambda x: int(x, 0), default=0)
    d.add_argument("--length", type=lambda x: int(x, 0))
    a = sub.add_parser("all")
    a.add_argument("outdir")
    a.add_argument("--compare", help="firmware .bin expected at 0x0800C000 in the internal dump")
    args = ap.parse_args()

    port = Port(args.port or find_port())
    internal_size, external_size = info(port)
    if args.cmd == "info":
        return
    if args.cmd == "dump":
        sizes = {0: internal_size, 1: external_size, **SIZES}
        length = args.length if args.length is not None else sizes[args.region] - args.offset
        open(args.out, "wb").write(dump(port, args.region, args.offset, length, REGIONS[args.region]))
        return

    os.makedirs(args.outdir, exist_ok=True)
    report = []
    data = {}
    for region in (3, 2, 4, 0, 1):
        size = {0: internal_size, 1: external_size, **SIZES}[region]
        data[region] = dump(port, region, 0, size, REGIONS[region])
        open(os.path.join(args.outdir, f"{region}-{REGIONS[region]}.bin"), "wb").write(data[region])
    if args.compare:
        fw = open(args.compare, "rb").read()
        got = data[0][0xC000:0xC000 + len(fw)]
        verdict = "IDENTICAL" if got == fw else "DIFFERENT"
        report.append(f"Internal flash at 0x0800C000 vs {args.compare} ({len(fw)} bytes): {verdict}")
    report += internal_report(data[0]) + [""] + external_report(data[1])
    ob = data[3]
    report += ["", f"Option bytes: {ob.hex()}  (byte 1 bits 15:8 of OPTCR = RDP level; 0xAA = level 0)"]
    text = "\n".join(report)
    open(os.path.join(args.outdir, "map.txt"), "w").write(text + "\n")
    print(text)


main()
