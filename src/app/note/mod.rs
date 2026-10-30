// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 launchpad-Pro-Mod contributors

//! Note Mode (Launchpad X). The layouts and settings live in the `notemode`
//! crate; this file connects it to the surface, USB MIDI and the LEDs. The top
//! row buttons are handled by their physical position (raw events), so the
//! printed arrows keep meaning octave and semitone whatever the rotation is.

use crate::app::apptrait::App;
use crate::app::events::{AftertouchEvent, MidiEvent, SurfaceEvent};
use crate::app::performance::pad_velocity;
use crate::driver;
use crate::sys::led;
use crate::sys::midi::MidiPort;
use crate::sys::settings;
use crate::sys::store;
use notemode::{LED_COUNT, NoteMode, NoteOut, PREFS_LEN};
use safestore::Item;

struct DriverNotes;

impl NoteOut for DriverNotes {
    fn note_on(&mut self, channel: u8, note: u8, velocity: u8) {
        driver::send_midi(MidiPort::Midi, &[0x90 | channel, note, velocity]);
    }

    fn note_off(&mut self, channel: u8, note: u8) {
        driver::send_midi(MidiPort::Midi, &[0x80 | channel, note, 0]);
    }
}

pub struct NoteApp {
    mode: NoteMode,
    loaded: bool,
    visible: bool,
    shown: [u32; LED_COUNT],
    frame: [u32; LED_COUNT],
}

impl NoteApp {
    pub const fn new() -> Self {
        Self {
            mode: NoteMode::new(),
            loaded: false,
            visible: false,
            shown: [u32::MAX; LED_COUNT],
            frame: [0; LED_COUNT],
        }
    }

    fn draw(&mut self) {
        self.mode.render(&mut self.frame);
        for index in 1..LED_COUNT {
            if self.frame[index] == self.shown[index] {
                continue;
            }
            self.shown[index] = self.frame[index];
            // The top row (91..=98) is lit by physical position, like its events.
            if index >= 91 {
                led::set_raw(index as u8, self.frame[index]);
            } else if matches!(index % 10, 1..=8) {
                led::set(index as u8, self.frame[index]);
            }
        }
    }
}

impl App for NoteApp {
    fn on_enter(&mut self) {
        if !self.loaded {
            // The saved preferences are applied once, when the mode first starts.
            self.loaded = true;
            let mut buf = [0u8; PREFS_LEN];
            if store::load(Item::NotePrefs, &mut buf) == Some(PREFS_LEN) {
                self.mode.import_prefs(&buf);
            }
        }
        self.visible = true;
        self.shown = [u32::MAX; LED_COUNT];
        self.draw();
    }

    fn on_exit(&mut self) {
        self.visible = false;
        self.mode.exit(&mut DriverNotes);
    }

    fn on_surface(&mut self, event: SurfaceEvent) {
        // Only the 8x8 grid arrives here; the buttons are handled in `on_surface_raw`.
        if !matches!(event.index % 10, 1..=8) || !(11..=88).contains(&event.index) {
            return;
        }
        let velocity = if event.pressed { pad_velocity(event.value) } else { 0 };
        self.mode.pad(event.index, event.pressed, velocity, &mut DriverNotes);
    }

    fn on_surface_raw(&mut self, event: SurfaceEvent) {
        self.mode.button(event.index, event.pressed, &mut DriverNotes);
    }

    fn on_midi(&mut self, _event: MidiEvent) {}

    fn on_aftertouch(&mut self, event: AftertouchEvent) {
        let Some(note) = self.mode.sounding(event.index) else {
            return;
        };
        let channel = self.mode.channel();
        match settings::with(|settings| settings.aftertouch_mode) {
            1 => driver::send_midi(MidiPort::Midi, &[0xa0 | channel, note, event.value]),
            2 => driver::send_midi(MidiPort::Midi, &[0xd0 | channel, event.value]),
            _ => {}
        }
    }

    fn on_tick(&mut self) {
        if self.visible {
            self.draw();
            // Saving blocks for a moment, so it waits until the settings page is
            // closed and no note sounds. A failed save is dropped, not retried.
            if self.mode.take_dirty_when_idle() {
                store::save(Item::NotePrefs, &self.mode.export_prefs());
            }
        }
    }
}
