// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 launchpad-Pro-Mod contributors

use crate::*;

const UP: u8 = 91;
const DOWN: u8 = 92;
const LEFT: u8 = 93;
const RIGHT: u8 = 94;
const PLAY: u8 = 96;
const SHIFT: u8 = 98;
const RECORD: u8 = 19;
const SOLO: u8 = 29;
const MUTE: u8 = 39;
const CLEAR: u8 = 49;
const DUPLICATE: u8 = 59;
const QUANTIZE: u8 = 69;
const V_STEPS: u8 = 89;
const V_PATTERNS: u8 = 79;
const V_SCENES: u8 = 69;
const V_VELOCITY: u8 = 59;
const V_PROBABILITY: u8 = 49;
const V_MICRO: u8 = 39;
const V_MUTATION: u8 = 29;
const V_SETTINGS: u8 = 19;

struct Log(Vec<Vec<u8>>);

impl MidiOut for Log {
    fn send(&mut self, msg: &[u8]) {
        self.0.push(msg.to_vec());
    }
}

fn setup() -> (NovSeq, Log) {
    let mut s = NovSeq::new();
    s.enter();
    (s, Log(vec![]))
}

thread_local! {
    static SLOTS: std::cell::RefCell<Vec<Option<Vec<u8>>>> = std::cell::RefCell::new(vec![None; 8]);
    static FAIL_SAVES: std::cell::Cell<bool> = std::cell::Cell::new(false);
}

struct Mem;

impl Storage for Mem {
    fn save_with(&mut self, slot: u8, fill: impl FnOnce(&mut [u8]) -> Option<usize>) -> bool {
        let mut buf = vec![0u8; MAX_PROJECT_BYTES];
        let Some(len) = fill(&mut buf) else { return false };
        if FAIL_SAVES.with(|f| f.get()) {
            return false;
        }
        SLOTS.with(|s| s.borrow_mut()[slot as usize] = Some(buf[..len].to_vec()));
        true
    }

    fn load_with(&mut self, slot: u8, apply: impl FnOnce(&[u8]) -> bool) -> bool {
        match SLOTS.with(|s| s.borrow()[slot as usize].clone()) {
            Some(bytes) => apply(&bytes),
            None => false,
        }
    }
}

fn press(s: &mut NovSeq, out: &mut Log, index: u8) {
    s.button(index, true, 90, out, &mut Mem);
}

fn release(s: &mut NovSeq, out: &mut Log, index: u8) {
    s.button(index, false, 0, out, &mut Mem);
}

fn tap(s: &mut NovSeq, out: &mut Log, index: u8) {
    press(s, out, index);
    release(s, out, index);
}

fn view(s: &mut NovSeq, out: &mut Log, button: u8) {
    press(s, out, SHIFT);
    tap(s, out, button);
    release(s, out, SHIFT);
}

fn step(s: &NovSeq, t: usize, n: usize) -> Step {
    s.project.tracks[t].pattern().steps[n]
}

/// Runs the clock for `pulses` pulses at 120 bpm (20833 us each).
fn run(s: &mut NovSeq, out: &mut Log, from_us: u64, pulses: u64) -> u64 {
    let end = from_us + pulses * 20_833;
    let mut t = from_us;
    while t < end {
        s.tick(t, out);
        t += 1000;
    }
    end
}

fn pulse(s: &mut NovSeq, out: &mut Log, n: u32) {
    for _ in 0..n {
        s.midi_in([0xf8, 0, 0], out);
    }
}

fn external(s: &mut NovSeq, out: &mut Log) {
    s.project.external_clock = true;
    tap(s, out, PLAY);
}

#[test]
fn all_zero_state_is_valid_until_the_first_enter() {
    let s = NovSeq::new();
    assert!(!s.project.ready);
    assert!(s.project.tracks[0].pattern().steps.iter().all(Step::is_empty));
}

#[test]
fn defaults_are_drums_on_track_one_scales_on_the_rest() {
    let (s, _) = setup();
    let p = &s.project;
    assert!(p.tracks[0].kind() == TrackKind::Drum && p.tracks[1].kind() == TrackKind::Scale);
    assert_eq!([p.tracks[0].channel, p.tracks[3].channel], [0, 3]);
    assert_eq!(p.tracks[0].pattern().length, 32);
    assert_eq!(p.bpm, 120);
}

#[test]
fn tap_on_empty_step_sets_default_note_and_tap_again_clears() {
    let (mut s, mut out) = setup();
    tap(&mut s, &mut out, 81);
    assert!(step(&s, 0, 0).has(DRUM_BASE) && step(&s, 0, 0).velocity == DEFAULT_VELOCITY);
    tap(&mut s, &mut out, 81);
    assert!(step(&s, 0, 0).is_empty());
}

#[test]
fn second_row_is_steps_9_to_16_and_right_pages_to_17() {
    let (mut s, mut out) = setup();
    tap(&mut s, &mut out, 71);
    assert!(!step(&s, 0, 8).is_empty());
    tap(&mut s, &mut out, RIGHT);
    tap(&mut s, &mut out, 81);
    assert!(!step(&s, 0, 16).is_empty());
    tap(&mut s, &mut out, LEFT);
    assert_eq!(s.ui.page(), 0);
}

