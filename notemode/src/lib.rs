// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 launchpad-Pro-Mod contributors

//! Note Mode for an 8x8 grid with eight buttons on the top and the right:
//! chromatic, scale, drum and chord layouts, overlap, root and scale choice,
//! octave and semitone shifts, and a settings page. Pad index = row * 10 + column, rows and
//! columns 1..=8, bottom left = 11 (programmer layout).

#![cfg_attr(not(test), no_std)]

mod chord;

pub const LED_COUNT: usize = 100;

pub const UP: u8 = 91;
pub const DOWN: u8 = 92;
pub const LEFT: u8 = 93;
pub const RIGHT: u8 = 94;
pub const NOTE: u8 = 96;

/// Bottom-left pad plays this note (before octave and semitone shifts) in the
/// chromatic and scale layouts.
const BASE_NOTE: i16 = 36;
const MIN_OCTAVE: i8 = -3;
const MAX_OCTAVE: i8 = 5;
const MAX_TRANSPOSE: i8 = 11;
/// First note of the drum layout's bottom-left 4x4 block (General MIDI kick).
const DRUM_FIRST: u8 = 36;
const DRUM_MAX_FIRST: u8 = 112;

pub const SCALE_COUNT: usize = 16;

/// Semitone steps from the root, in the order of the Launchpad Pro Mk3 list.
const SCALES: [&[u8]; SCALE_COUNT] = [
    &[0, 2, 3, 5, 7, 8, 10],    // Natural Minor
    &[0, 2, 4, 5, 7, 9, 11],    // Natural Major
    &[0, 2, 3, 5, 7, 9, 10],    // Dorian
    &[0, 1, 3, 5, 7, 8, 10],    // Phrygian
    &[0, 2, 4, 5, 7, 9, 10],    // Mixolydian
    &[0, 2, 3, 5, 7, 9, 11],    // Melodic Minor
    &[0, 2, 3, 5, 7, 8, 11],    // Harmonic Minor
    &[0, 2, 3, 4, 5, 7, 9, 10], // Bebop Dorian
    &[0, 3, 5, 6, 7, 10],       // Blues
    &[0, 3, 5, 7, 10],          // Minor Pentatonic
    &[0, 2, 3, 6, 7, 8, 11],    // Hungarian Minor
    &[0, 2, 3, 6, 7, 9, 10],    // Ukrainian Dorian
    &[0, 1, 4, 6, 7, 9, 11],    // Marva
    &[0, 1, 3, 6, 7, 8, 11],    // Todi
    &[0, 2, 4, 6, 8, 10],       // Whole Tone
    &[0, 2, 3, 7, 8],           // Hirajoshi
];

const PURPLE: u32 = 0xa020f0;
const BLUE: u32 = 0x0040ff;
const WHITE: u32 = 0xffffff;
const DIM_WHITE: u32 = 0x181818;
const GREEN: u32 = 0x00ff40;
const DIM_RED: u32 = 0x300000;
const DIM_BLUE: u32 = 0x000828;
const YELLOW: u32 = 0x504000;

pub trait NoteOut {
    fn note_on(&mut self, channel: u8, note: u8, velocity: u8);
    fn note_off(&mut self, channel: u8, note: u8);
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Layout {
    Chromatic,
    Scale,
    Drum,
    Chord,
}

/// A few notes played by one pad (a chord), or one note for the other layouts.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct Chord {
    len: u8,
    notes: [u8; MAX_CHORD],
}

pub const MAX_CHORD: usize = 6;

impl Chord {
    pub const EMPTY: Self = Self { len: 0, notes: [0; MAX_CHORD] };

    pub fn notes(&self) -> &[u8] {
        &self.notes[..self.len as usize]
    }

