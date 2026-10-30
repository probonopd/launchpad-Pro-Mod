// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 launchpad-Pro-Mod contributors

use super::*;
use std::vec::Vec;

#[derive(Default)]
struct Rec {
    on: Vec<(u8, u8, u8)>,
    off: Vec<(u8, u8)>,
}

impl NoteOut for Rec {
    fn note_on(&mut self, c: u8, n: u8, v: u8) {
        self.on.push((c, n, v));
    }
    fn note_off(&mut self, c: u8, n: u8) {
        self.off.push((c, n));
    }
}

fn grid(m: &NoteMode) -> Vec<Vec<Option<u8>>> {
    (1..=8).map(|r| (1..=8).map(|c| m.note_for(r, c)).collect()).collect()
}

#[test]
fn chromatic_default_is_a_guitar_like_layout() {
    let m = NoteMode::new();
    // Overlap 5: an octave is two rows up and two pads across, and the first
    // pad of a row equals the sixth pad of the row below.
    assert_eq!(m.note_for(1, 1), Some(36));
    assert_eq!(m.note_for(1, 2), Some(37));
    assert_eq!(m.note_for(2, 1), Some(41));
    assert_eq!(m.note_for(2, 1), m.note_for(1, 6));
    assert_eq!(m.note_for(3, 3), Some(36 + 12));
}

#[test]
fn sequential_chromatic_gives_every_pad_a_unique_note() {
    let mut m = NoteMode::new();
    m.button(NOTE, true, &mut Rec::default());
    m.pad(81, true, 100, &mut Rec::default()); // sequential
    m.button(NOTE, true, &mut Rec::default());
    let g = grid(&m);
    let mut notes: Vec<u8> = g.iter().flatten().flatten().copied().collect();
    let total = notes.len();
    notes.sort();
    notes.dedup();
    assert_eq!(notes.len(), total);
    assert_eq!(m.note_for(1, 1), Some(36));
    assert_eq!(m.note_for(2, 1), Some(44));
}

#[test]
fn scale_layout_plays_only_scale_notes_from_the_root() {
    let mut m = NoteMode::new();
    let mut out = Rec::default();
    m.button(NOTE, true, &mut out);
    m.pad(12, true, 100, &mut out); // Scale layout
    m.pad(66, true, 100, &mut out); // 'A' as root? column 6 white key = A (pc 9)
    m.pad(52, true, 100, &mut out); // scale index 1 = Natural Major
    m.button(NOTE, true, &mut out);
    assert_eq!(m.layout(), Layout::Scale);
    // Root A (pc 9), major: bottom row climbs A B C# D E F# G# A.
    let row: Vec<u8> = (1..=8).map(|c| m.note_for(1, c).unwrap()).collect();
    assert_eq!(row, vec![45, 47, 49, 50, 52, 54, 56, 57]);
    for note in grid(&m).iter().flatten().flatten() {
        let rel = (*note as i16 - 9).rem_euclid(12);
        assert!([0, 2, 4, 5, 7, 9, 11].contains(&rel), "note {note} not in A major");
    }
}

#[test]
fn sequential_scale_only_overlays_octaves_of_the_root() {
    let mut m = NoteMode::new();
    let mut out = Rec::default();
    m.button(NOTE, true, &mut out);
    m.pad(12, true, 100, &mut out);
    m.pad(81, true, 100, &mut out);
    m.button(NOTE, true, &mut out);
    // Seven-note scale: the 8th pad is the root an octave up, and so is the
    // first pad of the next row.
    assert_eq!(m.note_for(1, 8), m.note_for(2, 1));
}

#[test]
fn octave_and_semitone_shifts_and_range() {
    let mut m = NoteMode::new();
    let mut out = Rec::default();
    m.button(UP, true, &mut out);
    assert_eq!(m.note_for(1, 1), Some(48));
    m.button(RIGHT, true, &mut out);
    assert_eq!(m.note_for(1, 1), Some(49));
    m.button(LEFT, true, &mut out);
    m.button(LEFT, true, &mut out);
    assert_eq!(m.note_for(1, 1), Some(47));
    for _ in 0..30 {
        m.button(UP, true, &mut out);
    }
    // Clamped, and pads beyond 127 are dark and silent.
    assert!(m.note_for(8, 8).is_none());
    assert!(m.note_for(1, 1).is_some());
}

