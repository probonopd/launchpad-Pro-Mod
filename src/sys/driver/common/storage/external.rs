// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2025-2026 Anthony Hofmeister
// Copyright (C) 2026 ZephyrCodesStuff

use core::cmp::min;

#[cfg(feature = "launchpad-pro-mk3")]
use core::convert::Infallible;
#[cfg(feature = "launchpad-pro-mk3")]
use embedded_hal::digital::v2::OutputPin;

use embassy_stm32::Peri;
use embassy_stm32::gpio::{Level, Output, Speed};
use embassy_stm32::mode::Blocking;
use embassy_stm32::peripherals;
use embassy_stm32::spi::mode::Master;
use embassy_stm32::spi::{Config as SpiConfig, Spi};
use embassy_stm32::time::Hertz;

use embedded_storage::nor_flash::{
    check_erase, check_read, check_write, ErrorType, NorFlash, NorFlashErrorKind, ReadNorFlash,
};
use spi_memory::series25::Flash;
use spi_memory::{BlockDevice, Read};

pub const PAGE_SIZE: usize = 256;
pub const SECTOR_SIZE: usize = 4096;

#[cfg(feature = "launchpad-pro-mk3")]
pub const TOTAL_SIZE: u32 = 16 * 1024 * 1024;
#[cfg(not(feature = "launchpad-pro-mk3"))]
pub const TOTAL_SIZE: u32 = 1024 * 1024;

#[cfg(feature = "launchpad-pro-mk3")]
pub const SETTINGS_SIZE: u32 = (SECTOR_SIZE - PAGE_SIZE) as u32;
#[cfg(not(feature = "launchpad-pro-mk3"))]
pub const SETTINGS_SIZE: u32 = 8 * 1024;

#[cfg(feature = "launchpad-pro-mk3")]
pub const SETTINGS_OFFSET: u32 = TOTAL_SIZE - 2 * SECTOR_SIZE as u32 + PAGE_SIZE as u32;
#[cfg(not(feature = "launchpad-pro-mk3"))]
pub const SETTINGS_OFFSET: u32 = TOTAL_SIZE - SETTINGS_SIZE;

// The window for saved data (projects and preferences, see the `safestore` crate).
// It ends where the settings window begins and lies far above the identity block
// (0x0000-0x1FFF) and the bootloader's staging area for updates (0x3000-0x3FFFF).
#[cfg(feature = "launchpad-x")]
pub const STORE_WINDOW_SECTORS: usize = 38;
#[cfg(feature = "launchpad-x")]
pub const STORE_WINDOW_BYTES: u32 = STORE_WINDOW_SECTORS as u32 * SECTOR_SIZE as u32;
#[cfg(feature = "launchpad-x")]
pub const STORE_WINDOW_OFFSET: u32 = SETTINGS_OFFSET - STORE_WINDOW_BYTES;
#[cfg(feature = "launchpad-x")]
const _: () = assert!(STORE_WINDOW_OFFSET % SECTOR_SIZE as u32 == 0);
#[cfg(feature = "launchpad-x")]
const _: () = assert!(STORE_WINDOW_OFFSET >= 0x0008_0000, "the store window must stay far above the staging area");
#[cfg(feature = "launchpad-x")]
const _: () = assert!(STORE_WINDOW_OFFSET + STORE_WINDOW_BYTES == SETTINGS_OFFSET);
// The window is only used on the chip it was checked on (Macronix, 8 Mbit).
#[cfg(feature = "launchpad-x")]
const STORE_JEDEC_ID: [u8; 3] = [0xc2, 0x20, 0x14];

#[cfg(feature = "launchpad-pro-mk3")]
const LEGACY_SETTINGS_OFFSET: u32 = TOTAL_SIZE - 2 * SECTOR_SIZE as u32;

#[cfg(feature = "launchpad-pro-mk3")]
const _: () = assert!(LEGACY_SETTINGS_OFFSET == 0x00ff_e000);
#[cfg(feature = "launchpad-pro-mk3")]
const _: () = assert!(SETTINGS_OFFSET == 0x00ff_e100);

// Seq95 project slots. Far away from the stock firmware's data at the start
// and the settings at the end of the chip. Writes are additionally refused for
// any sector that holds data not written by Seq95 (see `write_project_slot`).
#[cfg(feature = "launchpad-pro-mk3")]
pub const PROJECT_REGION_OFFSET: u32 = 0x00f0_0000;
#[cfg(feature = "launchpad-pro-mk3")]
pub const PROJECT_SLOT_SIZE: u32 = 2 * SECTOR_SIZE as u32;
#[cfg(feature = "launchpad-pro-mk3")]
pub const PROJECT_SLOT_COUNT: u32 = 8;
#[cfg(feature = "launchpad-pro-mk3")]
const _: () = assert!(
    PROJECT_REGION_OFFSET + PROJECT_SLOT_COUNT * PROJECT_SLOT_SIZE <= LEGACY_SETTINGS_OFFSET
);

