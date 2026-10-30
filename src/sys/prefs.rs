// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 launchpad-Pro-Mod contributors

//! Launchpad X setup preferences that CoreFW's own settings record has no room
//! for (pad trigger threshold, aftertouch threshold, MIDI clock in and out, Vegas
//! mode, LED sleep). They live in the store
//! window, not in the CoreFW settings record, so that record keeps its shared
//! format and stays readable by the other devices' firmware.

use core::cell::UnsafeCell;

use crate::sys::driver::launchpad_x::inputs;
use crate::sys::store;
use safestore::Item;

// Format 1 had only the two thresholds; it is still read, the rest takes the defaults.
const FORMAT: u8 = 2;
const LEN_V1: usize = 3;
const LEN: usize = 7;
pub const TRIGGER_LEVELS: u8 = 4;
pub const AFTERTOUCH_LEVELS: u8 = 3;

#[derive(Copy, Clone)]
pub struct Prefs {
    pub trigger: u8,
    pub aftertouch: u8,
    /// Incoming MIDI clock and transport reach the sequencer and the LEDs.
    pub rx_clock: bool,
    /// The sequencer may send MIDI clock and transport.
    pub tx_clock: bool,
    /// After five idle minutes the LEDs run a light show.
    pub vegas: bool,
    /// After the idle time the LEDs go dark.
    pub sleep: bool,
}

impl Prefs {
    const DEFAULT: Self =
        Self { trigger: 0, aftertouch: 1, rx_clock: true, tx_clock: true, vegas: false, sleep: false };

    fn decode(raw: &[u8]) -> Option<Self> {
        let current = raw.len() == LEN && raw[0] == FORMAT;
        let old = raw.len() == LEN_V1 && raw[0] == 1;
        if !(current || old) || raw[1] >= TRIGGER_LEVELS || raw[2] >= AFTERTOUCH_LEVELS {
            return None;
        }
        let mut prefs = Self { trigger: raw[1], aftertouch: raw[2], ..Self::DEFAULT };
        if current {
            if raw[3..].iter().any(|&b| b > 1) {
                return None;
            }
            prefs.rx_clock = raw[3] == 1;
            prefs.tx_clock = raw[4] == 1;
            prefs.vegas = raw[5] == 1;
            prefs.sleep = raw[6] == 1;
        }
        Some(prefs)
    }
}

struct Global(UnsafeCell<Prefs>);

// SAFETY: only used from the single main task.
unsafe impl Sync for Global {}

static PREFS: Global = Global(UnsafeCell::new(Prefs::DEFAULT));

pub fn get() -> Prefs {
    unsafe { *PREFS.0.get() }
}

/// Applies the saved values; defaults (the old fixed thresholds) when nothing valid is saved.
pub fn load() {
    let mut raw = [0u8; LEN];
    let prefs = match store::load(Item::SetupPrefs, &mut raw) {
        Some(len) => Prefs::decode(&raw[..len]).unwrap_or(Prefs::DEFAULT),
        None => Prefs::DEFAULT,
    };
    set(prefs);
}

pub fn set(prefs: Prefs) {
    unsafe { *PREFS.0.get() = prefs };
    inputs::set_trigger_level(prefs.trigger);
    inputs::set_aftertouch_threshold(prefs.aftertouch);
}

pub fn save() -> bool {
    let p = get();
    store::save(
        Item::SetupPrefs,
        &[FORMAT, p.trigger, p.aftertouch, p.rx_clock as u8, p.tx_clock as u8, p.vegas as u8, p.sleep as u8],
    )
}

/// Updates one preference and applies it.
pub fn update(f: impl FnOnce(&mut Prefs)) {
    let mut prefs = get();
    f(&mut prefs);
    set(prefs);
}

/// True for the MIDI messages the Setup MIDI page can switch off: clock, start, continue, stop.
pub fn is_clock_message(status: u8) -> bool {
    matches!(status, 0xf8 | 0xfa | 0xfb | 0xfc)
}
