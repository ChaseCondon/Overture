//! # overture-ipc
//!
//! Shared-memory IPC primitives for out-of-process plugin sandboxing.
//!
//! Each sandboxed plugin runs in its own child process. The audio engine
//! and the plugin process communicate via shared-memory ring buffers — one
//! per stream:
//!
//! - **MIDI in** — host → plugin
//! - **Audio out** — plugin → host
//! - **Parameter changes** — bidirectional
//! - **Heartbeat** — a sequence-number tick the host watches per audio
//!   callback to detect plugin hangs
//!
//! Process spawning and lifecycle live in Stardust core (since sandbox
//! policy is application-specific). This crate provides the protocol +
//! buffer layout the two sides agree on.
//!
//! This is the v0.1 scaffold — implementation lands in v0.3 (Plugin
//! Sandboxing + CLAP).

#![doc(html_root_url = "https://docs.rs/overture-ipc/0.0.1")]
