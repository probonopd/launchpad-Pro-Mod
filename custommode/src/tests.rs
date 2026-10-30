// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 launchpad-Pro-Mod contributors

use super::*;
use std::vec::Vec;

#[derive(Default)]
struct Rec {
    events: Vec<(&'static str, u8, u8, u8)>,
}

impl ModeOut for Rec {
    fn note_on(&mut self, c: u8, n: u8, v: u8) {
        self.events.push(("on", c, n, v));
    }
    fn note_off(&mut self, c: u8, n: u8) {
        self.events.push(("off", c, n, 0));
    }
    fn control_change(&mut self, c: u8, cc: u8, v: u8) {
        self.events.push(("cc", c, cc, v));
    }
    fn program_change(&mut self, c: u8, p: u8) {
        self.events.push(("pc", c, p, 0));
    }
}

fn modes() -> CustomModes {
    let mut m = CustomModes::new();
    m.start();
    m
}

fn go(m: &mut CustomModes, mode: u8) {
    m.select(mode, &mut Rec::default());
}

#[test]
fn custom_one_has_eight_vertical_unipolar_faders_on_cc_7_to_14() {
    let mut m = modes();
    let mut out = Rec::default();
    m.pad(11, true, 100, &mut out); // column 1, bottom: value 0
    m.pad(81, true, 100, &mut out); // column 1, top: value 127
    m.pad(48, true, 100, &mut out); // column 8, row 4 (position 3)
    assert_eq!(out.events, vec![("cc", 0, 7, 0), ("cc", 0, 7, 127), ("cc", 0, 14, 54)]);
}

#[test]
fn custom_two_has_horizontal_bipolar_faders_on_cc_15_to_22() {
    let mut m = modes();
    go(&mut m, 1);
    let mut out = Rec::default();
    m.pad(88, true, 100, &mut out); // top row = first fader, rightmost pad
    m.pad(11, true, 100, &mut out); // bottom row = last fader, leftmost pad
    assert_eq!(out.events, vec![("cc", 0, 15, 127), ("cc", 0, 22, 0)]);
    let mut frame = [0u32; LED_COUNT];
    m.render(&mut frame);
    // Top fader at position 7: pads 5..=8 lit (above the middle), 1..=4 dark.
    assert_ne!(frame[85], 0x080808);
    assert_ne!(frame[88], 0x080808);
    assert_eq!(frame[84], 0x080808);
    // A fader never moved sits in the middle: only one pad lit.
    assert_ne!(frame[25], 0x080808);
    assert_eq!(frame[24], 0x080808);
    assert_eq!(frame[26], 0x080808);
}

#[test]
fn drum_rack_plays_notes_with_the_velocity_of_the_press() {
    let mut m = modes();
    go(&mut m, 2);
    let mut out = Rec::default();
    m.pad(11, true, 90, &mut out);
    m.pad(11, false, 0, &mut out);
    m.pad(88, true, 50, &mut out);
    assert_eq!(out.events, vec![("on", 0, 36, 90), ("off", 0, 36, 0), ("on", 0, 99, 50)]);
}

#[test]
fn keyboard_layout_has_white_keys_below_black_keys() {
    let mut m = modes();
    go(&mut m, 3);
    let mut out = Rec::default();
    for pad in [11, 12, 21, 22, 23, 13, 18] {
        m.pad(pad, true, 100, &mut out);
        m.pad(pad, false, 0, &mut out);
    }
    let notes: Vec<u8> = out.events.iter().filter(|e| e.0 == "on").map(|e| e.2).collect();
    // C D, (row 2 col 1 is empty), C# D#, E, C(next)
    assert_eq!(notes, vec![36, 38, 37, 39, 40, 48]);
    // A pad without a key does nothing.
    let before = out.events.len();
    m.pad(21, true, 100, &mut out);
    assert_eq!(out.events.len(), before);
}

#[test]
fn program_change_modes_cover_0_to_127() {
    let mut m = modes();
    go(&mut m, 4);
    let mut out = Rec::default();
    m.pad(11, true, 100, &mut out);
    m.pad(88, true, 100, &mut out);
    go(&mut m, 5);
    m.pad(11, true, 100, &mut out);
    m.pad(88, true, 100, &mut out);
    let programs: Vec<u8> = out.events.iter().map(|e| e.2).collect();
    assert_eq!(programs, vec![0, 63, 64, 127]);
}

#[test]
fn legacy_modes_are_lit_by_incoming_notes() {
    let mut m = modes();
    go(&mut m, 7); // programmer layout
    let mut out = Rec::default();
    m.pad(45, true, 100, &mut out);
    m.pad(45, false, 0, &mut out);
    assert_eq!(out.events[0], ("on", 0, 45, 100));
    let mut frame = [0u32; LED_COUNT];
    m.render(&mut frame);
    assert_eq!(frame[45], 0);
    assert!(m.midi_in(0x90, 45, 21));
    m.render(&mut frame);
    assert_eq!(frame[45], PALETTE_FLAG | 21);
    assert!(m.midi_in(0x80, 45, 0));
    m.render(&mut frame);
    assert_eq!(frame[45], 0);
    // Other channels and the non-lit modes do not light.
    assert!(!m.midi_in(0x91, 45, 21));
    go(&mut m, 2);
    assert!(!m.midi_in(0x90, 36, 21));
    go(&mut m, 6); // unlit drum rack
    assert!(m.midi_in(0x90, 36, 5));
}

#[test]
fn toggle_pads_send_on_then_off() {
    let mut m = modes();
    let mut def = ModeDef::EMPTY;
    def.pads[0] = PadDef {
        action: Action::Cc { cc: 20, on: 100, off: 10 },
        toggle: true,
        channel: Some(4),
        ..PadDef::NONE
    };
    let mut out = Rec::default();
    m.load(def, &mut out);
    for _ in 0..2 {
        m.pad(11, true, 100, &mut out);
        m.pad(11, false, 0, &mut out);
    }
    assert_eq!(out.events, vec![("cc", 4, 20, 100), ("cc", 4, 20, 10)]);
}

#[test]
fn master_channel_is_per_mode_and_chosen_with_the_channel_view() {
    let mut m = modes();
    let mut out = Rec::default();
    m.set_channel_view(true, &mut out);
    m.pad(13, true, 100, &mut out); // channel 3
    m.pad(25, true, 100, &mut out); // channel 13
    assert_eq!(m.master_channel(), 12);
    // Playing is off while choosing.
    assert!(out.events.is_empty());
    m.set_channel_view(false, &mut out);
    m.pad(11, true, 100, &mut out);
    assert_eq!(out.events, vec![("cc", 12, 7, 0)]);
    // Another mode has its own channel.
    go(&mut m, 2);
    assert_eq!(m.master_channel(), 0);
    go(&mut m, 0);
    assert_eq!(m.master_channel(), 12);
    let mut frame = [0u32; LED_COUNT];
    m.set_channel_view(true, &mut out);
    m.render(&mut frame);
    assert_eq!(frame[25], GREEN);
    assert_eq!(frame[11], DIM_RED);
}

#[test]
fn switching_modes_releases_held_notes() {
    let mut m = modes();
    go(&mut m, 2);
    let mut out = Rec::default();
    m.pad(11, true, 100, &mut out);
    m.select(3, &mut out);
    assert_eq!(out.events, vec![("on", 0, 36, 100), ("off", 0, 36, 0)]);
}

#[test]
fn mode_buttons_run_top_to_bottom_and_show_the_chosen_mode() {
    assert_eq!(CustomModes::mode_for_button_row(8), Some(0));
    assert_eq!(CustomModes::mode_for_button_row(1), Some(7));
    assert_eq!(CustomModes::mode_for_button_row(9), None);
    let mut m = modes();
    go(&mut m, 2);
    let mut frame = [0u32; LED_COUNT];
    m.render(&mut frame);
    assert_eq!(frame[69], PALE_GREEN); // mode 3 = row 6
    assert_eq!(frame[89], DIM_WHITE);
}

#[test]
fn aftertouch_target_follows_held_note_pads() {
    let mut m = modes();
    go(&mut m, 2);
    let mut out = Rec::default();
    assert_eq!(m.aftertouch_target(11), None);
    m.pad(11, true, 100, &mut out);
    assert_eq!(m.aftertouch_target(11), Some((0, 36)));
    m.pad(11, false, 0, &mut out);
    assert_eq!(m.aftertouch_target(11), None);
}

#[test]
fn indexes_outside_the_grid_are_ignored() {
    let mut m = modes();
    let mut out = Rec::default();
    for i in [0u8, 9, 10, 19, 90, 99, 100, 255] {
        m.pad(i, true, 100, &mut out);
        m.pad(i, false, 0, &mut out);
    }
    assert!(out.events.is_empty());
}

#[test]
fn preferences_round_trip_and_invalid_ones_are_refused() {
    let mut m = modes();
    let mut out = Rec::default();
    m.set_channel_view(true, &mut out);
    m.pad(13, true, 100, &mut out); // channel 3 for Custom 1
    m.set_channel_view(false, &mut out);
    go(&mut m, 5);
    let saved = m.export_prefs();
    let mut fresh = modes();
    assert!(fresh.import_prefs(&saved));
    assert_eq!(fresh.mode(), 5);
    go(&mut fresh, 0);
    assert_eq!(fresh.master_channel(), 2);
    let mut bad = saved;
    bad[1] = 8;
    assert!(!fresh.import_prefs(&bad));
    let mut bad = saved;
    bad[4] = 16;
    assert!(!fresh.import_prefs(&bad));
    assert!(!fresh.import_prefs(&saved[..5]));
}

#[test]
fn saving_is_requested_when_idle() {
    let mut m = modes();
    let mut out = Rec::default();
    assert!(!m.take_dirty_when_idle());
    go(&mut m, 3);
    m.pad(11, true, 100, &mut out);
    assert!(!m.take_dirty_when_idle(), "not while a pad is held");
    m.pad(11, false, 0, &mut out);
    assert!(m.take_dirty_when_idle());
    assert!(!m.take_dirty_when_idle());
}
