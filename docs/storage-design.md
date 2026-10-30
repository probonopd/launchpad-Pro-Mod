# Saving data on the Launchpad X: design and safety

> This design and its safety argument are specific to the Launchpad X (one unit tested,
> external flash Macronix C2 20 14). They were not examined for other models; see
> [tested-hardware.md](tested-hardware.md).

The firmware saves three kinds of data in the Launchpad's external flash chip:

| What | When it is saved | Where |
| --- | --- | --- |
| **Sequencer projects**, 8 slots | when you tap a save pad in the Projects overlay (Shift + Probability) | `project 0` to `project 7` |
| **Note Mode preferences**: layout, overlap, scale, root note, MIDI channel, the **chord bank** | automatically, a moment after you close the Note settings page or finish editing the chord bank, and only when no note sounds | `note preferences` |
| **Custom Mode preferences**: the chosen mode and the master channel of each mode | automatically, after you choose a mode or change a channel (with Capture MIDI released and no pad held) | `custom preferences` |

Everything else (sequencer working state, toggles, fader positions, octave shifts,
sustain) is still forgotten when the Launchpad is unplugged. The Setup settings
(brightness, velocity, aftertouch, palette) were already saved by CoreFW in its own
area and are not affected.

The rest of this page is for people who want to know **why this cannot brick the
Launchpad** and how it was checked.

## What "brick-safe" means here

A brick is a Launchpad that can no longer be reflashed over USB. That can only
happen if the **bootloader** is damaged, or if the data the bootloader needs
(its identity block, the validity record of the application) is damaged. Saved data
is written to the **external flash chip**, a different chip on a different bus from
the one that holds the bootloader and the firmware. The data we write therefore cannot
touch the bootloader. What could still go wrong is writing into a part of the
external chip that the bootloader uses (its identity block or its staging area for
updates), or losing user data. The design prevents both with independent layers.

## Where the data lives

The external flash is 1 MB. This is the map we measured (see
[launchpad-x-technical-reference.md](launchpad-x-technical-reference.md)):

| Range | Content |
| --- | --- |
| `0x000000`-`0x001FFF` | identity block (serial number): **never touched** |
| `0x003000`-`0x03FFFF` | bootloader staging area for updates: **never touched** |
| `0x040000`-`0x0D9FFF` | erased, **not used by us** |
| **`0x0D8000`-`0x0FDFFF`** | **our store window: 38 sectors of 4 KB (152 KB)** |
| `0x0FE000`-`0x0FFFFF` | CoreFW's own settings |

The store window was completely erased on the unit we measured. It lies about 616 KB
above the end of the staging area and about 890 KB above the identity block.

Inside the window (sector numbers count from the window start):

| Sectors | Item |
| --- | --- |
| 0-1 | `setup preferences` (pad and aftertouch thresholds, MIDI clock, Vegas mode, LED sleep): two banks of one sector |
| 2-5, 6-9, ... 30-33 | `project 0` to `project 7`: **two banks of two sectors** each (8 KB per bank) |
| 34-35 | `note preferences`: two banks of one sector |
| 36-37 | `custom preferences`: two banks of one sector |

The window grew downward when the setup preferences were added (36 to 38 sectors), so
every older item kept its place in the chip; a host test checks this.

A project is 7,147 bytes, so it fits into one bank (capacity 7,936 bytes).

## The record format

Each bank begins with one 256-byte page for the **header** and then holds the data
in whole pages. The header (32 bytes) holds: a magic number, a version, the item
number, the length, a **generation** counter, the **CRC32 of the data** and the
CRC32 of the header itself.

## How a save works (and why a power cut cannot hurt)

1. Find the item's two banks. Find the **newest valid** bank (header correct,
   data CRC correct, highest generation).
2. Choose the **other** bank as the target. The newest good copy is never written
   to.
3. Erase the target bank's sectors (4 KB sector erase only).
4. Program the data, page by page.
5. Program the **header last**, in one go. Only now does the bank become valid.
6. **Read the bank back** and check the header and the CRC. If anything is wrong the
   save reports an error (the red flash on the pad) and the old copy is still the
   newest valid one.

If the power fails at **any** step, the target bank has no complete, CRC-correct
header, so it is treated as empty, and the previous copy is still there and valid.
The next save simply repeats the procedure. Two consecutive saves alternate between
the two banks, which also spreads the wear.