#[test]
fn holding_a_step_and_pressing_play_pads_builds_a_chord() {
    let (mut s, mut out) = setup();
    press(&mut s, &mut out, 81);
    tap(&mut s, &mut out, 11);
    tap(&mut s, &mut out, 12);
    tap(&mut s, &mut out, 13);
    release(&mut s, &mut out, 81);
    let data = step(&s, 0, 0);
    assert!(data.has(36) && data.has(37) && data.has(38));
    assert_eq!(data.velocity, 90);
}

#[test]
fn holding_two_steps_edits_both_and_does_not_toggle_on_release() {
    let (mut s, mut out) = setup();
    // The later step is held first: pressing a later step while holding one is a tie.
    press(&mut s, &mut out, 83);
    press(&mut s, &mut out, 81);
    tap(&mut s, &mut out, 15);
    release(&mut s, &mut out, 81);
    release(&mut s, &mut out, 83);
    assert!(step(&s, 0, 0).has(40) && step(&s, 0, 2).has(40));
    assert!(step(&s, 0, 1).is_empty());
}

#[test]
fn a_step_holds_eight_notes_and_refuses_a_ninth() {
    let mut data = Step::ZERO;
    for n in 1..=8 {
        assert!(data.toggle(n));
    }
    assert!(!data.toggle(20));
    assert!(data.toggle(3));
    assert!(!data.has(3));
    assert!(data.toggle(20));
}

#[test]
fn removing_the_last_note_resets_the_step() {
    let mut data = Step::ZERO;
    data.toggle(40);
    data.skip = 3;
    data.toggle(40);
    assert!(data == Step::ZERO);
}

#[test]
fn play_pads_sound_on_the_track_channel_and_release_the_same_note() {
    let (mut s, mut out) = setup();
    tap(&mut s, &mut out, DOWN); // track 2, scale, channel 1
    tap(&mut s, &mut out, 11);
    assert_eq!(out.0, vec![vec![0x91, 48, 90], vec![0x81, 48, 0]]);
}

#[test]
fn up_goes_to_the_previous_track_and_wraps() {
    let (mut s, mut out) = setup();
    tap(&mut s, &mut out, UP);
    assert_eq!(s.ui.track(), 3);
    tap(&mut s, &mut out, DOWN);
    assert_eq!(s.ui.track(), 0);
}

#[test]
fn playback_sends_notes_per_track_channel_and_gates_them() {
    let (mut s, mut out) = setup();
    s.project.tracks[0].pattern_mut().steps[0].toggle(36);
    s.project.tracks[1].pattern_mut().steps[0].toggle(48);
    s.project.tracks[1].pattern_mut().steps[0].toggle(52);
    tap(&mut s, &mut out, PLAY);
    s.tick(0, &mut out);
    assert_eq!(out.0, vec![vec![0x90, 36, 100], vec![0x91, 48, 100], vec![0x91, 52, 100]]);
    out.0.clear();
    run(&mut s, &mut out, 1000, 6);
    assert!(out.0.contains(&vec![0x80, 36, 0]));
    assert!(out.0.contains(&vec![0x81, 52, 0]));
}

#[test]
fn playhead_follows_the_steps_and_the_pattern_loops_after_32() {
    let (mut s, mut out) = setup();
    tap(&mut s, &mut out, PLAY);
    let t = run(&mut s, &mut out, 0, 6 * 5);
    assert_eq!(s.engine.playhead(0), Some(4));
    run(&mut s, &mut out, t, 6 * 28);
    assert_eq!(s.engine.playhead(0), Some(0));
}

#[test]
fn stop_silences_and_play_restarts_while_shift_play_continues() {
    let (mut s, mut out) = setup();
    s.project.tracks[0].pattern_mut().steps[3].toggle(36);
    tap(&mut s, &mut out, PLAY);
    let t = run(&mut s, &mut out, 0, 6 * 3 + 2);
    assert!(out.0.contains(&vec![0x90, 36, 100]));
    out.0.clear();
    tap(&mut s, &mut out, PLAY);
    assert!(out.0.contains(&vec![0x80, 36, 0]));
    press(&mut s, &mut out, SHIFT);
    tap(&mut s, &mut out, PLAY);
    release(&mut s, &mut out, SHIFT);
    run(&mut s, &mut out, t + 1, 6);
    assert_eq!(s.engine.playhead(0), Some(4));
    tap(&mut s, &mut out, PLAY);
    assert!(!s.engine.running());
    tap(&mut s, &mut out, PLAY);
    assert!(s.engine.running());
    s.tick(t + 400_000, &mut out);
    // The catch-up after a restart plays at most a few pulses from the first step.
    assert!(s.engine.playhead(0).unwrap() <= 1);
}

#[test]
fn clear_held_clears_a_step_and_clear_then_shift_empties_the_pattern() {
    let (mut s, mut out) = setup();
    tap(&mut s, &mut out, 81);
    tap(&mut s, &mut out, 82);
    press(&mut s, &mut out, CLEAR);
    tap(&mut s, &mut out, 81);
    release(&mut s, &mut out, CLEAR);
    assert!(step(&s, 0, 0).is_empty() && !step(&s, 0, 1).is_empty());
    press(&mut s, &mut out, CLEAR);
    tap(&mut s, &mut out, SHIFT);
    release(&mut s, &mut out, CLEAR);
    assert!(step(&s, 0, 1).is_empty());
}

