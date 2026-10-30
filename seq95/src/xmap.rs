// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 launchpad-Pro-Mod contributors

//! Control layout for the Launchpad X. The sequencer UI is written for the Pro
//! Mk3, which has 36 edge buttons; the X has 16 and the host keeps Session for
//! the Setup hold. This maps X programmer-mode indexes to the sequencer's
//! indexes and back for the LEDs.
//!
//! Top row:    Up/Down = previous/next track (Shift: transpose +-12),
//!             Left/Right = page (Shift: tempo), Note = Play,
//!             Custom = host toggle, Capture MIDI = Shift.
//! Right col:  lanes; with Shift held: Record, Clear, Duplicate, Fixed length,
//!             Mute, Solo, Projects. Clear and Mute are holds like on the Pro
//!             (hold, then tap a step, page or voice); a tap without touching
//!             the grid clears or mutes the whole current track instead.
//!             Solo toggles the current track at once.

use crate::engine::MidiOut;
use crate::ui;
use crate::{Seq95, Storage};

pub const X_LED_COUNT: usize = 100;

pub const X_UP: u8 = 91;
pub const X_DOWN: u8 = 92;
pub const X_LEFT: u8 = 93;
pub const X_RIGHT: u8 = 94;
pub const X_NOTE: u8 = 96;
pub const X_CUSTOM: u8 = 97;
pub const X_CAPTURE: u8 = 98;

/// What a held right-column button does on release.
#[derive(Copy, Clone, Eq, PartialEq)]
enum Held {
    Nothing,
    /// Forward the release to this sequencer index.
    Forward(u8),
}

pub struct XSurface {
    shift: bool,
    /// Indexed by row 1..=8 of the right column. The layer chosen at press time
    /// decides the release, even if Shift was let go in between.
    held: [Held; 9],
    /// Clear or Mute pressed and not yet used on the grid: a release now is a
    /// tap that applies to the whole current track.
    track_tap: [bool; 9],
    grid_used: bool,
}

impl XSurface {
    pub const fn new() -> Self {
        Self {
            shift: false,
            held: [Held::Nothing; 9],
            track_tap: [false; 9],
            grid_used: false,
        }
    }

    pub fn reset(&mut self) {
        self.shift = false;
        self.held = [Held::Nothing; 9];
        self.track_tap = [false; 9];
        self.grid_used = false;
    }

    pub fn button(
        &mut self,
        seq: &mut Seq95,
        index: u8,
        pressed: bool,
        velocity: u8,
        out: &mut impl MidiOut,
        storage: &mut impl Storage,
    ) {
        match index {
            X_CAPTURE => {
                self.shift = pressed;
                seq.button(ui::SHIFT, pressed, velocity, out, storage);
            }
            X_UP | X_DOWN => {
                if !pressed {
                    return;
                }
                if self.shift {
                    let target = if index == X_UP { ui::UP } else { ui::DOWN };
                    seq.button(target, true, velocity, out, storage);
                } else {
                    seq.step_track(index == X_DOWN, out);
                }
            }
            X_LEFT => seq.button(ui::LEFT, pressed, velocity, out, storage),
            X_RIGHT => seq.button(ui::RIGHT, pressed, velocity, out, storage),
            X_NOTE => seq.button(ui::PLAY, pressed, velocity, out, storage),
            _ => {
                let row = index / 10;
                let col = index % 10;
                if !(1..=8).contains(&row) {
                    return;
                }
                if (1..=8).contains(&col) {
                    if pressed {
                        self.grid_used = true;
                    }
                    seq.button(index, pressed, velocity, out, storage);
                } else if col == 9 {
                    self.side_button(seq, row, index, pressed, velocity, out, storage);
                }
            }
        }
    }

    fn side_button(
        &mut self,
        seq: &mut Seq95,
        row: u8,
        index: u8,
        pressed: bool,
        velocity: u8,
        out: &mut impl MidiOut,
        storage: &mut impl Storage,
    ) {
        let slot = row as usize;
        if !pressed {
            if let Held::Forward(target) = self.held[slot] {
                seq.button(target, false, velocity, out, storage);
            }
            self.held[slot] = Held::Nothing;
            if self.track_tap[slot] {
                self.track_tap[slot] = false;
                if !self.grid_used {
                    match row {
                        2 => seq.clear_current(),
                        5 => seq.toggle_mute_current(),
                        _ => {}
                    }
                }
            }
            return;
        }

        if !self.shift {
            self.held[slot] = Held::Forward(index);
            seq.button(index, true, velocity, out, storage);
            return;
        }

        // Rows count from the bottom: 1 = Notes ... 8 = Scale.
        let forward = match row {
            1 => Some(ui::RECORD),
            2 => Some(ui::CLEAR),
            3 => Some(ui::DUPLICATE),
            4 => Some(ui::FIXED_LENGTH),
            5 => Some(ui::MUTE),
            7 => Some(ui::PROJECTS),
            _ => None,
        };
        if row == 6 {
            seq.toggle_solo_current();
        }
        if row == 2 || row == 5 {
            self.track_tap[slot] = true;
            self.grid_used = false;
        }
        if let Some(target) = forward {
            self.held[slot] = Held::Forward(target);
            seq.button(target, true, velocity, out, storage);
        } else {
            self.held[slot] = Held::Nothing;
        }
    }

    /// Draws the X surface. Every index written is below `X_LED_COUNT`.
    pub fn render(&self, seq: &Seq95, frame: &mut [u32; X_LED_COUNT]) {
        let mut src = [0u32; ui::LED_COUNT];
        seq.render(&mut src);
        frame.fill(0);
        for row in 1..=8usize {
            for col in 1..=9usize {
                frame[row * 10 + col] = src[row * 10 + col];
            }
        }
        let track_colour = src[seq.current_track() + 1];
        frame[X_UP as usize] = track_colour;
        frame[X_DOWN as usize] = track_colour;
        frame[X_LEFT as usize] = src[ui::LEFT as usize];
        frame[X_RIGHT as usize] = src[ui::RIGHT as usize];
        frame[X_NOTE as usize] = src[ui::PLAY as usize];
        frame[X_CUSTOM as usize] = src[ui::CUSTOM as usize];
        frame[X_CAPTURE as usize] = src[ui::SHIFT as usize];
    }
}

impl Default for XSurface {
    fn default() -> Self {
        Self::new()
    }
}
