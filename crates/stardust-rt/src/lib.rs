//! # stardust-rt
//!
//! Real-time-safe primitives for stardust-core.
//!
//! Everything in here is safe to call from an audio callback: no allocations,
//! no syscalls, no locks that may block. Anything that doesn't meet that bar
//! belongs in another crate.
//!
//! ## Planned modules
//!
//! - `queue` — SPSC ring buffers (re-exporting [`rtrb`]) wrapped in
//!   higher-level types for UI ↔ audio commands and audio ↔ plugin IPC
//! - `voices` — pre-allocated voice tracker for stuck-note prevention
//!   across patch changes
//! - `mailbox` — bounded lock-free mailboxes for command + event passing
//! - `arena` — pre-allocated arenas for transient audio-thread storage
//!
//! This is the v0.1 scaffold — modules land in v0.2.

#![doc(html_root_url = "https://docs.rs/stardust-rt/0.0.1")]
