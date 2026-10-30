// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 launchpad-Pro-Mod contributors

//! Custom Modes (Launchpad X). The modes live in the `custommode` crate; this
//! file connects them to the surface, USB MIDI and the LEDs. The right-hand
//! button column picks the mode (top to bottom = Custom 1 to 8, where the
//! Launchpad Pro Mk3 uses its Track Select buttons), and holding Capture MIDI
//! shows the master channel pads (the Pro's Shift + Custom).

use crate::app::apptrait::App;
use crate::app::events::{AftertouchEvent, MidiEvent, SurfaceEvent};
use crate::app::performance::pad_velocity;
use crate::driver;
use crate::sys::led;
use crate::sys::midi::MidiPort;
use crate::sys::settings;
use crate::sys::store;
use custommode::{CustomModes, LED_COUNT, ModeOut, PALETTE_FLAG, PREFS_LEN};
use safestore::Item;

const CAPTURE_MIDI: u8 = 98;

struct DriverModes;

impl ModeOut for DriverModes {
    fn note_on(&mut self, channel: u8, note: u8, velocity: u8) {
        driver::send_midi(MidiPort::Midi, &[0x90 | channel, note, velocity]);
    }

    fn note_off(&mut self, channel: u8, note: u8) {
        driver::send_midi(MidiPort::Midi, &[0x80 | channel, note, 0]);
    }

    fn control_change(&mut self, channel: u8, cc: u8, value: u8) {
        driver::send_midi(MidiPort::Midi, &[0xb0 | channel, cc, value]);
    }

    fn program_change(&mut self, channel: u8, program: u8) {
        driver::send_midi(MidiPort::Midi, &[0xc0 | channel, program]);
    }
}

pub struct CustomApp {
    modes: CustomModes,
    started: bool,
    visible: bool,
    shown: [u32; LED_COUNT],
    frame: [u32; LED_COUNT],
}

impl CustomApp {
    pub const fn new() -> Self {
        Self {
            modes: CustomModes::new(),
            started: false,
            visible: false,
            shown: [u32::MAX; LED_COUNT],
            frame: [0; LED_COUNT],
        }
    }

    fn draw(&mut self) {
        self.modes.render(&mut self.frame);
        for index in 1..LED_COUNT {
            let color = self.frame[index];
            if color == self.shown[index] {
                continue;
            }
            self.shown[index] = color;
            if index % 10 == 9 {
                // The mode buttons are lit by physical position, like their events.
                led::set_raw(index as u8, color);
            } else if (11..=88).contains(&index) && (1..=8).contains(&(index % 10)) {
                if color & PALETTE_FLAG != 0 {
                    led::set_palette(index as u8, (color & 0x7f) as u8);
                } else {
                    led::set(index as u8, color);
                }
            }
        }
    }
}

impl App for CustomApp {
    fn on_enter(&mut self) {
        if !self.started {
            self.modes.start();
            self.started = true;
            let mut buf = [0u8; PREFS_LEN];
            if store::load(Item::CustomPrefs, &mut buf) == Some(PREFS_LEN) {
                self.modes.import_prefs(&buf);
            }
        }
        self.visible = true;
        self.shown = [u32::MAX; LED_COUNT];
        self.draw();
    }

    fn on_exit(&mut self) {
        self.visible = false;
        self.modes.set_channel_view(false, &mut DriverModes);
        self.modes.release_all(&mut DriverModes);
    }

    fn on_surface(&mut self, event: SurfaceEvent) {
        if !(11..=88).contains(&event.index) || !(1..=8).contains(&(event.index % 10)) {
            return;
        }
        let velocity = if event.pressed { pad_velocity(event.value) } else { 0 };
        self.modes.pad(event.index, event.pressed, velocity, &mut DriverModes);
    }

    fn on_surface_raw(&mut self, event: SurfaceEvent) {
        if event.index == CAPTURE_MIDI {
            self.modes.set_channel_view(event.pressed, &mut DriverModes);
        } else if event.pressed && event.index % 10 == 9 {
            if let Some(mode) = CustomModes::mode_for_button_row(event.index / 10) {
                self.modes.select(mode, &mut DriverModes);
            }
        }
    }

    fn on_midi(&mut self, event: MidiEvent) {
        if event.port == MidiPort::Midi {
            self.modes.midi_in(event.status, event.data1, event.data2);
        }
    }

    fn on_aftertouch(&mut self, event: AftertouchEvent) {
        let Some((channel, note)) = self.modes.aftertouch_target(event.index) else {
            return;
        };
        match settings::with(|settings| settings.aftertouch_mode) {
            1 => driver::send_midi(MidiPort::Midi, &[0xa0 | channel, note, event.value]),
            2 => driver::send_midi(MidiPort::Midi, &[0xd0 | channel, event.value]),
            _ => {}
        }
    }

    fn on_tick(&mut self) {
        if self.visible {
            self.draw();
            // Saving blocks for a moment, so it waits until the channel view is
            // closed and no pad is held. A failed save is dropped, not retried.
            if self.modes.take_dirty_when_idle() {
                store::save(Item::CustomPrefs, &self.modes.export_prefs());
            }
        }
    }
}