#[test]
fn release_matches_the_pad_even_after_a_shift() {
    let mut m = NoteMode::new();
    let mut out = Rec::default();
    m.pad(11, true, 90, &mut out);
    m.button(UP, true, &mut out); // octave shift while held
    m.pad(11, false, 0, &mut out);
    assert_eq!(out.on, vec![(0, 36, 90)]);
    assert_eq!(out.off, vec![(0, 36)]);
}

#[test]
fn two_pads_on_one_note_release_it_only_when_both_are_up() {
    let mut m = NoteMode::new();
    let mut out = Rec::default();
    // Overlap 5: pad 21 (row 2 col 1) and pad 16 share a note.
    assert_eq!(m.note_for(2, 1), m.note_for(1, 6));
    m.pad(21, true, 100, &mut out);
    m.pad(16, true, 100, &mut out);
    m.pad(21, false, 0, &mut out);
    assert!(out.off.is_empty());
    m.pad(16, false, 0, &mut out);
    assert_eq!(out.off.len(), 1);
}

#[test]
fn drum_layout_has_four_blocks_and_scrolls() {
    let mut m = NoteMode::new();
    let mut out = Rec::default();
    m.button(NOTE, true, &mut out);
    m.pad(13, true, 100, &mut out); // Drum
    m.button(NOTE, true, &mut out);
    assert_eq!(m.layout(), Layout::Drum);
    assert_eq!(m.note_for(1, 1), Some(36));
    assert_eq!(m.note_for(4, 4), Some(51));
    assert_eq!(m.note_for(5, 1), Some(52));
    assert_eq!(m.note_for(1, 5), Some(68));
    assert_eq!(m.note_for(8, 8), Some(99));
    m.button(UP, true, &mut out);
    assert_eq!(m.note_for(1, 1), Some(52));
    m.button(RIGHT, true, &mut out);
    assert_eq!(m.note_for(1, 1), Some(56));
}

#[test]
fn settings_channel_and_notes_stop_when_opening_them() {
    let mut m = NoteMode::new();
    let mut out = Rec::default();
    m.pad(11, true, 100, &mut out);
    m.button(NOTE, true, &mut out);
    assert_eq!(out.off.len(), 1, "opening settings releases sounding notes");
    m.pad(38, true, 100, &mut out); // row 3 col 8 = channel 8
    assert_eq!(m.channel(), 7);
    m.pad(22, true, 100, &mut out); // row 2 col 2 = channel 10
    assert_eq!(m.channel(), 9);
    // Pads in settings play nothing.
    let before = out.on.len();
    m.pad(55, true, 100, &mut out);
    assert_eq!(out.on.len(), before);
}

#[test]
fn render_marks_root_scale_and_pressed_pads() {
    let mut m = NoteMode::new();
    let mut out = Rec::default();
    let mut frame = [0u32; LED_COUNT];
    m.render(&mut frame);
    assert_eq!(frame[11], PURPLE); // C is the root of the default C minor
    assert_eq!(frame[12], 0); // C# is outside C minor, blank in chromatic
    m.pad(11, true, 100, &mut out);
    m.render(&mut frame);
    assert_eq!(frame[11], WHITE);
    assert_eq!(frame[NOTE as usize], GREEN);
}

#[test]
fn indexes_outside_the_grid_are_ignored() {
    let mut m = NoteMode::new();
    let mut out = Rec::default();
    for index in [0u8, 9, 10, 19, 90, 99, 100, 255] {
        m.pad(index, true, 100, &mut out);
        m.pad(index, false, 0, &mut out);
    }
    assert!(out.on.is_empty() && out.off.is_empty());
    assert!(!m.button(1, true, &mut out));
}

mod chord_layout {
    use super::*;

    /// Opens the settings, chooses the chord layout, closes them again.
    fn chord_mode() -> NoteMode {
        let mut m = NoteMode::new();
        let mut out = Rec::default();
        m.button(NOTE, true, &mut out);
        m.pad(14, true, 100, &mut out);
        m.button(NOTE, true, &mut out);
        assert_eq!(m.layout(), Layout::Chord);
        m
    }

