// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 launchpad-Pro-Mod contributors

//! Custom Modes for an 8x8 grid: every pad can send a note, a control change or a
//! program change, and a row or column of eight pads can act as a fader. The
//! eight built-in modes follow the default Custom Modes of the Launchpad Pro Mk3
//! user guide. A mode is plain data (`ModeDef`), so modes made elsewhere (for
//! example loaded from Novation Components) can use the same engine.
//!
//! Pad index = row * 10 + column, rows and columns 1..=8, bottom left = 11.

#![cfg_attr(not(test), no_std)]

pub const LED_COUNT: usize = 100;
pub const MODES: usize = 8;

/// Size of the saved preferences: format byte, the chosen mode and the master channels.
pub const PREFS_LEN: usize = 2 + MODES;
const PREFS_FORMAT: u8 = 1;

/// A value in a rendered frame with this bit set is a Novation palette index
/// (the low 7 bits) instead of a 0xRRGGBB color.
pub const PALETTE_FLAG: u32 = 0x0100_0000;

const WHITE: u32 = 0xffffff;
const DIM_WHITE: u32 = 0x101010;
const PALE_GREEN: u32 = 0x40ff40;
const GREEN: u32 = 0x00ff40;
const DIM_RED: u32 = 0x300000;
const AMBER: u32 = 0xff8000;
const DIM_AMBER: u32 = 0x201000;
const CYAN: u32 = 0x00c0ff;
const DIM_CYAN: u32 = 0x001820;
const BLUE: u32 = 0x0040ff;
const PURPLE: u32 = 0xa020f0;
const DIM_PURPLE: u32 = 0x180830;

