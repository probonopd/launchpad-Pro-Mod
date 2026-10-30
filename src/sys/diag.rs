// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 launchpad-Pro-Mod contributors

//! Read-only memory diagnostic (feature `diag`, Launchpad X only). It answers
//! SysEx requests on the MIDI port with chunks of the internal flash (including
//! the bootloader), the external SPI flash and a few system areas, so that a
//! computer can map which areas are erased before anything is ever written there.
//!
//! This module never writes or erases anything: internal memory is read with
//! plain loads from fixed, bounds-checked ranges, the external flash with the
//! read-only driver path. Every failure is answered with an explicit error
//! message, never with substituted data.
//!
//! Requests (manufacturer id 0x7D, "non-commercial", then "LPXD"):
//!   F0 7D 4C 50 58 44 01 F7                              info
//!   F0 7D 4C 50 58 44 02 <region> <off x4, 7 bits each, big endian> <len> F7   read
//! Replies use the same prefix with command 0x41 (info), 0x42 (read) or 0x7F (error).
//! Data bytes are packed 7 raw bytes into 8 SysEx bytes (an MSB byte first).

use crate::driver;
use crate::sys::midi::MidiPort;

const PREFIX: [u8; 6] = [0xf0, 0x7d, b'L', b'P', b'X', b'D'];
const CMD_INFO: u8 = 0x01;
const CMD_READ: u8 = 0x02;
const REPLY_INFO: u8 = 0x41;
const REPLY_READ: u8 = 0x42;
const REPLY_ERROR: u8 = 0x7f;

/// The USB transmit path takes at most 32 packets (96 bytes) per message.
const CHUNK: usize = 56;
const REPLY_MAX: usize = 96;

const ERR_BAD_REQUEST: u8 = 1;
const ERR_BAD_REGION: u8 = 2;
const ERR_RANGE: u8 = 3;
const ERR_LENGTH: u8 = 4;
const ERR_READ_FAILED: u8 = 5;

const REGION_INTERNAL: u8 = 0;
const REGION_EXTERNAL: u8 = 1;
const REGION_OTP: u8 = 2;
const REGION_OPTION_BYTES: u8 = 3;
const REGION_SYSTEM_ROM: u8 = 4;

const INTERNAL_BASE: u32 = 0x0800_0000;
/// STM32F401RE; the size register is checked against it.
const INTERNAL_MAX: u32 = 0x8_0000;
const FLASH_SIZE_KB_REG: u32 = 0x1fff_7a22;
const UID_BASE: u32 = 0x1fff_7a10;
const OTP_BASE: u32 = 0x1fff_7800;
const OTP_LEN: u32 = 0x230;
const OPTION_BYTES_BASE: u32 = 0x1fff_c000;
const OPTION_BYTES_LEN: u32 = 16;
const SYSTEM_ROM_BASE: u32 = 0x1fff_0000;
const SYSTEM_ROM_LEN: u32 = 0x7800;

/// Plain loads from a memory-mapped, always-readable address range.
fn read_mapped(address: u32, out: &mut [u8]) {
    for (i, byte) in out.iter_mut().enumerate() {
        // The caller bounds-checked the range against the fixed tables above.
        *byte = unsafe { core::ptr::read_volatile((address as usize + i) as *const u8) };
    }
}

fn internal_len() -> Option<u32> {
    let mut kb = [0u8; 2];
    read_mapped(FLASH_SIZE_KB_REG, &mut kb);
    let kb = u16::from_le_bytes(kb) as u32;
    if kb == 0 || kb > 1024 {
        return None;
    }
    Some((kb * 1024).min(INTERNAL_MAX))
}

struct Reply {
    buf: [u8; REPLY_MAX],
    len: usize,
}

impl Reply {
    fn new(command: u8) -> Self {
        let mut reply = Self { buf: [0; REPLY_MAX], len: 0 };
        for byte in PREFIX {
            reply.push(byte);
        }
        reply.push(command);
        reply
    }

    fn push(&mut self, byte: u8) {
        self.buf[self.len] = byte;
        self.len += 1;
    }

