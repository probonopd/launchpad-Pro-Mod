# Feature comparison with the Novation Launchpad Pro [MK3] firmware, and what is missing

> Status in this table means "works on the Launchpad X", the only model we tested on
> (see [tested-hardware.md](tested-hardware.md)). It says nothing about other models.

Compared on 2026-10-30 against Novation's **Launchpad Pro [MK3] User Guide** (version 6.0 on
userguides.novationmusic.com): Introduction, Getting Started, Hardware overview, Interface
(Session, Note, Chord, Custom Modes), Sequencer, Setup page and Appendix.

**Answer: no, we are not feature complete.** The Note, Chord and Custom Modes of the guide are
mostly there, and several Setup and Session items are partly there, but Novation's
**Sequencer** (the biggest feature of the Pro) is a different, much larger design than the
Seq95 sequencer we ship, and Components integration, Ableton Live integration and some Setup
options are missing.

Caveats: the guide is the only source (the original firmware was not available to compare
behavior); the Pro has more buttons and a DIN MIDI section that the Launchpad X lacks, so
"complete" here means *everything that is possible on the X's hardware*; the guide leaves some
details open (marked in the notes).

Legend: ✅ done, ◐ partly done, ❌ missing, ⛔ not possible on the Launchpad X hardware.

## 1. Feature by feature

### Session mode (Ableton Live) and Live integration

| Feature in the guide | Status | Notes |
| --- | --- | --- |
| Session view clip grid, navigation arrows, scene launch | ◐ | The firmware's DAW mode (CoreFW) implements the Novation DAW protocol; opens with a Session tap. **Never tried with Live.** |
| Session overview (hold Session) | ❌/? | Not known to exist in the DAW mode |
| Select / clear / duplicate / double / quantise a clip | ◐/⛔ | Needs the Pro's Clear, Duplicate, Quantise buttons; the X has no such buttons |
| Record quantise toggle, Fixed length (bars) | ❌ | Same reason |
| Track controls: record arm, mute, solo, stop clip, volume, pan, sends, device | ◐ | The X protocol has its own mixer/fader pages (the DAW mode has "native faders"); the Pro's bottom row does not exist |
| Undo, redo, tap tempo, click, Capture MIDI | ❌ | Shift functions of buttons the X does not have |
| Print to clip | ❌ | Needs the sequencer first (see below) and the Live link |
| Momentary view switching, record arm colors | ◐ | Part of the DAW protocol; untested |

### Note mode

| Feature | Status | Notes |
| --- | --- | --- |
| Chromatic layout, scale layout | ✅ | Both, with blue/purple/blank colors |
| Overlap: 5 finger, 4, 3, 2, sequential | ✅ | |
| 16 scales, root selection with piano, MIDI channel | ✅ | Names from the guide; the intervals are standard ones |
| Octave (up/down) and semitone (left/right) shifts | ✅ | |
| Drum layout | ◐ | Static 4x4 blocks; the Pro follows the armed Ableton drum rack, shows filled slots in yellow, selected pad in blue, and switches layout automatically: ❌ (needs the Live link) |
| Settings page (Shift + Note) | ✅ | Reached by pressing Note again |
| Settings kept | ✅ | Saved in our storage (the Pro keeps them in the project) |

### Chord mode

| Feature | Status | Notes |
| --- | --- | --- |
| Triad column, note area (3rd, 5th, 7th, 5th up), 14 chord bank slots | ✅ | Our reading of the guide; the guide's pictures were not available |
| Chord bank lock, sustain | ✅ | |
| Saving chords | ✅ | The Pro stores the chord bank in the project; we store it with the Note preferences |
| Own Chord button | ⛔ | Chord is a layout of Note Mode on the X |

### Custom modes

| Feature | Status | Notes |
| --- | --- | --- |
| 8 default Custom Modes (faders, drum rack, keyboard, program changes, lit legacy layouts) | ✅ | The guide and its appendix disagree on what Custom 6 is (program changes 64-127 or notes); we followed the interface page |
| Master MIDI channel per mode | ◐ | Done; the Pro's view also shows the other modes' channels |
| **Edit Custom Modes in Novation Components** (notes, CC, program change, toggle/momentary/trigger, on/off colors, faders, "back up online") | ❌ | Not started: the Components data format and handshake are unknown (needs captures); see `custom-mode-format-notes.md` |
| Backup of projects and modes with Components | ❌ | Same |

### Sequencer

