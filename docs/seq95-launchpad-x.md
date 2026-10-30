# Seq95 on the Launchpad X

Seq95 is a Launchpad95-style step sequencer that runs on the Launchpad itself.
It needs no computer software: it has its own clock and sends MIDI notes
straight out of the Launchpad's USB port. This page covers the Launchpad X
version. The general concepts are the same as in [seq95.md](seq95.md), which
describes the Launchpad Pro Mk3 version; this page lists what differs.

> **Status: experimental.** The sequencer logic and the button layout are covered by
> 28 unit tests on a computer. The firmware has been flashed to a real Launchpad X,
> but the sequencer itself has not been confirmed in use. Read
> [Installing](#installing-and-getting-back-to-stock) and the safety notes before
> flashing. For a plain-language introduction see the [User Manual](user-manual.md).

- 5 independent tracks, each drum or melodic, each on its own MIDI channel
- up to 64 steps per track, with a loop range and its own step rate
  (1/4 ... 1/32, including triplets)
- per-step note probability, velocity, gate length and octave
- live transposition (buttons, an on-grid keyboard, or incoming MIDI notes)
- 16 scales with a selectable root note
- internal clock with tap tempo, or follow external MIDI clock; optional MIDI clock out
- keeps playing in the background while another mode is on screen

**What is different from the Pro Mk3:**

- The X has only 16 buttons around the grid, so most functions moved to a
  **Shift** layer (see [Controls](#controls)).
- **Projects can be saved** (8 slots, see [Projects](#projects)); the rest of the
  working state is lost when you unplug the Launchpad.
- There is no TRS MIDI output on the X, so that output option does nothing.

## Entering and leaving

**Hold Capture MIDI and press Custom** to switch to the sequencer from any mode
(**Custom** alone opens the [Custom Modes](custom-modes-launchpad-x.md), as
printed on the button). Press **Custom** again, with or without Capture MIDI, to
return to the mode you came from. Playback keeps running while the sequencer is
not on screen. Hold **Session** to open Setup; a short tap on Session opens the
Ableton Live Session mode.

## Controls

```
  Up    Down   Left   Right  Session  Note   Custom  Capture
 track  track  page   page   (Setup)  Play   (exit)  SHIFT
  prev  next   back   fwd            ...
┌──────────────────────────────────────────────────────┬────────────┐
│                                                      │ Scale      │
│                                                      │ Probability│
│                                                      │ Rate       │
│                    8 x 8 grid                        │ Random     │
│            (drum or melodic editor, or an            │ Length     │
│             overlay, see below)                      │ Octave     │
│                                                      │ Velocity   │
│                                                      │ Notes      │
└──────────────────────────────────────────────────────┴────────────┘
```

Button names follow the labels printed on the Launchpad X. **Capture MIDI**
is the Shift button. It is also the key you hold while plugging in USB to start
the bootloader, but that matters only at power-up.

### Top row

| Button | Alone | With Shift held |
| --- | --- | --- |
| **Up** | Previous track (wraps around) | Transpose +1 octave |
| **Down** | Next track (wraps around) | Transpose -1 octave |
| **Left** | Previous page of 8 steps | Tempo -1 BPM |
| **Right** | Next page of 8 steps | Tempo +1 BPM |
| **Session** | Hold to open Setup (not used by the sequencer) | |
| **Note** | Play / stop. With external clock it arms and waits for clock | |
| **Custom** | Leave the sequencer (enter it with Capture MIDI held) | |
| **Capture MIDI** | **Shift** (hold) | |

The Up and Down buttons are lit in the color of the current track.

### Right column

Alone, the right column buttons choose what the grid edits (melodic tracks) or
act as hold modifiers (drum tracks), exactly as on the Pro Mk3. From the top:

| Button | Melodic track | Drum track |
| --- | --- | --- |
| **Scale** | Open or close the scale overlay | same |
| **Probability** | Edit lane: probability | Hold, then tap a step to lower its probability |
| **Rate** | Cycle the track's step rate | same |
| **Random** | Randomize the current lane within the loop | Randomize the current voice |
| **Length** | Edit lane: gate length | |
| **Octave** | Edit lane: octave | |
| **Velocity** | Edit lane: velocity | Hold, then tap a step to lower its velocity |
| **Notes** | Edit lane: notes | |

With **Shift** held, the same buttons become the function layer:

| Shift + button | Action |
| --- | --- |
| **Notes** | **Record**: hold it to turn the grid into the transposition keyboard |
| **Velocity** | **Clear**: hold it, then tap a step, a page pad or a drum voice to erase it. Tap it without touching the grid to erase the whole current track |
| **Octave** | **Duplicate**: copy the loop after itself and double the loop length |
| **Length** | **Fixed Length**: hold it, then tap a page pad to set the loop end |
| **Random** | **Mute**: hold it, then tap a step or a drum voice to mute or unmute it. Tap it without touching the grid to mute or unmute the whole current track |
| **Rate** | **Solo**: solo or unsolo the current track at once |
| **Probability** | **Projects** overlay (load and save, see below) |
| **Scale** | nothing |

**Hold-type functions.** Record, Fixed Length, Clear and Mute stay active as
long as you keep their button down, even if you let go of Shift first. So the
usual way is: press Shift, press and hold the function button, release Shift,
then use the grid. Clear and Mute decide on release: if you tapped the grid
while holding them, they edited that step, page or voice; if not, they apply to
the whole current track. Solo is a one-shot on the current track. Select the
track with Up/Down first.

Shift also switches the grid to the **settings overlay** for as long as it is
held (see below). Take care that the pads you tap while holding Shift are
settings, not steps.

## Tracks, pages and loops

Each track has 8 pages of 8 steps (64 steps). The bottom row of the grid holds
the 8 page pads on melodic tracks. The pads show the loop range dimly, the page
you are viewing brightly, and the playing page in white.

- Tap a page pad to view that page.
- Hold one page pad and tap another to set the loop to that range.
- Shift + **Length** (hold), then tap a page pad: set the loop end there.
- Shift + **Octave**: copy the loop after itself and double its length.

## Melodic tracks

The top 7 grid rows are 8 steps (columns) by 7 values (rows). The bottom row
holds the page pads. The right column (without Shift) picks what the 7 rows
edit:

| Lane | Rows mean | Tap a pad to... |
| --- | --- | --- |
| **Notes** | 7 scale degrees, root at the bottom | turn the step on at that note; tap the lit pad again to turn it off |
| **Octave** | -3 ... +3 octaves (middle row = 0) | set the octave |
| **Velocity** | 7 levels, 16 ... 127 | set the velocity |
| **Length** | 1/4, 1/2, 3/4, 1, 2, 4, 8 steps | set the gate length |
| **Probability** | 12 %, 25 %, 37 %, 50 %, 62 %, 75 %, 100 % | set how likely the step plays |

Notes follow the project scale and root note, plus the live transposition.

## Drum tracks

```
 rows 8-5:  32 steps of the selected voice (two views of 32 = 64 steps)
 rows 4-1:  columns 1-4: 16 drum voices; columns 5-8 of rows 4 and 3: 8 page pads
```

- Tap a **voice pad** to select it. It also plays the sound, using the pad's velocity.
- Tap a **step** to toggle a hit. New hits take their velocity from how hard you press.
- Hold **Velocity** (right column) and tap a step to step its velocity down.
- Hold **Probability** and tap a step to step its probability down. Steps under 100 % show in cyan.

Voices play chromatically from the track's base note: 36 (C1) by default, which
matches General MIDI drums. Drum tracks are never transposed.

## Settings overlay (hold Shift)

While **Shift** is held, the grid shows the settings of the current track and
the project. Tap a pad while holding Shift to change a setting.

| Row | Pads |
| --- | --- |
| 8-7 | MIDI channel 1-16 of the current track |
| 6 | 1 = Drum, 2 = Melodic. 4 = output on the USB MIDI port, 5 = output on the USB DAW port. (6 = TRS out, not on the X) |
| 5 | Step rate: 1/4, 1/4T, 1/8, 1/8T, 1/16, 1/16T, 1/32, 1/32T |
| 4 | 1 = internal clock, 2 = external clock. 4 = send MIDI clock. 8 = transpose from MIDI channel 16 on/off |
| 3 | Melodic: root octave 1-8. Drum: base note C0 / C1 / C2 / C3 |
| 2 | 1 = tap tempo. 5 = -10 BPM, 6 = -1, 7 = +1, 8 = +10 |
| 1 | Tempo presets 80 ... 150 BPM |

## Scale overlay (Scale button)

Rows 8-7 are a piano for the root note. Rows 5-4 pick one of 16 scales: Major,
Minor, Dorian, Phrygian, Lydian, Mixolydian, Locrian, Harmonic minor, Melodic
minor, Major pentatonic, Minor pentatonic, Blues, Whole tone, Diminished,
Hungarian minor, Chromatic. Press **Scale** again to close the overlay.

## Live transposition

- **Shift + Up / Down**: transpose by an octave while playing. (The single
  semitone buttons of the Pro Mk3 do not exist on the X; use the keyboard
  below for finer steps.)
- Hold **Record** (Shift + **Notes**, then release Shift): the grid becomes
  three one-octave piano keyboards. Rows 1-2 give 0 ... +12, rows 3-4 give
  -12 ... 0, rows 5-6 give +12 ... +24. The top-right pad resets to 0.
- Play notes into the Launchpad's USB MIDI port on **channel 16**. Middle C
  (60) means no transposition. These notes are used by the sequencer and not
  passed to other modes. Turn this off in the settings overlay.

Transposition applies to melodic tracks only.

## Projects

Hold **Shift** and tap **Probability** to open the Projects overlay (tap it again to
close it):

| Where | What |
| --- | --- |
| Row 8 (top), pads 1 to 8 | **Load** project 1 to 8. Green = something is saved there, white = the one you loaded last |
| Row 7, pads 1 to 8 | **Save** the current work into project 1 to 8 |
| Row 5, pad 8 | **New**: start an empty project (saved projects stay) |

A pad flashes white if it worked and red if it did not. A project holds all 5 tracks,
the tempo, scale and clock settings. Projects are saved in a protected area of the
chip's external flash through a power-fail-safe store: a save writes a new copy and only
then makes it valid, reads it back to check it, and never touches the previous copy;
see [storage-design.md](storage-design.md). Saving takes a moment during which the
pads and the clock stand still, so save while the sequence is stopped.

## MIDI output

Each track sends notes on its own channel. The defaults are: track 1 = drums on
channel 10, tracks 2-5 = melodic on channels 1-4. Output goes to the
Launchpad X's USB **MIDI** port (the second USB MIDI port; the exact name
depends on your operating system) and can also be sent to the **DAW** port per
track. Connect a synthesizer, drum machine or DAW to that port and set it to
receive on the matching channels. A Launchpad X has no DIN or TRS MIDI output.

## Not available on the X

These Pro Mk3 functions need extra buttons, which the X does not have:

- Select a track directly by number (use Up / Down, which cycle through all 5)
- Transpose by a single semitone with a button
- Saving and loading projects
- TRS MIDI out

## Installing and getting back to stock

> **Read this first.** Flashing custom firmware is at your own risk. This
> build has not been tested on hardware. The steps below are designed so that
> a failure can be recovered from, but one assumption could not be verified
> without the device: that the Launchpad's built-in bootloader always starts
> when you hold **Capture MIDI** while plugging in USB, even if the installed
> firmware is broken. **Prove that on your own unit first** (step 1 below)
> before you flash anything custom. If the bootloader port ever does not
> appear, stop: that is the one case that cannot be fixed without a hardware
> programmer.

What you need: the Launchpad X, a USB cable and a computer with a browser that
supports WebMIDI (Chrome), or Python 3 with a MIDI library for `tools/flash.py`.
Keep a copy of Novation's stock firmware file (`LPX-351.syx`) on two different
disks, and note its checksum:

```
sha256 a30c68166bca0ba4e586553b835f90bfeaeb3691a1707c2e2388462ee144e2d4  LPX-351.syx
```

Novation Components (https://components.novationmusic.com) can always restore
the official firmware.

### Entering the bootloader

1. Unplug the Launchpad from USB.
2. Hold **Capture MIDI** (top right).
3. While holding it, plug the USB cable back in.

A MIDI device named like "Launchpad MIDI Bootloader" appears. To leave it
without flashing, unplug the Launchpad.

### Flashing sequence (do these in order)

1. **Rehearse recovery.** On your working Launchpad X, enter the bootloader and
   flash the stock `LPX-351.syx` back. Confirm the Launchpad starts normally
   afterwards. Only continue when this works.
2. **Flash the upstream CoreFW** for the Launchpad X (`core-launchpad-x.syx`
   from the CoreFW project) and confirm it starts. Then enter the bootloader
   again and flash stock once more. This proves the whole route with firmware
   that other people already use.
3. **Flash the Seq95 build** (`build/lpx/fw.syx`, built with `cargo lpx`). Check
   it starts. Then power-cycle with **Capture MIDI** held and check the
   bootloader still appears.

Never flash a `.syx` that was built for another Launchpad model. Before
flashing a build you made yourself, run the Gate 0 check that comes with the
project (`tools/verify_lpx.py`); it must print `PASS`.

### Going back to Novation's firmware

Enter the bootloader and send the stock `LPX-351.syx` with the same tool, or
use Novation Components. The Launchpad restarts into the official firmware on
its own.

### Building from source

```
export PATH=$HOME/.cargo/bin:$PATH     # a recent Rust, not the distro's old one
cargo lpx                              # writes build/lpx/fw.syx
```

`cargo lpx` needs `arm-none-eabi-objcopy` (Debian: `binutils-arm-none-eabi`).
The image must stay inside the 80 KB application area above the bootloader;
the build fails to link if it grows too large, which is the safe outcome.

## Troubleshooting

| Problem | What to do |
| --- | --- |
| Pads dark after flashing | Hold **Capture MIDI**, replug USB and flash stock `LPX-351.syx` |
| No bootloader device appears | Try another USB cable and port; try another computer. If it still never appears, stop and ask for help; do not keep re-flashing |
| The sequencer does not open | Hold Capture MIDI while pressing Custom (Custom alone opens the Custom Modes). The firmware must be the Seq95 build, and you must not be in Setup or the boot animation |
| Shift settings pop up while editing | Capture MIDI is Shift. Release it, then tap steps |
| No sound | The sequencer sends MIDI only. Route the Launchpad's MIDI port to an instrument or DAW and match the channels |