    /// 7 raw bytes become 8 SysEx bytes: an MSB byte, then the low 7 bits of each.
    fn push_packed(&mut self, raw: &[u8]) {
        for group in raw.chunks(7) {
            let mut msbs = 0u8;
            for (i, byte) in group.iter().enumerate() {
                msbs |= (byte >> 7) << i;
            }
            self.push(msbs);
            for byte in group {
                self.push(byte & 0x7f);
            }
        }
    }

    fn finish_and_send(mut self) {
        self.push(0xf7);
        driver::send_midi(MidiPort::Midi, &self.buf[..self.len]);
    }
}

fn error(code: u8) {
    let mut reply = Reply::new(REPLY_ERROR);
    reply.push(code);
    reply.finish_and_send();
}

/// Handles a request. Returns true if the message was a diagnostic message.
pub fn execute(data: &[u8]) -> bool {
    if !data.starts_with(&PREFIX) {
        return false;
    }
    match data.get(PREFIX.len()) {
        Some(&CMD_INFO) if data.len() == PREFIX.len() + 2 => info(),
        Some(&CMD_READ) if data.len() == PREFIX.len() + 8 => {
            let offset = data[PREFIX.len() + 2..PREFIX.len() + 6]
                .iter()
                .fold(0u32, |acc, byte| (acc << 7) | (*byte & 0x7f) as u32);
            read(data[PREFIX.len() + 1], offset, data[PREFIX.len() + 6] as usize);
        }
        _ => error(ERR_BAD_REQUEST),
    }
    true
}

fn info() {
    let Some(internal) = internal_len() else {
        error(ERR_READ_FAILED);
        return;
    };
    let mut raw = [0u8; 2 + 12 + 16 + 1 + 3 + 4];
    raw[..2].copy_from_slice(&((internal / 1024) as u16).to_le_bytes());
    read_mapped(UID_BASE, &mut raw[2..14]);
    read_mapped(OPTION_BYTES_BASE, &mut raw[14..30]);
    match driver::flash_jedec() {
        Some(jedec) => {
            raw[30] = 1;
            raw[31..34].copy_from_slice(&jedec);
        }
        None => raw[30] = 0,
    }
    raw[34..38].copy_from_slice(&driver::flash_physical_size().to_le_bytes());
    let mut reply = Reply::new(REPLY_INFO);
    reply.push_packed(&raw);
    reply.finish_and_send();
}

fn read(region: u8, offset: u32, len: usize) {
    if len == 0 || len > CHUNK {
        error(ERR_LENGTH);
        return;
    }
    let (base, size) = match region {
        REGION_INTERNAL => match internal_len() {
            Some(size) => (INTERNAL_BASE, size),
            None => return error(ERR_READ_FAILED),
        },
        REGION_EXTERNAL => (0, driver::flash_physical_size()),
        REGION_OTP => (OTP_BASE, OTP_LEN),
        REGION_OPTION_BYTES => (OPTION_BYTES_BASE, OPTION_BYTES_LEN),
        REGION_SYSTEM_ROM => (SYSTEM_ROM_BASE, SYSTEM_ROM_LEN),
        _ => return error(ERR_BAD_REGION),
    };
    if offset.checked_add(len as u32).is_none_or(|end| end > size) {
        error(ERR_RANGE);
        return;
    }

    let mut raw = [0u8; CHUNK];
    let raw = &mut raw[..len];
    if region == REGION_EXTERNAL {
        if !driver::read_flash_checked(offset, raw) {
            error(ERR_READ_FAILED);
            return;
        }
    } else {
        read_mapped(base + offset, raw);
    }

    let sum = raw.iter().fold(0u16, |acc, byte| acc.wrapping_add(*byte as u16)) & 0x3fff;
    let mut reply = Reply::new(REPLY_READ);
    reply.push(region);
    for shift in [21, 14, 7, 0] {
        reply.push(((offset >> shift) & 0x7f) as u8);
    }
    reply.push(len as u8);
    reply.push_packed(raw);
    reply.push((sum >> 7) as u8);
    reply.push((sum & 0x7f) as u8);
    reply.finish_and_send();
}
