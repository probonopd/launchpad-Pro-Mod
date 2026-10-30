// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 launchpad-Pro-Mod contributors

//! Novation-style step sequencer for the Launchpad X: four tracks of 32 steps,
//! up to eight notes per step, a Play Area for live notes. Hardware independent;
//! the host feeds buttons, MIDI and a microsecond clock. Indexes use the
//! programmer-mode layout (grid 11..88, edges 91..98 and 19..89).
//!
//! Steps, patterns (chains), scenes, recording with quantise and gate, velocity,
//! probability, mutation and micro steps, pattern and track settings. Projects
//! (saving) are handled by the host through `Project`.

#![cfg_attr(not(test), no_std)]

mod engine;
mod model;
mod projectio;
mod ui;

pub use engine::{Engine, MidiOut};
pub use model::*;
pub use projectio::{decode, encode, Storage, MAX_PROJECT_BYTES, PROJECT_SLOTS};
pub use ui::{Ui, View, LED_COUNT};

pub struct NovSeq {
    pub project: Project,
    pub engine: Engine,
    pub ui: Ui,
}

impl NovSeq {
    pub const fn new() -> Self {
        Self { project: Project::new(), engine: Engine::new(), ui: Ui::new() }
    }

    pub fn tick(&mut self, now_us: u64, out: &mut impl MidiOut) {
        self.ui.now_us = now_us;
        self.engine.tick(now_us, &mut self.project, out);
    }

    /// Returns true if the message was used (clock and transport only).
    pub fn midi_in(&mut self, msg: [u8; 3], out: &mut impl MidiOut) -> bool {
        self.engine.midi_in(msg, &mut self.project, out)
    }

    pub fn enter(&mut self) {
        if !self.project.ready {
            self.project.reset();
            self.engine.seed(self.ui.now_us as u32);
        }
        self.ui.reset();
    }

    pub fn exit(&mut self, out: &mut impl MidiOut) {
        self.ui.stop_previews(&self.project, out);
        self.ui.reset();
    }

    pub fn button(
        &mut self,
        index: u8,
        pressed: bool,
        value: u8,
        out: &mut impl MidiOut,
        storage: &mut impl Storage,
    ) {
        self.ui.button(&mut self.project, &mut self.engine, index, pressed, value, out, storage);
    }

    pub fn render(&self, frame: &mut [u32; LED_COUNT]) {
        self.ui.render(&self.project, &self.engine, frame);
    }
}

#[cfg(test)]
mod tests;