#[cfg(feature = "launchpad-pro-mk3")]
const EXPECTED_JEDEC_MANUFACTURER: u8 = 0xc2;
#[cfg(feature = "launchpad-pro-mk3")]
const EXPECTED_JEDEC_CAPACITY: u8 = 0x18;
// The MCU runs at 216 MHz. Keep CS high for about 1 us between commands;
// this comfortably exceeds the flash chip's minimum CS deselect time.
#[cfg(feature = "launchpad-pro-mk3")]
const CS_DESELECT_DELAY_CYCLES: u32 = 216;

#[cfg(feature = "launchpad-pro-mk3")]
pub struct FlashCs<'d>(Output<'d>);

#[cfg(feature = "launchpad-pro-mk3")]
impl OutputPin for FlashCs<'_> {
    type Error = Infallible;

    fn set_low(&mut self) -> Result<(), Self::Error> {
        self.0.set_low();
        Ok(())
    }

    fn set_high(&mut self) -> Result<(), Self::Error> {
        self.0.set_high();
        cortex_m::asm::delay(CS_DESELECT_DELAY_CYCLES);
        Ok(())
    }
}

#[cfg(feature = "launchpad-pro-mk3")]
type FlashCsPin<'d> = FlashCs<'d>;
#[cfg(not(feature = "launchpad-pro-mk3"))]
type FlashCsPin<'d> = Output<'d>;

#[derive(Clone, Copy)]
pub struct ExtFlashInfo {
    pub present: bool,
    pub jedec_id: [u8; 3],
    pub status1: u8,
}

pub struct ExtFlash<'d> {
    flash: Option<Flash<Spi<'d, Blocking, Master>, FlashCsPin<'d>>>,
    sector_buf: [u8; SECTOR_SIZE],
    jedec_id: [u8; 3],
    present: bool,
}

impl<'d> ExtFlash<'d> {
    #[cfg(feature = "launchpad-x")]
    pub fn new(
        spi2: Peri<'d, peripherals::SPI2>,
        pb13: Peri<'d, peripherals::PB13>,
        pb15: Peri<'d, peripherals::PB15>,
        pb14: Peri<'d, peripherals::PB14>,
        pb12: Peri<'d, peripherals::PB12>,
    ) -> Self {
        let mut spi_cfg = SpiConfig::default();
        spi_cfg.frequency = Hertz(10_500_000);

        let spi = Spi::new_blocking(spi2, pb13, pb15, pb14, spi_cfg);
        let cs = Output::new(pb12, Level::High, Speed::VeryHigh);

        let mut this = Self {
            flash: Flash::init(spi, cs).ok(),
            sector_buf: [0xff; SECTOR_SIZE],
            jedec_id: [0; 3],
            present: false,
        };
        this.probe();
        this
    }

    #[cfg(feature = "launchpad-mini-mk3")]
    pub fn new(
        spi1: Peri<'d, peripherals::SPI1>,
        pa5: Peri<'d, peripherals::PA5>,
        pa7: Peri<'d, peripherals::PA7>,
        pa6: Peri<'d, peripherals::PA6>,
        pa2: Peri<'d, peripherals::PA2>,
    ) -> Self {
        let mut spi_cfg = SpiConfig::default();
        spi_cfg.frequency = Hertz(10_500_000);

        let spi = Spi::new_blocking(spi1, pa5, pa7, pa6, spi_cfg);
        let cs = Output::new(pa2, Level::High, Speed::VeryHigh);

        let mut this = Self {
            flash: Flash::init(spi, cs).ok(),
            sector_buf: [0xff; SECTOR_SIZE],
            jedec_id: [0; 3],
            present: false,
        };
        this.probe();
        this
    }

