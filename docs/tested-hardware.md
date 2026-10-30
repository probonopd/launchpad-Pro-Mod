# What has been tested, and on which hardware

**Everything described in this documentation was developed for and tested on the
Novation Launchpad X, and only on the Launchpad X.** This applies to the Launchpad X
modes (Note Mode, Chord, Custom Modes, Session tap), both sequencers (Launchpad95 style
and Novation style), saving to the external flash, the Setup pages and extras, and the
brick-safety analysis. Nothing of it has been run on any other Launchpad model.

## The unit we tested on

One Launchpad X, with:

- application USB id 1235:0103, bootloader 1235:1103 ("LPX BL M"), bootloader version
  0238, Device Inquiry family 03 11 in the bootloader and 03 01 in the application;
- 128 KB internal flash, 64 KB RAM, read protection level 0;
- external flash: Macronix, JEDEC id C2 20 14, 1 MB;
- original firmware 2.0.1 (stock 351) before CoreFW; flashed from Linux with `amidi`.

A Launchpad X with a different external flash chip, bootloader version or hardware
revision may behave differently. The store refuses to write unless the chip answers
C2 20 14 (see storage-design.md), so such a unit would not save, but we have not tried one.

## Other models

| Model | Builds? | Run on hardware by us? |
| --- | --- | --- |
| Launchpad X | yes | **yes** (this is what everything above refers to) |
| Launchpad Mini Mk3 | compiles (2026-10-30) | **no** |
| Launchpad Pro Mk3 | compiles (2026-10-30) | **no** |
| Launchpad Pro (Mk1) | compiles (2026-10-30) | **no** |
| Launchpad Mk2 | compiles (2026-10-30) | **no** |
| Launchpad S | compiles (2026-10-30) | **no** |
| Launchpad Mini Mk1 | compiles (2026-10-30) | **no** |

"Compiles" means only that `cargo <device>` finishes without errors or warnings after our
changes. It says nothing about whether the firmware runs, boots or is safe on that model.

Our changes to code that other models share are small and mostly switched on only for the
Launchpad X (`launchpad-x` feature): a default no-op method `set_blank` in the driver
trait, a `led::redraw()` function, and the host's reaction to Setup pages, Note and Custom
buttons, all behind `cfg` for the X. The Launchpad Pro Mk3 sequencer (Seq95, from the
upstream project drusanow/launchpad-Pro-Mod) is not ours and was not tested by us; our
Launchpad X port of it was changed to use a safe store, which the Pro Mk3 does not use.

**Do not flash a build for another model on the strength of this documentation.** Use the
upstream project's own releases and instructions for those models.

## What on the Launchpad X is still untested

- Chord layout of Note Mode, the Custom Modes, tapping Session for the DAW mode and the
  Setup velocity and aftertouch pages in use were only partly exercised; the sequencers,
  the saving and the Setup extras were confirmed by the user on the device.
- Session mode with Ableton Live (we had no Live).
- Loading Custom Modes made with Novation Components (the file format is not decoded).
- The setup preference record and the Novation-style project record were not checked with
  a flash dump (the sequencer project of the Launchpad95 style and the stock areas were).
- Long runs: Vegas mode and LED sleep were checked briefly, not over hours.

## Safety statement

The no-brick analysis (docs/lpx-port-safety.md, storage-design.md) is specific to the
Launchpad X's memory map and bootloader. It does not carry over to other models: their
bootloaders, flash sizes and external chips were not examined.
