// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 launchpad-Pro-Mod contributors

//! Button handling and LED rendering. Like the project, the state here is valid
//! when all bytes are zero (see model.rs), so flags are stored inverted where the
//! default is "on".

use crate::engine::{Engine, MidiOut};
use crate::model::*;
use crate::projectio::{self, Storage, PROJECT_SLOTS};

pub const LED_COUNT: usize = 100;

// Button numbers of the Launchpad X programmer layout.
pub const UP: u8 = 91;
pub const DOWN: u8 = 92;
pub const LEFT: u8 = 93;
pub const RIGHT: u8 = 94;
pub const PLAY: u8 = 96; // Note button
pub const SHIFT: u8 = 98; // Capture MIDI button
// Right column, bottom to top: Record Arm, Solo, Mute, Stop Clip, Send B, Send A, Pan, Volume.
pub const RECORD: u8 = 19;
pub const SOLO: u8 = 29;
pub const MUTE: u8 = 39;
pub const CLEAR: u8 = 49;
pub const DUPLICATE: u8 = 59;
pub const QUANTIZE: u8 = 69;
pub const PROJECTS: u8 = 89; // Volume

const STEPS_PER_PAGE: u8 = 16;
const PAGES: u8 = (STEPS as u8) / STEPS_PER_PAGE;
const TRACK_COLORS: [u32; TRACKS] = [0xff2000, 0x00c040, 0x2080ff, 0xe0c000];

#[derive(Copy, Clone, Eq, PartialEq)]
#[repr(u8)]
pub enum View {
    Steps = 0,
    Patterns,
    Scenes,
    Velocity,
    Probability,
    Micro,
    Mutation,
    Settings,
    /// Load and save; opened by the top button of the right column.
    Projects,
}

/// Shift + right column picks the view, from the top.
const VIEW_BUTTONS: [(u8, View); 8] = [
    (89, View::Steps),
    (79, View::Patterns),
    (69, View::Scenes),
    (59, View::Velocity),
    (49, View::Probability),
    (39, View::Micro),
    (29, View::Mutation),
    (19, View::Settings),
];

const SETTINGS_PAGES: u8 = 3;

pub struct Ui {
    /// Latest clock value, so a start begins at once instead of catching up.
    pub now_us: u64,
    track: u8,
    page: u8,
    view: View,
    settings_page: u8,
    shift: bool,
    clear_held: bool,
    duplicate_held: bool,
    /// What a duplicate copies from, plus one; 0 = nothing chosen yet.
    duplicate_source: u8,
    /// Steps held down, bit n = step n.
    held: u32,
    held_edited: bool,
    /// Pattern pad held down, track and pattern plus one (0 = none).
    held_pattern: u8,
    pattern_edited: bool,
    /// Note sounding for each pad held in the Play Area (0 = none).
    previews: [u8; LED_COUNT],
    record: bool,
    /// Inverted: recording rounds to steps unless this is set.
    unquantized: bool,
    /// Step plus one and start pulse of each pad that is being recorded.
    recording: [u8; LED_COUNT],
    recording_start: [u32; LED_COUNT],
    /// Result of the last load or save (1 done, 2 failed) with the slot, shown until `notice_until`.
    notice: u8,
    notice_pad: u8,
    notice_until: u64,
}

fn scale(color: u32, num: u32, den: u32) -> u32 {
    let ch = |shift: u32| ((((color >> shift) & 0xff) * num / den) & 0xff) << shift;
    ch(16) | ch(8) | ch(0)
}

impl Ui {
    pub const fn new() -> Self {
        Self {
            now_us: 0,
            track: 0,
            page: 0,
            view: View::Steps,
            settings_page: 0,
            shift: false,
            clear_held: false,
            duplicate_held: false,
            duplicate_source: 0,
            held: 0,
            held_edited: false,
            held_pattern: 0,
            pattern_edited: false,
            previews: [0; LED_COUNT],
            record: false,
            unquantized: false,
            recording: [0; LED_COUNT],
            recording_start: [0; LED_COUNT],
            notice: 0,
            notice_pad: 0,
            notice_until: 0,
        }
    }

