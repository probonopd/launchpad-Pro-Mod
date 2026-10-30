// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 launchpad-Pro-Mod contributors

//! Novation-style sequencer on the Launchpad X. The sequencer core lives in the
//! `novseq` crate; this file connects it to the surface and USB MIDI. Playback
//! keeps running while other modes are on screen.

use crate::app::apptrait::App;
use crate::app::events::{AftertouchEvent, MidiEvent, SurfaceEvent};
use crate::driver;
use crate::sys::led;
use crate::sys::midi::MidiPort;
use crate::sys::{prefs, store};
use novseq::{LED_COUNT, MAX_PROJECT_BYTES, MidiOut, NovSeq, Storage};
use safestore::Item;

// The slots and the largest encoded project must fit what the store provides.
const _: () = assert!(novseq::PROJECT_SLOTS == safestore::PROJECT_SLOTS);
const _: () = assert!(MAX_PROJECT_BYTES <= safestore::project_capacity());

struct DriverOut;

impl MidiOut for DriverOut {
    fn send(&mut self, msg: &[u8]) {
        // Setup > MIDI can keep the sequencer from sending clock and transport.
        if prefs::is_clock_message(msg[0]) && !prefs::get().tx_clock {
            return;
        }
        driver::send_midi(MidiPort::Midi, msg);
        driver::send_midi(MidiPort::Daw, msg);
    }
}

struct FlashStorage;

// Projects are saved through the `safestore` crate like the other sequencer's: two banks
// per slot, the last good copy stays untouched while a new one is written, and a save only
// succeeds if it reads back correctly. The buffer lives on the stack only during the call.
impl Storage for FlashStorage {
    fn save_with(&mut self, slot: u8, fill: impl FnOnce(&mut [u8]) -> Option<usize>) -> bool {
        let mut buf = [0u8; MAX_PROJECT_BYTES];
        match fill(&mut buf) {
            Some(len) => store::save(Item::Project(slot), &buf[..len]),
            None => false,
        }
    }

    fn load_with(&mut self, slot: u8, apply: impl FnOnce(&[u8]) -> bool) -> bool {
        let mut buf = [0u8; MAX_PROJECT_BYTES];
        match store::load(Item::Project(slot), &mut buf) {
            Some(len) => apply(&buf[..len]),
            None => false,
        }
    }
}

pub struct NovSeqApp {
    seq: NovSeq,
    visible: bool,
    // Zero in a fresh static: the LEDs are all sent on entering (`redraw`), so the
    // 13 KB project next to these never needs initial data from flash.
    shown: [u32; LED_COUNT],
    redraw: bool,
    frame: [u32; LED_COUNT],
}

impl NovSeqApp {
    pub const fn new() -> Self {
        Self {
            seq: NovSeq::new(),
            visible: false,
            shown: [0; LED_COUNT],
            redraw: false,
            frame: [0; LED_COUNT],
        }
    }

    /// Runs the clock. Called every host tick whichever mode is on screen.
    pub fn background_tick(&mut self) {
        self.seq.tick(embassy_time::Instant::now().as_micros(), &mut DriverOut);
    }

    /// Returns true if the message must not reach the active mode.
    pub fn midi_in(&mut self, event: &MidiEvent) -> bool {
        self.seq.midi_in([event.status, event.data1, event.data2], &mut DriverOut)
    }

    fn draw(&mut self) {
        self.seq.render(&mut self.frame);
        for index in 1..LED_COUNT {
            // 9, 99 and 100 are not LEDs.
            if matches!(index, 9 | 99 | 100) {
                continue;
            }
            if self.redraw || self.frame[index] != self.shown[index] {
                self.shown[index] = self.frame[index];
                led::set(index as u8, self.frame[index]);
            }
        }
        self.redraw = false;
    }
}

impl App for NovSeqApp {
    fn on_enter(&mut self) {
        self.visible = true;
        self.redraw = true;
        self.seq.enter();
        self.draw();
    }

    fn on_exit(&mut self) {
        self.visible = false;
        self.seq.exit(&mut DriverOut);
    }

    fn on_surface(&mut self, event: SurfaceEvent) {
        self.seq.button(event.index, event.pressed, event.value, &mut DriverOut, &mut FlashStorage);
    }

    fn on_midi(&mut self, _event: MidiEvent) {}

    fn on_aftertouch(&mut self, _event: AftertouchEvent) {}

    fn on_tick(&mut self) {
        if self.visible {
            self.draw();
        }
    }
}
