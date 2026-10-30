# Custom Modes on the Launchpad X

The Launchpad X can turn its 8x8 grid into a control surface: every pad sends a
note, a control change (CC) or a program change, and a row or column of pads can
be a fader. This follows the Custom Modes of the Launchpad Pro [MK3] (section
"Using Launchpad Pro [MK3]'s Custom Modes" of Novation's interface guide).

> **Status: experimental.** The eight built-in modes and the engine are covered by 12
> tests on a computer; the firmware has been flashed but the modes have not been
> confirmed one by one on the device. For a plain-language introduction see the
> [User Manual](user-manual.md). Loading Custom Modes
> that were made in Novation Components is **not implemented yet**, see
> [Modes made in Novation Components](#modes-made-in-novation-components).

## Using it

1. Press **Custom** (top row). The grid shows the last used Custom Mode
   (Custom 1 at first). Press **Custom** again to leave.
2. The **eight buttons in the right-hand column** choose the mode: the top button
   is Custom 1, the bottom one Custom 8. The chosen mode's button is lit pale
   green, the others dim white. (On the Pro Mk3 these are the Track Select
   buttons under the grid; the X has none, so the scene buttons take their place.)
3. **Hold Capture MIDI** to see the master MIDI channel of the current mode in
   the two bottom rows of pads: channels 1 to 8 in the bottom row, 9 to 16 above.
   The current channel is green, the others dim red. Tap a pad to change it, then
   let go of Capture MIDI. Every mode has its own channel (all start on channel 1).

Playing is off while Capture MIDI is held. Notes that are held when you change
mode, hold Capture MIDI or leave are ended. Toggle states, fader positions and
channels are kept in RAM and reset when the Launchpad is unplugged. The **chosen mode
and the master channel of each mode are saved** (a moment after you change them) and
come back after you unplug the Launchpad; see [storage-design.md](storage-design.md).

All messages go out on the Launchpad's **MIDI** port.

## The eight built-in modes

| Mode | What the pads do |
| --- | --- |
| **Custom 1** | Eight unipolar **vertical faders**, one per column, CC 7 to 14 (left to right). Tap a pad to set the level; the pad row at that height and everything below it lights |
| **Custom 2** | Eight bipolar **horizontal faders**, one per row, CC 15 to 22 (the top row is CC 15). The bar is drawn from the middle: right of the middle for high values, left of it for low values. A fader starts in the middle |
| **Custom 3** | **Drum rack** layout, the same notes as the drum layout of Note Mode (bottom left = note 36, four 4x4 blocks). Velocity follows the press, aftertouch is sent as set up in Setup |
| **Custom 4** | **Chromatic keyboard**: pairs of rows form a piano, white keys (C to B and the next C) in the lower row, black keys in the row above, four octaves from C at note 36 |
| **Custom 5** | **Program changes 0 to 63**, one per pad, bottom left = 0 |
| **Custom 6** | **Program changes 64 to 127** |
| **Custom 7** | The drum rack layout of Custom 3, but **unlit**: incoming MIDI notes light the pads, the velocity of the note chooses the color (Novation palette) |
| **Custom 8** | The **Programmer-mode layout** (note = row x 10 + column, bottom left = 11), unlit and lit by incoming notes like Custom 7 |

Faders: the eight positions of a fader send the CC values 0, 18, 36, 54, 72, 90,
108 and 127 (position x 127 / 7); a bipolar fader therefore has no exact middle
value, it sends 54 or 72 next to the middle.

In Custom 7 and 8 notes must arrive on the mode's master channel (channel 1 at
first) to light the pads.

## Differences from the Launchpad Pro Mk3

| Pro Mk3 | Launchpad X | Why |
| --- | --- | --- |
| Track Select buttons choose the mode | The right-hand button column | The X has no Track Select buttons |
| Shift + Custom shows the master channels, with the bottom two rows for the current mode and dim red rows for the others | Hold Capture MIDI; only the current mode's channel is shown | Capture MIDI is the Shift button of this firmware. Showing the other modes' channels would need pads that are not free on the 8x8 grid |
| Components can edit the modes | The eight built-in modes only for now | See below |
| Custom 1 to 8 are remembered | The chosen mode and the master channels are saved; toggle states and fader positions are not | These small preferences use the safe store; storing whole modes would need the Components format |

## Modes made in Novation Components

Not implemented. What is known and what is missing is in
[custom-mode-format-notes.md](custom-mode-format-notes.md). The engine itself is
ready for it: a mode is one data structure (`ModeDef` in the `custommode` crate),
the built-in modes are just generated instances of it, and a loaded mode would
use the same code to play, light and send.

## Design decisions

1. **Same eight modes as the guide, in the same order**, so the behavior matches
   what Pro Mk3 users know.
2. **One data structure for all modes.** Built-in and, later, Components modes
   use the same engine, so the faders, toggles, colors and lighting are written
   and tested once.
3. **The right-hand column picks the mode**, top to bottom. It is the only free
   column of eight buttons next to the pads, and its arrows suggest moving
   through a list.
4. **Capture MIDI shows the master channel.** It is the Shift button of this
   firmware (the sequencer uses it the same way), so one modifier keeps its
   meaning everywhere.
5. **Colors are RGB for the built-in modes** and the Novation palette only where
   the color comes from a note velocity (Custom 7 and 8), as on the Pro.