    fn sorted(chord: &[u8]) -> Vec<u8> {
        let mut v = chord.to_vec();
        v.sort();
        v
    }

    #[test]
    fn note_area_first_column_runs_through_the_scale() {
        let m = chord_mode();
        // C natural minor from C2 (36): C D Eb F G Ab Bb C
        let col: Vec<u8> = (1..=8).map(|r| m.note_for(r, 2).unwrap()).collect();
        assert_eq!(col, vec![36, 38, 39, 41, 43, 44, 46, 48]);
    }

    #[test]
    fn a_row_holds_root_third_up_fifth_seventh_fifth_up() {
        let m = chord_mode();
        // Bottom row: C minor: root C, third Eb an octave up, fifth G, seventh Bb, fifth G up.
        let row: Vec<u8> = (2..=6).map(|c| m.note_for(1, c).unwrap()).collect();
        assert_eq!(row, vec![36, 51, 43, 46, 55]);
        assert!(m.note_for(1, 7).is_none() && m.note_for(1, 1).is_none());
    }

    #[test]
    fn triad_column_plays_three_notes_inside_one_octave() {
        let mut m = chord_mode();
        let mut out = Rec::default();
        m.pad(11, true, 100, &mut out); // degree 1: C minor triad
        let on: Vec<u8> = out.on.iter().map(|x| x.1).collect();
        assert_eq!(sorted(&on), vec![36, 39, 43]);
        m.pad(41, true, 100, &mut out); // degree 4: F minor triad, folded under C+12
        let on: Vec<u8> = out.on[3..].iter().map(|x| x.1).collect();
        assert!(on.iter().all(|n| (36..48).contains(n)), "{on:?}");
        assert_eq!(sorted(&on), vec![36, 41, 44]);
    }

    #[test]
    fn filling_a_slot_by_holding_it_then_playing() {
        let mut m = chord_mode();
        let mut out = Rec::default();
        m.pad(27, true, 100, &mut out); // empty bank slot 0 held (row 2, col 7)
        m.pad(11, true, 100, &mut out); // triad pad adds C Eb G
        m.pad(11, false, 0, &mut out);
        m.pad(27, false, 0, &mut out); // let go: assignment ends
        out.on.clear();
        m.pad(21, true, 100, &mut out); // an ordinary pad is not added any more
        m.pad(21, false, 0, &mut out);
        out.on.clear();
        m.pad(27, true, 90, &mut out); // the slot now plays the chord
        let on: Vec<u8> = out.on.iter().map(|x| x.1).collect();
        assert_eq!(sorted(&on), vec![36, 39, 43]);
    }

    #[test]
    fn filling_a_slot_with_notes_held_first() {
        let mut m = chord_mode();
        let mut out = Rec::default();
        m.pad(12, true, 100, &mut out); // C (row 1 col 2)
        m.pad(14, true, 100, &mut out); // G (row 1 col 4)
        m.pad(38, true, 100, &mut out); // empty slot 7 (row 3, col 8): takes the held notes
        m.pad(38, false, 0, &mut out);
        m.pad(12, false, 0, &mut out);
        m.pad(14, false, 0, &mut out);
        out.on.clear();
        m.pad(38, true, 100, &mut out);
        let on: Vec<u8> = out.on.iter().map(|x| x.1).collect();
        assert_eq!(sorted(&on), vec![36, 43]);
    }

    #[test]
    fn lock_stops_new_assignments() {
        let mut m = chord_mode();
        let mut out = Rec::default();
        m.pad(18, true, 100, &mut out); // lock on (row 1, col 8)
        m.pad(18, false, 0, &mut out);
        m.pad(27, true, 100, &mut out); // empty slot: nothing is assigned
        m.pad(11, true, 100, &mut out);
        m.pad(11, false, 0, &mut out);
        m.pad(27, false, 0, &mut out);
        out.on.clear();
        m.pad(27, true, 100, &mut out);
        assert!(out.on.is_empty(), "slot must still be empty");
    }