    #[cfg(feature = "launchpad-pro-mk3")]
    pub fn new(
        spi1: Peri<'d, peripherals::SPI1>,
        pa5: Peri<'d, peripherals::PA5>,
        pb5: Peri<'d, peripherals::PB5>,
        pb4: Peri<'d, peripherals::PB4>,
        pa15: Peri<'d, peripherals::PA15>,
    ) -> Self {
        let mut spi_cfg = SpiConfig::default();
        spi_cfg.frequency = Hertz(10_500_000);

        let spi = Spi::new_blocking(spi1, pa5, pb5, pb4, spi_cfg);
        let cs = FlashCs(Output::new(pa15, Level::High, Speed::VeryHigh));

        let mut this = Self {
            flash: Flash::init(spi, cs).ok(),
            sector_buf: [0xff; SECTOR_SIZE],
            jedec_id: [0; 3],
            present: false,
        };
        this.probe();
        this
    }

    pub const fn settings_size(&self) -> u32 {
        SETTINGS_SIZE
    }

    pub fn read_settings(&mut self, offset: u32, data: &mut [u8]) {
        if data.is_empty() {
            return;
        }
        if self.read(offset, data).is_err() {
            data.fill(0xff);
        }
    }

    pub fn write_settings(&mut self, offset: u32, data: &[u8]) {
        let _ = self.write(offset, data);
    }

    #[cfg(feature = "launchpad-pro-mk3")]
    pub fn migrate_legacy_settings(
        &mut self,
        wire_size: usize,
        validate: fn(&[u8]) -> bool,
    ) -> Result<bool, NorFlashErrorKind> {
        if wire_size == 0 || wire_size > SECTOR_SIZE - PAGE_SIZE {
            return Err(NorFlashErrorKind::OutOfBounds);
        }

        let (flash, sector_buf) = (&mut self.flash, &mut self.sector_buf);
        let flash = flash.as_mut().ok_or(NorFlashErrorKind::Other)?;

        sector_buf.fill(0xff);
        flash
            .read(LEGACY_SETTINGS_OFFSET, sector_buf)
            .map_err(|_| NorFlashErrorKind::Other)?;

        if !validate(&sector_buf[..wire_size]) {
            return Ok(false);
        }

        sector_buf.copy_within(0..wire_size, PAGE_SIZE);
        sector_buf[..PAGE_SIZE].fill(0xff);

        flash
            .erase_sectors(LEGACY_SETTINGS_OFFSET, 1)
            .map_err(|_| NorFlashErrorKind::Other)?;

        for page_off in (0..SECTOR_SIZE).step_by(PAGE_SIZE) {
            if !sector_buf[page_off..page_off + PAGE_SIZE]
                .iter()
                .all(|byte| *byte == 0xff)
            {
                flash
                    .write_bytes(
                        LEGACY_SETTINGS_OFFSET + page_off as u32,
                        &mut sector_buf[page_off..page_off + PAGE_SIZE],
                    )
                    .map_err(|_| NorFlashErrorKind::Other)?;
            }
        }

        let mut verify = [0xff; PAGE_SIZE];
        for page_off in (0..SECTOR_SIZE).step_by(PAGE_SIZE) {
            flash
                .read(LEGACY_SETTINGS_OFFSET + page_off as u32, &mut verify)
                .map_err(|_| NorFlashErrorKind::Other)?;
            if verify != sector_buf[page_off..page_off + PAGE_SIZE] {
                return Err(NorFlashErrorKind::Other);
            }
        }

        Ok(true)
    }

    /// Reads from a Seq95 project slot. Returns false on error or a bad slot.
    #[cfg(feature = "launchpad-pro-mk3")]
    pub fn read_project_slot(&mut self, slot: u32, offset: u32, data: &mut [u8]) -> bool {
        if slot >= PROJECT_SLOT_COUNT || offset as usize + data.len() > PROJECT_SLOT_SIZE as usize {
            return false;
        }
        let Some(flash) = self.flash.as_mut() else {
            return false;
        };
        flash
            .read(PROJECT_REGION_OFFSET + slot * PROJECT_SLOT_SIZE + offset, data)
            .is_ok()
    }

