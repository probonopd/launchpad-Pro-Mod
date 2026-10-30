// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 launchpad-Pro-Mod contributors

//! Compact project encoding for the store. Only patterns and steps that hold something
//! are written, so a typical project is a few KB while a completely full one (13 KB)
//! does not fit a slot: `encode` then returns None and nothing is saved.

use crate::model::*;

/// Largest encoded project the hosts must have room for.
pub const MAX_PROJECT_BYTES: usize = 7936;

const MAGIC: [u8; 4] = *b"NSQ\x01";

/// Project storage provided by the host. Slots are `0..PROJECT_SLOTS`.
pub trait Storage {
    /// Lets `fill` write an encoded project (at most `MAX_PROJECT_BYTES`) into a
    /// temporary buffer and stores what it reports. False if nothing was stored.
    fn save_with(&mut self, slot: u8, fill: impl FnOnce(&mut [u8]) -> Option<usize>) -> bool;
    /// Gives the stored bytes to `apply`; false if there are none or `apply` refuses.
    fn load_with(&mut self, slot: u8, apply: impl FnOnce(&[u8]) -> bool) -> bool;
}

pub const PROJECT_SLOTS: u8 = 8;

struct Writer<'a> {
    buf: &'a mut [u8],
    len: usize,
}

impl Writer<'_> {
    fn put(&mut self, bytes: &[u8]) -> Option<()> {
        let end = self.len.checked_add(bytes.len())?;
        self.buf.get_mut(self.len..end)?.copy_from_slice(bytes);
        self.len = end;
        Some(())
    }
}

fn is_default(pattern: &Pattern) -> bool {
    pattern.is_empty()
        && pattern.length as usize == STEPS
        && pattern.direction == 0
        && pattern.rate == DEFAULT_RATE
        && pattern.swing == 0
}

/// Writes the project into `buf`; None if it does not fit.
pub fn encode(project: &Project, buf: &mut [u8]) -> Option<usize> {
    let mut w = Writer { buf, len: 0 };
    w.put(&MAGIC)?;
    w.put(&project.bpm.to_le_bytes())?;
    w.put(&[project.external_clock as u8 | (project.send_clock as u8) << 1, project.scene])?;
    for c in &project.chain {
        w.put(&[c.first, c.last])?;
    }
    for scene in &project.scenes {
        for c in scene {
            w.put(&[c.first, c.last])?;
        }
    }
    for track in &project.tracks {
        let mut mask = 0u8;
        for (p, pattern) in track.patterns.iter().enumerate() {
            if !is_default(pattern) {
                mask |= 1 << p;
            }
        }
        w.put(&[track.kind, track.scale, track.root, track.channel])?;
        w.put(&[track.muted as u8 | (track.solo as u8) << 1, track.current, mask])?;
        for (p, pattern) in track.patterns.iter().enumerate() {
            if mask & (1 << p) == 0 {
                continue;
            }
            let mut steps = 0u32;
            for (s, step) in pattern.steps.iter().enumerate() {
                if !step.is_empty() {
                    steps |= 1 << s;
                }
            }
            w.put(&[pattern.length, pattern.direction, pattern.rate, pattern.swing])?;
            w.put(&steps.to_le_bytes())?;
            for step in pattern.steps.iter().filter(|s| !s.is_empty()) {
                let count = step.notes.iter().take_while(|&&n| n != NO_NOTE).count() as u8;
                w.put(&[count])?;
                w.put(&step.notes[..count as usize])?;
                w.put(&[step.velocity, step.gate, step.skip | step.delay << 3 | step.mutate << 6])?;
            }
        }
    }
    Some(w.len)
}

struct Reader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl Reader<'_> {
    fn take(&mut self, n: usize) -> Option<&[u8]> {
        let bytes = self.buf.get(self.pos..self.pos.checked_add(n)?)?;
        self.pos += n;
        Some(bytes)
    }

    fn byte(&mut self) -> Option<u8> {
        Some(self.take(1)?[0])
    }
}

fn selection(r: &mut Reader) -> Option<Selection> {
    let (first, last) = (r.byte()?, r.byte()?);
    if first as usize >= PATTERNS || last as usize >= PATTERNS || first > last {
        return None;
    }
    Some(Selection { first, last })
}

