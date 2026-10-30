// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 launchpad-Pro-Mod contributors

//! Data model. Everything is valid when all bytes are zero, so the project lives
//! in zero-initialized RAM (no 13 KB copy from flash at startup); the real defaults
//! are set by `Project::reset`, which the sequencer calls before first use.

pub const TRACKS: usize = 4;
pub const STEPS: usize = 32;
pub const PATTERNS: usize = 8;
pub const SCENES: usize = 16;
pub const MAX_NOTES: usize = 8;
/// Note 0 marks a free slot; it is not playable on this device.
pub const NO_NOTE: u8 = 0;
pub const DEFAULT_VELOCITY: u8 = 100;
pub const MIN_BPM: u16 = 40;
pub const MAX_BPM: u16 = 240;
pub const DEFAULT_BPM: u16 = 120;
pub const MAX_GATE: u8 = 191;
pub const MAX_DELAY: u8 = 5;
pub const PROBABILITY_LEVELS: u8 = 8;
pub const MUTATION_LEVELS: u8 = 4;
pub const DRUM_BASE: u8 = 36;
pub const NO_SCENE: u8 = 0xff;

/// Pulses (24 per quarter note) of each sync rate, in the order of the Rate pads:
/// 1/4, 1/4T, 1/8, 1/8T, 1/16, 1/16T, 1/32, 1/32T.
pub const RATE_PULSES: [u8; 8] = [24, 16, 12, 8, 6, 4, 3, 2];
pub const DEFAULT_RATE: u8 = 4;

pub const SCALE_COUNT: usize = 12;
/// Semitone offsets of each scale; the second table gives the number of notes.
pub const SCALES: [[u8; 7]; SCALE_COUNT] = [
    [0, 2, 4, 5, 7, 9, 11], // major
    [0, 2, 3, 5, 7, 8, 10], // natural minor
    [0, 2, 3, 5, 7, 9, 10], // dorian
    [0, 2, 4, 5, 7, 9, 10], // mixolydian
    [0, 2, 4, 6, 7, 9, 11], // lydian
    [0, 1, 3, 5, 7, 8, 10], // phrygian
    [0, 1, 3, 5, 6, 8, 10], // locrian
    [0, 2, 3, 5, 7, 8, 11], // harmonic minor
    [0, 3, 5, 7, 10, 0, 0], // minor pentatonic
    [0, 2, 4, 7, 9, 0, 0],  // major pentatonic
    [0, 3, 5, 6, 7, 10, 0], // blues
    [0, 2, 4, 6, 8, 10, 0], // whole tone
];
pub const SCALE_LEN: [u8; SCALE_COUNT] = [7, 7, 7, 7, 7, 7, 7, 7, 5, 5, 6, 6];

#[derive(Copy, Clone, Eq, PartialEq)]
#[repr(u8)]
pub enum TrackKind {
    /// Pads follow the chosen scale around the root.
    Scale = 0,
    /// 4 x 8 pads, one note each, from note 36.
    Drum = 1,
    /// Every semitone, rows a fourth apart.
    Chromatic = 2,
}