#[test]
fn duplicate_copies_source_step_to_the_next_pressed_step() {
    let (mut s, mut out) = setup();
    press(&mut s, &mut out, 81);
    tap(&mut s, &mut out, 11);
    tap(&mut s, &mut out, 12);
    release(&mut s, &mut out, 81);
    press(&mut s, &mut out, DUPLICATE);
    tap(&mut s, &mut out, 81);
    tap(&mut s, &mut out, 84);
    release(&mut s, &mut out, DUPLICATE);
    assert!(step(&s, 0, 3) == step(&s, 0, 0) && step(&s, 0, 0).has(36));
}

#[test]
fn shift_left_right_changes_tempo_within_limits() {
    let (mut s, mut out) = setup();
    press(&mut s, &mut out, SHIFT);
    tap(&mut s, &mut out, RIGHT);
    assert_eq!(s.project.bpm, 121);
    for _ in 0..300 {
        tap(&mut s, &mut out, LEFT);
    }
    assert_eq!(s.project.bpm, MIN_BPM);
}

#[test]
fn scale_pads_overlap_three_degrees_per_row() {
    let mut t = Track::ZERO;
    t.root = 60;
    assert_eq!(play_note(&t, 0, 0), Some(60));
    assert_eq!(play_note(&t, 0, 3), Some(65));
    assert_eq!(play_note(&t, 1, 0), Some(65));
    assert_eq!(play_note(&t, 2, 1), Some(72));
    assert_eq!(play_note(&t, 2, 2), Some(74));
}

#[test]
fn every_scale_starts_on_its_root_and_stays_inside_the_octave() {
    for scale in 0..SCALE_COUNT {
        let len = SCALE_LEN[scale] as usize;
        assert_eq!(SCALES[scale][0], 0);
        assert!(SCALES[scale][..len].windows(2).all(|w| w[0] < w[1]));
        assert!(SCALES[scale][len - 1] < 12);
    }
}

#[test]
fn drum_pads_are_four_rows_of_eight_and_chromatic_rows_are_fourths() {
    let mut t = Track::ZERO;
    t.kind = TrackKind::Drum as u8;
    assert_eq!(play_note(&t, 0, 0), Some(36));
    assert_eq!(play_note(&t, 3, 7), Some(67));
    assert_eq!(play_note(&t, 4, 0), None);
    t.kind = TrackKind::Chromatic as u8;
    t.root = 40;
    assert_eq!(play_note(&t, 1, 2), Some(47));
}

// ---- recording ----------------------------------------------------------

#[test]
fn recording_rounds_to_the_nearest_step() {
    let (mut s, mut out) = setup();
    tap(&mut s, &mut out, RECORD);
    let t = {
        tap(&mut s, &mut out, PLAY);
        run(&mut s, &mut out, 0, 13)
    };
    press(&mut s, &mut out, 11);
    assert!(step(&s, 0, 2).has(36));
    release(&mut s, &mut out, 11);
    run(&mut s, &mut out, t, 3);
    press(&mut s, &mut out, 12);
    assert!(step(&s, 0, 3).has(37));
}

#[test]
fn recording_early_in_a_step_rounds_down() {
    let (mut s, mut out) = setup();
    tap(&mut s, &mut out, RECORD);
    tap(&mut s, &mut out, PLAY);
    run(&mut s, &mut out, 0, 15);
    press(&mut s, &mut out, 11);
    assert!(step(&s, 0, 2).has(36));
}

#[test]
fn unquantized_recording_keeps_the_timing_as_a_micro_step() {
    let (mut s, mut out) = setup();
    tap(&mut s, &mut out, RECORD);
    tap(&mut s, &mut out, QUANTIZE);
    tap(&mut s, &mut out, PLAY);
    run(&mut s, &mut out, 0, 17); // last pulse 16: pulse 4 of step 2
    press(&mut s, &mut out, 11);
    assert!(step(&s, 0, 2).has(36));
    assert_eq!(step(&s, 0, 2).delay, 4);
}

#[test]
fn held_time_becomes_the_gate() {
    let (mut s, mut out) = setup();
    tap(&mut s, &mut out, RECORD);
    tap(&mut s, &mut out, PLAY);
    let t = run(&mut s, &mut out, 0, 13);
    press(&mut s, &mut out, 11);
    run(&mut s, &mut out, t, 10);
    release(&mut s, &mut out, 11);
    assert_eq!(step(&s, 0, 2).gate, 10);
}

#[test]
fn overdub_keeps_notes_and_nothing_records_when_not_armed_or_stopped() {
    let (mut s, mut out) = setup();
    s.project.tracks[0].pattern_mut().steps[2].toggle(40);
    tap(&mut s, &mut out, 11);
    assert!(!step(&s, 0, 0).has(36));
    tap(&mut s, &mut out, RECORD);
    tap(&mut s, &mut out, 12);
    assert!(s.project.tracks[0].pattern().steps.iter().all(|st| !st.has(37)));
    tap(&mut s, &mut out, PLAY);
    run(&mut s, &mut out, 0, 13);
    tap(&mut s, &mut out, 11);
    assert!(step(&s, 0, 2).has(40) && step(&s, 0, 2).has(36));
    tap(&mut s, &mut out, 11);
    assert!(step(&s, 0, 2).has(36));
}

