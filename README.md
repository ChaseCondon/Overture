<div align="center">

# stardust-core

**A cross-platform Rust audio library for live performance applications.**

Audio I/O, MIDI I/O, VST3 and CLAP plugin hosting, real-time DSP primitives.

[![Build status](https://img.shields.io/github/actions/workflow/status/StardustMT/stardust-core/ci.yml?branch=main&label=build)](https://github.com/StardustMT/stardust-core/actions)
[![Crates.io](https://img.shields.io/crates/v/stardust-core.svg)](https://crates.io/crates/stardust-core)
[![Documentation](https://docs.rs/stardust-core/badge.svg)](https://docs.rs/stardust-core)
[![License: Apache 2.0](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Wiki](https://img.shields.io/badge/docs-wiki-green.svg)](https://github.com/StardustMT/stardust-core/wiki)

</div>

---

> [!WARNING]
> **Pre-alpha.** stardust-core is in early-stage development. Not yet published to crates.io. The API will change.

## What is stardust-core?

stardust-core is the audio engine that powers [Stardust](https://github.com/StardustMT/stardust-pit), packaged as a general-purpose Rust crate so any audio application can use it.

It sits above the lower-level audio and MIDI ecosystem (`cpal`, `midir`) and provides the things live-performance applications typically need on top:

- Real-time-safe audio I/O across CoreAudio, WASAPI, and ASIO (Windows)
- MIDI input and output with hot-plug detection
- VST3 plugin hosting via a small C++ shim, exposed through a clean Rust API
- CLAP plugin hosting via [`clack`](https://github.com/prokopyl/clack)
- Out-of-process plugin sandboxing for crash isolation
- Lock-free queues and ring buffers for UI ↔ audio thread communication
- Built-in DSP effects (EQ, reverb, compression)
- Voice-tracking primitives to prevent stuck notes on patch changes

## Why a separate crate?

The audio engine is generally reusable, and there is no reason to lock it inside one application. Splitting it out means anyone can build a live host, a plugin chainer, or any other real-time audio tool without reimplementing the same primitives.

stardust-core is licensed Apache 2.0 (Stardust itself is GPL v3) so it can be embedded in commercial and proprietary applications without licensing friction.

## Quick start

> [!NOTE]
> Pre-release. Examples below illustrate the intended API; not yet on crates.io.

```toml
[dependencies]
stardust-core = "0.1"
```

```rust
use stardust-core::audio::AudioEngine;
use stardust-core::midi::MidiInput;

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

- **[Project wiki](https://github.com/StardustMT/stardust-core/wiki)** — architecture, API guides, examples, contributing
- **[API reference (docs.rs)](https://docs.rs/stardust-core)** — published with the first crates.io release

## Modules

| Module | Purpose |
|---|---|
| `audio` | CPAL wrapper, device management, real-time thread setup |
| `midi` | midir wrapper, MIDI routing, hot-plug detection |
| `plugins::vst3` | VST3 plugin hosting (C++ shim + Rust API) |
| `plugins::clap` | CLAP plugin hosting via `clack` |
| `plugins::sandbox` | Out-of-process plugin orchestration |
| `effects` | Built-in DSP: EQ, reverb, compression |
| `util::lockfree` | Lock-free queues, ring buffers |
| `util::voices` | Voice tracking, stuck-note prevention |

## Contributing

Contributions welcome — especially around plugin-format support, DSP, and platform-specific audio backends.

See [CONTRIBUTING.md](CONTRIBUTING.md). Open an [issue](https://github.com/StardustMT/stardust-core/issues) or [discussion](https://github.com/StardustMT/stardust-core/discussions).

## License

[Apache 2.0](LICENSE). Permissive license with explicit patent grant — safe to embed in commercial applications.