| Feature | Status | Notes |
| --- | --- | --- |
| Play, stop, continue (Shift + Play) | ◐ | Play/stop only |
| 4 tracks | ◐ | We have 5 tracks (Seq95) |
| **32-step patterns, 8 patterns per track** | ❌ | We have one 64-step sequence per track |
| **Steps view with Play Area; up to 8 notes per step** | ❌ | One note per melodic step; drum tracks have 16 voices |
| Assign notes by holding a note and pressing a step (or the reverse), preview, red markers | ❌ | |
| Clear and duplicate steps | ◐ | Clear step/track yes; duplicate loop only |
| **Live recording** with unquantised micro-step recording and record quantise | ❌ | (Our Record button is a transposition keyboard) |
| Gate length by holding a step and pressing another | ◐ | We have a length lane (7 values) |
| **Patterns view**: chain up to 8 patterns, queue, instant switch, clear, duplicate | ❌ | |
| **Scenes** (16): assign, chain, queue, instant switch, clear, duplicate | ❌ | |
| Pattern settings: sync rate (8 rates) | ✅ | Per track rate |
| Pattern settings: direction (forward, backward, ping-pong, random) | ❌ | |
| Pattern start and end points | ◐ | Loop range in blocks of 8 |
| Velocity per step (16 levels) | ◐ | 7 levels |
| Probability per step (8 levels, per note) | ◐ | 7 levels, per step |
| **Mutation** per step | ❌ | |
| **Micro steps** | ❌ | |
| Tempo and swing views | ◐ | Tempo yes, **swing** ❌ |
| Print to clip (Ableton Live) | ❌ | |
| **Projects: 64 slots**, with colors; project also holds Note/Chord settings and Custom channels | ◐ | 8 slots, sequencer only |
| Sequencer settings: scale and root (shared with Note/Chord) | ◐ | The sequencer has its own scale and root |
| Per-track type: Drum, Scale, Chromatic | ◐ | Drum or melodic (scale) |
| Per-track MIDI channel | ✅ | |
| Sync: receive and transmit clock | ◐ | In the sequencer settings; not as global Setup options |
| TRS MIDI out/in, Out2/Thru | ⛔ | The X has no DIN/TRS ports |

### Setup menu and modes

| Feature | Status | Notes |
| --- | --- | --- |
| LED brightness (8 levels) | ✅ | |
| **Vegas mode** (light show after 5 minutes) and its toggle | ✅ | Setup page 2, pad 51 |
| **LED sleep** | ✅ | Setup page 2, pad 52; the LEDs are scanned out dark, colors are kept |
| Velocity on/off, 3 curves | ✅ | |
| **Pad trigger threshold** (4 levels) | ✅ | Setup page 3, pads 51-54 |
| Aftertouch mode: off, polyphonic, channel | ✅ | |
| **Aftertouch threshold** (off, low, high) | ✅ | Setup page 4, pads 51-53 (three levels; off is the aftertouch mode Off) |
| **MIDI settings**: receive clock, transmit clock, Out2/Thru | ✅/⛔ | Clock toggles: Setup page 5. Out2/Thru does not exist on the X |
| **Fader velocity on/off** | ⛔ | Our faders send plain control changes; there is no velocity to switch |
| Live, Programmer and Legacy mode switch | ✅ | Performance mode takes the role of Legacy; its layout should be compared with the original Pro User layout |
| Powers up in Live mode | ✅ | Powers up in Note Mode |
| Bootloader menu (device ID, mass storage, brightness, versions) | ✅ | It is Novation's own bootloader and is untouched |

### Documentation and getting started

