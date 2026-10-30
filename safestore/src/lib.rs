// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 launchpad-Pro-Mod contributors

//! Saving data in a small fixed window of a NOR flash chip, built so that a bug or a
//! power loss can never damage anything outside the window or lose the last good copy.
//!
//! Safety rules, all enforced here and again by the driver below this crate:
//!
//! 1. The crate only knows *window-relative* addresses. It cannot name any byte
//!    outside the window, and every erase and program is checked against the window
//!    and against the bank being written.
//! 2. The only erase is the erase of one 4 KB sector. There is no chip or block erase.
//! 3. Every item has two banks (A and B). A save always writes the bank that does NOT
//!    hold the newest valid copy, so the last good copy is never touched.
//! 4. A bank becomes valid only by its header, which is programmed last, in one page
//!    that is programmed exactly once. A bank without a complete, CRC-correct header is
//!    treated as empty.
//! 5. After writing, the bank is read back and its CRC checked; a mismatch is an error.
//! 6. Nothing is guessed: unreadable or corrupt data is reported as absent.
//!
//! Bank layout (page = 256 bytes, sector = 4096 bytes):
//!   page 0:  header, 32 bytes (the rest of the page stays erased)
//!   page 1.. the payload, in whole pages
//!
//! Header (little endian): magic u32 | version u8 | item u8 | length u16 |
//!   generation u32 | payload crc32 u32 | header crc32 u32 | 12 bytes zero.
//! The magic is part of the single header program, so a torn write fails the CRC.

#![cfg_attr(not(test), no_std)]

pub const SECTOR: usize = 4096;
pub const PAGE: usize = 256;
const HEADER_LEN: usize = 32;
const MAGIC: u32 = 0x5358_504c; // "LPXS"
const VERSION: u8 = 1;

pub const PROJECT_SLOTS: u8 = 8;
/// Sectors of one bank of a project (8 KB: a project is 7147 bytes).
const PROJECT_BANK_SECTORS: usize = 2;
/// Sectors of one bank of a preferences item.
const PREFS_BANK_SECTORS: usize = 1;
const PROJECTS_SECTORS: usize = PROJECT_SLOTS as usize * 2 * PROJECT_BANK_SECTORS;
/// The setup preferences come first in the window. The window grew downwards when
/// they were added, so every older item keeps its place in the chip.
const SETUP_SECTORS: usize = 2 * PREFS_BANK_SECTORS;
const PREFS_ITEMS: usize = 2;
/// Total window size in sectors; the driver must provide exactly this many.
pub const WINDOW_SECTORS: usize = SETUP_SECTORS + PROJECTS_SECTORS + PREFS_ITEMS * 2 * PREFS_BANK_SECTORS;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Item {
    Project(u8),
    NotePrefs,
    CustomPrefs,
    SetupPrefs,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Error {
    /// The driver reported a failure.
    Io,
    /// Item or length outside what the layout allows.
    Range,
    /// The data written did not read back correctly; the old copy is still intact.
    Verify,
    /// The destination buffer is smaller than the stored data.
    BufferTooSmall,
}

/// The window of the flash chip, addressed from 0 to `WINDOW_SECTORS * SECTOR`.
pub trait Window {
    fn read(&mut self, offset: u32, buf: &mut [u8]) -> Result<(), Error>;
    /// Erases one whole sector of the window.
    fn erase_sector(&mut self, sector: usize) -> Result<(), Error>;
    /// Programs bytes that lie inside a single page. The bytes must be erased.
    fn program(&mut self, offset: u32, data: &[u8]) -> Result<(), Error>;
}

pub fn crc32(data: &[u8]) -> u32 {
    crc32_update(0xffff_ffff, data) ^ 0xffff_ffff
}

fn crc32_update(mut crc: u32, data: &[u8]) -> u32 {
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xedb8_8320 & 0u32.wrapping_sub(crc & 1));
        }
    }
    crc
}

#[derive(Copy, Clone)]
struct Bank {
    first_sector: usize,
    sectors: usize,
}

impl Bank {
    fn offset(&self) -> u32 {
        (self.first_sector * SECTOR) as u32
    }

    fn capacity(&self) -> usize {
        self.sectors * SECTOR - PAGE
    }
}

fn banks(item: Item) -> Option<(u8, [Bank; 2])> {
    let (id, first, sectors) = match item {
        Item::SetupPrefs => (PROJECT_SLOTS + 2, 0, PREFS_BANK_SECTORS),
        Item::Project(slot) if slot < PROJECT_SLOTS => (
            slot,
            SETUP_SECTORS + slot as usize * 2 * PROJECT_BANK_SECTORS,
            PROJECT_BANK_SECTORS,
        ),
        Item::NotePrefs => (PROJECT_SLOTS, SETUP_SECTORS + PROJECTS_SECTORS, PREFS_BANK_SECTORS),
        Item::CustomPrefs => (
            PROJECT_SLOTS + 1,
            SETUP_SECTORS + PROJECTS_SECTORS + 2 * PREFS_BANK_SECTORS,
            PREFS_BANK_SECTORS,
        ),
        _ => return None,
    };
    Some((id, [Bank { first_sector: first, sectors }, Bank { first_sector: first + sectors, sectors }]))
}

/// The largest payload of a project, usable in compile-time checks.
pub const fn project_capacity() -> usize {
    PROJECT_BANK_SECTORS * SECTOR - PAGE
}

/// The largest payload an item can hold.
pub fn max_len(item: Item) -> usize {
    banks(item).map_or(0, |(_, b)| b[0].capacity())
}

