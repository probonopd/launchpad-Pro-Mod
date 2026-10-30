// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 launchpad-Pro-Mod contributors

use super::*;
use std::vec::Vec;

/// A NOR flash model: erase sets a sector to 0xFF, program can only clear bits and
/// is an error on a byte that is not erased where a 1 would be needed. A power loss
/// after `fail_after` steps leaves garbage where the operation was running.
struct Sim {
    data: Vec<u8>,
    steps: usize,
    fail_after: Option<usize>,
    /// Every erase and program that was attempted: (kind, offset, length).
    log: Vec<(char, u32, usize)>,
    dead: bool,
}

impl Sim {
    fn new() -> Self {
        Self { data: vec![0xff; WINDOW_SECTORS * SECTOR], steps: 0, fail_after: None, log: Vec::new(), dead: false }
    }

    fn step(&mut self) -> bool {
        if self.dead {
            return false;
        }
        if self.fail_after == Some(self.steps) {
            self.dead = true;
            return false;
        }
        self.steps += 1;
        true
    }

    fn reboot(&mut self) {
        self.dead = false;
        self.fail_after = None;
        self.steps = 0;
    }
}

impl Window for Sim {
    fn read(&mut self, offset: u32, buf: &mut [u8]) -> Result<(), Error> {
        let o = offset as usize;
        assert!(o + buf.len() <= self.data.len(), "read outside the window");
        buf.copy_from_slice(&self.data[o..o + buf.len()]);
        Ok(())
    }

    fn erase_sector(&mut self, sector: usize) -> Result<(), Error> {
        assert!(sector < WINDOW_SECTORS, "erase outside the window");
        self.log.push(('e', sector as u32, SECTOR));
        let start = sector * SECTOR;
        if !self.step() {
            // Interrupted erase: undefined contents.
            for (i, b) in self.data[start..start + SECTOR].iter_mut().enumerate() {
                *b = (i as u8).wrapping_mul(37) ^ 0xa5;
            }
            return Err(Error::Io);
        }
        self.data[start..start + SECTOR].fill(0xff);
        Ok(())
    }

    fn program(&mut self, offset: u32, data: &[u8]) -> Result<(), Error> {
        let o = offset as usize;
        assert!(o + data.len() <= self.data.len(), "program outside the window");
        assert_eq!(o / PAGE, (o + data.len() - 1) / PAGE, "program crosses a page");
        self.log.push(('p', offset, data.len()));
        for (i, &b) in data.iter().enumerate() {
            if !self.step() {
                // Torn byte: some of its bits cleared.
                self.data[o + i] &= b | 0x5a;
                return Err(Error::Io);
            }
            assert_eq!(self.data[o + i] & b, b, "programmed a byte that is not erased (a 0 bit would need a 1)");
            self.data[o + i] &= b;
        }
        Ok(())
    }
}

fn payload(n: usize, seed: u8) -> Vec<u8> {
    (0..n).map(|i| (i as u8).wrapping_mul(7).wrapping_add(seed)).collect()
}

#[test]
fn window_has_the_expected_size() {
    assert_eq!(WINDOW_SECTORS, 38);
    assert_eq!(max_len(Item::Project(0)), 2 * SECTOR - PAGE);
    assert!(max_len(Item::Project(7)) >= 7147);
    assert_eq!(max_len(Item::Project(8)), 0);
}

#[test]
fn empty_window_has_nothing() {
    let mut s = Store::new(Sim::new());
    let mut buf = [0u8; 64];
    for slot in 0..PROJECT_SLOTS {
        assert_eq!(s.exists(Item::Project(slot)), Ok(false));
        assert_eq!(s.load(Item::Project(slot), &mut buf), Ok(None));
    }
    assert_eq!(s.load(Item::NotePrefs, &mut buf), Ok(None));
    assert_eq!(s.load(Item::CustomPrefs, &mut buf), Ok(None));
}

#[test]
fn save_and_load_round_trip_for_every_item() {
    let mut s = Store::new(Sim::new());
    let items = [Item::Project(0), Item::Project(3), Item::Project(7), Item::NotePrefs, Item::CustomPrefs];
    for (k, item) in items.iter().enumerate() {
        let data = payload(if matches!(item, Item::Project(_)) { 7147 } else { 100 + k }, k as u8);
        s.save(*item, &data).unwrap();
        let mut out = vec![0u8; 8000];
        assert_eq!(s.load(*item, &mut out), Ok(Some(data.len())));
        assert_eq!(&out[..data.len()], &data[..]);
    }
    // Items do not disturb each other.
    let mut out = vec![0u8; 8000];
    assert_eq!(s.load(Item::NotePrefs, &mut out), Ok(Some(101 + 2)));
}

