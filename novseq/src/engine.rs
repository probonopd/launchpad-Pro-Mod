// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 launchpad-Pro-Mod contributors

use crate::model::*;

pub trait MidiOut {
    fn send(&mut self, msg: &[u8]);
}

const PPQN: u32 = 24;
const MAX_CATCH_UP_PULSES: u32 = 8;

#[derive(Copy, Clone)]
struct Voice {
    notes: [u8; MAX_NOTES],
    off_at: u32,
}

impl Voice {
    const SILENT: Voice = Voice { notes: [NO_NOTE; MAX_NOTES], off_at: 0 };
}

/// Where one track is in its pattern.
#[derive(Copy, Clone)]
struct Lane {
    /// Pulse of the next step time.
    next_boundary: u32,
    /// Position in the pattern; none before the first step of a run.
    position: Option<u8>,
    /// A new pattern begins with its first step at the next step time.
    restart: bool,
    /// Steps played in this pass, to find the end of the pattern in any direction.
    played: u8,
    /// Ping-pong direction, true while going backwards.
    reverse: bool,
    /// A step waits for its delay (swing and micro step): (step, pulse to play it).
    pending: Option<(u8, u32)>,
    /// Pattern or chain that takes over at the end of the current pass.
    queued: Option<Selection>,
    voice: Voice,
}

impl Lane {
    const NEW: Lane = Lane {
        next_boundary: 0,
        position: None,
        restart: false,
        played: 0,
        reverse: false,
        pending: None,
        queued: None,
        voice: Voice::SILENT,
    };
}

pub struct Engine {
    running: bool,
    pulse: u32,
    next_us: u64,
    lanes: [Lane; TRACKS],
    /// Zero until first use; kept zero so the engine lives in zeroed RAM.
    rng: u32,
}

impl Engine {
    pub const fn new() -> Self {
        Self { running: false, pulse: 0, next_us: 0, lanes: [Lane::NEW; TRACKS], rng: 0 }
    }

    pub fn running(&self) -> bool {
        self.running
    }

    /// Pulse counter, for measuring how long a pad was held.
    pub fn pulse(&self) -> u32 {
        self.pulse
    }

    pub fn seed(&mut self, value: u32) {
        self.rng = value | 1;
    }

    fn random(&mut self) -> u32 {
        if self.rng == 0 {
            self.rng = 0x2545_f491;
        }
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.rng = x;
        x
    }

    /// Step of the track that was played last, if running.
    pub fn playhead(&self, track: usize) -> Option<u8> {
        if self.running { self.lanes[track].position } else { None }
    }

    pub fn queued(&self, track: usize) -> Option<Selection> {
        self.lanes[track].queued
    }

    /// Where a note played now belongs: the nearest step (or the step it falls in when
    /// not quantised) and the micro step delay for the latter.
    pub fn record_position(&self, project: &Project, track: usize, quantize: bool) -> Option<(u8, u8)> {
        let lane = &self.lanes[track];
        let position = lane.position?;
        if !self.running {
            return None;
        }
        let pattern = project.tracks[track].pattern();
        let period = pattern.period();
        // `pulse` is the next pulse to play, so the one just played is pulse - 1;
        // the lane's step began one period before its next boundary.
        let since = (self.pulse - 1).wrapping_sub(lane.next_boundary.wrapping_sub(period));
        if quantize {
            let step = if since * 2 >= period { (position + 1) % pattern.length.max(1) } else { position };
            Some((step, 0))
        } else {
            Some((position, since.min(MAX_DELAY as u32).min(period - 1) as u8))
        }
    }

    /// Starts from the first step, or from where it stopped if `resume`.
    pub fn start(&mut self, project: &Project, resume: bool, now_us: u64, out: &mut impl MidiOut) {
        if !resume {
            self.pulse = 0;
            for t in 0..TRACKS {
                let queued = self.lanes[t].queued;
                self.lanes[t] = Lane::NEW;
                self.lanes[t].queued = queued;
            }
        }
        for lane in self.lanes.iter_mut() {
            lane.next_boundary = self.pulse;
            lane.pending = None;
        }
        self.running = true;
        self.next_us = now_us;
        if project.send_clock && !project.external_clock {
            out.send(&[if resume { 0xfb } else { 0xfa }]);
        }
    }

    pub fn stop(&mut self, project: &Project, out: &mut impl MidiOut) {
        self.running = false;
        for lane in self.lanes.iter_mut() {
            lane.pending = None;
        }
        self.all_off(project, out);
        if project.send_clock && !project.external_clock {
            out.send(&[0xfc]);
        }
    }