#[test]
fn pressing_a_later_step_while_holding_one_ties_the_gate() {
    let (mut s, mut out) = setup();
    tap(&mut s, &mut out, 81);
    press(&mut s, &mut out, 81);
    tap(&mut s, &mut out, 84);
    release(&mut s, &mut out, 81);
    assert_eq!(step(&s, 0, 0).gate, 17);
    assert!(step(&s, 0, 0).has(36) && step(&s, 0, 3).is_empty());
}

#[test]
fn playback_holds_a_note_for_its_gate() {
    let (mut s, mut out) = setup();
    s.project.tracks[0].pattern_mut().steps[0].toggle(36);
    s.project.tracks[0].pattern_mut().steps[0].gate = 14;
    tap(&mut s, &mut out, PLAY);
    s.tick(0, &mut out);
    out.0.clear();
    run(&mut s, &mut out, 1, 12);
    assert!(!out.0.contains(&vec![0x80, 36, 0]));
    run(&mut s, &mut out, 12 * 20_833 + 1, 4);
    assert!(out.0.contains(&vec![0x80, 36, 0]));
}

// ---- step parameters -----------------------------------------------------

#[test]
fn velocity_view_sets_one_of_sixteen_levels_on_held_steps() {
    let (mut s, mut out) = setup();
    tap(&mut s, &mut out, 81);
    view(&mut s, &mut out, V_VELOCITY);
    press(&mut s, &mut out, 81);
    tap(&mut s, &mut out, 28); // row 2, col 8: level 15
    tap(&mut s, &mut out, 11); // level 0
    assert_eq!(step(&s, 0, 0).velocity, 7);
    tap(&mut s, &mut out, 28);
    release(&mut s, &mut out, 81);
    assert_eq!(step(&s, 0, 0).velocity, 127);
    assert!(!step(&s, 0, 0).is_empty(), "releasing in a parameter view must not toggle");
}

#[test]
fn probability_view_sets_eight_levels() {
    let (mut s, mut out) = setup();
    tap(&mut s, &mut out, 81);
    view(&mut s, &mut out, V_PROBABILITY);
    press(&mut s, &mut out, 81);
    tap(&mut s, &mut out, 11);
    release(&mut s, &mut out, 81);
    assert_eq!(step(&s, 0, 0).probability(), 0);
    assert_eq!(step(&s, 0, 0).skip, 7);
}

#[test]
fn a_step_with_the_lowest_probability_plays_about_one_in_eight() {
    let (mut s, mut out) = setup();
    s.project.tracks[0].pattern_mut().steps[0].toggle(36);
    s.project.tracks[0].pattern_mut().steps[0].skip = 7;
    s.project.tracks[0].pattern_mut().length = 1;
    tap(&mut s, &mut out, PLAY);
    run(&mut s, &mut out, 0, 6 * 800);
    let plays = out.0.iter().filter(|m| m[0] == 0x90).count();
    assert!(plays > 40 && plays < 200, "{plays} of 800");
}

#[test]
fn a_step_with_full_probability_always_plays() {
    let (mut s, mut out) = setup();
    s.project.tracks[0].pattern_mut().steps[0].toggle(36);
    s.project.tracks[0].pattern_mut().length = 1;
    tap(&mut s, &mut out, PLAY);
    run(&mut s, &mut out, 0, 6 * 50);
    assert_eq!(out.0.iter().filter(|m| m[0] == 0x90).count(), 50);
}

#[test]
fn micro_step_delays_the_note_by_pulses() {
    let (mut s, mut out) = setup();
    tap(&mut s, &mut out, 81);
    view(&mut s, &mut out, V_MICRO);
    press(&mut s, &mut out, 81);
    tap(&mut s, &mut out, 14); // delay 3
    release(&mut s, &mut out, 81);
    assert_eq!(step(&s, 0, 0).delay, 3);
    out.0.clear();
    tap(&mut s, &mut out, PLAY);
    run(&mut s, &mut out, 0, 3);
    assert!(out.0.iter().all(|m| m[0] != 0x90), "not yet: {:?}", out.0);
    run(&mut s, &mut out, 3 * 20_833, 1);
    assert!(out.0.iter().any(|m| m[0] == 0x90));
}

#[test]
fn mutation_view_sets_level_and_mutated_notes_stay_in_the_scale() {
    let (mut s, mut out) = setup();
    tap(&mut s, &mut out, DOWN); // track 2 (scale)
    s.project.tracks[1].pattern_mut().steps[0].toggle(60);
    s.project.tracks[1].pattern_mut().length = 1;
    view(&mut s, &mut out, V_MUTATION);
    press(&mut s, &mut out, 81);
    tap(&mut s, &mut out, 14); // level 3
    release(&mut s, &mut out, 81);
    assert_eq!(step(&s, 1, 0).mutate, 3);
    tap(&mut s, &mut out, PLAY);
    run(&mut s, &mut out, 0, 6 * 300);
    let major = [0, 2, 4, 5, 7, 9, 11];
    let notes: Vec<u8> = out.0.iter().filter(|m| m[0] == 0x91).map(|m| m[1]).collect();
    assert!(notes.iter().all(|n| major.contains(&(n % 12))), "{notes:?}");
    assert!(notes.iter().any(|&n| n != 60), "never mutated");
    assert!(notes.iter().any(|&n| n == 60), "always mutated");
}

