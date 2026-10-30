# Documentation

**New here? Start with the [User Manual](user-manual.md).** It explains everything the
firmware does and how to operate it, with pictures, for people who just want to use
their Launchpad X.

> **Tested on the Launchpad X only.** Everything here was developed and tested on a
> Launchpad X. Other models compile but were never run by us. Details:
> [tested-hardware.md](tested-hardware.md).

## For users

| Document | What it is |
| --- | --- |
| [user-manual.md](user-manual.md) | The complete manual: quick start, all modes, Setup, MIDI, installing, going back, troubleshooting |
| [launchpad-x-modes.md](launchpad-x-modes.md) | How the modes of the Launchpad Pro Mk3 guide map to the Launchpad X buttons, and why |
| [note-mode-launchpad-x.md](note-mode-launchpad-x.md) | Note Mode in detail: layouts, Chord, settings, design decisions |
| [custom-modes-launchpad-x.md](custom-modes-launchpad-x.md) | The eight Custom Modes in detail |
| [seq95-launchpad-x.md](seq95-launchpad-x.md) | The step sequencer in detail |
| [../flash/README.md](../flash/README.md) | How to upload a firmware file with the flashing scripts |
| [tested-hardware.md](tested-hardware.md) | Exactly which hardware was tested (one Launchpad X), which models only compile, what is still untested |
| [storage-design.md](storage-design.md) | What is saved, when, and why saving cannot brick the Launchpad |
| [feature-comparison.md](feature-comparison.md) | What the Novation Launchpad Pro [MK3] guide describes that we do not have yet, and a prioritized list |
| [novation-sequencer.md](novation-sequencer.md) | The Novation-style sequencer option for the Launchpad X: controls and status |
| [seq95.md](seq95.md) | The same sequencer on the Launchpad Pro Mk3 (original description) |

## For people who work on the firmware

| Document | What it is |
| --- | --- |
| [launchpad-x-technical-reference.md](launchpad-x-technical-reference.md) | What we measured on a real Launchpad X: chips, flash maps, how updates work, identity messages |
| [lpx-port-safety.md](lpx-port-safety.md) | The chronological log: every gate, every flash, every result and open risk |
| [flash-diagnostic.md](flash-diagnostic.md) | The read-only memory dump tool |
| [custom-mode-format-notes.md](custom-mode-format-notes.md) | What is known about Novation Components Custom Mode data, and what is missing |
| [design-loadable-modes.md](design-loadable-modes.md) | A design idea (not built): modes loaded from the external flash |

## The pictures

The pictures in `img/` are drawn from the real code, so they show what the firmware shows:

```
cd tools/framedump && cargo run --quiet | python3 -I ../make_pictures.py ../../docs/img
```

## License

This repository, its documentation and its pictures are licensed under the GNU General
Public License, version 3 only (GPL-3.0-only), the same license as the original CoreFW
firmware it is based on. See the `LICENSE` file. Every source file carries an
`SPDX-License-Identifier: GPL-3.0-only` line. No code from other projects was copied into the
files added here; the information about Novation Components Custom Modes in
`custom-mode-format-notes.md` describes a data format only, learned from a third-party tool
whose code was not used.