impl TrackKind {
    pub fn from_index(i: u8) -> Option<Self> {
        match i {
            0 => Some(Self::Scale),
            1 => Some(Self::Drum),
            2 => Some(Self::Chromatic),
            _ => None,
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq)]
#[repr(u8)]
pub enum Direction {
    Forward = 0,
    Backward = 1,
    PingPong = 2,
    Random = 3,
}

impl Direction {
    pub fn from_index(i: u8) -> Option<Self> {
        match i {
            0 => Some(Self::Forward),
            1 => Some(Self::Backward),
            2 => Some(Self::PingPong),
            3 => Some(Self::Random),
            _ => None,
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub struct Step {
    pub notes: [u8; MAX_NOTES],
    pub velocity: u8,
    /// Pulses the notes sound; 0 = one step of the pattern's rate minus one pulse.
    pub gate: u8,
    /// Chance that the step is skipped, 0 (never) to 7: probability level is 7 - skip.
    pub skip: u8,
    /// Delay in pulses after the step time (micro step), 0..=MAX_DELAY.
    pub delay: u8,
    /// Chance that the notes are varied on a pass, 0..MUTATION_LEVELS.
    pub mutate: u8,
}

impl Step {
    pub const ZERO: Step =
        Step { notes: [NO_NOTE; MAX_NOTES], velocity: 0, gate: 0, skip: 0, delay: 0, mutate: 0 };
    const FRESH: Step = Step { velocity: DEFAULT_VELOCITY, ..Step::ZERO };

    pub fn is_empty(&self) -> bool {
        self.notes[0] == NO_NOTE
    }

    pub fn has(&self, note: u8) -> bool {
        self.notes.contains(&note)
    }

    /// Probability level 0..8 (chance (level + 1) / 8).
    pub fn probability(&self) -> u8 {
        PROBABILITY_LEVELS - 1 - self.skip
    }

    /// Adds the note, or removes it if present. False if the step is full.
    pub fn toggle(&mut self, note: u8) -> bool {
        if let Some(i) = self.notes.iter().position(|&n| n == note) {
            // Keep the used notes at the front.
            self.notes.copy_within(i + 1.., i);
            self.notes[MAX_NOTES - 1] = NO_NOTE;
            if self.is_empty() {
                self.clear();
            }
            return true;
        }
        if self.is_empty() {
            *self = Step::FRESH;
        }
        match self.notes.iter().position(|&n| n == NO_NOTE) {
            Some(i) => {
                self.notes[i] = note;
                true
            }
            None => false,
        }
    }

    /// Adds the note if it is not in the step yet. False if the step is full.
    pub fn add(&mut self, note: u8) -> bool {
        self.has(note) || self.toggle(note)
    }

    pub fn clear(&mut self) {
        *self = Step::ZERO;
    }
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub struct Pattern {
    pub steps: [Step; STEPS],
    /// 1..=STEPS
    pub length: u8,
    pub direction: u8,
    /// Index into RATE_PULSES.
    pub rate: u8,
    /// Delay of every second step in pulses, 0..=3 (50, 58, 67, 75 %).
    pub swing: u8,
}

impl Pattern {
    pub const ZERO: Pattern =
        Pattern { steps: [Step::ZERO; STEPS], length: 0, direction: 0, rate: 0, swing: 0 };

    /// Empties the pattern and restores its settings.
    pub fn reset(&mut self) {
        *self = Pattern::ZERO;
        self.length = STEPS as u8;
        self.rate = DEFAULT_RATE;
    }

    pub fn is_empty(&self) -> bool {
        self.steps.iter().all(Step::is_empty)
    }

    pub fn direction(&self) -> Direction {
        Direction::from_index(self.direction).unwrap_or(Direction::Forward)
    }

    pub fn period(&self) -> u32 {
        RATE_PULSES[(self.rate as usize).min(7)] as u32
    }

    /// Gate of a step in pulses.
    pub fn gate_of(&self, step: &Step) -> u32 {
        if step.gate == 0 { self.period().saturating_sub(1).max(1) } else { step.gate as u32 }
    }
}

#[derive(Copy, Clone)]
pub struct Track {
    pub kind: u8,
    pub scale: u8,
    pub root: u8,
    pub channel: u8,
    pub muted: bool,
    pub solo: bool,
    /// Pattern being played and edited.
    pub current: u8,
    pub patterns: [Pattern; PATTERNS],
}

impl Track {
    pub const ZERO: Track = Track {
        kind: 0,
        scale: 0,
        root: 0,
        channel: 0,
        muted: false,
        solo: false,
        current: 0,
        patterns: [Pattern::ZERO; PATTERNS],
    };

    pub fn kind(&self) -> TrackKind {
        TrackKind::from_index(self.kind).unwrap_or(TrackKind::Scale)
    }

    pub fn pattern(&self) -> &Pattern {
        &self.patterns[self.current as usize % PATTERNS]
    }

    pub fn pattern_mut(&mut self) -> &mut Pattern {
        &mut self.patterns[self.current as usize % PATTERNS]
    }

    /// The note a plain tap on an empty step puts in it.
    pub fn default_note(&self) -> u8 {
        match self.kind() {
            TrackKind::Drum => DRUM_BASE,
            _ => self.root.max(1),
        }
    }
}

/// Note of a Play Area pad. `row` counts from the bottom (0..6), `col` 0..8.
pub fn play_note(track: &Track, row: u8, col: u8) -> Option<u8> {
    let note = match track.kind() {
        TrackKind::Drum => {
            if row >= 4 {
                return None;
            }
            DRUM_BASE as u16 + row as u16 * 8 + col as u16
        }
        TrackKind::Scale => {
            let scale = (track.scale as usize).min(SCALE_COUNT - 1);
            let len = SCALE_LEN[scale] as u16;
            // Rows overlap by three degrees, so a row up is a third up.
            let degree = row as u16 * 3 + col as u16;
            track.root as u16 + (degree / len) * 12 + SCALES[scale][(degree % len) as usize] as u16
        }
        TrackKind::Chromatic => track.root as u16 + row as u16 * 5 + col as u16,
    };
    if (1..=127).contains(&note) { Some(note as u8) } else { None }
}

/// True if the pad is a root note of the scale (or the first drum pad).
pub fn is_root(track: &Track, row: u8, col: u8) -> bool {
    match (track.kind(), play_note(track, row, col)) {
        (TrackKind::Drum, _) => false,
        (_, Some(n)) => n % 12 == track.root % 12,
        _ => false,
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct Selection {
    pub first: u8,
    pub last: u8,
}

pub struct Project {
    /// False until `reset` has set the defaults.
    pub ready: bool,
    pub tracks: [Track; TRACKS],
    pub bpm: u16,
    /// Patterns each track plays; more than one plays as a chain.
    pub chain: [Selection; TRACKS],
    pub scenes: [[Selection; TRACKS]; SCENES],
    pub scene: u8,
    pub external_clock: bool,
    pub send_clock: bool,
}

impl Project {
    pub const fn new() -> Self {
        Self {
            ready: false,
            tracks: [Track::ZERO; TRACKS],
            bpm: 0,
            chain: [Selection { first: 0, last: 0 }; TRACKS],
            scenes: [[Selection { first: 0, last: 0 }; TRACKS]; SCENES],
            scene: 0,
            external_clock: false,
            send_clock: false,
        }
    }

    /// Empty project with the factory settings: track 1 drums, 2-4 scales.
    pub fn reset(&mut self) {
        self.ready = true;
        self.bpm = DEFAULT_BPM;
        self.external_clock = false;
        self.send_clock = false;
        for (t, track) in self.tracks.iter_mut().enumerate() {
            *track = Track::ZERO;
            track.channel = t as u8;
            if t == 0 {
                track.kind = TrackKind::Drum as u8;
                track.root = DRUM_BASE;
            } else {
                track.root = 48 + 12 * (t as u8 - 1);
            }
            for pattern in track.patterns.iter_mut() {
                pattern.reset();
            }
            self.chain[t] = Selection { first: 0, last: 0 };
        }
        // Scene n plays pattern n % 8 of every track until the player stores others.
        for s in 0..SCENES {
            let p = (s % PATTERNS) as u8;
            self.scenes[s] = [Selection { first: p, last: p }; TRACKS];
        }
        self.scene = NO_SCENE;
    }
}
