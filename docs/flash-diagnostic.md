# Read-only flash diagnostic (Launchpad X)

Purpose: find out, without writing anything, which parts of the two flash
memories are erased (all FF) or unused, so that a place for saving projects and
settings can later be chosen **safely**. Nothing in this tool writes or erases.

It is a separate firmware build, not part of normal builds:

```
cargo lpxdiag      # writes build/lpx-diag/fw.syx and build/core-launchpad-x-diag.syx
```

## What it reads

| Region | What | Size |
| --- | --- | --- |
| 0 | Internal flash of the STM32F401RE, `0x08000000` upward, **including the Novation bootloader** | 512 KB (from the chip's size register, capped) |
| 1 | External SPI flash (settings and other data) | 1 MB (the size the firmware assumes) |
| 2 | OTP area and unique ID | 0x230 |
| 3 | Option bytes (read-out protection and write-protection state) | 16 |
| 4 | ST's factory ROM bootloader | 0x7800 |

A dump of region 0 also gives a **backup copy of the Novation bootloader**,
which exists nowhere else (see section 18 of the safety notes).

## How it works

The Launchpad answers SysEx requests on its **MIDI** port (not the DAW port).
Message prefix: `F0 7D 4C 50 58 44` (manufacturer id 0x7D, then "LPXD").

- `01` info: flash sizes, unique id, option bytes, JEDEC id of the SPI flash chip.
- `02 <region> <offset: 4 x 7 bits> <length 1..56>` read.
- Replies `41` (info), `42` (data, 7 bytes packed into 8 SysEx bytes, followed by
  a checksum), `7F <code>` (error: 1 bad request, 2 bad region, 3 out of range,
  4 bad length, 5 read failed).

Safety properties, all in `src/sys/diag.rs`:
- Internal memory is read with plain loads from fixed ranges that are checked
  before every read; an out-of-range request is answered with an error.
- The external flash is read with the read-only driver path. A failed read is
  reported as an error, never as substituted `FF` data.
- No code path calls any write, erase or unlock function. The built image
  contains no flash unlock keys and no flash controller address (checked, see the
  safety notes), so it also cannot program the internal flash by accident.
- The host side retries a chunk on timeout or checksum mismatch and stops the run
  if it cannot get a clean chunk.

## Using it

1. Build and check: `cargo lpxdiag`, then
   `python3 tools/verify_lpx.py target/thumbv7em-none-eabihf/release/core build/lpx-diag/fw.bin build/lpx-diag/fw.syx tools/syxtool.py`
2. Flash `build/lpx-diag/fw.syx` through the bootloader as for any other build
   (`amidi -p <bootloader port> -i 10 -s fw.syx`), after the usual hash and
   bootloader-identity check.
3. Close other MIDI programs, then:

```
python3 tools/lpx_dump.py info
python3 tools/lpx_dump.py all dumps/ --compare build/lpx-diag/fw.bin
```

`all` writes the raw dumps (`0-internal.bin`, `1-external.bin`, ...) and
`dumps/map.txt`: a sector map of the internal flash (erased, zeros, or used up to
which byte) and a run-length map of the external flash in 4 KB sectors. With
`--compare`, the application region of the dump is compared to the firmware file:
a mismatch means the dump path is wrong and nothing from it may be trusted.

Expect about 1.5 MB of data. At 56 bytes per request this is roughly 28000
requests; the progress line shows the speed.

4. Flash the normal firmware (`cargo lpx`) again when done. The diagnostic build
   has no other function (it contains the same modes as the normal build).

## Checking saved data

`python3 tools/verify_store.py <1-external.bin>` lists the items saved in the store window
of a dump of region 1 and checks their CRCs; `python3 tools/verify_store.py --diff before
after` checks that two dumps differ only where they may (store window, CoreFW settings and
the update staging area). See [storage-design.md](storage-design.md).

## Host test

`sh tools/diag_host_test/run.sh` compiles the real `diag.rs` against stubs on the
computer, asks it for chunks and checks the replies with the parser of
`tools/lpx_dump.py` (packing, checksum, range and error handling). The first run
of this test found a wrong request-length check that would have made the device
answer every read with an error, before the firmware ever reached a device.
