# stardust-sfz

Stardust's first-party CLAP SFZ sampler plugin. Lives in the
`stardust-core` workspace as the reference / dogfood plugin for the
Stardust host's CLAP support. Future first-party plugins (EQ, reverb,
limiter) will follow the same shape and live in sibling
`crates/stardust-*/` directories.

## Status

v0.1 — workable for real instruments, still POC-class. Supports
enough SFZ to load and play velocity-layered pianos, looped pads,
sustained organs, drum kits, and most basic preset libraries with
correct envelopes and sustain-pedal behaviour. Intentionally skips
filters, LFOs, round-robin, release samples, and CC modulation; those
land in later phases.

Supported sections + opcode inheritance: `<global>` → `<master>` →
`<group>` → `<region>` — opcodes set at any level cascade into every
narrower scope, regions can override anything inherited. `<control>`
honours `default_path`.

Supported opcodes (anywhere in the inheritance chain):

- **mapping**: `sample`, `key`, `lokey`, `hikey`, `pitch_keycenter`,
  `lovel`, `hivel`
- **amplitude**: `volume` (dB), `pan` (-100..100, equal-power)
- **pitch**: `tune` (cents), `transpose` (semitones)
- **envelope**: `ampeg_attack`, `ampeg_decay`, `ampeg_sustain`
  (percent), `ampeg_release` (all seconds)
- **loop**: `loop_mode` (`no_loop` / `one_shot` / `loop_continuous`),
  `loop_start`, `loop_end` (frame indices)

Note values accept either MIDI numbers (`60`) or note names (`c4`,
`c#3`, `eb-1`).

MIDI handling:

- **CC 64** sustain pedal — note-offs deferred while held; pedal-up
  releases everything that was waiting.
- **CC 123** all notes off.
- **Pitch bend** ±2 semitones (general MIDI default).

Unknown opcodes are silently dropped, so partial-support instruments
still load and play (without the unimplemented bits).

## Building

```bash
cargo build --release -p stardust-sfz
```

This produces a `cdylib` at `target/release/`:

- Linux:   `libstardust_sfz.so`
- macOS:   `libstardust_sfz.dylib`
- Windows: `stardust_sfz.dll`

CLAP hosts look for files with the `.clap` extension. Rename or
symlink the build output:

```bash
# Linux
cp target/release/libstardust_sfz.so ~/.clap/stardust-sfz.clap

# macOS
cp target/release/libstardust_sfz.dylib ~/Library/Audio/Plug-Ins/CLAP/stardust-sfz.clap

# Windows (PowerShell, as admin)
Copy-Item target\release\stardust_sfz.dll "$env:COMMONPROGRAMFILES\CLAP\stardust-sfz.clap"
```

Verify the host finds it:

```bash
cargo run -p stardust-poc --bin stardust-poc-clap-list
```

You should see `Stardust SFZ` in the listing.

## Loading an SFZ file

For the v0 POC, the plugin reads the SFZ path from
`STARDUST_SFZ_PATH` at instantiation. The plugin still loads if the
variable is unset or the file fails to parse — it just produces silence.

```bash
STARDUST_SFZ_PATH=/path/to/instrument.sfz <host>
```

Future iterations will use the CLAP `state` extension so hosts can
persist the loaded SFZ in patch files, plus a plugin GUI with a file
picker.

## Trying without a CLAP host

The `render_to_wav` example drives the engine directly and writes an
ascending C-major arpeggio to a WAV file:

```bash
cargo run -p stardust-sfz --example render_to_wav -- path/to/instrument.sfz out.wav
```

Useful for sanity-checking parser + sample loading + engine output
without standing up a host.

## Architecture notes

- `sfz.rs` — SFZ text → `SfzFile { regions: Vec<Region> }`. Pure
  parsing, no I/O.
- `sample.rs` — WAV decoding via `hound`. Samples decode fully into
  RAM at load time; streaming-from-disk is a later phase.
- `instrument.rs` — combines the two: resolves region sample paths
  against the SFZ's directory, deduplicates samples shared across
  regions, returns a load report (instrument + per-file errors).
- `engine.rs` — polyphonic playback. Pre-allocated voice pool (32
  voices), pitch-shifted linear-interpolated playback, simple linear
  release on note-off. Audio-thread safe (no allocations, no locks).
- `lib.rs` — CLAP plugin shell via `clack-plugin`. Declares stereo
  audio out + CLAP/MIDI note in, routes note events to the engine,
  renders into the host's output buffer.

## RAM safety

Samples decode to f32 in RAM at load time. Two caps prevent runaway
libraries from OOM-ing the host (configurable via `LoadLimits`):

- **per-sample**: 64 MiB default
- **per-instrument**: 512 MiB total default

Both are soft — oversized samples are skipped with a clear error
into `LoadReport::errors`, and the rest of the instrument loads.
Streaming-from-disk via `stardust_rt::RingBuffer` is a future phase
for true multi-GB libraries.

## Future work

- CLAP state extension (persist loaded SFZ in patch files)
- Plugin GUI with file picker
- Round-robin (`seq_position`, `seq_length`)
- Release samples (`trigger=release`)
- Filters + LFOs (`fil_type`, `cutoff`, `lfoN_*`)
- CC modulation matrix (`amplitude_oncc7`, `cutoff_oncc1`, …)
- Per-region `bend_up` / `bend_down` overrides
- Streaming sample playback for libraries that don't fit in RAM