    /// Adds a note unless it is there already or the chord is full.
    pub fn push(&mut self, note: u8) -> bool {
        if self.notes().contains(&note) || self.len as usize == MAX_CHORD {
            return false;
        }
        self.notes[self.len as usize] = note;
        self.len += 1;
        true
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

pub const BANK_SLOTS: usize = 14;

/// Size of the saved preferences: format byte, layout, overlap, scale, root, channel
/// and the chord bank (a length byte and six notes per slot).
pub const PREFS_LEN: usize = 6 + BANK_SLOTS * (1 + MAX_CHORD);
const PREFS_FORMAT: u8 = 1;

pub struct NoteMode {
    layout: Layout,
    /// 0 = sequential, otherwise 2..=5 pads per row offset.
    overlap: u8,
    scale: u8,
    root: u8,
    octave: i8,
    transpose: i8,
    channel: u8,
    drum_first: u8,
    settings: bool,
    /// Notes each pad started, so the release matches even if shifts changed.
    pad_notes: [Chord; 64],
    /// How many pads currently sound each note (two pads can share a note).
    held: [u8; 128],
    /// Chord layout: the saved chords, the lock, the sustain latch, the notes
    /// kept sounding by the sustain, and the empty slot being filled.
    bank: [Chord; BANK_SLOTS],
    bank_locked: bool,
    sustain: bool,
    sustained: u128,
    assigning: Option<u8>,
    /// Preferences changed since they were last taken for saving.
    dirty: bool,
}

fn pad_slot(index: u8) -> Option<usize> {
    let (row, col) = (index / 10, index % 10);
    if (1..=8).contains(&row) && (1..=8).contains(&col) {
        Some(((row - 1) * 8 + (col - 1)) as usize)
    } else {
        None
    }
}

impl NoteMode {
    pub const fn new() -> Self {
        Self {
            layout: Layout::Chromatic,
            overlap: 5,
            scale: 0,
            root: 0,
            octave: 0,
            transpose: 0,
            channel: 0,
            drum_first: DRUM_FIRST,
            settings: false,
            pad_notes: [Chord::EMPTY; 64],
            held: [0; 128],
            bank: [Chord::EMPTY; BANK_SLOTS],
            bank_locked: false,
            sustain: false,
            sustained: 0,
            assigning: None,
            dirty: false,
        }
    }

    pub fn layout(&self) -> Layout {
        self.layout
    }

    pub fn channel(&self) -> u8 {
        self.channel
    }

    pub fn in_settings(&self) -> bool {
        self.settings
    }

    /// The note a pad would play now, or `None` outside 0..=127 or off the layout.
    pub fn note_for(&self, row: u8, col: u8) -> Option<u8> {
        let (r, c) = ((row - 1) as i16, (col - 1) as i16);
        let shift = 12 * self.octave as i16 + self.transpose as i16;
        let note = match self.layout {
            Layout::Chromatic => {
                let step = if self.overlap == 0 { 8 } else { self.overlap as i16 };
                BASE_NOTE + c + step * r + shift
            }
            Layout::Scale => {
                let steps = SCALES[self.scale as usize];
                let n = steps.len() as i16;
                let step = if self.overlap == 0 { n } else { self.overlap as i16 };
                let degree = c + step * r;
                BASE_NOTE + self.root as i16 + 12 * (degree / n) + steps[(degree % n) as usize] as i16 + shift
            }
            Layout::Drum => {
                // Four 4x4 blocks: bottom-left, top-left, bottom-right, top-right.
                let block = (if c >= 4 { 2 } else { 0 }) + (if r >= 4 { 1 } else { 0 });
                self.drum_first as i16 + 16 * block + (r % 4) * 4 + (c % 4)
            }
            Layout::Chord => return chord::single_note(self, row, col),
        };
        if (0..=127).contains(&note) { Some(note as u8) } else { None }
    }

    fn in_scale(&self, note: u8) -> bool {
        let rel = (note as i16 - self.root as i16).rem_euclid(12) as u8;
        SCALES[self.scale as usize].contains(&rel)
    }

    pub fn pad(&mut self, index: u8, pressed: bool, velocity: u8, out: &mut impl NoteOut) {
        let Some(slot) = pad_slot(index) else { return };
        if self.settings {
            if pressed {
                self.settings_press(index / 10, index % 10);
            }
            return;
        }
        if self.layout == Layout::Chord {
            chord::pad(self, index / 10, index % 10, pressed, velocity, out);
            return;
        }
        if pressed {
            if !self.pad_notes[slot].is_empty() {
                return;
            }
            if let Some(note) = self.note_for(index / 10, index % 10) {
                self.sound(slot, &[note], velocity, out);
            }
        } else {
            self.release_slot(slot, out);
        }
    }

    /// Starts the notes of a pad: remembered per pad, counted per note.
    fn sound(&mut self, slot: usize, notes: &[u8], velocity: u8, out: &mut impl NoteOut) {
        for &note in notes {
            if self.pad_notes[slot].push(note) {
                self.held[note as usize] = self.held[note as usize].saturating_add(1);
                self.sustained &= !(1u128 << note);
                out.note_on(self.channel, note, velocity.clamp(1, 127));
            }
        }
    }

    fn release_slot(&mut self, slot: usize, out: &mut impl NoteOut) {
        let chord = self.pad_notes[slot];
        self.pad_notes[slot] = Chord::EMPTY;
        for &note in chord.notes() {
            let count = &mut self.held[note as usize];
            *count = count.saturating_sub(1);
            if *count == 0 {
                if self.sustain {
                    self.sustained |= 1u128 << note;
                } else {
                    out.note_off(self.channel, note);
                }
            }
        }
    }

    /// Note a pad is sounding now (for aftertouch).
    pub fn sounding(&self, index: u8) -> Option<u8> {
        self.pad_notes[pad_slot(index)?].notes().first().copied()
    }

    pub fn release_all(&mut self, out: &mut impl NoteOut) {
        for slot in 0..64 {
            self.release_slot(slot, out);
        }
    }

    /// Top-row buttons. Returns true if the button was used.
    pub fn button(&mut self, index: u8, pressed: bool, out: &mut impl NoteOut) -> bool {
        if !matches!(index, UP | DOWN | LEFT | RIGHT | NOTE) {
            return false;
        }
        if !pressed {
            return true;
        }
        if index == NOTE {
            self.release_all(out);
            self.release_sustained(out);
            self.assigning = None;
            self.settings = !self.settings;
            return true;
        }
        if self.settings {
            return true;
        }
        let dir: i8 = if index == UP || index == RIGHT { 1 } else { -1 };
        match (self.layout, index) {
            (Layout::Drum, UP | DOWN) => self.scroll_drums(16 * dir as i16),
            (Layout::Drum, _) => self.scroll_drums(4 * dir as i16),
            (_, UP | DOWN) => self.octave = (self.octave + dir).clamp(MIN_OCTAVE, MAX_OCTAVE),
            _ => self.transpose = (self.transpose + dir).clamp(-MAX_TRANSPOSE, MAX_TRANSPOSE),
        }
        true
    }

    fn scroll_drums(&mut self, by: i16) {
        self.drum_first = (self.drum_first as i16 + by).clamp(0, DRUM_MAX_FIRST as i16) as u8;
    }

    pub fn exit(&mut self, out: &mut impl NoteOut) {
        self.release_all(out);
        self.release_sustained(out);
        self.assigning = None;
        self.settings = false;
    }

    /// Ends every note the sustain latch is keeping alive.
    fn release_sustained(&mut self, out: &mut impl NoteOut) {
        for note in 0..128u8 {
            if self.sustained & (1u128 << note) != 0 && self.held[note as usize] == 0 {
                out.note_off(self.channel, note);
            }
        }
        self.sustained = 0;
    }

    fn settings_press(&mut self, row: u8, col: u8) {
        match row {
            8 => match col {
                1 => self.overlap = 0,
                2..=5 => self.overlap = col,
                _ => {}
            },
            7 => {
                if let Some(pc) = black_key(col) {
                    self.root = pc;
                }
            }
            6 => {
                if let Some(pc) = white_key(col) {
                    self.root = pc;
                }
            }
            5 => self.scale = col - 1,
            4 => self.scale = col + 7,
            3 => self.channel = col - 1,
            2 => self.channel = col + 7,
            _ => match col {
                1 => self.layout = Layout::Chromatic,
                2 => self.layout = Layout::Scale,
                3 => self.layout = Layout::Drum,
                4 => self.layout = Layout::Chord,
                8 => {
                    self.octave = 0;
                    self.transpose = 0;
                    self.drum_first = DRUM_FIRST;
                }
                _ => {}
            },
        }
        self.dirty = true;
    }

    /// True once after the preferences changed and nothing is being played, so that
    /// the caller can save them without interrupting a note.
    pub fn take_dirty_when_idle(&mut self) -> bool {
        if self.dirty && !self.settings && self.pad_notes.iter().all(|c| c.is_empty()) {
            self.dirty = false;
            return true;
        }
        false
    }

    /// The preferences worth keeping: layout, overlap, scale, root, channel, chord bank.
    pub fn export_prefs(&self) -> [u8; PREFS_LEN] {
        let mut out = [0u8; PREFS_LEN];
        out[0] = PREFS_FORMAT;
        out[1] = match self.layout {
            Layout::Chromatic => 0,
            Layout::Scale => 1,
            Layout::Drum => 2,
            Layout::Chord => 3,
        };
        out[2] = self.overlap;
        out[3] = self.scale;
        out[4] = self.root;
        out[5] = self.channel;
        for (i, chord) in self.bank.iter().enumerate() {
            let at = 6 + i * (1 + MAX_CHORD);
            out[at] = chord.len;
            out[at + 1..at + 1 + MAX_CHORD].copy_from_slice(&chord.notes);
        }
        out
    }

    /// Takes saved preferences. Anything that is not exactly valid is refused and
    /// leaves the current state untouched.
    pub fn import_prefs(&mut self, data: &[u8]) -> bool {
        if data.len() != PREFS_LEN || data[0] != PREFS_FORMAT {
            return false;
        }
        let layout = match data[1] {
            0 => Layout::Chromatic,
            1 => Layout::Scale,
            2 => Layout::Drum,
            3 => Layout::Chord,
            _ => return false,
        };
        let (overlap, scale, root, channel) = (data[2], data[3], data[4], data[5]);
        if !(overlap == 0 || (2..=5).contains(&overlap)) || scale as usize >= SCALE_COUNT || root > 11 || channel > 15 {
            return false;
        }
        let mut bank = [Chord::EMPTY; BANK_SLOTS];
        for (i, slot) in bank.iter_mut().enumerate() {
            let at = 6 + i * (1 + MAX_CHORD);
            let len = data[at];
            let notes = &data[at + 1..at + 1 + MAX_CHORD];
            if len as usize > MAX_CHORD || notes.iter().any(|n| *n > 127) {
                return false;
            }
            slot.len = len;
            slot.notes.copy_from_slice(notes);
        }
        self.layout = layout;
        self.overlap = overlap;
        self.scale = scale;
        self.root = root;
        self.channel = channel;
        self.bank = bank;
        self.dirty = false;
        true
    }

    pub fn render(&self, frame: &mut [u32; LED_COUNT]) {
        frame.fill(0);
        for index in [UP, DOWN, LEFT, RIGHT] {
            frame[index as usize] = DIM_WHITE;
        }
        frame[NOTE as usize] = GREEN;
        if self.settings {
            self.render_settings(frame);
            return;
        }
        for row in 1..=8u8 {
            for col in 1..=8u8 {
                let index = (row * 10 + col) as usize;
                let pressed = !self.pad_notes[((row - 1) * 8 + (col - 1)) as usize].is_empty();
                if self.layout == Layout::Chord {
                    frame[index] = if pressed { WHITE } else { chord::color(self, row, col) };
                    continue;
                }
                frame[index] = match self.note_for(row, col) {
                    None => 0,
                    Some(_) if pressed => WHITE,
                    Some(note) => match self.layout {
                        Layout::Drum => YELLOW,
                        _ if (note as i16 - self.root as i16).rem_euclid(12) == 0 => PURPLE,
                        _ if self.in_scale(note) => BLUE,
                        Layout::Scale => BLUE,
                        Layout::Chromatic | Layout::Chord => 0,
                    },
                };
            }
        }
    }

    fn render_settings(&self, frame: &mut [u32; LED_COUNT]) {
        let on = |sel: bool| if sel { WHITE } else { DIM_BLUE };
        frame[81] = on(self.overlap == 0);
        for col in 2..=5u8 {
            frame[(80 + col) as usize] = on(self.overlap == col);
        }
        frame[11] = if self.layout == Layout::Chromatic { WHITE } else { DIM_RED };
        frame[12] = if self.layout == Layout::Scale { GREEN } else { DIM_RED };
        frame[13] = if self.layout == Layout::Drum { YELLOW | 0x303000 } else { DIM_RED };
        frame[14] = if self.layout == Layout::Chord { 0xff6000 } else { DIM_RED };
        for col in 1..=8u8 {
            for (row, key) in [(7u8, black_key(col)), (6u8, white_key(col))] {
                if let Some(pc) = key {
                    frame[(row * 10 + col) as usize] = if pc == self.root {
                        PURPLE
                    } else if SCALES[self.scale as usize].contains(&((pc as i16 - self.root as i16).rem_euclid(12) as u8)) {
                        BLUE
                    } else {
                        DIM_WHITE
                    };
                }
            }
            frame[(50 + col) as usize] = on(self.scale == col - 1);
            frame[(40 + col) as usize] = on(self.scale == col + 7);
            frame[(30 + col) as usize] = on(self.channel == col - 1);
            frame[(20 + col) as usize] = on(self.channel == col + 7);
        }
        frame[18] = DIM_WHITE;
    }
}

impl Default for NoteMode {
    fn default() -> Self {
        Self::new()
    }
}

/// Pitch class of the upper piano row (black keys) at a column.
fn black_key(col: u8) -> Option<u8> {
    match col {
        2 => Some(1),
        3 => Some(3),
        5 => Some(6),
        6 => Some(8),
        7 => Some(10),
        _ => None,
    }
}

/// Pitch class of the lower piano row (white keys) at a column.
fn white_key(col: u8) -> Option<u8> {
    match col {
        1 => Some(0),
        2 => Some(2),
        3 => Some(4),
        4 => Some(5),
        5 => Some(7),
        6 => Some(9),
        7 => Some(11),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
