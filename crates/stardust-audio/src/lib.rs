//! # stardust-audio
//!
//! Real-time-safe audio I/O for stardust-core.
//!
//! Wraps [`cpal`] with a higher-level API tailored for live-performance
//! applications. The intent is to give Stardust (and any other stardust-core
//! consumer) a stable surface for opening audio devices, configuring buffer
//! sizes, running an audio callback, and handling hot-plug events — without
//! exposing CPAL's per-host quirks.
//!
//! ## Platform coverage
//!
//! - **macOS**: CoreAudio
//! - **Windows**: WASAPI by default; ASIO via the `asio` feature
//! - **Linux**: ALSA (development only — not a supported runtime target)
//!
//! Most of the public surface lives in `device`, `stream`, and `callback`
//! modules. None of those exist yet — this is the v0.1 scaffold.

#![doc(html_root_url = "https://docs.rs/stardust-audio/0.0.1")]
