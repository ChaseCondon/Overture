//! # Overture
//!
//! Cross-platform Rust audio library for live performance applications.
//!
//! This is the umbrella crate. It re-exports the individual Overture crates
//! behind feature flags. For most applications, depend on `overture` directly
//! and enable the features you need.
//!
//! ## Features
//!
//! | Feature  | Enables                                     |
//! |----------|---------------------------------------------|
//! | `audio`  | [`overture_audio`] — CPAL audio I/O         |
//! | `midi`   | [`overture_midi`] — midir MIDI I/O          |
//! | `plugin` | [`overture_plugin`] — VST3 + CLAP hosting   |
//! | `dsp`    | [`overture_dsp`] — built-in effects         |
//! | `rt`     | [`overture_rt`] — RT-safe primitives        |
//! | `ipc`    | [`overture_ipc`] — sandboxing IPC           |
//! | `full`   | All of the above                            |
//!
//! Defaults: `audio`, `midi`, `rt`.

#[cfg(feature = "audio")]
pub use overture_audio as audio;

#[cfg(feature = "midi")]
pub use overture_midi as midi;

#[cfg(feature = "plugin")]
pub use overture_plugin as plugin;

#[cfg(feature = "dsp")]
pub use overture_dsp as dsp;

#[cfg(feature = "rt")]
pub use overture_rt as rt;

#[cfg(feature = "ipc")]
pub use overture_ipc as ipc;