#[test]
fn saves_alternate_banks_and_keep_the_previous_copy_until_the_new_one_is_complete() {
    let mut s = Store::new(Sim::new());
    s.save(Item::NotePrefs, &payload(50, 1)).unwrap();
    s.save(Item::NotePrefs, &payload(60, 2)).unwrap();
    s.save(Item::NotePrefs, &payload(70, 3)).unwrap();
    let sim = s.into_inner();
    // The erases of the three saves hit alternating sectors of the item's two banks.
    let erased: Vec<u32> = sim.log.iter().filter(|e| e.0 == 'e').map(|e| e.1).collect();
    assert_eq!(erased, vec![34, 35, 34]);
}

#[test]
fn rejects_unknown_items_and_too_much_data() {
    let mut s = Store::new(Sim::new());
    assert_eq!(s.save(Item::Project(8), &[1]), Err(Error::Range));
    assert_eq!(s.save(Item::NotePrefs, &payload(SECTOR, 0)), Err(Error::Range));
    assert_eq!(s.save(Item::Project(0), &payload(8000, 0)), Err(Error::Range));
    let mut small = [0u8; 4];
    s.save(Item::NotePrefs, &payload(10, 0)).unwrap();
    assert_eq!(s.load(Item::NotePrefs, &mut small), Err(Error::BufferTooSmall));
    // Nothing was erased for the rejected requests.
    let sim = s.into_inner();
    assert_eq!(sim.log.iter().filter(|e| e.0 == 'e').count(), 1);
}

#[test]
fn only_sector_erases_inside_the_item_and_pages_are_programmed_once() {
    let mut s = Store::new(Sim::new());
    s.save(Item::Project(2), &payload(7147, 9)).unwrap();
    let sim = s.into_inner();
    // The setup preferences take sectors 0 and 1, so project 2 owns sectors 10..14. A
    // first save writes bank A = sectors 10 and 11.
    let erased: Vec<u32> = sim.log.iter().filter(|e| e.0 == 'e').map(|e| e.1).collect();
    assert_eq!(erased, vec![10, 11]);
    let lo = 10 * SECTOR as u32;
    let hi = 12 * SECTOR as u32;
    let mut pages = std::collections::BTreeSet::new();
    for &(kind, off, len) in &sim.log {
        if kind == 'p' {
            assert!(off >= lo && off + len as u32 <= hi, "program outside the bank");
            assert!(pages.insert(off / PAGE as u32), "a page was programmed twice");
        }
    }
}

#[test]
fn corrupt_newest_copy_falls_back_to_the_previous_one() {
    let mut s = Store::new(Sim::new());
    s.save(Item::NotePrefs, &payload(80, 1)).unwrap();
    s.save(Item::NotePrefs, &payload(90, 2)).unwrap();
    let mut sim = s.into_inner();
    // Flip a payload byte of the newest copy (bank B = sector 35).
    sim.data[35 * SECTOR + PAGE + 5] ^= 0xff;
    let mut s = Store::new(sim);
    let mut out = [0u8; 200];
    assert_eq!(s.load(Item::NotePrefs, &mut out), Ok(Some(80)));
    assert_eq!(&out[..80], &payload(80, 1)[..]);
}

#[test]
fn generation_counter_wraps() {
    let mut s = Store::new(Sim::new());
    // The first copy gets the highest generation; the next save wraps to 0 and must still win.
    s.save_inner(Item::NotePrefs, &payload(8, 1), Some(u32::MAX)).unwrap();
    s.save(Item::NotePrefs, &payload(8, 2)).unwrap();
    let mut out = [0u8; 16];
    assert_eq!(s.load(Item::NotePrefs, &mut out), Ok(Some(8)));
    assert_eq!(&out[..8], &payload(8, 2)[..]);
    s.save(Item::NotePrefs, &payload(8, 3)).unwrap();
    assert_eq!(s.load(Item::NotePrefs, &mut out), Ok(Some(8)));
    assert_eq!(&out[..8], &payload(8, 3)[..]);
}

