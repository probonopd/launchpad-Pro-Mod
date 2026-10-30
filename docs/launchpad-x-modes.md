# Modes of the Launchpad X firmware (and how they map to the Pro Mk3)

This firmware (CoreFW with the additions from this repository) gives the
Launchpad X the modes of Novation's "Launchpad Pro [MK3] interface" guide as far
as the X's buttons allow. The X has 16 buttons around the pads: eight in the top
row (Up, Down, Left, Right, **Session**, **Note**, **Custom**, **Capture MIDI**)
and a column of eight scene buttons on the right. It has no Chord, Sequencer,
Shift or Track Select buttons.

| Mode of the guide | On the Launchpad X | Document |
| --- | --- | --- |
| **Session** (Ableton Live) | **Tap Session.** This opens the existing DAW mode (clip launching, faders and the other Session functions of the CoreFW DAW support, driven by Ableton's control surface script). Hold Session for Setup | CoreFW's DAW mode |
| **Note** | The **startup mode**; **press Note** from Performance, Custom or Session mode to come back. Chromatic, Scale and Drum layouts; settings page by pressing Note again | [note-mode-launchpad-x.md](note-mode-launchpad-x.md) |
| **Chord** | A **layout of Note Mode**, chosen on the Note settings page (no Chord button) | [note-mode-launchpad-x.md](note-mode-launchpad-x.md) |
| **Custom** | **Press Custom.** The eight built-in Custom Modes, chosen with the right-hand column; master channel with Capture MIDI held. Modes from Components: not yet | [custom-modes-launchpad-x.md](custom-modes-launchpad-x.md) |
| **Sequencer** (Seq95) | **Hold Capture MIDI and press Custom** | [seq95-launchpad-x.md](seq95-launchpad-x.md) |
| Performance, Programmer | Choose them in **Setup** (hold Session) | CoreFW |

Capture MIDI is the Shift button of this firmware wherever a mode needs one.

## Decisions and why

1. **Printed labels keep their meaning:** Note opens Note Mode, Custom the Custom
   Modes, Session the Session mode (hold: Setup), the arrows move and shift.
2. **Modes without a button become layouts or chords.** Chord Mode is a Note Mode
   layout because there is no Chord button; the sequencer is a Capture MIDI +
   Custom chord because it is the one mode with no printed label on the X.
3. **The right-hand column replaces the Track Select buttons** where a mode needs
   to pick one of eight things (the Custom Modes).
4. **A short Session tap opens the Session mode, a long hold opens Setup.** Both
   are on the one button the X has for this; the hold keeps the pre-existing
   CoreFW behavior.
5. **Modes that depend on Ableton Live** (the drum layout following a drum rack,
   clip colors) cannot work without a link to Live in the firmware; they are static
   layouts or left to the DAW mode.