// ---- pattern settings ----------------------------------------------------

#[test]
fn length_setting_shortens_the_loop() {
    let (mut s, mut out) = setup();
    view(&mut s, &mut out, V_SETTINGS);
    tap(&mut s, &mut out, 83); // step 3 -> length 3
    assert_eq!(s.project.tracks[0].pattern().length, 3);
    tap(&mut s, &mut out, PLAY);
    run(&mut s, &mut out, 0, 6 * 3 + 1);
    assert_eq!(s.engine.playhead(0), Some(0));
}

#[test]
fn direction_backward_pingpong_and_random() {
    let (mut s, mut out) = setup();
    s.project.tracks[0].pattern_mut().length = 4;
    s.project.tracks[0].pattern_mut().direction = Direction::Backward as u8;
    tap(&mut s, &mut out, PLAY);
    let mut seen = vec![];
    for i in 0..8u64 {
        run(&mut s, &mut out, i * 6 * 20_833, 6);
        seen.push(s.engine.playhead(0).unwrap());
    }
    assert_eq!(seen, vec![3, 2, 1, 0, 3, 2, 1, 0]);

    let (mut s, mut out) = setup();
    s.project.tracks[0].pattern_mut().length = 4;
    s.project.tracks[0].pattern_mut().direction = Direction::PingPong as u8;
    tap(&mut s, &mut out, PLAY);
    let mut seen = vec![];
    for i in 0..8u64 {
        run(&mut s, &mut out, i * 6 * 20_833, 6);
        seen.push(s.engine.playhead(0).unwrap());
    }
    assert_eq!(seen, vec![0, 1, 2, 3, 2, 1, 0, 1]);

    let (mut s, mut out) = setup();
    s.project.tracks[0].pattern_mut().length = 4;
    s.project.tracks[0].pattern_mut().direction = Direction::Random as u8;
    tap(&mut s, &mut out, PLAY);
    let mut seen = std::collections::BTreeSet::new();
    for i in 0..64u64 {
        run(&mut s, &mut out, i * 6 * 20_833, 6);
        let p = s.engine.playhead(0).unwrap();
        assert!(p < 4);
        seen.insert(p);
    }
    assert!(seen.len() >= 3);
}

#[test]
fn rate_changes_the_step_length() {
    let (mut s, mut out) = setup();
    s.project.tracks[0].pattern_mut().rate = 6; // 1/32: 3 pulses
    tap(&mut s, &mut out, PLAY);
    run(&mut s, &mut out, 0, 7);
    assert_eq!(s.engine.playhead(0), Some(2));
    s.project.tracks[0].pattern_mut().rate = 0; // 1/4: 24 pulses
    let (mut s, mut out) = setup();
    s.project.tracks[0].pattern_mut().rate = 0;
    tap(&mut s, &mut out, PLAY);
    run(&mut s, &mut out, 0, 25);
    assert_eq!(s.engine.playhead(0), Some(1));
}

#[test]
fn swing_delays_every_second_step() {
    let (mut s, mut out) = setup();
    s.project.tracks[0].pattern_mut().steps[1].toggle(36);
    s.project.tracks[0].pattern_mut().swing = 3;
    tap(&mut s, &mut out, PLAY);
    run(&mut s, &mut out, 0, 6 + 2);
    assert!(out.0.iter().all(|m| m[0] != 0x90));
    run(&mut s, &mut out, 8 * 20_833, 2);
    assert!(out.0.iter().any(|m| m[0] == 0x90));
}

#[test]
fn settings_pad_choices_reach_the_pattern_and_the_track() {
    let (mut s, mut out) = setup();
    view(&mut s, &mut out, V_SETTINGS);
    tap(&mut s, &mut out, 62); // direction 1
    tap(&mut s, &mut out, 53); // rate 2
    tap(&mut s, &mut out, 44); // swing 3
    let p = *s.project.tracks[0].pattern();
    assert_eq!((p.direction, p.rate, p.swing), (1, 2, 3));
    view(&mut s, &mut out, V_SETTINGS); // second page
    tap(&mut s, &mut out, 82); // channel 2 (index 1)
    tap(&mut s, &mut out, 72); // channel 10
    assert_eq!(s.project.tracks[0].channel, 9);
    tap(&mut s, &mut out, 63); // track type 2: chromatic
    tap(&mut s, &mut out, 53); // scale 2
    tap(&mut s, &mut out, 41); // scale 8
    tap(&mut s, &mut out, 35); // root pitch class 4
    tap(&mut s, &mut out, 13); // octave 2
    let t = &s.project.tracks[0];
    assert_eq!((t.kind, t.scale), (2, 8));
    assert_eq!(t.root, 2 * 12 + 4);
    view(&mut s, &mut out, V_SETTINGS); // third page
    tap(&mut s, &mut out, 81);
    tap(&mut s, &mut out, 82);
    assert!(s.project.external_clock && s.project.send_clock);
}

// ---- patterns, chains, scenes ------------------------------------------------