    /// Makes `selection` the pattern or chain of the track: at the end of the current
    /// pass when running, at once otherwise.
    pub fn select(&mut self, project: &mut Project, track: usize, selection: Selection) {
        if self.running {
            self.lanes[track].queued = Some(selection);
        } else {
            self.lanes[track].queued = None;
            self.lanes[track].position = None;
            self.lanes[track].played = 0;
            self.lanes[track].reverse = false;
            project.chain[track] = selection;
            project.tracks[track].current = selection.first;
        }
    }

    fn period_us(bpm: u16) -> u64 {
        60_000_000 / (bpm.max(1) as u64 * PPQN as u64)
    }

    pub fn tick(&mut self, now_us: u64, project: &mut Project, out: &mut impl MidiOut) {
        if !self.running || project.external_clock {
            return;
        }
        let period = Self::period_us(project.bpm);
        let mut fired = 0;
        while now_us >= self.next_us {
            if fired >= MAX_CATCH_UP_PULSES {
                self.next_us = now_us;
                break;
            }
            if project.send_clock {
                out.send(&[0xf8]);
            }
            self.play_pulse(project, out);
            self.next_us += period;
            fired += 1;
        }
    }

    /// Clock and transport from outside; used only with the external clock on.
    /// Returns true if the message was consumed.
    pub fn midi_in(&mut self, msg: [u8; 3], project: &mut Project, out: &mut impl MidiOut) -> bool {
        if !project.external_clock {
            return false;
        }
        match msg[0] {
            0xf8 => {
                if self.running {
                    self.play_pulse(project, out);
                }
                true
            }
            0xfa => {
                self.start(project, false, 0, out);
                true
            }
            0xfb => {
                self.start(project, true, 0, out);
                true
            }
            0xfc => {
                self.stop(project, out);
                true
            }
            _ => false,
        }
    }

    pub fn all_off(&mut self, project: &Project, out: &mut impl MidiOut) {
        for t in 0..TRACKS {
            self.release_voice(t, project, out);
        }
    }

    /// Forgets where every track was, e.g. after the project was replaced.
    pub fn reload(&mut self, project: &Project, out: &mut impl MidiOut) {
        if self.running {
            self.stop(project, out);
        }
        self.all_off(project, out);
        self.lanes = [Lane::NEW; TRACKS];
    }

    /// Silences one track, e.g. when it is muted.
    pub fn all_off_track(&mut self, t: usize, project: &Project, out: &mut impl MidiOut) {
        self.release_voice(t, project, out);
    }

    fn release_voice(&mut self, t: usize, project: &Project, out: &mut impl MidiOut) {
        let channel = project.tracks[t].channel & 0x0f;
        for &n in self.lanes[t].voice.notes.iter().filter(|&&n| n != NO_NOTE) {
            out.send(&[0x80 | channel, n, 0]);
        }
        self.lanes[t].voice = Voice::SILENT;
    }

    fn audible(project: &Project, t: usize) -> bool {
        let track = &project.tracks[t];
        if project.tracks.iter().any(|x| x.solo) { track.solo } else { !track.muted }
    }

    fn play_pulse(&mut self, project: &mut Project, out: &mut impl MidiOut) {
        for t in 0..TRACKS {
            if self.lanes[t].voice.notes[0] != NO_NOTE && self.pulse >= self.lanes[t].voice.off_at {
                self.release_voice(t, project, out);
            }
            if self.lanes[t].next_boundary == self.pulse {
                self.boundary(t, project);
            }
            if let Some((step, at)) = self.lanes[t].pending {
                if self.pulse >= at {
                    self.lanes[t].pending = None;
                    self.fire(t, step, project, out);
                }
            }
        }
        self.pulse = self.pulse.wrapping_add(1);
    }

