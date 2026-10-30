// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 launchpad-Pro-Mod contributors

//! The sequencer linked into this Launchpad X build (chosen with the
//! `sequencer-*` features). Both styles offer the same app interface.

#[cfg(feature = "sequencer-launchpad95")]
pub use crate::app::seq95::Seq95App as SequencerApp;

#[cfg(feature = "sequencer-novation")]
pub use crate::app::novseq::NovSeqApp as SequencerApp;