The header page is programmed exactly once and every other page is programmed
exactly once as well, so the chip is never asked to program a page twice.

**This is tested, not assumed.** The `safestore` crate has host tests that simulate the
flash chip (erase sets bytes to FF, programming can only clear bits, programming a
byte that is not erased is an error) and **cut the power at every single step** of a
save of a preference record and of a full-size project, also during the very first
save of an item. After every cut they reboot, load, and require that either the
old or the new copy is complete and correct, and that a further save works. They
also check that bank sectors are erased in the expected alternating order, that
nothing is programmed outside the item's bank and that no page is programmed twice.
Corrupt data (a flipped bit) is detected, the older copy is used instead, and a
chip that "accepts" a write but does not keep it is reported by the read-back.

## Three independent layers keep every write inside the window

1. **The `safestore` crate** only knows window-relative addresses. It checks every
   erase and program against the window and against the bank in use. There is no
   way to ask it for an address outside.
2. **The flash driver** (`src/sys/driver/common/storage/external.rs`) adds the
   window offset itself, refuses any offset, sector or length outside the window,
   refuses a program that would cross a page (a page program wraps inside its page),
   and refuses to touch the chip unless it answers with the expected chip id
   (Macronix `C2 20 14`; on another chip saving is simply not available).
   Compile-time checks require the window to be sector aligned, to end exactly where
   CoreFW's settings begin, and to stay above `0x080000`.
3. **The built firmware was checked.** Only the sector-erase, page-program and read
   operations of the chip driver are linked: there is no chip erase and no block erase
   in the image.

## Other safety facts

- Loading never changes anything. Preferences are applied only if every field is
  valid; anything else is ignored (the defaults stay).
- A failed save is reported (projects: red pad) or dropped (preferences); it is
  not retried in a loop.
- Saving takes a moment (erasing a sector takes tens of milliseconds; a project
  save takes longer, estimated at a few hundred milliseconds, not yet measured) during which the pads and clock stand still.
  Preferences are therefore saved only when nothing is held and the settings are
  closed, and projects only when you ask.
- The firmware cannot reach the internal flash that holds the bootloader; nothing in
  this feature changes that (see section 18 of [lpx-port-safety.md](lpx-port-safety.md)).
- The save feature adds about 3.3 KB to the firmware. The 7 KB buffer for encoding a project lives on the stack only while saving or loading; keeping it permanently in the sequencer once made the firmware run out of stack at startup (section 31 of [lpx-port-safety.md](lpx-port-safety.md)), and the release check now refuses any build whose startup stack does not fit.

## What is not guaranteed

- **A hang, not a brick.** The chip driver waits for the chip without a time limit.
  If the chip itself were broken, a save could hang the firmware. The Launchpad can
  then still be recovered through the bootloader (hold Capture MIDI while plugging in).
  The chip id check before every operation makes this very unlikely.
- **Novation's own firmware** might use part of the window if you go back to the
  original firmware and use features that store data there. We found the area empty
  on the unit we measured. The consequence would be lost saved data, not a brick.
- **Flash wear.** NOR flash survives roughly 100,000 erase cycles per sector. Each
  save erases the sectors of one bank, and the banks alternate.
- **No password, no encryption.** The data is stored as is.

## How the first real write was verified

Before the first save, a dump of the whole external chip was taken with the
read-only diagnostic firmware (`tools/lpx_dump.py`). After saving projects and
preferences, the chip was dumped again. `tools/verify_store.py --diff before after`
checks that the two dumps differ **only** in the store window, CoreFW's settings
and the update staging area (which the bootloader rewrites at every firmware update);
`tools/verify_store.py dump` lists every saved item and checks its CRC. Results
are in section 30 of [lpx-port-safety.md](lpx-port-safety.md).

## Source

| File | Role |
| --- | --- |
| `safestore/` | the format, the save and load logic and the power-cut tests |
| `src/sys/driver/common/storage/external.rs` | the bounds-checked window access of the chip driver |
| `src/sys/store.rs` | connects the two |
| `src/app/note/mod.rs`, `src/app/custom/mod.rs`, `src/app/seq95/mod.rs` | what is saved, and when |
| `tools/verify_store.py` | read-only checker for dumps |