    pub fn track(&self) -> usize {
        self.track as usize
    }

    pub fn page(&self) -> u8 {
        self.page
    }

    pub fn view(&self) -> View {
        self.view
    }

    pub fn recording_armed(&self) -> bool {
        self.record
    }

    /// Forget held buttons, e.g. when the screen is shown again.
    pub fn reset(&mut self) {
        self.shift = false;
        self.clear_held = false;
        self.duplicate_held = false;
        self.duplicate_source = 0;
        self.held = 0;
        self.held_edited = false;
        self.held_pattern = 0;
        self.pattern_edited = false;
    }

    pub fn stop_previews(&mut self, project: &Project, out: &mut impl MidiOut) {
        self.recording = [0; LED_COUNT];
        for pad in 0..LED_COUNT {
            self.release_preview(pad as u8, project, out);
        }
    }

    fn release_preview(&mut self, pad: u8, project: &Project, out: &mut impl MidiOut) {
        let note = self.previews[pad as usize];
        if note != 0 {
            self.previews[pad as usize] = 0;
            out.send(&[0x80 | project.tracks[self.track as usize].channel, note, 0]);
        }
    }

    /// Step number for a step-row pad (rows 8 and 7), none for other pads.
    fn step_of(&self, index: u8) -> Option<u8> {
        let (row, col) = (index / 10, index % 10);
        if !(1..=8).contains(&col) || !(7..=8).contains(&row) {
            return None;
        }
        Some(self.page * STEPS_PER_PAGE + (8 - row) * 8 + (col - 1))
    }

    pub fn button(
        &mut self,
        project: &mut Project,
        engine: &mut Engine,
        index: u8,
        pressed: bool,
        value: u8,
        out: &mut impl MidiOut,
        storage: &mut impl Storage,
    ) {
        let (row, col) = (index / 10, index % 10);
        if (1..=8).contains(&row) && (1..=8).contains(&col) {
            self.grid(project, engine, index, pressed, value, out, storage);
            return;
        }
        let track = self.track as usize;
        if self.shift && pressed {
            if let Some(&(_, view)) = VIEW_BUTTONS.iter().find(|(b, _)| *b == index) {
                self.select_view(view);
                return;
            }
        }
        match index {
            SHIFT => {
                self.shift = pressed;
                if pressed && self.clear_held {
                    // Hold Clear and tap Shift: empties the whole pattern.
                    project.tracks[track].pattern_mut().reset();
                }
            }
            PROJECTS if pressed => {
                let view = if self.view == View::Projects { View::Steps } else { View::Projects };
                self.select_view(view);
            }
            RECORD if pressed => self.record = !self.record,
            SOLO if pressed => project.tracks[track].solo = !project.tracks[track].solo,
            MUTE if pressed => {
                project.tracks[track].muted = !project.tracks[track].muted;
                engine.all_off_track(track, project, out);
            }
            CLEAR => self.clear_held = pressed,
            DUPLICATE => {
                self.duplicate_held = pressed;
                self.duplicate_source = 0;
            }
            QUANTIZE if pressed => self.unquantized = !self.unquantized,
            PLAY if pressed => {
                if engine.running() {
                    engine.stop(project, out);
                } else {
                    // Shift continues where the sequence stopped instead of restarting.
                    engine.start(project, self.shift, self.now_us, out);
                }
            }
            UP | DOWN if pressed => {
                self.stop_previews(project, out);
                self.held = 0;
                let step = if index == UP { TRACKS as u8 - 1 } else { 1 };
                self.track = (self.track + step) % TRACKS as u8;
            }
            LEFT | RIGHT if pressed => {
                if self.shift {
                    let delta: i16 = if index == RIGHT { 1 } else { -1 };
                    project.bpm = (project.bpm as i16 + delta).clamp(MIN_BPM as i16, MAX_BPM as i16) as u16;
                } else {
                    self.page = if index == RIGHT { (self.page + 1).min(PAGES - 1) } else { self.page.saturating_sub(1) };
                }
            }
            _ => {}
        }
    }

