// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Simon Peter
use std::cell::RefCell;
mod driver {
    use super::SENT;
    pub fn send_midi(_p: crate::sys::midi::MidiPort, d: &[u8]) { SENT.with(|s| s.borrow_mut().push(d.to_vec())); }
    pub fn flash_physical_size() -> u32 { 1024 * 1024 }
    pub fn flash_jedec() -> Option<[u8; 3]> { Some([0xef, 0x40, 0x14]) }
    pub fn read_flash_checked(offset: u32, out: &mut [u8]) -> bool {
        if offset == 0x12345 { return false; }
        for (i, b) in out.iter_mut().enumerate() { *b = ((offset as usize + i) * 7 + 3) as u8 ^ 0x80; }
        true
    }
}
mod sys { pub mod midi { #[derive(Clone, Copy)] pub enum MidiPort { Midi } } }
mod diag;
thread_local! { static SENT: RefCell<Vec<Vec<u8>>> = RefCell::new(vec![]); }
fn run(req: &[u8]) -> Vec<u8> {
    SENT.with(|s| s.borrow_mut().clear());
    assert!(diag::execute(req));
    SENT.with(|s| s.borrow()[0].clone())
}
fn hex(v: &[u8]) -> String { v.iter().map(|b| format!("{b:02x}")).collect() }
fn main() {
    let p = [0xf0u8, 0x7d, b'L', b'P', b'X', b'D'];
    let mk = |region: u8, off: u32, len: u8| { let mut v = p.to_vec(); v.extend([0x02, region, ((off>>21)&0x7f) as u8, ((off>>14)&0x7f) as u8, ((off>>7)&0x7f) as u8, (off&0x7f) as u8, len, 0xf7]); v };
    println!("read {}", hex(&run(&mk(1, 0x1000, 56))));
    println!("read_odd {}", hex(&run(&mk(1, 0x0fffc0, 13))));
    println!("fail {}", hex(&run(&mk(1, 0x12345, 8))));
    println!("range {}", hex(&run(&mk(1, 0x0fffff, 8))));
    println!("len0 {}", hex(&run(&mk(1, 0, 0))));
    println!("len57 {}", hex(&run(&mk(1, 0, 57))));
    println!("region9 {}", hex(&run(&mk(9, 0, 8))));
    println!("garbage {}", hex(&run(&[p.as_slice(), &[0x55, 0xf7]].concat())));
    assert!(!diag::execute(&[0xf0, 0x00, 0x20, 0x29, 0xf7]));
    let maxlen = SENT.with(|_| 0);
    let _ = maxlen;
}
