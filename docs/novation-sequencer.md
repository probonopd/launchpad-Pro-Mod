# Novation-style sequencer (Launchpad X)

Tested on the Launchpad X only (see [tested-hardware.md](tested-hardware.md)).

Build option: `LPX_SEQUENCER=novation cargo lpx` (default is `launchpad95`, Seq95).
Open it like Seq95: hold Capture MIDI and press Custom. Code: crate `novseq`, glue
`src/app/novseq/mod.rs`. Modeled on the Launchpad Pro Mk3 sequencer; where the X has
fewer buttons the choices below are ours.

## What it has

4 tracks, 8 patterns per track, 32 steps per pattern, up to 8 notes per step, 16 scenes,
chains, 3 track types (Scale, Drum, Chromatic) with 12 scales, per-step velocity (16
levels), probability (8), micro-step delay (0-5 pulses), mutation (4), gate, per-pattern
length, direction (forward, backward, ping-pong, random), sync rate (1/4 to 1/32 and
triplets), swing, recording with or without quantise, mute and solo, internal or
external clock, clock output, 8 project slots.

## Layout

Right column, plain (bottom to top): Record Arm = record, Solo, Mute, Stop Clip = Clear
(hold), Send B = Duplicate (hold), Send A = Quantise on/off, Pan unused, Volume = Projects.
Hold Shift (Capture MIDI) and the right column becomes the view selector, from the top:
Steps, Patterns, Scenes, Velocity, Probability, Micro step, Mutation, Settings (press
Settings again for its next page).
Top row: Up/Down = previous/next track, Left/Right = steps 1-16 / 17-32, Note = play/stop
(Shift + Note = continue), Shift + Left/Right = tempo.

## Views

- **Steps**: rows 8-7 are 16 steps, rows 6-1 the Play Area (Scale: scale around the root,
  three degrees per row; Drum: 4 x 8 pads from note 36; Chromatic: rows a fourth apart).
  Tap a step to set or clear it. Hold steps and press Play Area pads to build chords
  (up to 8 notes). Hold a step and tap a *later* step to tie (gate). Hold Clear and tap a step
  to clear it, hold Clear and tap Shift to empty the pattern. Hold Duplicate, tap a source
  step, tap a target step to copy.
- **Velocity / Probability / Micro step / Mutation**: rows 8-7 are the steps. Hold one
  or more steps (they highlight) and press a value pad in row 1 (Velocity: rows 1-2, 16
  levels; Probability: 8; Micro: 6; Mutation: 4). The held step's value is shown white.
- **Patterns**: rows 8-5 are tracks 1-4, columns patterns 1-8. Tap = play that pattern
  (when running it takes over at the end of the current pass); hold one pad and press
  another in the same row = chain, played in turn. Pressing a pad also selects that track.
  The pattern you edit is always the track's playing one. Clear/Duplicate work on patterns.
- **Scenes**: the two step rows are scenes 1-16. Tap = launch (every track queues its
  stored patterns). Shift + pad = store the current patterns in the scene. Scenes 1-8 start
  as pattern 1-8 on all tracks, 9-16 repeat them.
- **Settings**, page 1 (pattern): rows 8-7 length (tap the last step), row 6 direction,
  row 5 rate, row 4 swing (0-3 pulses). Page 2 (track): rows 8-7 MIDI channel, row 6 type,
  rows 5-4 scale, rows 3-2 root note, row 1 octave. Page 3 (project): pad 1 external clock,
  pad 2 send clock.
- **Projects** (Volume): row 8 load, row 7 save, bottom-right pad of row 5 new project.
  Green = done, red = failed (empty slot, project too big, flash error). Saving uses the
  same brick-safe store as everything else (see storage-design.md).

## Recording

Arm Record Arm, press Note. Notes land on the nearest step (Quantise on), or in the step they
fall in with the offset kept as a micro-step delay (Quantise off). The time a pad is held
becomes the gate. Existing notes stay (overdub).

## Differences from the Pro Mk3

- 16 steps are visible at a time (Left/Right pages), not 32; the Play Area is 6 rows.
- 8 project slots, not 64, and a project that is completely full (about 13 KB) is refused
  because a slot holds 7.9 KB; typical projects are a few KB.
- Only the playing pattern of a track can be edited.
- No Print to clip: it needs Live's clip interface, which the X does not offer; and no
  Legacy/Live-mode clip launching from inside the sequencer.
- Mutation is our interpretation: a mutated step varies its notes by up to two scale steps
  on a pass, with a chance of 25/50/75 %.
- Swing delays every second step by 0-3 pulses (50, 58, 67, 75 %).
- Scenes are stored selections, edited with Shift + pad.
