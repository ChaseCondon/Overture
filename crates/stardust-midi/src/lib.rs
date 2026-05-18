//! # stardust-midi
//!
//! MIDI input and output for stardust-core, with hot-plug detection and
//! device-profile-aware handling.
//!
//! Wraps [`midir`] (CoreMIDI / WinMM / ALSA) behind a higher-level API that:
//!
//! - Lists devices and watches for hot-plug events
//! - Routes incoming MIDI into a lock-free queue consumed by the audio thread
//! - Applies per-device input transforms (debounce, sustain-slot detection,
//!   stuck-CC mitigation) so the higher layers see clean events
//!
//! This is the v0.1 scaffold — public modules will land in v0.2.

#![doc(html_root_url = "https://docs.rs/stardust-midi/0.0.1")]