    fn select_view(&mut self, view: View) {
        if view == View::Settings && self.view == View::Settings {
            self.settings_page = (self.settings_page + 1) % SETTINGS_PAGES;
        } else {
            self.settings_page = 0;
        }
        self.view = view;
        self.held = 0;
        self.held_edited = false;
        self.held_pattern = 0;
    }

    fn grid(
        &mut self,
        project: &mut Project,
        engine: &mut Engine,
        index: u8,
        pressed: bool,
        value: u8,
        out: &mut impl MidiOut,
        storage: &mut impl Storage,
    ) {
        let (row, col) = (index / 10, index % 10);
        match self.view {
            View::Steps => {
                if let Some(step) = self.step_of(index) {
                    self.step_pad(project, step, pressed);
                } else if row <= 6 {
                    self.play_pad(project, engine, index, pressed, value, out);
                }
            }
            View::Velocity | View::Probability | View::Micro | View::Mutation => {
                if let Some(step) = self.step_of(index) {
                    self.step_pad(project, step, pressed);
                } else if pressed && row <= 2 {
                    self.value_pad(project, row, col);
                }
            }
            View::Patterns => {
                if (5..=8).contains(&row) {
                    self.pattern_pad(project, engine, 8 - row, col - 1, pressed);
                }
            }
            View::Scenes => {
                // The two step rows are the 16 scene pads, whatever page is shown.
                if (7..=8).contains(&row) && pressed {
                    self.scene_pad(project, engine, (8 - row) * 8 + col - 1);
                }
            }
            View::Settings => {
                if pressed {
                    self.settings_pad(project, index);
                }
            }
            View::Projects => {
                if pressed {
                    self.project_pad(project, engine, row, col - 1, out, storage);
                }
            }
        }
    }

    /// Row 8 loads, row 7 saves, the top right pad of row 5 starts a new project.
    fn project_pad(
        &mut self,
        project: &mut Project,
        engine: &mut Engine,
        row: u8,
        slot: u8,
        out: &mut impl MidiOut,
        storage: &mut impl Storage,
    ) {
        let done = match row {
            8 if slot < PROJECT_SLOTS => storage.load_with(slot, |bytes| {
                // The project is replaced only if the stored one is valid.
                projectio::decode(project, bytes)
            }),
            7 if slot < PROJECT_SLOTS => {
                storage.save_with(slot, |buf| projectio::encode(project, buf))
            }
            5 if slot == 7 => {
                project.reset();
                true
            }
            _ => return,
        };
        if done && row != 7 {
            engine.reload(project, out);
            self.held = 0;
            self.track = 0;
        }
        self.notice = if done { 1 } else { 2 };
        self.notice_pad = row * 10 + slot + 1;
        self.notice_until = self.now_us + 800_000;
    }

    fn step_pad(&mut self, project: &mut Project, step: u8, pressed: bool) {
        let track = self.track as usize;
        let steps_view = self.view == View::Steps;
        let bit = 1u32 << step;
        if pressed {
            if self.clear_held {
                project.tracks[track].pattern_mut().steps[step as usize].clear();
                return;
            }
            if self.duplicate_held && steps_view {
                let pattern = project.tracks[track].pattern_mut();
                match self.duplicate_source {
                    0 => self.duplicate_source = step + 1,
                    from => pattern.steps[step as usize] = pattern.steps[from as usize - 1],
                }
                return;
            }
            if steps_view && self.held.count_ones() == 1 {
                let held = self.held.trailing_zeros() as u8;
                if step > held {
                    // Holding a step and pressing a later one ties the notes up to it.
                    let pattern = project.tracks[track].pattern_mut();
                    let pulses = (step - held) as u32 * pattern.period() - 1;
                    pattern.steps[held as usize].gate = pulses.min(MAX_GATE as u32) as u8;
                    self.held_edited = true;
                    return;
                }
            }
            if self.held != 0 {
                self.held_edited = true;
            }
            self.held |= bit;
        } else if self.held & bit != 0 {
            self.held &= !bit;
            if steps_view && !self.held_edited && self.held == 0 {
                // A tap that did not add notes toggles the whole step.
                let default = project.tracks[track].default_note();
                let data = &mut project.tracks[track].pattern_mut().steps[step as usize];
                if data.is_empty() {
                    data.toggle(default);
                } else {
                    data.clear();
                }
            }
            if self.held == 0 {
                self.held_edited = false;
            }
        }
    }

