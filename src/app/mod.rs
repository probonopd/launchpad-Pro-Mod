// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2025-2026 Anthony Hofmeister

pub mod apptrait;
pub mod boot;
pub mod daw;
pub mod events;
pub mod host;
#[cfg(feature = "launchpad-x")]
pub mod custom;
#[cfg(feature = "launchpad-x")]
pub mod note;
pub mod palette_editor;
pub mod performance;
pub mod programmer;
#[cfg(any(feature = "launchpad-pro-mk3", all(feature = "launchpad-x", feature = "sequencer-launchpad95")))]
pub mod seq95;
#[cfg(all(feature = "launchpad-x", feature = "sequencer-novation"))]
pub mod novseq;
#[cfg(feature = "launchpad-x")]
pub mod sequencer;
pub mod setup;

#[cfg(all(feature = "launchpad-x", feature = "sequencer-launchpad95", feature = "sequencer-novation"))]
compile_error!("choose exactly one sequencer style: sequencer-launchpad95 or sequencer-novation");
#[cfg(all(feature = "launchpad-x", not(feature = "sequencer-launchpad95"), not(feature = "sequencer-novation")))]
compile_error!("choose a sequencer style: sequencer-launchpad95 or sequencer-novation");

pub use crate::sys::midi::MidiPort;
pub use apptrait::App;
pub use boot::{BootAnimationApp, BootApp};
pub use events::{AftertouchEvent, MidiEvent, SurfaceEvent};
pub use host::AppHost;

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum AppId {
    Boot,
    Setup,
    Performance,
    Programmer,
    Daw,
    PaletteEditor,
    #[cfg(any(feature = "launchpad-pro-mk3", feature = "launchpad-x"))]
    Seq95,
    #[cfg(feature = "launchpad-x")]
    Note,
    #[cfg(feature = "launchpad-x")]
    Custom,
}
