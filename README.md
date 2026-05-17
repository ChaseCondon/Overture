<div align="center">

# Overture

**Cross-platform Rust audio library for live performance applications.**

Audio I/O, MIDI I/O, VST3 / CLAP plugin hosting, real-time DSP primitives.

[![Build status](https://img.shields.io/github/actions/workflow/status/ChaseCondon/overture/ci.yml?branch=main&label=build)](https://github.com/ChaseCondon/overture/actions)
[![Crates.io](https://img.shields.io/crates/v/overture.svg)](https://crates.io/crates/overture)
[![Documentation](https://docs.rs/overture/badge.svg)](https://docs.rs/overture)
[![License: Apache 2.0](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Wiki](https://img.shields.io/badge/docs-wiki-green.svg)](https://github.com/ChaseCondon/overture/wiki)

</div>

---

> ⚠️ **Pre-alpha.** Overture is in early-stage development. Not yet published to crates.io. API will change.

## What is Overture?

Overture is the audio engine that powers [Stardust](https://github.com/ChaseCondon/stardust) — but designed as a general-purpose library that any Rust audio app can use.

It wraps the lower-level audio + MIDI ecosystem (`cpal`, `midir`) behind ergonomic abstractions and provides what most live-performance apps need on top:

- Real-time-safe audio I/O across CoreAudio, WASAPI, ASIO (Windows)
- MIDI input/output with hot-plug detection
- VST3 plugin hosting (via a small C++ shim, hidden behind a clean Rust API)
- CLAP plugin hosting via [`clack`](https://github.com/prokopyl/clack)
- Out-of-process plugin sandboxing for crash isolation
- Lock-free queues and ring buffers for UI ↔ audio thread communication
- Built-in effects DSP (EQ, reverb, compression)
- Voice-tracking primitives to prevent stuck notes on patch changes

## Why a separate crate?

Because the audio engine should be reusable by anyone, not locked inside Stardust. Build your own MainStage alternative, host plugins in your DAW, or wire up a kiosk-grade live audio kiosk — Overture is the building blocks.

Licensed Apache 2.0 (vs Stardust's GPL v3) precisely so it can ship inside commercial and proprietary apps.

## Quickstart

*(Pre-release — coming with first crates.io publish.)*

```toml
[dependencies]
overture = "0.1"
```

```rust
use overture::audio::AudioEngine;
use overture::midi::MidiInput;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let engine = AudioEngine::default()?;
    let midi_in = MidiInput::default_device()?;

    // ... wire MIDI to a plugin, plugin to audio out
    engine.run()?;
    Ok(())
}
```

See [`examples/`](examples/) for runnable code.

## Documentation

📚 **[Project Wiki](https://github.com/ChaseCondon/overture/wiki)** — architecture, API guides, examples, contributing.

📖 **[API reference (docs.rs)](https://docs.rs/overture)** — when published.

## Modules

| Module | Purpose |
|---|---|
| `audio` | CPAL wrapper, audio device management, real-time thread setup |
| `midi` | midir wrapper, MIDI routing, hot-plug detection |
| `plugins::vst3` | VST3 plugin hosting (C++ shim + Rust API) |
| `plugins::clap` | CLAP plugin hosting via `clack` |
| `plugins::sandbox` | Out-of-process plugin orchestration |
| `effects` | Built-in DSP: EQ, reverb, compression |
| `util::lockfree` | Lock-free queues, ring buffers |
| `util::voices` | Voice tracking, stuck-note prevention |

## Contributing

Contributions welcome — especially around plugin format support, DSP, and platform-specific audio backends.

See [CONTRIBUTING.md](CONTRIBUTING.md). Open an [issue](https://github.com/ChaseCondon/overture/issues) or [discussion](https://github.com/ChaseCondon/overture/discussions).

## License

[Apache 2.0](LICENSE). Permissive license with explicit patent grant — embed in commercial apps freely.