/// Reads a project. With `target` none it only checks the data; the project is only
/// changed when the whole input was found valid, so a bad slot cannot half-load.
fn parse(buf: &[u8], mut target: Option<&mut Project>) -> Option<()> {
    let mut r = Reader { buf, pos: 0 };
    if r.take(4)? != MAGIC {
        return None;
    }
    let bpm = u16::from_le_bytes([r.byte()?, r.byte()?]);
    if !(MIN_BPM..=MAX_BPM).contains(&bpm) {
        return None;
    }
    let flags = r.byte()?;
    let scene = r.byte()?;
    if scene != NO_SCENE && scene as usize >= SCENES {
        return None;
    }
    if let Some(p) = target.as_deref_mut() {
        p.reset();
        p.bpm = bpm;
        p.external_clock = flags & 1 != 0;
        p.send_clock = flags & 2 != 0;
        p.scene = scene;
    }
    for t in 0..TRACKS {
        let c = selection(&mut r)?;
        if let Some(p) = target.as_deref_mut() {
            p.chain[t] = c;
        }
    }
    for s in 0..SCENES {
        for t in 0..TRACKS {
            let c = selection(&mut r)?;
            if let Some(p) = target.as_deref_mut() {
                p.scenes[s][t] = c;
            }
        }
    }
    for t in 0..TRACKS {
        let [kind, scale, root, channel, tflags, current, mask] = [
            r.byte()?, r.byte()?, r.byte()?, r.byte()?, r.byte()?, r.byte()?, r.byte()?,
        ];
        if TrackKind::from_index(kind).is_none()
            || scale as usize >= SCALE_COUNT
            || !(1..=127).contains(&root)
            || channel >= 16
            || current as usize >= PATTERNS
        {
            return None;
        }
        if let Some(p) = target.as_deref_mut() {
            let track = &mut p.tracks[t];
            (track.kind, track.scale, track.root, track.channel) = (kind, scale, root, channel);
            (track.muted, track.solo, track.current) = (tflags & 1 != 0, tflags & 2 != 0, current);
        }
        for pi in 0..PATTERNS {
            if mask & (1 << pi) == 0 {
                continue;
            }
            let [length, direction, rate, swing] = [r.byte()?, r.byte()?, r.byte()?, r.byte()?];
            if !(1..=STEPS as u8).contains(&length) || direction > 3 || rate > 7 || swing > 3 {
                return None;
            }
            let steps = u32::from_le_bytes(r.take(4)?.try_into().ok()?);
            if let Some(p) = target.as_deref_mut() {
                let pattern = &mut p.tracks[t].patterns[pi];
                (pattern.length, pattern.direction, pattern.rate, pattern.swing) = (length, direction, rate, swing);
            }
            for s in 0..STEPS {
                if steps & (1 << s) == 0 {
                    continue;
                }
                let count = r.byte()? as usize;
                if !(1..=MAX_NOTES).contains(&count) {
                    return None;
                }
                let notes = r.take(count)?;
                if notes.iter().any(|n| !(1..=127).contains(n)) {
                    return None;
                }
                let mut step = Step::ZERO;
                step.notes[..count].copy_from_slice(notes);
                let [velocity, gate, packed] = [r.byte()?, r.byte()?, r.byte()?];
                step.velocity = velocity;
                step.gate = gate;
                step.skip = packed & 7;
                step.delay = packed >> 3 & 7;
                step.mutate = packed >> 6;
                if !(1..=127).contains(&velocity) || gate > MAX_GATE || step.delay > MAX_DELAY {
                    return None;
                }
                if let Some(p) = target.as_deref_mut() {
                    p.tracks[t].patterns[pi].steps[s] = step;
                }
            }
        }
    }
    (r.pos == buf.len()).then_some(())
}

/// Replaces the project with the encoded one. False, and the project untouched, if the
/// data is not a valid project.
pub fn decode(project: &mut Project, buf: &[u8]) -> bool {
    parse(buf, None).is_some() && parse(buf, Some(project)).is_some()
}
