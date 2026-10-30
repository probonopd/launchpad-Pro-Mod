// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 launchpad-Pro-Mod contributors

//! Chord layout of Note Mode, following the description of Chord Mode in the
//! Launchpad Pro Mk3 user guide:
//!
//! - column 1: one pad per scale degree plays a triad, folded into one octave
//! - columns 2 to 6, the note area: row r holds the notes of the chord built on
//!   scale degree r: root, third (one octave up), fifth, seventh, fifth (one
//!   octave up). The first column of the note area runs through the scale, and
//!   any three adjacent pads of a row form a triad.
//! - columns 7 and 8, rows 2 to 8: the chord bank, 14 slots. Hold an empty slot
//!   and press notes or triads to fill it (or hold notes and press an empty
//!   slot). A filled slot plays its chord.
//! - row 1: sustain (column 7) and bank lock (column 8)
//!
//! The text of the guide does not say exactly how the five pads of a row are
//! ordered; the order above is our reading of it.

use super::*;

const ORANGE: u32 = 0xff6000;
const SUSTAIN_ON: u32 = 0x00ff40;
const SUSTAIN_OFF: u32 = 0x002800;
const LOCK_ON: u32 = 0xff0000;
const LOCK_OFF: u32 = 0x300000;

fn shift(m: &NoteMode) -> i16 {
    12 * m.octave as i16 + m.transpose as i16
}

/// Semitones above the scale root of a scale degree (degree 0 is the root).
fn degree_offset(m: &NoteMode, degree: i16) -> i16 {
    let steps = SCALES[m.scale as usize];
    let n = steps.len() as i16;
    12 * degree.div_euclid(n) + steps[degree.rem_euclid(n) as usize] as i16
}

fn root_note(m: &NoteMode) -> i16 {
    BASE_NOTE + m.root as i16 + shift(m)
}

fn midi(note: i16) -> Option<u8> {
    if (0..=127).contains(&note) { Some(note as u8) } else { None }
}

/// The single note of a note-area pad (columns 2 to 6).
pub(crate) fn single_note(m: &NoteMode, row: u8, col: u8) -> Option<u8> {
    let degree = row as i16 - 1;
    let (add, octave) = match col {
        2 => (0, 0),
        3 => (2, 12),
        4 => (4, 0),
        5 => (6, 0),
        6 => (4, 12),
        _ => return None,
    };
    midi(root_note(m) + degree_offset(m, degree + add) + octave)
}

/// The triad of a row, with every tone folded into the octave above the root note.
fn triad(m: &NoteMode, row: u8) -> Chord {
    let degree = row as i16 - 1;
    let low = root_note(m);
    let mut chord = Chord::EMPTY;
    for add in [0, 2, 4] {
        let offset = degree_offset(m, degree + add);
        match midi(low + offset.rem_euclid(12)) {
            Some(note) => {
                chord.push(note);
            }
            None => return Chord::EMPTY,
        }
    }
    chord
}

/// Bank slot of a pad: columns 7 and 8, rows 2 to 8.
fn bank_index(row: u8, col: u8) -> Option<usize> {
    if (7..=8).contains(&col) && (2..=8).contains(&row) {
        Some(((col - 7) * 7 + (row - 2)) as usize)
    } else {
        None
    }
}

pub(crate) fn pad(m: &mut NoteMode, row: u8, col: u8, pressed: bool, velocity: u8, out: &mut impl NoteOut) {
    let slot = ((row - 1) * 8 + (col - 1)) as usize;
    if !pressed {
        if let (Some(held), Some(b)) = (m.assigning, bank_index(row, col)) {
            if held as usize == b {
                m.assigning = None;
                m.dirty = true;
            }
        }
        m.release_slot(slot, out);
        return;
    }

    if row == 1 && col == 7 {
        m.sustain = !m.sustain;
        if !m.sustain {
            m.release_sustained(out);
        }
        return;
    }
    if row == 1 && col == 8 {
        m.bank_locked = !m.bank_locked;
        m.assigning = None;
        return;
    }

    if let Some(b) = bank_index(row, col) {
        let chord = m.bank[b];
        if !chord.is_empty() {
            if m.pad_notes[slot].is_empty() {
                m.sound(slot, chord.notes(), velocity, out);
            }
        } else if !m.bank_locked {
            // Notes already held are stored at once; notes pressed while the empty
            // slot stays down are added by the branch below.
            let mut held = Chord::EMPTY;
            for pad_chord in m.pad_notes.iter() {
                for &note in pad_chord.notes() {
                    held.push(note);
                }
            }
            m.bank[b] = held;
            m.assigning = Some(b as u8);
        }
        return;
    }

    if !m.pad_notes[slot].is_empty() {
        return;
    }
    let mut notes = Chord::EMPTY;
    if col == 1 {
        notes = triad(m, row);
    } else if let Some(note) = single_note(m, row, col) {
        notes.push(note);
    }
    if notes.is_empty() {
        return;
    }
    if let Some(b) = m.assigning {
        for &note in notes.notes() {
            m.bank[b as usize].push(note);
        }
    }
    m.sound(slot, notes.notes(), velocity, out);
}

/// Color of a pad in the chord layout when it is not being pressed.
pub(crate) fn color(m: &NoteMode, row: u8, col: u8) -> u32 {
    if row == 1 && col == 7 {
        return if m.sustain { SUSTAIN_ON } else { SUSTAIN_OFF };
    }
    if row == 1 && col == 8 {
        return if m.bank_locked { LOCK_ON } else { LOCK_OFF };
    }
    if let Some(b) = bank_index(row, col) {
        return if m.assigning == Some(b as u8) {
            ORANGE
        } else if m.bank[b].is_empty() {
            DIM_WHITE
        } else {
            WHITE
        };
    }
    if col == 1 {
        return if triad(m, row).is_empty() { 0 } else { ORANGE };
    }
    match single_note(m, row, col) {
        None => 0,
        Some(note) if (note as i16 - m.root as i16).rem_euclid(12) == 0 => PURPLE,
        Some(_) => BLUE,
    }
}