    fn play_pad(
        &mut self,
        project: &mut Project,
        engine: &Engine,
        index: u8,
        pressed: bool,
        value: u8,
        out: &mut impl MidiOut,
    ) {
        let track = self.track as usize;
        if !pressed {
            self.end_recording(project, engine, track, index);
            self.release_preview(index, project, out);
            return;
        }
        let Some(note) = play_note(&project.tracks[track], index / 10 - 1, index % 10 - 1) else {
            return;
        };
        let velocity = if value == 0 { DEFAULT_VELOCITY } else { value };
        if self.held != 0 {
            // Holding steps and pressing Play Area pads builds their chords.
            let mut held = self.held;
            while held != 0 {
                let step = held.trailing_zeros() as usize;
                held &= held - 1;
                let data = &mut project.tracks[track].pattern_mut().steps[step];
                data.toggle(note);
                if !data.is_empty() {
                    data.velocity = velocity;
                }
            }
            self.held_edited = true;
        } else if self.record {
            // Rounded to the nearest step (or kept as a micro step delay); the time the
            // pad is held becomes the gate.
            if let Some((step, delay)) = engine.record_position(project, track, !self.unquantized) {
                let data = &mut project.tracks[track].pattern_mut().steps[step as usize];
                if data.add(note) {
                    data.velocity = velocity;
                    data.delay = delay;
                    self.recording[index as usize] = step + 1;
                    self.recording_start[index as usize] = engine.pulse();
                }
            }
        }
        self.previews[index as usize] = note;
        out.send(&[0x90 | project.tracks[track].channel, note, velocity]);
    }

    fn end_recording(&mut self, project: &mut Project, engine: &Engine, track: usize, pad: u8) {
        let step = self.recording[pad as usize];
        if step != 0 {
            self.recording[pad as usize] = 0;
            let held = engine.pulse().wrapping_sub(self.recording_start[pad as usize]).clamp(1, MAX_GATE as u32);
            project.tracks[track].pattern_mut().steps[step as usize - 1].gate = held as u8;
        }
    }

    /// A value pad of the Velocity, Probability, Micro and Mutation views: sets the
    /// value on the held steps that have notes.
    fn value_pad(&mut self, project: &mut Project, row: u8, col: u8) {
        let track = self.track as usize;
        let level = (row - 1) * 8 + col - 1;
        let mut held = self.held;
        while held != 0 {
            let step = held.trailing_zeros() as usize;
            held &= held - 1;
            let data = &mut project.tracks[track].pattern_mut().steps[step];
            if data.is_empty() {
                continue;
            }
            match self.view {
                View::Velocity => data.velocity = ((level + 1) * 8 - 1).min(127),
                View::Probability if row == 1 => data.skip = PROBABILITY_LEVELS - 1 - level,
                View::Micro if row == 1 && level <= MAX_DELAY => data.delay = level,
                View::Mutation if row == 1 && level < MUTATION_LEVELS => data.mutate = level,
                _ => {}
            }
        }
        if self.held != 0 {
            self.held_edited = true;
        }
    }