pub trait ModeOut {
    fn note_on(&mut self, channel: u8, note: u8, velocity: u8);
    fn note_off(&mut self, channel: u8, note: u8);
    fn control_change(&mut self, channel: u8, cc: u8, value: u8);
    fn program_change(&mut self, channel: u8, program: u8);
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Action {
    None,
    /// `velocity` 0 means the velocity of the press.
    Note { note: u8, velocity: u8 },
    Cc { cc: u8, on: u8, off: u8 },
    Program { program: u8 },
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct PadDef {
    pub action: Action,
    /// 0..=15 to override the mode's master channel.
    pub channel: Option<u8>,
    pub toggle: bool,
    pub aftertouch: bool,
    pub off_color: u32,
    pub on_color: u32,
}

impl PadDef {
    pub const NONE: Self = Self {
        action: Action::None,
        channel: None,
        toggle: false,
        aftertouch: false,
        off_color: 0,
        on_color: 0,
    };
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct FaderDef {
    pub cc: u8,
    pub channel: Option<u8>,
    /// Horizontal faders use a row of pads, vertical ones a column.
    pub horizontal: bool,
    pub bipolar: bool,
    pub color: u32,
    /// The row (horizontal) or column (vertical) the fader uses, 1..=8.
    pub line: u8,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct ModeDef {
    pub pads: [PadDef; 64],
    pub faders: [Option<FaderDef>; 8],
    /// Pads are lit by incoming notes (velocity = Novation palette color)
    /// instead of by their own definition.
    pub lit_by_midi: bool,
}

impl ModeDef {
    pub const EMPTY: Self = Self { pads: [PadDef::NONE; 64], faders: [None; 8], lit_by_midi: false };
}

fn slot(row: u8, col: u8) -> usize {
    ((row - 1) * 8 + (col - 1)) as usize
}

fn drum_note(row: u8, col: u8) -> u8 {
    let block = (if col >= 5 { 2 } else { 0 }) + (if row >= 5 { 1 } else { 0 });
    36 + 16 * block + ((row - 1) % 4) * 4 + (col - 1) % 4
}

/// Piano layout: pairs of rows, white keys below black keys, four octaves.
fn keyboard_note(row: u8, col: u8) -> Option<u8> {
    let octave = (row - 1) / 2;
    let upper = (row - 1) % 2 == 1;
    let pitch = if upper {
        match col {
            2 => 1,
            3 => 3,
            5 => 6,
            6 => 8,
            7 => 10,
            _ => return None,
        }
    } else {
        match col {
            1 => 0,
            2 => 2,
            3 => 4,
            4 => 5,
            5 => 7,
            6 => 9,
            7 => 11,
            _ => 12,
        }
    };
    Some(36 + 12 * octave + pitch)
}

/// The built-in mode `index` (0..=7 are Custom 1 to 8 of the guide).
pub fn builtin(index: u8) -> ModeDef {
    let mut def = ModeDef::EMPTY;
    match index {
        // 8 unipolar vertical faders, CC 7 to 14.
        0 => {
            for k in 0..8u8 {
                def.faders[k as usize] = Some(FaderDef {
                    cc: 7 + k,
                    channel: None,
                    horizontal: false,
                    bipolar: false,
                    color: [GREEN, CYAN, BLUE, PURPLE, AMBER, GREEN, CYAN, BLUE][k as usize],
                    line: k + 1,
                });
            }
        }
        // 8 bipolar horizontal faders, CC 15 to 22, first fader on the top row.
        1 => {
            for k in 0..8u8 {
                def.faders[k as usize] = Some(FaderDef {
                    cc: 15 + k,
                    channel: None,
                    horizontal: true,
                    bipolar: true,
                    color: [AMBER, GREEN, CYAN, BLUE, PURPLE, AMBER, GREEN, CYAN][k as usize],
                    line: 8 - k,
                });
            }
        }
        // Drum rack layout, matching Note Mode's drum layout.
        2 | 6 => {
            def.lit_by_midi = index == 6;
            for row in 1..=8u8 {
                for col in 1..=8u8 {
                    def.pads[slot(row, col)] = PadDef {
                        action: Action::Note { note: drum_note(row, col), velocity: 0 },
                        aftertouch: true,
                        off_color: if def.lit_by_midi { 0 } else { 0x302000 },
                        on_color: WHITE,
                        ..PadDef::NONE
                    };
                }
            }
        }
        // Chromatic piano layout.
        3 => {
            for row in 1..=8u8 {
                for col in 1..=8u8 {
                    if let Some(note) = keyboard_note(row, col) {
                        let black = (row - 1) % 2 == 1;
                        def.pads[slot(row, col)] = PadDef {
                            action: Action::Note { note, velocity: 0 },
                            aftertouch: true,
                            off_color: if black { DIM_PURPLE } else { DIM_WHITE },
                            on_color: WHITE,
                            ..PadDef::NONE
                        };
                    }
                }
            }
        }
        // Program changes 0 to 63 and 64 to 127.
        4 | 5 => {
            for row in 1..=8u8 {
                for col in 1..=8u8 {
                    def.pads[slot(row, col)] = PadDef {
                        action: Action::Program { program: (index - 4) * 64 + (row - 1) * 8 + (col - 1) },
                        off_color: if index == 4 { DIM_AMBER } else { DIM_CYAN },
                        on_color: WHITE,
                        ..PadDef::NONE
                    };
                }
            }
        }
        // Programmer layout, unlit until notes arrive.
        _ => {
            def.lit_by_midi = true;
            for row in 1..=8u8 {
                for col in 1..=8u8 {
                    def.pads[slot(row, col)] = PadDef {
                        action: Action::Note { note: row * 10 + col, velocity: 0 },
                        aftertouch: true,
                        on_color: WHITE,
                        ..PadDef::NONE
                    };
                }
            }
        }
    }
    def
}

pub struct CustomModes {
    mode: u8,
    def: ModeDef,
    master: [u8; MODES],
    /// Toggle state of each pad.
    on: u64,
    /// Pads currently held.
    down: u64,
    /// Fader positions 0..=7.
    fader: [u8; 8],
    /// Colors set by incoming notes in the lit-by-MIDI modes (palette index).
    lit: [u8; 64],
    channel_view: bool,
    dirty: bool,
}

fn position_value(position: u8) -> u8 {
    (position as u16 * 127 / 7) as u8
}

impl CustomModes {
    pub const fn new() -> Self {
        Self {
            mode: 0,
            def: ModeDef::EMPTY,
            master: [0; MODES],
            on: 0,
            down: 0,
            fader: [0; 8],
            lit: [0; 64],
            channel_view: false,
            dirty: false,
        }
    }

    /// Loads the current mode's definition; call once before use.
    pub fn start(&mut self) {
        self.def = builtin(self.mode);
        self.reset_state();
    }

    /// Clears toggles and lighting; faders start at 0 (bipolar ones in the middle).
    fn reset_state(&mut self) {
        self.on = 0;
        self.lit = [0; 64];
        for k in 0..8 {
            self.fader[k] = match self.def.faders[k] {
                Some(f) if f.bipolar => 4,
                _ => 0,
            };
        }
    }

    /// True once after the mode or a channel changed and the channel view is closed
    /// and nothing is held, so the caller can save without interrupting playing.
    pub fn take_dirty_when_idle(&mut self) -> bool {
        if self.dirty && !self.channel_view && self.down == 0 {
            self.dirty = false;
            return true;
        }
        false
    }

    pub fn export_prefs(&self) -> [u8; PREFS_LEN] {
        let mut out = [0u8; PREFS_LEN];
        out[0] = PREFS_FORMAT;
        out[1] = self.mode;
        out[2..].copy_from_slice(&self.master);
        out
    }

    /// Takes saved preferences; anything not exactly valid is refused.
    pub fn import_prefs(&mut self, data: &[u8]) -> bool {
        if data.len() != PREFS_LEN || data[0] != PREFS_FORMAT || data[1] as usize >= MODES || data[2..].iter().any(|c| *c > 15) {
            return false;
        }
        self.mode = data[1];
        self.master.copy_from_slice(&data[2..]);
        self.def = builtin(self.mode);
        self.reset_state();
        self.dirty = false;
        true
    }

    pub fn mode(&self) -> u8 {
        self.mode
    }

    pub fn master_channel(&self) -> u8 {
        self.master[self.mode as usize]
    }

    pub fn select(&mut self, mode: u8, out: &mut impl ModeOut) {
        if mode as usize >= MODES || mode == self.mode {
            return;
        }
        self.release_all(out);
        self.mode = mode;
        self.def = builtin(mode);
        self.reset_state();
        self.dirty = true;
    }

    /// Replaces the current mode by another definition (for modes made elsewhere).
    pub fn load(&mut self, def: ModeDef, out: &mut impl ModeOut) {
        self.release_all(out);
        self.def = def;
        self.reset_state();
    }

    /// While true, the bottom two rows choose the master channel instead of playing.
    pub fn set_channel_view(&mut self, on: bool, out: &mut impl ModeOut) {
        if on {
            self.release_all(out);
        }
        self.channel_view = on;
    }

    fn channel_of(&self, pad: &PadDef) -> u8 {
        pad.channel.unwrap_or_else(|| self.master_channel())
    }

    pub fn release_all(&mut self, out: &mut impl ModeOut) {
        for s in 0..64u8 {
            if self.down & (1 << s) != 0 {
                self.release(s as usize, out);
            }
        }
    }

    fn release(&mut self, s: usize, out: &mut impl ModeOut) {
        self.down &= !(1 << s);
        let pad = self.def.pads[s];
        if pad.toggle {
            return;
        }
        if let Action::Note { note, .. } = pad.action {
            out.note_off(self.channel_of(&pad), note);
        }
    }

    pub fn pad(&mut self, index: u8, pressed: bool, velocity: u8, out: &mut impl ModeOut) {
        let (row, col) = (index / 10, index % 10);
        if !(1..=8).contains(&row) || !(1..=8).contains(&col) {
            return;
        }
        let s = slot(row, col);
        if self.channel_view {
            if pressed && row <= 2 {
                self.master[self.mode as usize] = (row - 1) * 8 + (col - 1);
                self.dirty = true;
            }
            return;
        }
        if let Some(fader) = self.fader_at(row, col) {
            if pressed {
                let position = if fader.horizontal { col - 1 } else { row - 1 };
                if let Some(k) = self.def.faders.iter().position(|f| *f == Some(fader)) {
                    self.fader[k] = position;
                }
                let channel = fader.channel.unwrap_or_else(|| self.master_channel());
                out.control_change(channel, fader.cc, position_value(position));
            }
            return;
        }
        let pad = self.def.pads[s];
        if pad.action == Action::None {
            return;
        }
        let channel = self.channel_of(&pad);
        if !pressed {
            if self.down & (1 << s) != 0 {
                self.release(s, out);
            }
            return;
        }
        if self.down & (1 << s) != 0 {
            return;
        }
        self.down |= 1 << s;
        let now_on = if pad.toggle { self.on ^= 1 << s; self.on & (1 << s) != 0 } else { true };
        match pad.action {
            Action::Note { note, velocity: fixed } => {
                let v = if fixed != 0 { fixed } else { velocity.clamp(1, 127) };
                if now_on {
                    out.note_on(channel, note, v);
                } else {
                    out.note_off(channel, note);
                }
            }
            Action::Cc { cc, on, off } => out.control_change(channel, cc, if now_on { on } else { off }),
            Action::Program { program } => out.program_change(channel, program),
            Action::None => {}
        }
    }

    fn fader_at(&self, row: u8, col: u8) -> Option<FaderDef> {
        self.def.faders.iter().flatten().copied().find(|f| {
            if f.horizontal { f.line == row } else { f.line == col }
        })
    }

    /// The note of a pad that has aftertouch enabled (for aftertouch messages).
    pub fn aftertouch_target(&self, index: u8) -> Option<(u8, u8)> {
        let (row, col) = (index / 10, index % 10);
        if !(1..=8).contains(&row) || !(1..=8).contains(&col) || self.channel_view {
            return None;
        }
        let pad = self.def.pads[slot(row, col)];
        match pad.action {
            Action::Note { note, .. } if pad.aftertouch && self.down & (1 << slot(row, col)) != 0 => {
                Some((self.channel_of(&pad), note))
            }
            _ => None,
        }
    }

    /// Incoming MIDI: lights pads in the lit-by-MIDI modes. Returns true if used.
    pub fn midi_in(&mut self, status: u8, data1: u8, data2: u8) -> bool {
        if !self.def.lit_by_midi || status & 0x0f != self.master_channel() {
            return false;
        }
        let on = match status & 0xf0 {
            0x90 => data2,
            0x80 => 0,
            _ => return false,
        };
        let mut used = false;
        for (s, pad) in self.def.pads.iter().enumerate() {
            if let Action::Note { note, .. } = pad.action {
                if note == data1 {
                    self.lit[s] = on;
                    used = true;
                }
            }
        }
        used
    }

    /// Mode buttons (the right column, top to bottom = modes 1 to 8): `row` is 1..=8.
    pub fn mode_for_button_row(row: u8) -> Option<u8> {
        if (1..=8).contains(&row) { Some(8 - row) } else { None }
    }

    pub fn render(&self, frame: &mut [u32; LED_COUNT]) {
        frame.fill(0);
        // The mode buttons: the chosen one pale green, the others dim white.
        for row in 1..=8u8 {
            let mode = 8 - row;
            frame[(row * 10 + 9) as usize] = if mode == self.mode { PALE_GREEN } else { DIM_WHITE };
        }
        if self.channel_view {
            for row in 1..=2u8 {
                for col in 1..=8u8 {
                    let channel = (row - 1) * 8 + (col - 1);
                    frame[(row * 10 + col) as usize] =
                        if channel == self.master_channel() { GREEN } else { DIM_RED };
                }
            }
            return;
        }
        for row in 1..=8u8 {
            for col in 1..=8u8 {
                let s = slot(row, col);
                let i = (row * 10 + col) as usize;
                if let Some(fader) = self.fader_at(row, col) {
                    frame[i] = self.fader_pad(&fader, row, col);
                    continue;
                }
                let pad = self.def.pads[s];
                if self.def.lit_by_midi {
                    let velocity = self.lit[s];
                    frame[i] = if self.down & (1 << s) != 0 {
                        pad.on_color
                    } else if velocity != 0 {
                        PALETTE_FLAG | velocity as u32
                    } else {
                        pad.off_color
                    };
                    continue;
                }
                let lit = if pad.toggle { self.on & (1 << s) != 0 } else { self.down & (1 << s) != 0 };
                frame[i] = if lit { pad.on_color } else { pad.off_color };
            }
        }
    }

    fn fader_pad(&self, fader: &FaderDef, row: u8, col: u8) -> u32 {
        let k = self.def.faders.iter().position(|f| *f == Some(*fader)).unwrap_or(0);
        let position = self.fader[k];
        let here = if fader.horizontal { col - 1 } else { row - 1 };
        let lit = if fader.bipolar {
            // The middle lies between positions 3 and 4.
            if position >= 4 { here >= 4 && here <= position } else { here <= 3 && here >= position }
        } else {
            here <= position
        };
        if lit { fader.color } else { 0x080808 }
    }
}

impl Default for CustomModes {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests;
