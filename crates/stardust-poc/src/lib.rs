//! # stardust-poc
//!
//! Integration tests / demo binaries that wire several stardust-core crates
//! together end-to-end. This crate intentionally has no public API — it
//! exists as a host for `[[bin]]` targets that exercise the full
//! MIDI → ring buffer → audio thread → synth pipeline before plugin
//! hosting lands in v0.2+.
//!
//! See `src/bin/stardust-poc-play.rs` for the canonical example.

#![doc(html_root_url = "https://docs.rs/stardust-poc/0.0.1")]
