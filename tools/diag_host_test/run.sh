#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-only
# Copyright (C) 2026 Simon Peter
# Host test of src/sys/diag.rs: compiles the real module against stubs (no
# device), asks it for chunks and checks the replies with the parser of
# tools/lpx_dump.py. Run from the repository root: sh tools/diag_host_test/run.sh
set -e
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
cp tools/diag_host_test/main.rs "$work/main.rs"
cp src/sys/diag.rs "$work/diag.rs"
(cd "$work" && rustc --edition 2024 -O main.rs -o dtest && ./dtest > out.txt)
python3 -I - "$work/out.txt" <<'PY'
import sys
src = open("tools/lpx_dump.py").read().replace("\nmain()\n", "\n")
ns = {}
exec(compile(src, "lpx_dump", "exec"), ns)
unpack, PREFIX = ns["unpack"], ns["PREFIX"]
lines = dict(l.split(" ", 1) for l in open(sys.argv[1]).read().splitlines())
expected = lambda off, n: bytes((((off + i) * 7 + 3) & 0xFF) ^ 0x80 for i in range(n))
for key, (off, n) in {"read": (0x1000, 56), "read_odd": (0xFFFC0, 13)}.items():
    r = bytes.fromhex(lines[key])
    assert r.startswith(PREFIX) and r[-1] == 0xF7 and len(r) <= 96, key
    assert all(b < 0x80 for b in r[1:-1]), "byte >= 0x80 inside the SysEx"
    body = r[len(PREFIX):-1]
    assert body[0] == 0x42 and body[1] == 1
    assert ((body[2] << 21) | (body[3] << 14) | (body[4] << 7) | body[5]) == off and body[6] == n
    raw = unpack(body[7:-2])[:n]
    assert raw == expected(off, n), key
    assert ((body[-2] << 7) | body[-1]) == (sum(raw) & 0x3FFF)
for key, code in {"fail": 5, "range": 3, "len0": 4, "len57": 4, "region9": 2, "garbage": 1}.items():
    assert bytes.fromhex(lines[key])[len(PREFIX):-1] == bytes([0x7F, code]), key
print("diag host test: PASS")
PY