#[test]
fn selecting_a_pattern_while_stopped_switches_at_once() {
    let (mut s, mut out) = setup();
    view(&mut s, &mut out, V_PATTERNS);
    tap(&mut s, &mut out, 83); // track 1 (row 8 = track 1)... row 8 is track 0
    tap(&mut s, &mut out, 73); // track 2, pattern 3
    assert_eq!(s.project.tracks[1].current, 2);
    assert_eq!(s.ui.track(), 1);
    assert_eq!(s.project.chain[1], Selection { first: 2, last: 2 });
}

#[test]
fn pattern_edits_go_to_the_selected_pattern_only() {
    let (mut s, mut out) = setup();
    view(&mut s, &mut out, V_PATTERNS);
    tap(&mut s, &mut out, 82);
    view(&mut s, &mut out, V_STEPS);
    tap(&mut s, &mut out, 81);
    assert!(s.project.tracks[0].patterns[1].steps[0].has(36));
    assert!(s.project.tracks[0].patterns[0].steps[0].is_empty());
}

#[test]
fn running_selection_waits_for_the_end_of_the_pass() {
    let (mut s, mut out) = setup();
    s.project.tracks[0].pattern_mut().length = 4;
    s.project.tracks[0].patterns[1].steps[0].toggle(50);
    s.project.tracks[0].patterns[1].length = 4;
    tap(&mut s, &mut out, PLAY);
    run(&mut s, &mut out, 0, 7);
    view(&mut s, &mut out, V_PATTERNS);
    tap(&mut s, &mut out, 82);
    assert_eq!(s.project.tracks[0].current, 0);
    assert_eq!(s.engine.queued(0), Some(Selection { first: 1, last: 1 }));
    run(&mut s, &mut out, 7 * 20_833, 6 * 3);
    assert_eq!(s.project.tracks[0].current, 1);
    assert!(out.0.iter().any(|m| m[1] == 50));
    assert_eq!(s.engine.queued(0), None);
}

#[test]
fn holding_a_pattern_and_pressing_another_in_its_row_makes_a_chain_that_plays_in_turn() {
    let (mut s, mut out) = setup();
    view(&mut s, &mut out, V_PATTERNS);
    press(&mut s, &mut out, 81);
    tap(&mut s, &mut out, 83);
    release(&mut s, &mut out, 81);
    assert_eq!(s.project.chain[0], Selection { first: 0, last: 2 });
    for p in 0..3 {
        s.project.tracks[0].patterns[p].length = 2;
    }
    tap(&mut s, &mut out, PLAY);
    let mut seen = vec![];
    for i in 0..8u64 {
        run(&mut s, &mut out, i * 6 * 20_833, 6);
        seen.push(s.project.tracks[0].current);
    }
    assert_eq!(seen, vec![0, 0, 1, 1, 2, 2, 0, 0]);
}

#[test]
fn clear_and_duplicate_work_on_patterns() {
    let (mut s, mut out) = setup();
    s.project.tracks[0].patterns[0].steps[5].toggle(40);
    view(&mut s, &mut out, V_PATTERNS);
    press(&mut s, &mut out, DUPLICATE);
    tap(&mut s, &mut out, 81);
    tap(&mut s, &mut out, 84);
    release(&mut s, &mut out, DUPLICATE);
    assert!(s.project.tracks[0].patterns[3].steps[5].has(40));
    press(&mut s, &mut out, CLEAR);
    tap(&mut s, &mut out, 84);
    release(&mut s, &mut out, CLEAR);
    assert!(s.project.tracks[0].patterns[3].is_empty());
    assert_eq!(s.project.tracks[0].patterns[3].length, 32);
}

#[test]
fn scenes_recall_one_pattern_per_track_and_can_be_stored() {
    let (mut s, mut out) = setup();
    view(&mut s, &mut out, V_SCENES);
    tap(&mut s, &mut out, 82); // scene 2 (index 1): pattern 1 on every track
    for t in 0..TRACKS {
        assert_eq!(s.project.tracks[t].current, 1);
    }
    assert_eq!(s.project.scene, 1);
    // choose other patterns, then store them in scene 11
    view(&mut s, &mut out, V_PATTERNS);
    tap(&mut s, &mut out, 85);
    tap(&mut s, &mut out, 76);
    view(&mut s, &mut out, V_SCENES);
    press(&mut s, &mut out, SHIFT);
    tap(&mut s, &mut out, 73); // row 7 col 3: scene 11
    release(&mut s, &mut out, SHIFT);
    tap(&mut s, &mut out, 81); // back to scene 1
    assert_eq!(s.project.tracks[0].current, 0);
    tap(&mut s, &mut out, 73);
    assert_eq!(s.project.tracks[0].current, 4);
    assert_eq!(s.project.tracks[1].current, 5);
}

// ---- mixer ------------------------------------------------------------------------

#[test]
fn mute_silences_a_track_and_solo_silences_the_others() {
    let (mut s, mut out) = setup();
    s.project.tracks[0].pattern_mut().steps[0].toggle(36);
    s.project.tracks[1].pattern_mut().steps[0].toggle(48);
    tap(&mut s, &mut out, MUTE); // track 1 muted
    tap(&mut s, &mut out, PLAY);
    s.tick(0, &mut out);
    assert_eq!(out.0, vec![vec![0x91, 48, 100]]);
    tap(&mut s, &mut out, PLAY);
    tap(&mut s, &mut out, MUTE);
    tap(&mut s, &mut out, DOWN);
    tap(&mut s, &mut out, SOLO); // track 2 solo: track 1 now silent
    out.0.clear();
    tap(&mut s, &mut out, PLAY);
    s.tick(1_000_000, &mut out);
    assert!(out.0.iter().all(|m| m[0] & 0x0f == 1));
}