    #[test]
    fn sustain_keeps_notes_until_switched_off_and_retriggers() {
        let mut m = chord_mode();
        let mut out = Rec::default();
        m.pad(17, true, 100, &mut out); // sustain on (row 1, col 7)
        m.pad(17, false, 0, &mut out);
        m.pad(12, true, 100, &mut out);
        m.pad(12, false, 0, &mut out);
        assert!(out.off.is_empty(), "sustained note must keep sounding");
        m.pad(12, true, 110, &mut out); // pressing it again plays it again
        m.pad(12, false, 0, &mut out);
        assert_eq!(out.on.len(), 2);
        m.pad(17, true, 100, &mut out); // sustain off releases it
        assert_eq!(out.off, vec![(0, 36)]);
    }

    #[test]
    fn chord_layout_renders_triads_orange_and_bank_white() {
        let mut m = chord_mode();
        let mut out = Rec::default();
        let mut frame = [0u32; LED_COUNT];
        m.render(&mut frame);
        assert_eq!(frame[11], 0xff6000);
        assert_eq!(frame[12], PURPLE); // C in the note area
        assert_eq!(frame[13], BLUE);
        m.pad(27, true, 100, &mut out);
        m.pad(11, true, 100, &mut out);
        m.pad(11, false, 0, &mut out);
        m.pad(27, false, 0, &mut out);
        m.render(&mut frame);
        assert_eq!(frame[27], WHITE);
        assert_eq!(frame[28], DIM_WHITE);
    }
}

mod prefs {
    use super::*;

    #[test]
    fn preferences_round_trip_including_the_chord_bank() {
        let mut m = NoteMode::new();
        let mut out = Rec::default();
        m.button(NOTE, true, &mut out);
        m.pad(12, true, 100, &mut out); // Scale layout
        m.pad(66, true, 100, &mut out); // root A
        m.pad(52, true, 100, &mut out); // Natural Major
        m.pad(22, true, 100, &mut out); // channel 10
        m.pad(14, true, 100, &mut out); // Chord layout
        m.button(NOTE, true, &mut out);
        m.pad(27, true, 100, &mut out);
        m.pad(11, true, 100, &mut out);
        m.pad(11, false, 0, &mut out);
        m.pad(27, false, 0, &mut out); // a chord saved in slot 0
        let saved = m.export_prefs();
        let mut fresh = NoteMode::new();
        assert!(fresh.import_prefs(&saved));
        assert_eq!(fresh.export_prefs(), saved);
        assert_eq!(fresh.layout(), Layout::Chord);
        assert_eq!(fresh.channel(), 9);
        let mut out = Rec::default();
        fresh.pad(27, true, 100, &mut out);
        assert!(!out.on.is_empty(), "the saved chord plays");
    }

    #[test]
    fn invalid_preferences_are_refused_and_change_nothing() {
        let good = NoteMode::new().export_prefs();
        for (i, bad) in [(0usize, 9u8), (1, 7), (2, 1), (2, 6), (3, 16), (4, 12), (5, 16), (6, 7), (7, 200)] {
            let mut data = good;
            data[i] = bad;
            let mut m = NoteMode::new();
            assert!(!m.import_prefs(&data), "byte {i} = {bad}");
            assert_eq!(m.export_prefs(), good, "state must be untouched");
        }
        let mut m = NoteMode::new();
        assert!(!m.import_prefs(&good[..good.len() - 1]));
        assert!(!m.import_prefs(&[]));
    }

    #[test]
    fn saving_is_requested_only_when_settings_are_closed_and_nothing_sounds() {
        let mut m = NoteMode::new();
        let mut out = Rec::default();
        assert!(!m.take_dirty_when_idle());
        m.button(NOTE, true, &mut out);
        m.pad(13, true, 100, &mut out); // a change while the settings are open
        assert!(!m.take_dirty_when_idle(), "not while the settings are open");
        m.button(NOTE, true, &mut out);
        m.pad(11, true, 100, &mut out);
        assert!(!m.take_dirty_when_idle(), "not while a note sounds");
        m.pad(11, false, 0, &mut out);
        assert!(m.take_dirty_when_idle());
        assert!(!m.take_dirty_when_idle(), "only once");
    }
}
