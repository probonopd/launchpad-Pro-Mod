# Note Mode on the Launchpad X

Note Mode turns the 8x8 grid into a playing surface for melodic and drum
instruments. It follows the Note Mode of the Launchpad Pro Mk3 (see Novation's
"Launchpad Pro [MK3] interface" guide, section "Using Launchpad Pro [MK3]'s Note
mode") and adapts it to the Launchpad X, which has far fewer buttons.

> **Status: experimental.** The layouts, Chord and the settings page are covered by
> 19 tests on a computer. The firmware has been flashed to a real Launchpad X and
> its pads were seen sending notes; the layouts themselves have not been checked
> one by one on the device. For a plain-language introduction see the
> [User Manual](user-manual.md).

- Four layouts: **Chromatic**, **Scale**, **Drum** and **Chord** (Chord Mode of the Pro Mk3)
- 16 scales and any of the 12 root notes, shown on the pads
- Five overlap settings (sequential, 2, 3, 4, 5 pads) that decide how the rows
  relate
- Octave and semitone shifts from the arrow buttons
- Velocity from how hard you press, and aftertouch, using the same velocity and
  aftertouch settings as the Performance mode
- MIDI channel 1 to 16

## Entering and leaving

1. **After the boot animation the Launchpad starts in Note Mode**, with the pads
   lit. (Earlier builds started in the plain Performance mode with dark pads.)
2. To come back to it from another mode, press **Note**: this works from the plain
   Performance mode, the Custom Modes and the Session (DAW) mode. In the
   sequencer the button is Play; press **Custom** there first.
   The Note button lights green and the pads show the layout.
3. Press **Note** again to open the **settings page**. Press it again to return
   to playing.

To leave Note Mode, hold **Session** to open Setup and choose another mode, or
press **Custom** to open the Custom Modes, or hold **Capture MIDI** and press
**Custom** to open the sequencer (press **Custom** again to come back to Note Mode).

The Note button does not start Note Mode from Programmer mode: that mode belongs to
the software that controls the Launchpad, which needs every button. Leave it
through Setup (hold **Session**).

## Playing

| Control | Chromatic and Scale layouts | Drum layout |
| --- | --- | --- |
| **Pads** | Play notes. Velocity follows how hard you press | Play drum slots |
| **Up / Down** | Octave up / down (range -3 to +5 octaves) | Scroll the visible drum area by 16 slots |
| **Left / Right** | Transpose the whole grid by one semitone down / up (range -11 to +11) | Scroll by 4 slots |
| **Note** | Open or close the settings page | same |
| **Session** | Hold: open Setup | same |
| **Custom** | Open the Custom Modes (with Capture MIDI held: the sequencer) | same |
| **Capture MIDI** | Not used in Note Mode | same |
| **Right column** (8 scene buttons) | Not used in Note Mode | same |

Notes stop sounding when you release the pad, even if you changed the octave
or the transposition while holding it. If two pads play the same note (as in
the overlapped layouts), the note ends when both are released.

Aftertouch goes out as polyphonic or channel aftertouch, or not at all, as set in
Setup. Opening the settings page or leaving Note Mode ends all sounding notes.

### Pad colors

| Color | Meaning |
| --- | --- |
| Blue | A note of the current scale |
| Purple | The root of the scale |
| White | A pad being pressed |
| Unlit | Chromatic layout: a note outside the scale. Any layout: no note at that place (beyond MIDI notes 0 to 127) |
| Yellow (dim) | Drum layout: a drum slot |
| Orange | Chord layout: a triad pad |

The four arrow buttons are lit dim white so you can see they are active.

## The layouts

**Chromatic** (the default). Every pad is a semitone. With the default overlap
of 5, each pad up a row adds five semitones, so the layout resembles a guitar:
an octave is two pads up and two pads across, and the first pad of a row plays
the same note as the sixth pad of the row below. The bottom-left pad is note 36
(C) with no shifts. The scale is shown by color; notes outside it are unlit but
still play.

**Scale.** Only notes of the current scale exist, so you cannot play out of key.
The bottom-left pad is the root. Each pad up a row adds *overlap* scale steps.
When the scale has fewer than 8 notes the octave is reached inside a row.

**Sequential overlap.** In the chromatic layout every pad is a unique note
(each row starts 8 semitones above the one below). In the scale layout each row
starts one octave above the one below, so only octaves of the root are shared.

**Drum.** Four 4x4 blocks, like a drum rack: the bottom-left block is the
first 16 slots (default notes 36 to 51, General MIDI kick and friends), the
top-left block the next 16, the bottom-right the next 16 and the top-right
the 16 after that. Up / Down scroll by whole blocks, Left / Right by 4 slots.
The pads are all lit yellow, because the Launchpad X cannot see which slots
of a drum rack are filled (see [Differences from the Pro Mk3](#differences-from-the-launchpad-pro-mk3)).

**Chord.** The Chord Mode of the Pro Mk3, as the user guide describes it. The
Launchpad X has no Chord button, so the layout is chosen on the settings page.

```
 col:   1        2     3     4     5     6        7        8
 row 8  triad    note area: scale degree 8        bank     bank
 ...    (orange) root 3rd+8 5th   7th   5th+8    bank     bank
 row 2  triad    ...                              bank     bank
 row 1  triad    root 3rd+8 5th   7th   5th+8    SUSTAIN  LOCK
```

- **Column 1, triads (orange):** one pad per scale degree plays the triad built
  on it, folded into one octave so the voicing stays close.
- **Columns 2 to 6, note area:** a row belongs to one scale degree and holds, from
  left to right, its root, its third (one octave up), its fifth, its seventh and
  its fifth (one octave up). The first column runs through the scale from the
  bottom up (for scales with fewer than seven notes the octave comes earlier).
  Any three adjacent pads of a row form a triad. The root of the scale is purple,
  other scale notes blue.
- **Columns 7 and 8, rows 2 to 8, chord bank:** 14 slots. Empty slots are dim
  white, filled slots bright white. **To fill a slot:** hold the empty slot and
  press notes or triads, or hold notes first and then press the empty slot.
  Release the slot to finish. **To play:** press a filled slot.
- **Lock (bottom right):** while locked (bright red) the bank cannot be changed,
  so you can hold chords from the bank with one finger and play notes on top.
- **Sustain (bottom row, column 7):** toggles a sustain latch (bright green when
  on). Released notes keep sounding until sustain is switched off; pressing a
  sustained note again plays it again with the new velocity.
- The chord bank is saved (see [Settings page](#settings-page)); the lock and the sustain
  are switched off again after unplugging. Scale, root, MIDI channel and the octave and semitone shifts are
  shared with the other layouts.

The guide does not spell out the exact order of the five pads of a note-area row;
the order above is our reading of its text and was not compared with a real
Launchpad Pro.

## Settings page

Press **Note** while in Note Mode. The Note button stays lit green and the
grid shows (row 8 is the top row, row 1 the bottom row):

```
 row 8:  [Seq][ 2 ][ 3 ][ 4 ][ 5 ]  .   .   .              overlap
 row 7:   .   C#   D#    .   F#   G#   A#    .             root note (black keys)
 row 6:   C   D    E     F   G    A    B     .             root note (white keys)
 row 5:   scales 1 to 8      Natural Minor ... Bebop Dorian
 row 4:   scales 9 to 16     Blues ... Hirajoshi
 row 3:   MIDI channels 1 to 8
 row 2:   MIDI channels 9 to 16
 row 1:  [Chrom][Scale][Drum][Chord] . . . [Reset]      layout | back to octave 0, no shift
```

- **Overlap** (row 8, left): Seq = sequential, 2 to 5 = pads per row. Selected is
  white, the others dim blue.
- **Layout** (row 1, left): Chromatic (selected: white), Scale (selected: green),
  Drum (selected: yellow), Chord (selected: orange). Unselected layouts are dim red.
- **Root note** (rows 7 and 6): a piano. The pad of the current root is purple,
  other notes of the current scale are blue, notes outside it dim white. Tap a
  pad to change the root.
- **Scale** (rows 5 and 4): 1 Natural Minor, 2 Natural Major, 3 Dorian,
  4 Phrygian, 5 Mixolydian, 6 Melodic Minor, 7 Harmonic Minor, 8 Bebop Dorian,
  9 Blues, 10 Minor Pentatonic, 11 Hungarian Minor, 12 Ukrainian Dorian,
  13 Marva, 14 Todi, 15 Whole Tone, 16 Hirajoshi. The selected scale is white,
  the others dim blue.
- **MIDI channel** (rows 3 and 2): the channel Note Mode transmits on. Defaults to 1.
- **Reset** (row 1, right): sets octave and transposition back to zero and the
  drum area back to the first block.

The defaults are the same as on the Pro Mk3: chromatic layout, overlap 5, C minor.

The layout, overlap, scale, root note, MIDI channel and the chord bank are **saved** a
moment after you close the settings page (or finish editing the chord bank) and are
restored after unplugging. The octave and semitone shifts start at zero again. The
save waits until no note sounds; it is described in [storage-design.md](storage-design.md).

## MIDI output

Note Mode sends Note On and Note Off on the Launchpad's **MIDI** port (the same
port as Performance mode), on the channel set in the settings page. Connect that
port to a synthesizer or DAW and set the instrument to receive on that channel.

## Differences from the Launchpad Pro Mk3

| Pro Mk3 | Launchpad X | Why |
| --- | --- | --- |
| Hold Shift and press Note for the settings | Press Note again | The X has no Shift label, and using one button saves a finger. Capture MIDI (the sequencer's Shift) is left free |
| Separate Note and Chord buttons, sharing scale, root and channel | One Note button; Chord is a layout chosen on the settings page, sharing the same settings | The X has no Chord button |
| Drum layout follows the Ableton drum rack that is armed, and shows filled slots | Static drum layout, always yellow | The X firmware has no link to Ableton Live. The layout and note numbers are the same as the drum layout of Performance mode |
| Switches to Drum layout automatically when a drum rack is armed | You choose the layout | Same reason |
| Scale list and names from Novation | Same 16 names, same order | The semitone steps for each scale are standard ones; Novation's page lists only the names |
| Note Mode settings are remembered | Saved (layout, overlap, scale, root, channel, chord bank) | Same, using the power-fail-safe store |

## Design decisions and rationale

These are the choices made while adapting Note Mode to the Launchpad X, and why.

1. **Arrow buttons keep their printed meaning.** The Pro Mk3 documentation uses
   Up / Down for the octave and Left / Right for a semitone. The X has the same
   four printed arrows, so they work exactly as labelled. They are read by
   physical position, so the printed arrow always does what it says, even when
   the Launchpad is rotated in Setup.
2. **The Note button is the mode button and the settings button.** It is printed
   "Note", it starts Note Mode, and pressing it again opens settings. The Pro
   needs Shift for this, but the X's Shift is already Capture MIDI in the
   sequencer, and keeping the Note button self-contained avoids a second
   modifier convention in a mode where both hands are busy playing.
3. **Session and Custom keep their meaning.** Hold Session still opens Setup (the
   way to reach Performance and Programmer mode), a tap opens the Ableton Session
   mode, and Custom opens the Custom Modes, as printed. Changing what a printed
   button does between modes would be confusing.
4. **Capture MIDI and the right-hand column are unused.** Nothing in Note Mode
   needs them, and an unused button that does nothing is better than a hidden
   function. They remain free for later features (for example sustain).
5. **Everything that does not fit on the buttons moves onto the pads.** The Pro
   Mk3 settings page already uses only the grid (overlap, layout toggle, scale
   viewer, scale list, MIDI channel), which fits into 8 x 8 on the X. The
   arrangement was chosen so related things are on neighboring rows: layout and
   overlap in the top row, the piano for the root below it, then scales, then
   channels. The lowest row has a single reset pad.
6. **Drum layout as a third layout.** The Pro switches to it automatically from
   Ableton Live. The X cannot know what is armed, so it is a choice in the
   settings. It reuses the 4x4 block arrangement and starting note of
   Performance mode, so users know it.
7. **Note-off is paired with the pad that started the note.** The note is stored
   at press time and a counter handles two pads sharing a note, so changing the
   octave or the overlap while holding never leaves a stuck note.
8. **Note Mode is the startup mode and the Note button returns to it** from
   Performance, Custom and Session mode. Programmer mode hands every button to host
   software, and the sequencer needs the button as Play.
9. **The Note logic is a separate crate.** `notemode/` has no hardware
   dependency, so the layouts, the range checks and the settings page are tested
   on a computer (`cd notemode && cargo test`), like the sequencer.
10. **Space.** The application area above the bootloader is 80 KB and was nearly
    full. To make room the sequencer's project saving and loading (which could not
    be used on the X anyway) is no longer built into the X firmware, the Note and
    sequencer crates are built for size, and the whole X firmware is built with
    size optimization (`cargo lpx` only; about 27% smaller than the speed build,
    leaving about 23 KB free). See [lpx-port-safety.md](lpx-port-safety.md).
