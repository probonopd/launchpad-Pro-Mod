// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2025-2026 Anthony Hofmeister

use crate::app::SurfaceEvent;
use crate::app::setup::page::Page;
use crate::app::setup::text::Text;
use crate::sys::{led, settings};

const TOGGLE_BUTTON: u8 = 31;
const CURVE_START: u8 = 21;
const CURVE_END: u8 = 23;
// Pad trigger threshold, four levels (light to firm), row 5.
#[cfg(feature = "launchpad-x")]
const TRIGGER_START: u8 = 51;

pub struct VelocityPage {
    text: Text,
}

impl VelocityPage {
    pub const fn new() -> Self {
        Self {
            text: Text::new(
                [0b10111110, 0b10111010, 0b10110010, 0b01011111],
                0b11100011,
                0xFFAA00,
                0xFFFBCC,
            ),
        }
    }

    fn draw(&self) {
        let (vel_enabled, vel_curve) = settings::with(|s| (s.velocity_enabled, s.velocity_curve));

        led::set(
            TOGGLE_BUTTON,
            if vel_enabled != 0 { 0x10ff10 } else { 0xff1010 },
        );

        if vel_enabled != 0 {
            for i in 0..3u8 {
                led::set(CURVE_START + i, 0x101010);
            }
            led::set(CURVE_START + vel_curve.min(2), 0xff4000);
        } else {
            for i in 0..3u8 {
                led::set(CURVE_START + i, 0x000000);
            }
        }

        #[cfg(feature = "launchpad-x")]
        {
            let level = crate::sys::prefs::get().trigger;
            for i in 0..crate::sys::prefs::TRIGGER_LEVELS {
                led::set(TRIGGER_START + i, if i == level { 0xffffff } else { 0x101010 });
            }
        }
    }
}

impl Page for VelocityPage {
    fn on_enter(&mut self) {
        led::set_raw(69, 0x551500);
        self.text.draw();
        self.draw();
    }

    fn on_surface(&mut self, event: SurfaceEvent) {
        if !event.pressed {
            return;
        }

        if event.index == TOGGLE_BUTTON {
            settings::update(|s| {
                s.velocity_enabled = if s.velocity_enabled != 0 { 0 } else { 1 };
            });
            self.draw();
            return;
        }

        if (CURVE_START..=CURVE_END).contains(&event.index) {
            if settings::with(|s| s.velocity_enabled) == 0 {
                return;
            }
            
            let new_curve = event.index - CURVE_START;
            settings::update(|s| s.velocity_curve = new_curve);
            self.draw();
        }
    }
}
