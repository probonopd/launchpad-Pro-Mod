---
name: brick-safe-flash-storage
description: Use when adding saving (projects, presets, settings) to embedded firmware that writes a NOR flash chip, especially on a device whose bootloader or identity data must never be damaged (Launchpad X CoreFW, but any SPI NOR). Covers the fixed-window two-bank store with header-last commit (safestore crate), the three layers of bounds checks, the power-cut test method, the verification-by-dump procedure with before/after diff, and the pitfalls hit (page wrap, unbounded busy wait, stack overflow when a buffer is added, verifier bugs).
---

# Brick-safe saving on a NOR flash chip

Verified on one Launchpad X only; never run on other models (docs/tested-hardware.md).

Worked out and verified on a real Launchpad X (Oct 2026). Full write-up: `docs/storage-design.md`;
code: `safestore/`, `src/sys/driver/common/storage/external.rs`, `src/sys/store.rs`,
`tools/verify_store.py`; log: `docs/lpx-port-safety.md` sections 30-33.

## Principles (do not weaken)

1. **Measure before choosing a place.** Dump the whole external chip read-only (diag firmware +
   `tools/lpx_dump.py`) and map it: identity block, bootloader staging area, erased areas, other
   firmware's settings. Pick a window that is erased, far from those, and ends at a known boundary.
2. **One fixed window.** The storage layer only knows window-relative addresses. Compile-time asserts:
   sector aligned, above the staging area, ends exactly where the next owner starts.
3. **Only sector erase and page program.** No chip erase, no block erase, no status-register writes.
   Check the linked image has no such functions (`nm`), and no internal-flash unlock keys/controller refs.
4. **Two banks per item, header written last.** A save erases the bank that does NOT hold the newest valid
   copy, programs the payload page by page, then programs a 32-byte header (magic, version, item,
   length, generation, payload CRC32, header CRC32) in ONE program, then reads back and checks the CRC.
   A bank is valid only with a complete CRC-correct header. Every page is programmed exactly once
   (header in its own 256-byte page, payload from page 1).
5. **Generation counter with wrapping compare**; the newest valid bank wins; a corrupt newest bank falls
   back to the older one.
6. **Three independent layers of bounds checks**: the store crate (window and bank), the chip driver
   (adds the offset itself; refuses out-of-window, page-crossing programs, and any chip whose JEDEC id is
   not the expected one), compile-time asserts.
7. **Fail hard, no fallbacks**: unreadable data is "absent", failed saves are reported (red pad) or dropped,
   never retried in a loop, never guessed.

## Test method (host, no hardware)

- Simulate the NOR chip: erase sets FF; program only clears bits and ASSERTS if a byte is not erased where
  a 1 bit would be needed; assert no program crosses a page; assert nothing outside the window.
- **Cut the power at every single step** of a save (every erase and every programmed byte; an interrupted
  erase leaves garbage, an interrupted byte clears only some bits), reboot, load: require either the old or
  the new copy complete and correct, and that a further save works. Do it for a small record, a full-size
  record, and the very first save of an item.
- Also test: bank alternation order, "no page programmed twice", corrupt newest copy -> older copy,
  generation wrap, a chip that accepts a write but does not keep it (read-back must report Verify error).

## Verifying on the device (do this for every new kind of write)

1. Flash the diag (read-only dump) build, dump the whole chip = baseline.
2. Flash the normal build, do one save; success is only the firmware's own read-back (white flash).
3. Flash the diag build again, dump again, then `tools/verify_store.py <dump>` (items valid, inner CRCs)
   and `tools/verify_store.py --diff baseline after`: only the store window, the app's old settings window
   and the bootloader's staging area may differ; identity block and the rest must be byte-identical.
4. Power-cycle and load the data back on the device. The flash of a firmware does not erase the window
   (only the staging area), so saved data survives updates.

## Pitfalls found (all real)

- `spi-memory` `write_bytes` splits every 256 bytes from the start address and a page program WRAPS inside
  its page: split writes at page boundaries yourself. `erase_sectors(addr, n)` steps by 256 bytes, so only
  call it with n = 1. `wait_done` has no timeout (a dead chip hangs the firmware; gate on the JEDEC id).
- **Adding a buffer can make the firmware not boot.** A 7 KB project buffer inside a struct that lives in the
  app host overflowed the startup stack twice (pads dark, USB never enumerates, `error -110`; the bootloader
  recovers it). Keep big buffers on the stack only while saving; build big statics in `#[inline(never)]`
  `init_*` functions with `StaticCell::init_with`; build big arrays in place (`[const { X }; N]`). The release
  check `tools/verify_lpx.py` computes the worst-case stack depth from the disassembly call graph (startup and
  below every app handler) and refuses a build without a 4 KB margin. Run `cargo lpx` right before it.
- Verifier bugs cost time: my first verify_store.py read the header CRC from bytes 16-19 instead of 20-23 and
  called a good record corrupt; test a verifier against a known-good record before believing a failure.
- Do not save preferences while notes sound or a settings page is open (saving blocks for tens to hundreds
  of ms); mark dirty and save when idle. Preferences must be imported only if every field is valid.

## Adapting to another device

Change the window constants, the expected chip id and the item table in `safestore/src/lib.rs`
(WINDOW_SECTORS, item layout) and the driver; re-run the power-cut tests; repeat the dump-before/after
verification. Keep the window outside anything the bootloader owns.