/// The most important test: cut the power at every single step of a save and check
/// that after the reboot either the old or the new data is there, intact, and that a
/// further save works.
fn power_cut_everywhere(item: Item, len: usize) {
    // Count the steps of a save onto a window that already holds an old copy.
    let old = payload(len, 11);
    let new = payload(len + if len > 100 { 0 } else { 7 }, 22);
    let mut probe = Store::new(Sim::new());
    probe.save(item, &old).unwrap();
    let before = probe.into_inner();
    let mut counter = Store::new(Sim { data: before.data.clone(), ..Sim::new() });
    counter.save(item, &new).unwrap();
    let total = counter.into_inner().steps;
    assert!(total > 10);

    let mut cuts = 0;
    for cut in 0..total {
        let mut sim = Sim { data: before.data.clone(), ..Sim::new() };
        sim.fail_after = Some(cut);
        let mut s = Store::new(sim);
        assert!(s.save(item, &new).is_err(), "cut {cut} must fail the save");
        let mut sim = s.into_inner();
        sim.reboot();
        let mut s = Store::new(sim);
        let mut out = vec![0u8; 8000];
        let got = s.load(item, &mut out).unwrap();
        match got {
            Some(n) if n == old.len() && out[..n] == old[..] => {}
            Some(n) if n == new.len() && out[..n] == new[..] => {}
            other => panic!("cut {cut}: neither the old nor the new copy: {other:?}"),
        }
        // The store is still usable.
        s.save(item, &payload(len, 33)).unwrap();
        assert_eq!(s.load(item, &mut out).unwrap(), Some(len));
        assert_eq!(out[..len], payload(len, 33)[..]);
        cuts += 1;
    }
    assert_eq!(cuts, total);
}

#[test]
fn power_cut_at_every_step_of_saving_prefs() {
    power_cut_everywhere(Item::NotePrefs, 112);
}

#[test]
fn power_cut_at_every_step_of_saving_a_project() {
    power_cut_everywhere(Item::Project(5), 7147);
}

#[test]
fn power_cut_during_the_very_first_save_leaves_nothing_or_the_data() {
    let data = payload(300, 4);
    let mut counter = Store::new(Sim::new());
    counter.save(Item::CustomPrefs, &data).unwrap();
    let total = counter.into_inner().steps;
    for cut in 0..total {
        let mut sim = Sim::new();
        sim.fail_after = Some(cut);
        let mut s = Store::new(sim);
        assert!(s.save(Item::CustomPrefs, &data).is_err());
        let mut sim = s.into_inner();
        sim.reboot();
        let mut s = Store::new(sim);
        let mut out = vec![0u8; 400];
        match s.load(Item::CustomPrefs, &mut out).unwrap() {
            None => {}
            Some(n) => assert_eq!(&out[..n], &data[..], "cut {cut}"),
        }
    }
}

#[test]
fn verify_failure_is_reported_and_keeps_the_old_copy() {
    struct Lying(Sim, bool);
    impl Window for Lying {
        fn read(&mut self, o: u32, b: &mut [u8]) -> Result<(), Error> {
            self.0.read(o, b)
        }
        fn erase_sector(&mut self, s: usize) -> Result<(), Error> {
            self.0.erase_sector(s)
        }
        fn program(&mut self, o: u32, d: &[u8]) -> Result<(), Error> {
            if self.1 && o as usize >= 35 * SECTOR + PAGE {
                // The chip "accepts" the payload but does not keep it.
                return Ok(());
            }
            self.0.program(o, d)
        }
    }
    let mut s = Store::new(Lying(Sim::new(), false));
    s.save(Item::NotePrefs, &payload(40, 1)).unwrap();
    let mut w = s.into_inner();
    w.1 = true;
    let mut s = Store::new(w);
    assert_eq!(s.save(Item::NotePrefs, &payload(40, 2)), Err(Error::Verify));
    let mut out = [0u8; 64];
    assert_eq!(s.load(Item::NotePrefs, &mut out), Ok(Some(40)));
    assert_eq!(&out[..40], &payload(40, 1)[..]);
}

#[test]
fn items_keep_their_place_in_the_chip_when_the_window_grew() {
    // Absolute offsets from the end of the window, as stored on the device before the
    // setup preferences were added (window of 36 sectors ending at 0x0FE000).
    let end = WINDOW_SECTORS * SECTOR;
    let old_start = end - 36 * SECTOR;
    for (item, old_first_sector) in [(Item::Project(0), 0usize), (Item::Project(7), 28), (Item::NotePrefs, 32), (Item::CustomPrefs, 34)] {
        let (_, bs) = banks(item).unwrap();
        assert_eq!(bs[0].first_sector * SECTOR, old_start + old_first_sector * SECTOR, "{item:?}");
    }
    let (_, setup) = banks(Item::SetupPrefs).unwrap();
    assert_eq!(setup[0].first_sector, 0);
}

#[test]
fn power_cut_at_every_step_of_saving_setup_prefs() {
    power_cut_everywhere(Item::SetupPrefs, 8);
}
