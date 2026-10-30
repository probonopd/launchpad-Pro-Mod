#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
# Copyright (C) 2026 Simon Peter
"""Read-only check of the saved data in a dump of the Launchpad X's external flash.

  verify_store.py <1-external.bin>          list every saved item and whether it is valid
  verify_store.py --diff <before> <after>   check that two dumps differ ONLY where they may

The store window is 36 sectors of 4 KB ending at 0x0FE000 (see safestore/src/lib.rs).
Dumps come from the diagnostic firmware: `python3 tools/lpx_dump.py dump 1 file.bin`.
"""
import struct, sys, zlib

SECTOR, PAGE = 4096, 256
WINDOW_END = 0x0FE000
WINDOW_SECTORS = 38
WINDOW_START = WINDOW_END - WINDOW_SECTORS * SECTOR
MAGIC = 0x5358504C
PROJECT_SLOTS = 8
ITEMS = [("setup preferences", 10, 0, 1)] + [(f"project {i}", i, 2 + 4 * i, 2) for i in range(PROJECT_SLOTS)] + [
    ("note preferences", 8, 34, 1), ("custom preferences", 9, 36, 1)]


def crc32(data):
    return zlib.crc32(data) & 0xFFFFFFFF


def bank_info(dump, item_id, first_sector, sectors):
    off = WINDOW_START + first_sector * SECTOR
    raw = dump[off:off + 32]
    if len(raw) < 32 or all(b == 0xFF for b in raw):
        return "empty"
    # magic, version, item, length, generation, payload crc, 4 reserved bytes, header crc
    magic, ver, iid, length, gen, pcrc = struct.unpack_from("<IBBHII", raw)
    (hcrc,) = struct.unpack_from("<I", raw, 20)
    if magic != MAGIC or ver != 1 or iid != item_id:
        return "no valid header"
    if crc32(raw[:20]) != hcrc:
        return "header CRC wrong (torn write?)"
    if length > sectors * SECTOR - PAGE:
        return "length too big"
    payload = dump[off + PAGE:off + PAGE + length]
    if crc32(payload) != pcrc:
        return f"generation {gen}: PAYLOAD CRC WRONG"
    extra = ""
    if item_id < PROJECT_SLOTS:
        # A sequencer project carries its own magic and CRC (seq95/src/model.rs).
        if payload[:6] != b"SEQ95P":
            return f"generation {gen}: store record valid but not a sequencer project"
        inner = struct.unpack_from("<I", payload, 7)[0]
        if crc32(payload[11:]) != inner:
            return f"generation {gen}: sequencer project CRC WRONG"
        extra = ", sequencer project CRC ok"
    return f"valid, generation {gen}, {length} bytes{extra}"


def report(path):
    dump = open(path, "rb").read()
    if len(dump) != 1024 * 1024:
        sys.exit("expected a 1 MB dump of the external flash")
    print(f"store window: {WINDOW_START:#08x}-{WINDOW_END - 1:#08x} ({WINDOW_SECTORS} sectors)")
    bad = False
    for name, item_id, first, sectors in ITEMS:
        a = bank_info(dump, item_id, first, sectors)
        b = bank_info(dump, item_id, first + sectors, sectors)
        print(f"  {name:20s} bank A: {a:42s} bank B: {b}")
        bad |= "WRONG" in a or "WRONG" in b
    # Everything in the window that belongs to no bank content must be erased.
    erased_tail = all(b == 0xFF for b in dump[0xFF000:0x100000])
    print("  last settings sector (0xFF000) erased:", erased_tail)
    sys.exit(1 if bad else 0)


def diff(before_path, after_path):
    a, b = open(before_path, "rb").read(), open(after_path, "rb").read()
    if len(a) != len(b):
        sys.exit("dumps differ in size")
    # Where a change is expected or tolerated.
    allowed = [
        ("store window", WINDOW_START, WINDOW_END),
        ("CoreFW settings", 0x0FE000, 0x100000),
        ("update staging area (rewritten at every firmware update)", 0x003000, 0x040000),
    ]
    runs, i = [], 0
    while i < len(a):
        if a[i] != b[i]:
            j = i
            while j < len(a) and a[j] != b[j]:
                j += 1
            runs.append((i, j))
            i = j
        else:
            i += 1
    bad = False
    for s, e in runs:
        where = next((n for n, lo, hi in allowed if lo <= s and e <= hi), None)
        print(f"  changed {s:#08x}-{e - 1:#08x} ({e - s} bytes): {where or 'NOT ALLOWED'}")
        bad |= where is None
    if not runs:
        print("  no differences")
    print("RESULT:", "FAIL: changes outside the allowed areas" if bad else "ok: only allowed areas changed")
    sys.exit(1 if bad else 0)


if len(sys.argv) == 2:
    report(sys.argv[1])
elif len(sys.argv) == 4 and sys.argv[1] == "--diff":
    diff(sys.argv[2], sys.argv[3])
else:
    sys.exit(__doc__)
