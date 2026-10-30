// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 launchpad-Pro-Mod contributors

//! Seq95 step sequencer mode (Launchpad Pro Mk3). The sequencer core lives in
//! the `seq95` crate; this file connects it to the CoreFW surface, USB MIDI and
//! external flash. Playback keeps running while other modes are on screen.

use crate::app::apptrait::App;
use crate::app::events::{AftertouchEvent, MidiEvent, SurfaceEvent};
use crate::driver;
use crate::sys::led;
use crate::sys::midi::MidiPort;
#[cfg(not(feature = "launchpad-x"))]
use seq95::LED_COUNT;
use seq95::{MidiOut, PORT_USB_DAW, PORT_USB_MIDI, Seq95, Storage};
#[cfg(feature = "launchpad-x")]
use crate::sys::{prefs, store};
#[cfg(feature = "launchpad-x")]
use safestore::Item;
#[cfg(feature = "launchpad-x")]
use seq95::xmap::{X_LED_COUNT, XSurface};

/// Surface size the LED cache and frame cover.
#[cfg(feature = "launchpad-x")]
const SURFACE_LEDS: usize = X_LED_COUNT;
#[cfg(not(feature = "launchpad-x"))]
const SURFACE_LEDS: usize = LED_COUNT;

// The project slots and the size of a project must fit what the store provides.
#[cfg(feature = "launchpad-x")]
const _: () = assert!(seq95::PROJECT_SLOTS == safestore::PROJECT_SLOTS);
#[cfg(feature = "launchpad-x")]
const _: () = assert!(seq95::PROJECT_WIRE_SIZE <= safestore::project_capacity());

struct DriverOut;

impl MidiOut for DriverOut {
    fn send(&mut self, ports: u8, msg: &[u8]) {
        // Setup > MIDI can keep the sequencer from sending clock and transport.
        #[cfg(feature = "launchpad-x")]
        if prefs::is_clock_message(msg[0]) && !prefs::get().tx_clock {
            return;
        }
        if ports & PORT_USB_MIDI != 0 {
            driver::send_midi(MidiPort::Midi, msg);
        }
        if ports & PORT_USB_DAW != 0 {
            driver::send_midi(MidiPort::Daw, msg);
        }
        // PORT_DIN: the Mk3 TRS MIDI output is not implemented in CoreFW yet.
    }
}

struct FlashStorage;

// The Launchpad X saves projects in its own window of the external flash through
// the `safestore` crate: two banks per slot, the last good copy is never touched
// while a new one is written, and a save only succeeds if it reads back correctly.
// The 7 KB buffer for encoding and decoding lives on the stack only while saving or
// loading, not in the app host, whose size the startup stack has to hold.
#[cfg(not(feature = "launchpad-pro-mk3"))]
impl Storage for FlashStorage {
    fn slot_used(&mut self, slot: u8) -> bool {
        store::exists(Item::Project(slot))
    }

    fn save_with(&mut self, slot: u8, fill: impl FnOnce(&mut [u8]) -> usize) -> bool {
        let mut buf = [0u8; seq95::PROJECT_WIRE_SIZE];
        let len = fill(&mut buf);
        store::save(Item::Project(slot), &buf[..len])
    }

    fn load_with(&mut self, slot: u8, apply: impl FnOnce(&[u8]) -> bool) -> bool {
        let mut buf = [0u8; seq95::PROJECT_WIRE_SIZE];
        store::load(Item::Project(slot), &mut buf).is_some() && apply(&buf)
    }
}

#[cfg(feature = "launchpad-pro-mk3")]
impl Storage for FlashStorage {
    fn slot_used(&mut self, slot: u8) -> bool {
        let mut header = [0xffu8; 8];
        driver::read_project_slot(slot, 0, &mut header) && seq95::looks_like_project(&header)
    }

    fn save_with(&mut self, slot: u8, fill: impl FnOnce(&mut [u8]) -> usize) -> bool {
        let mut buf = [0u8; seq95::PROJECT_WIRE_SIZE];
        let len = fill(&mut buf);
        driver::write_project_slot(slot, &buf[..len], seq95::looks_like_project)
    }

    fn load_with(&mut self, slot: u8, apply: impl FnOnce(&[u8]) -> bool) -> bool {
        let mut buf = [0u8; seq95::PROJECT_WIRE_SIZE];
        driver::read_project_slot(slot, 0, &mut buf) && apply(&buf)
    }
}

pub struct Seq95App {
    seq: Seq95,
    #[cfg(feature = "launchpad-x")]
    surface: XSurface,
    visible: bool,
    shown: [u32; SURFACE_LEDS],
    frame: [u32; SURFACE_LEDS],
}

impl Seq95App {
    pub const fn new() -> Self {
        Self {
            seq: Seq95::new(),
            #[cfg(feature = "launchpad-x")]
            surface: XSurface::new(),
            visible: false,
            shown: [u32::MAX; SURFACE_LEDS],
            frame: [0; SURFACE_LEDS],
        }
    }

    /// Runs the clock. Called every host tick whichever mode is on screen.
    pub fn background_tick(&mut self) {
        let now = embassy_time::Instant::now().as_micros();
        self.seq.tick(now, &mut DriverOut);
    }

    /// Clock, transport and transposition input. Returns true if the message
    /// must not reach the active mode (transposition notes).
    pub fn midi_in(&mut self, event: &MidiEvent) -> bool {
        let used = self
            .seq
            .midi_in([event.status, event.data1, event.data2], &mut DriverOut);
        used && event.status < 0xf0
    }

    fn draw(&mut self) {
        #[cfg(feature = "launchpad-x")]
        self.surface.render(&self.seq, &mut self.frame);
        #[cfg(not(feature = "launchpad-x"))]
        self.seq.render(&mut self.frame);
        for index in 1..SURFACE_LEDS {
            // 9, 99 and 100 are not LEDs.
            if matches!(index, 9 | 99 | 100) {
                continue;
            }
            if self.frame[index] != self.shown[index] {
                self.shown[index] = self.frame[index];
                led::set(index as u8, self.frame[index]);
            }
        }
    }
}

impl App for Seq95App {
    fn on_enter(&mut self) {
        self.visible = true;
        self.shown = [u32::MAX; SURFACE_LEDS];
        #[cfg(feature = "launchpad-x")]
        self.surface.reset();
        self.seq.enter();
        self.seq.engine.seed(embassy_time::Instant::now().as_ticks() as u32);
        self.draw();
    }

    fn on_exit(&mut self) {
        self.visible = false;
        #[cfg(feature = "launchpad-x")]
        self.surface.reset();
        self.seq.exit(&mut DriverOut);
    }

    #[cfg(feature = "launchpad-x")]
    fn on_surface(&mut self, event: SurfaceEvent) {
        self.surface.button(
            &mut self.seq,
            event.index,
            event.pressed,
            event.value,
            &mut DriverOut,
            &mut FlashStorage,
        );
    }

    #[cfg(not(feature = "launchpad-x"))]
    fn on_surface(&mut self, event: SurfaceEvent) {
        self.seq.button(
            event.index,
            event.pressed,
            event.value,
            &mut DriverOut,
            &mut FlashStorage,
        );
    }

    fn on_midi(&mut self, _event: MidiEvent) {}

    fn on_aftertouch(&mut self, _event: AftertouchEvent) {}

    fn on_tick(&mut self) {
        if self.visible {
            self.draw();
        }
    }
}
