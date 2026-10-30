// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 launchpad-Pro-Mod contributors

//! Saved data (sequencer projects, Note Mode and Custom Mode preferences) in the
//! store window of the external flash. All the rules (fixed window, two banks per
//! item, header written last, read back after writing) are in the `safestore`
//! crate and in the bounds checks of the flash driver; this file only connects
//! them. A failed save is reported to the caller and never retried here.

use crate::driver;
use crate::sys::driver::common::storage::external::STORE_WINDOW_SECTORS;
use safestore::{Error, Item, Store, Window};

const _: () = assert!(safestore::WINDOW_SECTORS == STORE_WINDOW_SECTORS);

struct Hardware;

impl Window for Hardware {
    fn read(&mut self, offset: u32, buf: &mut [u8]) -> Result<(), Error> {
        if driver::store_read(offset, buf) { Ok(()) } else { Err(Error::Io) }
    }

    fn erase_sector(&mut self, sector: usize) -> Result<(), Error> {
        if driver::store_erase_sector(sector) { Ok(()) } else { Err(Error::Io) }
    }

    fn program(&mut self, offset: u32, data: &[u8]) -> Result<(), Error> {
        if driver::store_program(offset, data) { Ok(()) } else { Err(Error::Io) }
    }
}

pub fn exists(item: Item) -> bool {
    Store::new(Hardware).exists(item).unwrap_or(false)
}

/// The newest valid copy of an item, or `None` if there is none.
pub fn load(item: Item, out: &mut [u8]) -> Option<usize> {
    Store::new(Hardware).load(item, out).ok().flatten()
}

/// True only if the data was written and read back correctly.
pub fn save(item: Item, data: &[u8]) -> bool {
    Store::new(Hardware).save(item, data).is_ok()
}
