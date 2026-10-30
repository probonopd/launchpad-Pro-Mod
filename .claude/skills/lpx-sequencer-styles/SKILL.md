---
name: lpx-sequencer-styles
description: Use when changing or extending the Launchpad X firmware sequencers (Seq95 or the Novation-style novseq crate), adding a view or parameter to novseq, adding another sequencer style, or putting a large state structure (kilobytes) into a no_std firmware static. Covers the LPX_SEQUENCER build option, the SequencerApp alias, the zero-initialized-RAM rule that keeps big statics out of flash, the novseq model/engine/ui/projectio split, the button map, how to test on the host, and the pitfalls hit.
---

# Launchpad X sequencer styles

Verified on one Launchpad X only; never run on other models (docs/tested-hardware.md).

Worked out Oct 2026. User docs: `docs/novation-sequencer.md`. Storage: `docs/storage-design.md`.

## Build option

`LPX_SEQUENCER=launchpad95|novation cargo lpx` (default launchpad95). xtask appends
`sequencer-<style>` to the features; `src/app/mod.rs` has `compile_error!` for none or both.
`src/app/sequencer.rs` aliases `SequencerApp` to the chosen app; the host and
`src/sys/driver/launchpad-x/mod.rs` only use the alias. A new style = new crate + glue app
with the same interface (`new`, `background_tick`, `midi_in(&MidiEvent) -> bool`, `App`),
one feature, one alias line. Always build and run `tools/verify_lpx.py` for BOTH styles
(`cargo lpx` right before the verifier; other builds overwrite target/).

## Rule: a big static must be all zero bytes

A static with any nonzero byte goes to `.data`: it is copied from flash at startup, adds its
size to the image, and the 80 KB image has little room. novseq's project is about 13 KB, so:
every struct in it (model, engine, ui, glue) has an all-zero valid state (`ZERO` consts,
note 0 = empty, flags stored inverted, `u32::MAX` "dirty" markers replaced by a `redraw`
bool), and the real defaults are written by `Project::reset()` on first `enter()`. Check with
`arm-none-eabi-size`/the verifier margin when adding a field with a nonzero default.
Startup still builds the static on the stack through `init_with` (startup needs ~18 KB for
novation); the verifier checks the worst-case stack, keep its margin above 4 KB.

## novseq layout (host-testable, no_std)

- `model.rs`: Step (8 notes, velocity, gate, skip, delay, mutate), Pattern, Track, Project,
  scales, `play_note`. `engine.rs`: per-track lanes (own rate, direction, swing, pending
  delayed step), queued patterns/chains applied at the next step time after a full pass
  (`pass_length`: ping-pong = 2L-2), mute/solo, probability/mutation with an xorshift RNG,
  internal and external clock. `ui.rs`: views, held steps (bitmask), record, rendering.
  `projectio.rs`: sparse encoding ("NSQ" + version), two-pass decode (validate, then apply),
  `Storage` trait implemented in `src/app/novseq/mod.rs` over safestore.
- Button map: plain right column = actions (Record 19, Solo 29, Mute 39, Clear 49 hold,
  Duplicate 59 hold, Quantise 69, Projects 89); Shift (Capture MIDI) + right column = views.
  Tap on a later step while holding one = tie (so multi-select in Steps view holds the later
  step first; parameter views multi-select freely).
- Tests: `cargo test -p novseq` (58). Drive it with `press/tap/view` helpers and a pulse
  loop; thread-local mock storage. Add a test with every behavior change.

## Pitfalls hit

- `pkill -f <pattern>` kills your own shell (exit 144); never use it.
- `tools/make_pictures.py` reads framedump output on stdin; run it piped, not bare.
- Sampling the playhead in tests at pulse boundaries: restart/continue tests must account
  for catch-up (up to 8 pulses per tick) and `ui.now_us`.
- Chain/queued switches done right after the last step fired made the edit target change
  one step early; do them at the next step time.
- Setup extras (thresholds, clock gates, Vegas, sleep) live in `src/sys/prefs.rs` /
  `idle.rs`, saved in the SetupPrefs store item; LED sleep works by scanning out an all-ones
  (dark) line in `grid.rs`, because brightness level 0 is not dark.