    /// A step time of the track: pick the step and schedule it.
    fn boundary(&mut self, t: usize, project: &mut Project) {
        self.end_of_pass(t, project);
        let pattern = project.tracks[t].pattern();
        let length = pattern.length.clamp(1, STEPS as u8);
        let period = pattern.period();
        let direction = pattern.direction();
        let rnd = if direction == Direction::Random { self.random() } else { 0 };
        let pulse = self.pulse;
        let lane = &mut self.lanes[t];
        lane.next_boundary = pulse.wrapping_add(period);

        let position = if lane.restart { None } else { lane.position };
        lane.restart = false;
        let step = match position {
            None => {
                lane.reverse = direction == Direction::Backward;
                if direction == Direction::Backward { length - 1 } else { 0 }
            }
            Some(position) => match direction {
                Direction::Forward => (position + 1) % length,
                Direction::Backward => (position + length - 1) % length,
                Direction::PingPong => {
                    if length == 1 {
                        0
                    } else if lane.reverse {
                        if position == 0 {
                            lane.reverse = false;
                            1
                        } else {
                            position - 1
                        }
                    } else if position + 1 >= length {
                        lane.reverse = true;
                        length - 2
                    } else {
                        position + 1
                    }
                }
                Direction::Random => (rnd % length as u32) as u8,
            },
        };
        lane.position = Some(step);
        lane.played += 1;

        // Every second step is delayed by the swing; the micro step delay comes on top.
        let second = lane.played % 2 == 0;
        let data = &pattern.steps[step as usize];
        let swing = if second { pattern.swing as u32 } else { 0 };
        let delay = (swing + data.delay as u32).min(period - 1);
        lane.pending = Some((step, pulse.wrapping_add(delay)));
    }

    /// Plays a step, then moves to the next pattern if the pass is over.
    fn fire(&mut self, t: usize, step: u8, project: &Project, out: &mut impl MidiOut) {
        let pattern = project.tracks[t].pattern();
        let data = pattern.steps[step as usize];
        if !data.is_empty() && Self::audible(project, t) {
            let chance = self.random() % PROBABILITY_LEVELS as u32;
            if chance >= data.skip as u32 {
                self.release_voice(t, project, out);
                let channel = project.tracks[t].channel & 0x0f;
                let mut notes = data.notes;
                if data.mutate > 0 && self.random() % MUTATION_LEVELS as u32 <= data.mutate as u32 - 1 {
                    let shift = self.random() % 5; // -2..=+2 pad steps
                    notes = self.mutated(&project.tracks[t], notes, shift as i8 - 2);
                }
                for &n in notes.iter().filter(|&&n| n != NO_NOTE) {
                    out.send(&[0x90 | channel, n, data.velocity.max(1)]);
                }
                self.lanes[t].voice =
                    Voice { notes, off_at: self.pulse.wrapping_add(pattern.gate_of(&data)) };
            }
        }
    }

    /// Notes moved by `shift` scale degrees (semitones on chromatic tracks, pads on drums).
    fn mutated(&self, track: &Track, mut notes: [u8; MAX_NOTES], shift: i8) -> [u8; MAX_NOTES] {
        for note in notes.iter_mut().filter(|n| **n != NO_NOTE) {
            *note = shifted(track, *note, shift);
        }
        notes
    }

    /// Steps in one pass over the pattern: its length, or a there-and-back for ping-pong.
    fn pass_length(pattern: &Pattern) -> u8 {
        let length = pattern.length.clamp(1, STEPS as u8);
        if pattern.direction() == Direction::PingPong { (2 * length as u16 - 2).max(1) as u8 } else { length }
    }

    /// At a step time, when the track has played a whole pass: queued patterns take
    /// over, and chains move on to their next pattern. The new pattern begins with
    /// its first step.
    fn end_of_pass(&mut self, t: usize, project: &mut Project) {
        if self.lanes[t].played < Self::pass_length(project.tracks[t].pattern()) {
            return;
        }
        self.lanes[t].played = 0;
        self.lanes[t].restart = true;
        let chain = project.chain[t];
        let next = match self.lanes[t].queued.take() {
            Some(selection) => {
                project.chain[t] = selection;
                selection.first
            }
            None if chain.last > chain.first => {
                let current = project.tracks[t].current;
                if current >= chain.last || current < chain.first { chain.first } else { current + 1 }
            }
            None => chain.first,
        };
        project.tracks[t].current = next;
    }
}

/// `note` moved by `shift` steps of the track's pad layout, clamped to MIDI range.
fn shifted(track: &Track, note: u8, shift: i8) -> u8 {
    let moved = match track.kind() {
        TrackKind::Scale => {
            let scale = (track.scale as usize).min(SCALE_COUNT - 1);
            let len = SCALE_LEN[scale] as i16;
            let rel = note as i16 - track.root as i16;
            let octave = rel.div_euclid(12);
            let within = rel.rem_euclid(12) as u8;
            // Nearest degree at or below the note.
            let mut degree = 0i16;
            for d in 0..len {
                if SCALES[scale][d as usize] <= within {
                    degree = d;
                }
            }
            let total = octave * len + degree + shift as i16;
            let (o, d) = (total.div_euclid(len), total.rem_euclid(len));
            track.root as i16 + o * 12 + SCALES[scale][d as usize] as i16
        }
        _ => note as i16 + shift as i16,
    };
    moved.clamp(1, 127) as u8
}
