// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 launchpad-Pro-Mod contributors

//! Seq95: a Launchpad95-inspired hardware step sequencer for the Launchpad Pro Mk3.
//!
//! The crate is hardware independent. A host feeds it button presses, incoming
//! MIDI and a microsecond clock, and gives it somewhere to send MIDI, draw LEDs
//! and store projects. Button and LED indexes use the Launchpad Pro Mk3
//! programmer-mode layout (grid 11..88, edges 1..8, 10..80, 19..89, 90..98, 101..108).

#![cfg_attr(not(test), no_std)]

pub mod engine;
pub mod model;
mod ui;
pub mod xmap;

pub use engine::{Engine, MidiOut};
pub use model::*;
pub use ui::{Lane, LED_COUNT};

/// Persistent project slots.
pub const PROJECT_SLOTS: u8 = 8;

/// Project storage provided by the host. Slots are `0..PROJECT_SLOTS`.
pub trait Storage {
    /// True if the slot holds something that looks like a saved project.
    fn slot_used(&mut self, slot: u8) -> bool;
    /// Lets `fill` write an encoded project (at most `PROJECT_WIRE_SIZE` bytes) into a
    /// temporary buffer and writes the bytes it reports to the slot. Returns false if
    /// the write failed or was refused. The buffer belongs to the host and is only
    /// needed for the moment of saving: a 7 KB buffer inside `Seq95` once made the
    /// startup of a small device run out of stack.
    fn save_with(&mut self, slot: u8, fill: impl FnOnce(&mut [u8]) -> usize) -> bool;
    /// Reads the slot into a temporary buffer and lets `apply` use it. Returns false
    /// if nothing could be read or `apply` refused the data.
    fn load_with(&mut self, slot: u8, apply: impl FnOnce(&[u8]) -> bool) -> bool;
}

pub struct Seq95 {
    pub project: Project,
    pub engine: Engine,
    ui: ui::Ui,
}

impl Seq95 {
    pub const fn new() -> Self {
        Self {
            project: Project::new(),
            engine: Engine::new(),
            ui: ui::Ui::new(),
        }
    }

    /// Call every millisecond or so, also while the sequencer is not on screen.
    pub fn tick(&mut self, now_us: u64, out: &mut impl MidiOut) {
        self.ui.now_us = now_us;
        self.engine.tick(now_us, &self.project, out);
    }

    /// Feed incoming MIDI (clock, start/stop, transposition notes).
    /// Returns true if the sequencer consumed the message.
    pub fn midi_in(&mut self, msg: [u8; 3], out: &mut impl MidiOut) -> bool {
        self.engine.midi_in(msg, &mut self.project, out)
    }

    /// Called when the sequencer screen is shown.
    pub fn enter(&mut self) {
        self.ui.release_all();
    }

    /// Called when another mode takes over the surface. Playback continues.
    pub fn exit(&mut self, out: &mut impl MidiOut) {
        self.ui.release_all();
        self.ui.stop_preview(&self.project, out);
    }

    pub fn button(
        &mut self,
        index: u8,
        pressed: bool,
        velocity: u8,
        out: &mut impl MidiOut,
        storage: &mut impl Storage,
    ) {
        ui::button(self, index, pressed, velocity, out, storage);
    }

    /// Draws the whole surface into `frame` (RGB 0xRRGGBB per LED index).
    pub fn render(&self, frame: &mut [u32; LED_COUNT]) {
        ui::render(self, frame);
    }

    #[cfg(feature = "persistence")]
    pub fn save_project(&mut self, slot: u8, storage: &mut impl Storage) -> bool {
        let project = &self.project;
        storage.save_with(slot, |buf| project.encode(buf))
    }

    #[cfg(feature = "persistence")]
    pub fn load_project(&mut self, slot: u8, storage: &mut impl Storage, out: &mut impl MidiOut) -> bool {
        let project = &mut self.project;
        if !storage.load_with(slot, |buf| project.decode_into(buf).is_ok()) {
            return false;
        }
        self.engine.all_notes_off(out);
        true
    }

    pub fn step_track(&mut self, forward: bool, out: &mut impl MidiOut) {
        ui::step_track(self, forward, out);
    }

    pub fn toggle_mute_current(&mut self) {
        ui::toggle_mute_current(self);
    }

    pub fn toggle_solo_current(&mut self) {
        ui::toggle_solo_current(self);
    }

    pub fn clear_current(&mut self) {
        ui::clear_current(self);
    }

    pub fn current_track(&self) -> usize {
        self.ui.track
    }
}

impl Default for Seq95 {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests;
