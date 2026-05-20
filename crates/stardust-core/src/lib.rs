//! # stardust-core
//!
//! Cross-platform Rust audio library for live performance applications.
//!
//! This is the umbrella crate. It re-exports the individual stardust-core crates
//! behind feature flags. For most applications, depend on `stardust-core` directly
//! and enable the features you need.
//!
//! ## Features
//!
//! | Feature  | Enables                                     |
//! |----------|---------------------------------------------|
//! | `audio`  | [`stardust_audio`] — CPAL audio I/O         |
//! | `midi`   | [`stardust_midi`] — midir MIDI I/O          |
//! | `plugin` | [`stardust_plugin`] — VST3 + CLAP hosting   |
//! | `dsp`    | [`stardust_dsp`] — built-in effects         |
//! | `rt`     | [`stardust_rt`] — RT-safe primitives        |
//! | `ipc`    | [`stardust_ipc`] — sandboxing IPC           |
//! | `patch`  | [`stardust_patch`] — patch-graph data model |
//! | `full`   | All of the above                            |
//!
//! Defaults: `audio`, `midi`, `rt`.

#[cfg(feature = "audio")]
pub use stardust_audio as audio;

#[cfg(feature = "midi")]
pub use stardust_midi as midi;

#[cfg(feature = "plugin")]
pub use stardust_plugin as plugin;

#[cfg(feature = "dsp")]
pub use stardust_dsp as dsp;

#[cfg(feature = "rt")]
pub use stardust_rt as rt;

#[cfg(feature = "ipc")]
pub use stardust_ipc as ipc;

#[cfg(feature = "patch")]
pub use stardust_patch as patch;
