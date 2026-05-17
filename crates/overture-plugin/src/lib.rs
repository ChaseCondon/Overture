//! # overture-plugin
//!
//! VST3 and CLAP plugin hosting for Overture.
//!
//! - **VST3** support uses a small C++ shim around the Steinberg VST3 SDK,
//!   called from Rust via FFI. The shim is built by `build.rs` (via the `cc`
//!   crate) when the `vst3` feature is enabled.
//! - **CLAP** support uses [`clack_host`].
//!
//! Out-of-process (sandboxed) hosting lives in `overture-ipc`. This crate
//! covers the in-process API surface: scanning, loading, parameter access,
//! preset state, and the audio-callback wiring.
//!
//! This is the v0.1 scaffold — public modules will land in v0.2 (VST3 only,
//! in-process) and v0.3 (CLAP + sandboxing).

#![doc(html_root_url = "https://docs.rs/overture-plugin/0.0.1")]
