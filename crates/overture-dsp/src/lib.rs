//! # overture-dsp
//!
//! Built-in audio DSP for Overture: EQ, reverb, compression, gain staging,
//! basic limiter. Pure Rust, allocation-free in the audio thread.
//!
//! The intent is to cover the common per-patch insert chain so a typical
//! Stardust patch doesn't need an external effect plugin for everyday
//! shaping. Heavier effect work belongs in VST3 / CLAP plugins via
//! [`overture-plugin`](../overture_plugin/index.html).
//!
//! This is the v0.1 scaffold — DSP modules land in v0.5 (MT Features).

#![doc(html_root_url = "https://docs.rs/overture-dsp/0.0.1")]
#![no_std]