    fn pattern_pad(&mut self, project: &mut Project, engine: &mut Engine, track: u8, pattern: u8, pressed: bool) {
        let tag = track * PATTERNS as u8 + pattern + 1;
        if pressed {
            self.track = track;
            if self.clear_held {
                project.tracks[track as usize].patterns[pattern as usize].reset();
                return;
            }
            if self.duplicate_held {
                match self.duplicate_source {
                    0 => self.duplicate_source = tag,
                    from => {
                        let from = from - 1;
                        let copy = project.tracks[(from / PATTERNS as u8) as usize].patterns[(from % PATTERNS as u8) as usize];
                        project.tracks[track as usize].patterns[pattern as usize] = copy;
                    }
                }
                return;
            }
            if self.held_pattern != 0 && (self.held_pattern - 1) / PATTERNS as u8 == track {
                // Holding a pattern and pressing another in its row chains them.
                let other = (self.held_pattern - 1) % PATTERNS as u8;
                let selection = Selection { first: pattern.min(other), last: pattern.max(other) };
                engine.select(project, track as usize, selection);
                self.pattern_edited = true;
                return;
            }
            self.held_pattern = tag;
            self.pattern_edited = false;
        } else if self.held_pattern == tag {
            self.held_pattern = 0;
            if !self.pattern_edited {
                engine.select(project, track as usize, Selection { first: pattern, last: pattern });
            }
        }
    }

    fn scene_pad(&mut self, project: &mut Project, engine: &mut Engine, scene: u8) {
        let s = scene as usize;
        if self.shift {
            // Shift + pad stores what is playing now in the scene.
            project.scenes[s] = project.chain;
            project.scene = scene;
        } else if self.clear_held {
            let p = (s % PATTERNS) as u8;
            project.scenes[s] = [Selection { first: p, last: p }; TRACKS];
        } else if self.duplicate_held {
            match self.duplicate_source {
                0 => self.duplicate_source = scene + 1,
                from => project.scenes[s] = project.scenes[from as usize - 1],
            }
        } else {
            for t in 0..TRACKS {
                engine.select(project, t, project.scenes[s][t]);
            }
            project.scene = scene;
        }
    }

    fn settings_pad(&mut self, project: &mut Project, index: u8) {
        let track = self.track as usize;
        let (row, col) = (index / 10, index % 10);
        let c = col - 1;
        match self.settings_page {
            0 => {
                let pattern = project.tracks[track].pattern_mut();
                if let Some(step) = self.step_of(index) {
                    pattern.length = step + 1;
                } else {
                    match row {
                        6 if c < 4 => pattern.direction = c,
                        5 => pattern.rate = c,
                        4 if c < 4 => pattern.swing = c,
                        _ => {}
                    }
                }
            }
            1 => {
                let t = &mut project.tracks[track];
                match row {
                    7 | 8 => t.channel = (8 - row) * 8 + c,
                    6 if c < 3 => t.kind = c,
                    5 => t.scale = c,
                    4 if c < 4 => t.scale = 8 + c,
                    3 => t.root = (t.root / 12 * 12 + c).max(1),
                    2 if c < 4 => t.root = (t.root / 12 * 12 + 8 + c).max(1),
                    1 => t.root = (c * 12 + t.root % 12).clamp(1, 127),
                    _ => {}
                }
            }
            _ => {
                if row == 8 {
                    match c {
                        0 => project.external_clock = !project.external_clock,
                        1 => project.send_clock = !project.send_clock,
                        _ => {}
                    }
                }
            }
        }
    }