// ---- clock ----------------------------------------------------------------------------

#[test]
fn external_clock_drives_the_steps_and_internal_ticks_do_nothing() {
    let (mut s, mut out) = setup();
    external(&mut s, &mut out);
    run(&mut s, &mut out, 0, 50);
    assert_eq!(s.engine.playhead(0), None);
    pulse(&mut s, &mut out, 1);
    assert_eq!(s.engine.playhead(0), Some(0));
    pulse(&mut s, &mut out, 6);
    assert_eq!(s.engine.playhead(0), Some(1));
}

#[test]
fn external_transport_messages_start_and_stop() {
    let (mut s, mut out) = setup();
    s.project.external_clock = true;
    assert!(s.midi_in([0xfa, 0, 0], &mut out));
    pulse(&mut s, &mut out, 1);
    assert!(s.engine.running());
    assert!(s.midi_in([0xfc, 0, 0], &mut out));
    assert!(!s.engine.running());
}

#[test]
fn internal_clock_ignores_incoming_clock() {
    let (mut s, mut out) = setup();
    assert!(!s.midi_in([0xf8, 0, 0], &mut out));
    assert!(!s.midi_in([0xfa, 0, 0], &mut out));
}

#[test]
fn clock_is_sent_when_enabled() {
    let (mut s, mut out) = setup();
    s.project.send_clock = true;
    tap(&mut s, &mut out, PLAY);
    run(&mut s, &mut out, 0, 4);
    assert_eq!(out.0[0], vec![0xfa]);
    assert!(out.0.iter().filter(|m| **m == vec![0xf8]).count() >= 4);
    tap(&mut s, &mut out, PLAY);
    assert_eq!(out.0.last().unwrap(), &vec![0xfc]);
}

// ---- rendering --------------------------------------------------------------------------------

#[test]
fn render_marks_steps_playhead_and_chord_notes() {
    let (mut s, mut out) = setup();
    tap(&mut s, &mut out, 81);
    let mut frame = [0u32; LED_COUNT];
    s.render(&mut frame);
    assert_ne!(frame[81], frame[82]);
    tap(&mut s, &mut out, PLAY);
    s.tick(0, &mut out);
    s.render(&mut frame);
    assert_eq!(frame[81], 0xffffff);
}

#[test]
fn every_view_renders_without_touching_non_led_indexes() {
    let (mut s, mut out) = setup();
    for button in [V_STEPS, V_PATTERNS, V_SCENES, V_VELOCITY, V_PROBABILITY, V_MICRO, V_MUTATION, V_SETTINGS, V_SETTINGS, V_SETTINGS] {
        view(&mut s, &mut out, button);
        let mut frame = [0u32; LED_COUNT];
        s.render(&mut frame);
        assert_eq!(frame[0], 0);
        assert_eq!(frame[9], 0);
        assert!(frame.iter().any(|&c| c != 0));
    }
}

#[test]
fn shift_shows_the_view_selector_on_the_right_column() {
    let (mut s, mut out) = setup();
    press(&mut s, &mut out, SHIFT);
    let mut frame = [0u32; LED_COUNT];
    s.render(&mut frame);
    assert_eq!(frame[V_STEPS as usize], 0xffffff);
    assert_ne!(frame[V_PATTERNS as usize], 0xffffff);
}

// ---- projects -----------------------------------------------------------------------------

const V_PROJECTS: u8 = 89;

fn busy_project(s: &mut NovSeq) {
    s.project.bpm = 97;
    s.project.send_clock = true;
    s.project.tracks[1].muted = true;
    s.project.tracks[2].kind = TrackKind::Chromatic as u8;
    s.project.tracks[2].root = 41;
    s.project.tracks[3].channel = 9;
    s.project.tracks[0].patterns[2].steps[7].toggle(36);
    s.project.tracks[0].patterns[2].steps[7].toggle(40);
    s.project.tracks[0].patterns[2].steps[7].gate = 12;
    s.project.tracks[0].patterns[2].steps[7].skip = 3;
    s.project.tracks[0].patterns[2].steps[7].delay = 4;
    s.project.tracks[0].patterns[2].steps[7].mutate = 2;
    s.project.tracks[0].patterns[2].steps[7].velocity = 71;
    s.project.tracks[0].patterns[2].length = 12;
    s.project.tracks[0].patterns[2].direction = 2;
    s.project.tracks[0].patterns[2].rate = 5;
    s.project.tracks[0].patterns[2].swing = 2;
    s.project.tracks[1].patterns[7].steps[31].toggle(100);
    s.project.chain[0] = Selection { first: 1, last: 2 };
    s.project.scenes[9][3] = Selection { first: 4, last: 6 };
    s.project.scene = 9;
    s.project.tracks[0].current = 2;
}

fn same(a: &Project, b: &Project) -> bool {
    let mut x = vec![0u8; MAX_PROJECT_BYTES];
    let mut y = vec![0u8; MAX_PROJECT_BYTES];
    let (lx, ly) = (encode(a, &mut x).unwrap(), encode(b, &mut y).unwrap());
    x[..lx] == y[..ly]
}