    /// Erases and rewrites a whole Seq95 project slot with `data`.
    ///
    /// `owned` decides whether the slot's current contents may be replaced:
    /// it receives the first bytes of the slot. A slot is also writable when
    /// it is fully erased. Anything else is left untouched and false is
    /// returned, so foreign data on the chip is never destroyed.
    #[cfg(feature = "launchpad-pro-mk3")]
    pub fn write_project_slot(&mut self, slot: u32, data: &[u8], owned: fn(&[u8]) -> bool) -> bool {
        if slot >= PROJECT_SLOT_COUNT || data.len() > PROJECT_SLOT_SIZE as usize {
            return false;
        }
        let base = PROJECT_REGION_OFFSET + slot * PROJECT_SLOT_SIZE;
        let (flash, sector_buf) = (&mut self.flash, &mut self.sector_buf);
        let Some(flash) = flash.as_mut() else {
            return false;
        };

        // Ownership check: ours, or completely blank.
        for sector in 0..(PROJECT_SLOT_SIZE as usize / SECTOR_SIZE) {
            if flash
                .read(base + (sector * SECTOR_SIZE) as u32, sector_buf)
                .is_err()
            {
                return false;
            }
            if sector == 0 && owned(&sector_buf[..]) {
                break;
            }
            if sector_buf.iter().any(|byte| *byte != 0xff) {
                return false;
            }
        }

        for sector in 0..(PROJECT_SLOT_SIZE as usize / SECTOR_SIZE) {
            let sector_base = base + (sector * SECTOR_SIZE) as u32;
            if flash.erase_sectors(sector_base, 1).is_err() {
                return false;
            }
            let start = sector * SECTOR_SIZE;
            if start >= data.len() {
                continue;
            }
            let end = min(data.len(), start + SECTOR_SIZE);
            sector_buf.fill(0xff);
            sector_buf[..end - start].copy_from_slice(&data[start..end]);
            for page_off in (0..SECTOR_SIZE).step_by(PAGE_SIZE) {
                let page = &mut sector_buf[page_off..page_off + PAGE_SIZE];
                if page.iter().all(|byte| *byte == 0xff) {
                    continue;
                }
                if flash.write_bytes(sector_base + page_off as u32, page).is_err() {
                    return false;
                }
            }
        }

        // Verify.
        let mut verify = [0u8; PAGE_SIZE];
        for (i, chunk) in data.chunks(PAGE_SIZE).enumerate() {
            let buf = &mut verify[..chunk.len()];
            if flash.read(base + (i * PAGE_SIZE) as u32, buf).is_err() || buf != chunk {
                return false;
            }
        }
        true
    }

    /// Reads the chip at a physical offset, outside the settings window that
    /// `read` is limited to. Read-only; used by the flash diagnostic.
    #[cfg(feature = "diag")]
    pub fn read_physical(&mut self, offset: u32, bytes: &mut [u8]) -> bool {
        if offset.checked_add(bytes.len() as u32).is_none_or(|end| end > TOTAL_SIZE) {
            return false;
        }
        match self.flash.as_mut() {
            Some(flash) => flash.read(offset, bytes).is_ok(),
            None => false,
        }
    }

    /// Reads inside the store window. Offsets are relative to the window.
    #[cfg(feature = "launchpad-x")]
    pub fn store_read(&mut self, offset: u32, bytes: &mut [u8]) -> bool {
        if !self.store_usable() || offset.checked_add(bytes.len() as u32).is_none_or(|end| end > STORE_WINDOW_BYTES) {
            return false;
        }
        match self.flash.as_mut() {
            Some(flash) => flash.read(STORE_WINDOW_OFFSET + offset, bytes).is_ok(),
            None => false,
        }
    }

    /// Erases one 4 KB sector of the store window. The only erase this chip is ever
    /// asked to do for saved data: no chip erase, no block erase.
    #[cfg(feature = "launchpad-x")]
    pub fn store_erase_sector(&mut self, sector: usize) -> bool {
        if !self.store_usable() || sector >= STORE_WINDOW_SECTORS {
            return false;
        }
        let address = STORE_WINDOW_OFFSET + (sector * SECTOR_SIZE) as u32;
        match self.flash.as_mut() {
            Some(flash) => flash.erase_sectors(address, 1).is_ok(),
            None => false,
        }
    }

    /// Programs bytes inside one page of the store window.
    #[cfg(feature = "launchpad-x")]
    pub fn store_program(&mut self, offset: u32, bytes: &[u8]) -> bool {
        let end = match offset.checked_add(bytes.len() as u32) {
            Some(end) if !bytes.is_empty() && end <= STORE_WINDOW_BYTES => end,
            _ => return false,
        };
        // A page program wraps inside its page, so a write must not cross one.
        if offset as usize / PAGE_SIZE != (end as usize - 1) / PAGE_SIZE || !self.store_usable() {
            return false;
        }
        let mut page = [0u8; PAGE_SIZE];
        let buf = &mut page[..bytes.len()];
        buf.copy_from_slice(bytes);
        match self.flash.as_mut() {
            Some(flash) => flash.write_bytes(STORE_WINDOW_OFFSET + offset, buf).is_ok(),
            None => false,
        }
    }