    pub fn render(&self, project: &Project, engine: &Engine, frame: &mut [u32; LED_COUNT]) {
        *frame = [0; LED_COUNT];
        let t = self.track as usize;
        let track = &project.tracks[t];
        let color = TRACK_COLORS[t];

        match self.view {
            View::Steps => {
                self.render_steps(project, engine, frame);
                self.render_play_area(track, color, frame);
            }
            View::Velocity | View::Probability | View::Micro | View::Mutation => {
                self.render_steps(project, engine, frame);
                self.render_values(track, color, frame);
            }
            View::Patterns => self.render_patterns(project, engine, frame),
            View::Scenes => {
                for step in 0..STEPS_PER_PAGE {
                    let index = 80 - (step as usize / 8) * 10 + 1 + step as usize % 8;
                    frame[index] = if project.scene == step { 0xffffff } else { 0x0030c0 };
                }
            }
            View::Settings => self.render_settings(project, color, frame),
            View::Projects => self.render_projects(frame),
        }

        frame[UP as usize] = color / 2;
        frame[DOWN as usize] = color / 2;
        frame[LEFT as usize] = if self.page > 0 { 0x303030 } else { 0x050505 };
        frame[RIGHT as usize] = if self.page + 1 < PAGES { 0x303030 } else { 0x050505 };
        frame[PROJECTS as usize] = if self.view == View::Projects { 0xffffff } else { 0x002040 };
        frame[PLAY as usize] = if engine.running() { 0x00ff00 } else { 0x003000 };
        frame[SHIFT as usize] = if self.shift { 0xffffff } else { 0x101010 };

        if self.shift {
            for (button, view) in VIEW_BUTTONS {
                frame[button as usize] = if view == self.view { 0xffffff } else { 0x002030 };
            }
        } else {
            frame[RECORD as usize] = if self.record { 0xff0000 } else { 0x300000 };
            frame[SOLO as usize] = if track.solo { 0xffff00 } else { 0x302800 };
            frame[MUTE as usize] = if track.muted { 0xff6000 } else { 0x301000 };
            frame[CLEAR as usize] = if self.clear_held { 0xff0000 } else { 0x300000 };
            frame[DUPLICATE as usize] = if self.duplicate_held { 0x4040ff } else { 0x080830 };
            frame[QUANTIZE as usize] = if self.unquantized { 0x103010 } else { 0x00ff40 };
        }
    }

    fn step_index(&self, step: u8) -> Option<usize> {
        let first = self.page * STEPS_PER_PAGE;
        if step < first || step >= first + STEPS_PER_PAGE {
            return None;
        }
        let local = step - first;
        Some(80 - (local as usize / 8) * 10 + 1 + local as usize % 8)
    }

    fn render_steps(&self, project: &Project, engine: &Engine, frame: &mut [u32; LED_COUNT]) {
        let t = self.track as usize;
        let color = TRACK_COLORS[t];
        let pattern = project.tracks[t].pattern();
        let playhead = engine.playhead(t);
        for step in 0..STEPS as u8 {
            let Some(index) = self.step_index(step) else { continue };
            let data = &pattern.steps[step as usize];
            let beyond = step >= pattern.length;
            frame[index] = if playhead == Some(step) {
                0xffffff
            } else if self.held & (1 << step) != 0 {
                0xff80ff
            } else if data.is_empty() {
                if beyond { 0x010101 } else { 0x080808 }
            } else {
                let lit = match self.view {
                    View::Velocity => scale(color, data.velocity as u32 + 20, 147),
                    View::Probability => scale(color, data.probability() as u32 + 1, 8),
                    View::Micro => if data.delay > 0 { 0x8020ff } else { color },
                    View::Mutation => if data.mutate > 0 { 0xc020c0 } else { color },
                    _ => color,
                };
                if beyond { scale(lit, 1, 4) } else { lit }
            };
        }
    }

    fn render_play_area(&self, track: &Track, color: u32, frame: &mut [u32; LED_COUNT]) {
        let held = (0..STEPS as u8).find(|s| self.held & (1 << s) != 0).map(|s| &track.pattern().steps[s as usize]);
        for row in 1..=6u8 {
            for col in 1..=8u8 {
                let Some(note) = play_note(track, row - 1, col - 1) else { continue };
                frame[(row * 10 + col) as usize] = if held.is_some_and(|d| d.has(note)) {
                    0xffffff
                } else if is_root(track, row - 1, col - 1) {
                    color
                } else if track.kind() == TrackKind::Drum {
                    scale(color, 1, 4)
                } else {
                    0x101010
                };
            }
        }
    }

