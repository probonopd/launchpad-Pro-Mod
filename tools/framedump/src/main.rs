// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Simon Peter
//! Renders the real mode code into LED frames and prints them as JSON, so the
//! pictures in the manual always show what the firmware shows.
//! Run: cd tools/framedump && cargo run --quiet | python3 -I ../make_pictures.py ../../docs/img

use custommode::{CustomModes, ModeOut};
use notemode::{NoteMode, NoteOut};
use seq95::xmap::{XSurface, X_CAPTURE, X_DOWN, X_LED_COUNT};
use seq95::{MidiOut, Seq95, Storage};

struct Nobody;
impl NoteOut for Nobody {
    fn note_on(&mut self, _: u8, _: u8, _: u8) {}
    fn note_off(&mut self, _: u8, _: u8) {}
}
impl ModeOut for Nobody {
    fn note_on(&mut self, _: u8, _: u8, _: u8) {}
    fn note_off(&mut self, _: u8, _: u8) {}
    fn control_change(&mut self, _: u8, _: u8, _: u8) {}
    fn program_change(&mut self, _: u8, _: u8) {}
}
impl MidiOut for Nobody {
    fn send(&mut self, _: u8, _: &[u8]) {}
}
impl Storage for Nobody {
    fn slot_used(&mut self, _: u8) -> bool {
        false
    }
    fn save_with(&mut self, _: u8, _: impl FnOnce(&mut [u8]) -> usize) -> bool {
        false
    }
    fn load_with(&mut self, _: u8, _: impl FnOnce(&[u8]) -> bool) -> bool {
        false
    }
}

fn frame_json(name: &str, frame: &[u32], notes: Option<Vec<Option<u8>>>) -> String {
    let colors: Vec<String> = frame.iter().map(|c| c.to_string()).collect();
    let notes = match notes {
        Some(n) => format!(
            "[{}]",
            n.iter().map(|x| x.map_or("null".to_string(), |v| v.to_string())).collect::<Vec<_>>().join(",")
        ),
        None => "null".to_string(),
    };
    format!("\"{name}\":{{\"frame\":[{}],\"notes\":{notes}}}", colors.join(","))
}

fn note_grid(m: &NoteMode) -> Vec<Option<u8>> {
    // Row 1 (bottom) to row 8, columns 1 to 8.
    (1..=8u8).flat_map(|r| (1..=8u8).map(move |c| (r, c))).map(|(r, c)| m.note_for(r, c)).collect()
}

fn main() {
    let mut out = Vec::new();
    let mut o = Nobody;
    let mut st = Nobody;
    let mut f = [0u32; notemode::LED_COUNT];

    let mut m = NoteMode::new();
    m.render(&mut f);
    out.push(frame_json("note_chromatic", &f, Some(note_grid(&m))));

    // Settings page: Note, then choose layouts via the bottom row.
    m.button(notemode::NOTE, true, &mut o);
    m.render(&mut f);
    out.push(frame_json("note_settings", &f, None));
    m.pad(12, true, 100, &mut o); // Scale
    m.button(notemode::NOTE, true, &mut o);
    m.render(&mut f);
    out.push(frame_json("note_scale", &f, Some(note_grid(&m))));
    m.button(notemode::NOTE, true, &mut o);
    m.pad(13, true, 100, &mut o); // Drum
    m.button(notemode::NOTE, true, &mut o);
    m.render(&mut f);
    out.push(frame_json("note_drum", &f, Some(note_grid(&m))));
    m.button(notemode::NOTE, true, &mut o);
    m.pad(14, true, 100, &mut o); // Chord
    m.button(notemode::NOTE, true, &mut o);
    m.render(&mut f);
    out.push(frame_json("note_chord", &f, Some(note_grid(&m))));
    // Chord with one filled bank slot, lock off, sustain on, a pad held.
    m.pad(27, true, 100, &mut o);
    m.pad(11, true, 100, &mut o);
    m.pad(11, false, 0, &mut o);
    m.pad(27, false, 0, &mut o);
    m.pad(17, true, 100, &mut o);
    m.pad(17, false, 0, &mut o);
    m.pad(14, true, 100, &mut o);
    m.render(&mut f);
    out.push(frame_json("note_chord_used", &f, Some(note_grid(&m))));

    let mut c = CustomModes::new();
    c.start();
    let mut cf = [0u32; custommode::LED_COUNT];
    for mode in 0..8u8 {
        c.select(mode, &mut o);
        c.render(&mut cf);
        out.push(frame_json(&format!("custom_{}", mode + 1), &cf, None));
    }
    c.select(0, &mut o);
    c.set_channel_view(true, &mut o);
    c.render(&mut cf);
    out.push(frame_json("custom_channels", &cf, None));

    let mut seq = Seq95::new();
    let mut x = XSurface::new();
    let mut xf = [0u32; X_LED_COUNT];
    x.render(&seq, &mut xf);
    out.push(frame_json("seq_drum", &xf, None));
    x.button(&mut seq, X_DOWN, true, 100, &mut o, &mut st);
    x.render(&seq, &mut xf);
    out.push(frame_json("seq_melodic", &xf, None));
    x.button(&mut seq, X_CAPTURE, true, 100, &mut o, &mut st);
    x.render(&seq, &mut xf);
    out.push(frame_json("seq_shift", &xf, None));

    // The Projects overlay with slots 1 and 3 in use.
    struct Used;
    impl Storage for Used {
        fn slot_used(&mut self, slot: u8) -> bool {
            slot == 0 || slot == 2
        }
        fn save_with(&mut self, _: u8, _: impl FnOnce(&mut [u8]) -> usize) -> bool {
            true
        }
        fn load_with(&mut self, _: u8, _: impl FnOnce(&[u8]) -> bool) -> bool {
            false
        }
    }
    let mut used = Used;
    let mut seq = Seq95::new();
    let mut x = XSurface::new();
    x.button(&mut seq, X_CAPTURE, true, 100, &mut o, &mut used);
    x.button(&mut seq, 79, true, 100, &mut o, &mut used);
    x.button(&mut seq, 79, false, 0, &mut o, &mut used);
    x.button(&mut seq, X_CAPTURE, false, 0, &mut o, &mut used);
    x.render(&seq, &mut xf);
    out.push(frame_json("seq_projects", &xf, None));

    println!("{{{}}}", out.join(","));
}
