// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 launchpad-Pro-Mod contributors

//! Vegas mode and LED sleep (Launchpad X): after a while without button presses the LEDs
//! run a light show, go dark, or both one after the other. The next press wakes them and
//! is not passed on, so waking the pad cannot play a note.

use core::cell::UnsafeCell;

use crate::driver;
use crate::sys::{led, prefs};

/// The host ticks once per millisecond.
const MINUTE_TICKS: u32 = 60_000;
const IDLE_TICKS: u32 = 5 * MINUTE_TICKS;
const FRAME_TICKS: u32 = 30;
const LED_COUNT: u8 = 99;

#[derive(Copy, Clone, Eq, PartialEq)]
enum State {
    Awake,
    Show,
    Dark,
}

struct Idle {
    ticks: u32,
    state: State,
    phase: u16,
}

struct Global(UnsafeCell<Idle>);

// SAFETY: only used from the single main task.
unsafe impl Sync for Global {}

static IDLE: Global = Global(UnsafeCell::new(Idle { ticks: 0, state: State::Awake, phase: 0 }));

fn with<R>(f: impl FnOnce(&mut Idle) -> R) -> R {
    unsafe { f(&mut *IDLE.0.get()) }
}

/// A button was pressed or released. Returns true if this woke the LEDs, in which case
/// the event is to be dropped.
pub fn touch() -> bool {
    with(|idle| {
        idle.ticks = 0;
        let was = idle.state;
        if was == State::Awake {
            return false;
        }
        idle.state = State::Awake;
        driver::set_blank(false);
        led::redraw();
        true
    })
}

/// A cheap rainbow wave: a position on the color wheel, brightest primary first.
fn wheel(position: u16) -> u32 {
    let p = (position % 192) as u32;
    let (r, g, b) = match p / 64 {
        0 => (63 - p % 64, p % 64, 0),
        1 => (0, 63 - p % 64, p % 64),
        _ => (p % 64, 0, 63 - p % 64),
    };
    (r * 3 << 16) | (g * 3 << 8) | (b * 3)
}

/// Called every host tick.
pub fn tick() {
    let prefs = prefs::get();
    if !prefs.vegas && !prefs.sleep {
        with(|idle| idle.ticks = 0);
        return;
    }
    with(|idle| {
        idle.ticks = idle.ticks.saturating_add(1);
        // With both on, the light show comes first and the dark five minutes later.
        let dark_at = if prefs.vegas { 2 * IDLE_TICKS } else { IDLE_TICKS };
        if prefs.sleep && idle.ticks >= dark_at {
            if idle.state != State::Dark {
                idle.state = State::Dark;
                driver::set_blank(true);
            }
        } else if prefs.vegas && idle.ticks >= IDLE_TICKS {
            if idle.state == State::Awake {
                idle.state = State::Show;
            }
            if idle.ticks % FRAME_TICKS == 0 {
                idle.phase = (idle.phase + 2) % 192;
                for index in 1..LED_COUNT {
                    // The grid and the edge buttons: a diagonal wave.
                    let wave = idle.phase + (index / 10 + index % 10) as u16 * 12;
                    driver::set_led(index, wheel(wave));
                }
            }
        }
    });
}