    fn render_values(&self, track: &Track, color: u32, frame: &mut [u32; LED_COUNT]) {
        let shown = (0..STEPS as u8).find(|s| self.held & (1 << s) != 0).map(|s| track.pattern().steps[s as usize]);
        let (count, selected): (u8, Option<u8>) = match self.view {
            View::Velocity => (16, shown.map(|d| (d.velocity / 8).min(15))),
            View::Probability => (8, shown.map(|d| d.probability())),
            View::Micro => (MAX_DELAY + 1, shown.map(|d| d.delay)),
            _ => (MUTATION_LEVELS, shown.map(|d| d.mutate)),
        };
        for level in 0..count {
            let index = 10 * (1 + level / 8) as usize + 1 + (level % 8) as usize;
            frame[index] = if selected == Some(level) {
                0xffffff
            } else {
                scale(color, level as u32 + 1, count as u32 + 2)
            };
        }
    }

    fn render_patterns(&self, project: &Project, engine: &Engine, frame: &mut [u32; LED_COUNT]) {
        for t in 0..TRACKS {
            let chain = project.chain[t];
            let queued = engine.queued(t);
            let color = TRACK_COLORS[t];
            for p in 0..PATTERNS as u8 {
                let index = (80 - t * 10) + 1 + p as usize;
                let held = self.held_pattern == t as u8 * PATTERNS as u8 + p + 1;
                frame[index] = if held {
                    0xff80ff
                } else if project.tracks[t].current == p {
                    color
                } else if queued.is_some_and(|q| (q.first..=q.last).contains(&p)) {
                    0x909090
                } else if (chain.first..=chain.last).contains(&p) {
                    scale(color, 1, 3)
                } else if !project.tracks[t].patterns[p as usize].is_empty() {
                    0x181818
                } else {
                    0x040404
                };
            }
        }
    }

    /// Layout only: the store is not asked which slots are used while drawing. The
    /// result of the last action is shown on the pad that was pressed.
    fn render_projects(&self, frame: &mut [u32; LED_COUNT]) {
        for slot in 0..PROJECT_SLOTS as usize {
            frame[80 + 1 + slot] = 0x002040;
            frame[70 + 1 + slot] = 0x402000;
        }
        frame[58] = 0x400000;
        if self.now_us < self.notice_until && self.notice != 0 {
            frame[self.notice_pad as usize] = if self.notice == 1 { 0x00ff00 } else { 0xff0000 };
        }
    }

    fn render_settings(&self, project: &Project, color: u32, frame: &mut [u32; LED_COUNT]) {
        let track = &project.tracks[self.track as usize];
        let pattern = track.pattern();
        let dim = 0x101010;
        let mark = |frame: &mut [u32; LED_COUNT], row: usize, count: u8, selected: u8| {
            for c in 0..count {
                frame[row * 10 + 1 + c as usize] = if c == selected { 0xffffff } else { dim };
            }
        };
        match self.settings_page {
            0 => {
                for step in 0..STEPS as u8 {
                    if let Some(index) = self.step_index(step) {
                        frame[index] = if step < pattern.length { color } else { dim };
                    }
                }
                mark(frame, 6, 4, pattern.direction);
                mark(frame, 5, 8, pattern.rate);
                mark(frame, 4, 4, pattern.swing);
            }
            1 => {
                mark(frame, 8, 8, track.channel);
                mark(frame, 7, 8, track.channel.wrapping_sub(8));
                mark(frame, 6, 3, track.kind);
                mark(frame, 5, 8, track.scale);
                mark(frame, 4, 4, track.scale.wrapping_sub(8));
                mark(frame, 3, 8, track.root % 12);
                mark(frame, 2, 4, (track.root % 12).wrapping_sub(8));
                mark(frame, 1, 8, track.root / 12);
            }
            _ => {
                frame[81] = if project.external_clock { 0x00ff00 } else { 0x300000 };
                frame[82] = if project.send_clock { 0x00ff00 } else { 0x300000 };
            }
        }
    }
}