    #[cfg(feature = "launchpad-x")]
    fn store_usable(&mut self) -> bool {
        self.probe();
        self.present && self.jedec_id == STORE_JEDEC_ID
    }

    pub fn info(&mut self) -> ExtFlashInfo {
        self.probe();
        ExtFlashInfo {
            present: self.present,
            jedec_id: self.jedec_id,
            status1: self.status1(),
        }
    }

    fn probe(&mut self) {
        let Some(flash) = self.flash.as_mut() else {
            self.jedec_id = [0; 3];
            self.present = false;
            return;
        };

        let Ok(id) = flash.read_jedec_id() else {
            self.jedec_id = [0; 3];
            self.present = false;
            return;
        };
        let device_id = id.device_id();
        self.jedec_id = [id.mfr_code(), device_id[0], device_id[1]];
        #[cfg(feature = "launchpad-pro-mk3")]
        {
            self.present = self.jedec_id[0] == EXPECTED_JEDEC_MANUFACTURER
                && self.jedec_id[2] == EXPECTED_JEDEC_CAPACITY;
        }
        #[cfg(not(feature = "launchpad-pro-mk3"))]
        {
            self.present = true;
        }
    }

    fn status1(&mut self) -> u8 {
        self.flash
            .as_mut()
            .and_then(|flash| flash.read_status().ok())
            .map(|status| status.bits())
            .unwrap_or(0xff)
    }
}

impl ErrorType for ExtFlash<'_> {
    type Error = NorFlashErrorKind;
}

impl ReadNorFlash for ExtFlash<'_> {
    const READ_SIZE: usize = 1;

    fn read(&mut self, offset: u32, bytes: &mut [u8]) -> Result<(), Self::Error> {
        check_read(self, offset, bytes.len())?;
        let flash = self.flash.as_mut().ok_or(NorFlashErrorKind::Other)?;
        flash
            .read(SETTINGS_OFFSET + offset, bytes)
            .map_err(|_| NorFlashErrorKind::Other)
    }

    fn capacity(&self) -> usize {
        if self.present {
            SETTINGS_SIZE as usize
        } else {
            0
        }
    }
}

impl NorFlash for ExtFlash<'_> {
    const WRITE_SIZE: usize = 1;
    const ERASE_SIZE: usize = SECTOR_SIZE;

    fn write(&mut self, offset: u32, bytes: &[u8]) -> Result<(), Self::Error> {
        check_write(self, offset, bytes.len())?;

        let mut rel_off = offset;
        let mut src = bytes;
        let mut writable = min((SETTINGS_SIZE - offset) as usize, src.len());
        let (flash, sector_buf) = (&mut self.flash, &mut self.sector_buf);
        let flash = flash.as_mut().ok_or(NorFlashErrorKind::Other)?;

        while writable != 0 {
            let abs_off = SETTINGS_OFFSET + rel_off;
            let sector_base = abs_off & !((SECTOR_SIZE as u32) - 1);
            let in_sector = (abs_off - sector_base) as usize;
            let chunk = min(writable, SECTOR_SIZE - in_sector);

            flash
                .read(sector_base, sector_buf)
                .map_err(|_| NorFlashErrorKind::Other)?;

            if sector_buf[in_sector..in_sector + chunk] != src[..chunk] {
                sector_buf[in_sector..in_sector + chunk].copy_from_slice(&src[..chunk]);
                flash
                    .erase_sectors(sector_base, 1)
                    .map_err(|_| NorFlashErrorKind::Other)?;

                for page_off in (0..SECTOR_SIZE).step_by(PAGE_SIZE) {
                    if !sector_buf[page_off..page_off + PAGE_SIZE]
                        .iter()
                        .all(|byte| *byte == 0xff)
                    {
                        flash
                            .write_bytes(
                                sector_base + page_off as u32,
                                &mut sector_buf[page_off..page_off + PAGE_SIZE],
                            )
                            .map_err(|_| NorFlashErrorKind::Other)?;
                    }
                }
            }

            rel_off += chunk as u32;
            src = &src[chunk..];
            writable -= chunk;
        }

        Ok(())
    }

    fn erase(&mut self, from: u32, to: u32) -> Result<(), Self::Error> {
        check_erase(self, from, to)?;
        let flash = self.flash.as_mut().ok_or(NorFlashErrorKind::Other)?;

        for sector_addr in (from..to).step_by(SECTOR_SIZE) {
            flash
                .erase_sectors(SETTINGS_OFFSET + sector_addr, 1)
                .map_err(|_| NorFlashErrorKind::Other)?;
        }

        Ok(())
    }
}