#[test]
fn a_project_round_trips_through_the_encoding() {
    let (mut s, _) = setup();
    busy_project(&mut s);
    let mut buf = vec![0u8; MAX_PROJECT_BYTES];
    let len = encode(&s.project, &mut buf).unwrap();
    assert!(len < 1200, "{len}");
    let (mut t, _) = setup();
    assert!(decode(&mut t.project, &buf[..len]));
    assert!(t.project.tracks[0].patterns[2] == s.project.tracks[0].patterns[2]);
    assert!(t.project.tracks[1].patterns[7].steps[31].has(100));
    assert_eq!(t.project.bpm, 97);
    assert!(t.project.send_clock && t.project.tracks[1].muted);
    assert_eq!(t.project.scenes[9][3], Selection { first: 4, last: 6 });
    assert_eq!(t.project.scene, 9);
    assert_eq!(t.project.tracks[2].root, 41);
    assert!(same(&s.project, &t.project));
}

#[test]
fn a_full_project_does_not_fit_and_is_not_saved() {
    let (mut s, _) = setup();
    for track in s.project.tracks.iter_mut() {
        for pattern in track.patterns.iter_mut() {
            for step in pattern.steps.iter_mut() {
                for n in 1..=8 {
                    step.toggle(n);
                }
            }
        }
    }
    let mut buf = vec![0u8; MAX_PROJECT_BYTES];
    assert!(encode(&s.project, &mut buf).is_none());
}

#[test]
fn damaged_or_foreign_data_is_refused_and_the_project_stays_untouched() {
    let (mut s, _) = setup();
    busy_project(&mut s);
    let mut buf = vec![0u8; MAX_PROJECT_BYTES];
    let len = encode(&s.project, &mut buf).unwrap();
    let (mut t, _) = setup();
    t.project.bpm = 77;
    for cut in [0, 3, 10, len / 2, len - 1] {
        assert!(!decode(&mut t.project, &buf[..cut]), "cut {cut}");
    }
    let mut extra = buf[..len].to_vec();
    extra.push(0);
    assert!(!decode(&mut t.project, &extra));
    let mut seq95 = buf[..len].to_vec();
    seq95[..6].copy_from_slice(b"SEQ95P");
    assert!(!decode(&mut t.project, &seq95));
    // every single flipped byte is either refused or leaves a playable project
    for i in 0..len {
        let mut bad = buf[..len].to_vec();
        bad[i] ^= 0xff;
        let mut p = Project::new();
        if decode(&mut p, &bad) {
            assert!(p.ready && p.bpm >= MIN_BPM && p.tracks.iter().all(|t| t.channel < 16 && t.root >= 1));
        }
    }
    assert_eq!(t.project.bpm, 77);
}

#[test]
fn save_and_load_through_the_projects_view() {
    let (mut s, mut out) = setup();
    busy_project(&mut s);
    tap(&mut s, &mut out, V_PROJECTS);
    assert!(s.ui.view() == View::Projects);
    tap(&mut s, &mut out, 73); // save to slot 3
    let mut frame = [0u32; LED_COUNT];
    s.render(&mut frame);
    assert_eq!(frame[73], 0x00ff00);
    let before = s.project.bpm;
    s.project.bpm = 150;
    s.project.tracks[0].patterns[2].reset();
    tap(&mut s, &mut out, 83); // load slot 3
    assert_eq!(s.project.bpm, before);
    assert!(!s.project.tracks[0].patterns[2].is_empty());
    tap(&mut s, &mut out, 84); // empty slot: refused, nothing changes
    s.render(&mut frame);
    assert_eq!(frame[84], 0xff0000);
    assert_eq!(s.project.bpm, before);
}

#[test]
fn a_failed_save_is_shown_and_loading_replaces_a_running_sequence_cleanly() {
    let (mut s, mut out) = setup();
    busy_project(&mut s);
    tap(&mut s, &mut out, V_PROJECTS);
    FAIL_SAVES.with(|f| f.set(true));
    tap(&mut s, &mut out, 71);
    let mut frame = [0u32; LED_COUNT];
    s.render(&mut frame);
    assert_eq!(frame[71], 0xff0000);
    FAIL_SAVES.with(|f| f.set(false));
    tap(&mut s, &mut out, 71);
    tap(&mut s, &mut out, PLAY);
    run(&mut s, &mut out, 0, 30);
    tap(&mut s, &mut out, 81);
    assert!(!s.engine.running(), "loading stops the sequence");
}

#[test]
fn new_project_resets_everything() {
    let (mut s, mut out) = setup();
    busy_project(&mut s);
    tap(&mut s, &mut out, V_PROJECTS);
    tap(&mut s, &mut out, 58);
    assert_eq!(s.project.bpm, DEFAULT_BPM);
    assert!(s.project.tracks[0].patterns[2].is_empty());
}

#[test]
fn projects_button_toggles_the_view() {
    let (mut s, mut out) = setup();
    tap(&mut s, &mut out, V_PROJECTS);
    tap(&mut s, &mut out, V_PROJECTS);
    assert!(s.ui.view() == View::Steps);
    view(&mut s, &mut out, V_STEPS);
    assert!(s.ui.view() == View::Steps);
}