#[derive(Copy, Clone)]
struct Header {
    len: usize,
    generation: u32,
    payload_crc: u32,
}

pub struct Store<W: Window> {
    window: W,
}

impl<W: Window> Store<W> {
    pub fn new(window: W) -> Self {
        Self { window }
    }

    pub fn into_inner(self) -> W {
        self.window
    }

    fn read_header(&mut self, bank: &Bank, item_id: u8) -> Option<Header> {
        let mut raw = [0u8; HEADER_LEN];
        self.window.read(bank.offset(), &mut raw).ok()?;
        let u32_at = |i: usize| u32::from_le_bytes([raw[i], raw[i + 1], raw[i + 2], raw[i + 3]]);
        if u32_at(0) != MAGIC || raw[4] != VERSION || raw[5] != item_id {
            return None;
        }
        if crc32(&raw[..20]) != u32_at(20) {
            return None;
        }
        let len = u16::from_le_bytes([raw[6], raw[7]]) as usize;
        if len > bank.capacity() {
            return None;
        }
        Some(Header { len, generation: u32_at(8), payload_crc: u32_at(12) })
    }

    /// Reads the payload of a bank in small pieces and checks its CRC.
    fn payload_valid(&mut self, bank: &Bank, header: &Header, mut out: Option<&mut [u8]>) -> bool {
        let mut crc = 0xffff_ffffu32;
        let mut done = 0usize;
        let mut piece = [0u8; 64];
        while done < header.len {
            let n = (header.len - done).min(piece.len());
            if self.window.read(bank.offset() + (PAGE + done) as u32, &mut piece[..n]).is_err() {
                return false;
            }
            crc = crc32_update(crc, &piece[..n]);
            if let Some(out) = out.as_deref_mut() {
                out[done..done + n].copy_from_slice(&piece[..n]);
            }
            done += n;
        }
        crc ^ 0xffff_ffff == header.payload_crc
    }

    /// The newest valid bank of an item: (bank index, header).
    fn newest(&mut self, id: u8, bs: &[Bank; 2]) -> Option<(usize, Header)> {
        let mut best: Option<(usize, Header)> = None;
        for (i, bank) in bs.iter().enumerate() {
            if let Some(h) = self.read_header(bank, id) {
                if self.payload_valid(bank, &h, None) {
                    let newer = match &best {
                        None => true,
                        Some((_, b)) => (h.generation.wrapping_sub(b.generation) as i32) > 0,
                    };
                    if newer {
                        best = Some((i, h));
                    }
                }
            }
        }
        best
    }

    pub fn exists(&mut self, item: Item) -> Result<bool, Error> {
        let (id, bs) = banks(item).ok_or(Error::Range)?;
        Ok(self.newest(id, &bs).is_some())
    }

    /// Reads the newest valid copy into `out`; `None` if there is no valid copy.
    pub fn load(&mut self, item: Item, out: &mut [u8]) -> Result<Option<usize>, Error> {
        let (id, bs) = banks(item).ok_or(Error::Range)?;
        let Some((i, h)) = self.newest(id, &bs) else { return Ok(None) };
        if out.len() < h.len {
            return Err(Error::BufferTooSmall);
        }
        if !self.payload_valid(&bs[i], &h, Some(&mut out[..h.len])) {
            return Ok(None);
        }
        Ok(Some(h.len))
    }

    /// Writes a new copy into the bank that does not hold the newest valid one.
    pub fn save(&mut self, item: Item, data: &[u8]) -> Result<(), Error> {
        self.save_inner(item, data, None)
    }

    fn save_inner(&mut self, item: Item, data: &[u8], generation_override: Option<u32>) -> Result<(), Error> {
        let (id, bs) = banks(item).ok_or(Error::Range)?;
        if data.len() > bs[0].capacity() || data.len() > u16::MAX as usize {
            return Err(Error::Range);
        }
        let (target, generation) = match self.newest(id, &bs) {
            Some((i, h)) => (1 - i, h.generation.wrapping_add(1)),
            None => (0, 1),
        };
        let generation = generation_override.unwrap_or(generation);
        let bank = bs[target];

        for s in 0..bank.sectors {
            self.window.erase_sector(bank.first_sector + s)?;
        }

        // Payload first, page by page.
        let mut at = 0usize;
        while at < data.len() {
            let n = (data.len() - at).min(PAGE);
            self.window.program(bank.offset() + (PAGE + at) as u32, &data[at..at + n])?;
            at += n;
        }

        // Header last, in one program, which makes the bank valid.
        let mut raw = [0u8; HEADER_LEN];
        raw[0..4].copy_from_slice(&MAGIC.to_le_bytes());
        raw[4] = VERSION;
        raw[5] = id;
        raw[6..8].copy_from_slice(&(data.len() as u16).to_le_bytes());
        raw[8..12].copy_from_slice(&generation.to_le_bytes());
        raw[12..16].copy_from_slice(&crc32(data).to_le_bytes());
        let header_crc = crc32(&raw[..20]);
        raw[20..24].copy_from_slice(&header_crc.to_le_bytes());
        self.window.program(bank.offset(), &raw)?;

        // Read back and check.
        match self.read_header(&bank, id) {
            Some(h) if h.generation == generation && h.len == data.len() && self.payload_valid(&bank, &h, None) => Ok(()),
            _ => Err(Error::Verify),
        }
    }
}

#[cfg(test)]
mod tests;
