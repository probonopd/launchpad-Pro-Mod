// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 launchpad-Pro-Mod contributors

//! Launchpad X only: which MIDI clock messages are used. Pad 1 of row 5 is the clock the
//! Launchpad receives, pad 2 the clock it sends (from the sequencer).

use crate::app::SurfaceEvent;
use crate::app::setup::page::Page;
use crate::sys::{led, prefs};

const RECEIVE: u8 = 51;
const SEND: u8 = 52;

pub struct MidiPage;

impl MidiPage {
    pub const fn new() -> Self {
        Self
    }

    fn draw(&self) {
        let p = prefs::get();
        led::set(RECEIVE, if p.rx_clock { 0x10ff10 } else { 0xff1010 });
        led::set(SEND, if p.tx_clock { 0x10ff10 } else { 0xff1010 });
    }
}

impl Page for MidiPage {
    fn on_enter(&mut self) {
        led::set_raw(49, 0x103030);
        self.draw();
    }

    fn on_surface(&mut self, event: SurfaceEvent) {
        if !event.pressed {
            return;
        }
        match event.index {
            RECEIVE => prefs::update(|p| p.rx_clock = !p.rx_clock),
            SEND => prefs::update(|p| p.tx_clock = !p.tx_clock),
            _ => return,
        }
        self.draw();
    }
}