| Feature | Status |
| --- | --- |
| Easy Start / mass storage drive | ⛔ (belongs to Novation's firmware and bootloader; the bootloader still provides it) |
| User manual, installation, recovery | ✅ (`user-manual.md`) |

## 2. Prioritized list of what to implement

Order = value for a Launchpad X user, divided by effort and risk. Each item says what it needs.
**Size and RAM are real limits:** about 10 KB of flash and 33 KB of stack margin are left, so the
big items require removing or reusing code (see item 7).

### P0: make sure what exists is right (do first, small)

1. **Verify on the device:** Chord layout, the eight Custom Modes, Session tap, Setup pages, the
   saving of the Note and Custom preferences (dump check), and Programmer/Performance
   behavior against the guide's "Legacy mode" layout. Fix what is wrong before adding more.
   *Effort: S, needs your time on the device.*

### P1: small additions that close visible gaps (a few hours each)

2. **Setup: pad trigger threshold** (4 levels) and **aftertouch threshold** (off, low, high) and
   **fader velocity on/off**. They map to constants of the pad driver (`inputs.rs`) and the Setup
   pages. *Effort: S-M. Risk: pad feel; test on the device.*
3. **Setup: MIDI page** with *receive clock* and *transmit clock* toggles (global), replacing the
   sequencer-only options. *Effort: S.*
4. **LED sleep** (all LEDs off until a pad is pressed) and **Vegas mode** (light show after 5
   minutes) with its toggle. *Effort: S (sleep), M (Vegas: an animation in few KB).*
5. **Custom Mode master channels view** showing all eight modes' channels like the Pro; custom
   modes in the saved preferences. *Effort: S.*
6. **Sequencer quick wins inside Seq95:** swing, 8 probability levels, 16 velocity levels,
   pattern direction (forward/backward/ping-pong/random), tempo/swing views, "continue"
   (Shift + Play). *Effort: M; the model changes, so saved projects need a format version.*

### P2: big value, needs a decision or input from you

7. **Decide the sequencer strategy (blocks everything below).** Novation's sequencer is a
   different design: 4 tracks x 8 patterns x 32 steps, up to 8 notes per step, per-step
   velocity, probability, mutation, micro steps and gate, 16 scenes, 64 projects. Its data is
   about 20 KB of RAM and 15-20 KB of flash, plus a project store far larger than our 144 KB
   window. Options: **(a)** replace Seq95 by a Novation-style sequencer (frees Seq95's 11 KB);
   **(b)** keep Seq95 and add Novation-style features step by step; **(c)** keep Seq95 as is
   and declare the Novation sequencer out of scope. *Recommendation: (a), phased as in P3.*
8. **Components Custom Modes** (the headline feature of the Custom button on all three Launchpads).
   Needs: a `.syx` exported from Components with a test mode, and a recording of Components'
   traffic to a stock X (see `custom-mode-format-notes.md`); then a parser into the existing
   `ModeDef`, storage of 8 modes (about 700 bytes each; one more store item), the handshake
   so that Components accepts the device, and unit tests with the real files. *Effort: L; blocked
   on the captures.*
9. **Session mode completeness:** test with Ableton Live and the Launchpad X control-surface
   script, then fill gaps (Session overview, Capture MIDI, mixer pages). *Effort: M-L; needs Live
   and your time; it is CoreFW's DAW code, so many things may already work.*

### P3: the Novation-style sequencer, in phases (if 7a or 7b is chosen)

10. **S1 steps view:** 32-step patterns, Play Area, multi-note steps, hold-to-assign, preview,
    clear and duplicate steps, play/stop/continue. *Effort: XL.*
11. **S2 recording:** live recording from the pads, record quantise on/off, unquantised micro-step
    placement, gate length by holding a step. *Effort: L.*
12. **S3 patterns and scenes:** 8 patterns per track, chains, queued and instant switching, 16
    scenes with chains. *Effort: L.*
13. **S4 step parameters:** velocity (16), probability (8), mutation, micro steps, pattern
    settings (rate, direction, start/end). *Effort: L.*
14. **S5 projects:** 16-64 project slots with colors (needs a bigger store window and compact
    encoding), settings stored with the project, per-track types (drum/scale/chromatic) and the
    shared scale and root. *Effort: L.*
15. **S6 print to clip** (Ableton): needs the Live link and a working DAW mode. *Effort: M, after 9.*

### P4: needs information or is probably not worth it

16. **Drum layout following Ableton's drum rack** (filled slots, auto switch): needs data from
    Live that the X protocol may not provide. *Probably not feasible.*
17. **Chord and Sequencer as buttons, DIN MIDI, Out2/Thru, Track Select buttons:** not possible
    on the Launchpad X; the current substitutes (layout, Capture MIDI chord, right-hand column)
    stay.

## 3. Suggested order

1. P0 verification, 2. P1 items 2-5 (Setup completeness, about a week of evenings),
3. item 7 (decide the sequencer strategy) together with item 6 (quick wins) or phase S1,
4. item 8 as soon as the Components captures are available, 5. item 9 with Live at hand,
6. the remaining sequencer phases. Everything that writes flash keeps the same gates:
host tests, the release check (`verify_lpx.py`, including the stack check), recovery through the
bootloader, and a dump before and after.

## Build-time sequencer style (Launchpad X)

`cargo lpx` links exactly one sequencer: `LPX_SEQUENCER=launchpad95` (default, Seq95)
or `LPX_SEQUENCER=novation` (Novation-style, planned phases S1-S6). Without a choice,
or with both, the build fails (`compile_error!`). The Novation style has all of phases S1-S5 (see novation-sequencer.md); print to clip is not possible on the X.

## Setup additions (Launchpad X)

Velocity page: pad trigger threshold, 4 levels, pads 51-54. Aftertouch page: threshold,
3 levels, pads 51-53. Saved in the store window (item `SetupPrefs`, sectors 0-1) when
Setup is left; defaults reproduce the previous fixed thresholds.
