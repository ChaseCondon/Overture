# stardust-sfz

Stardust's first-party CLAP SFZ sampler plugin. Lives in the
`stardust-core` workspace as the reference / dogfood plugin for the
Stardust host's CLAP support. Future first-party plugins (EQ, reverb,
limiter) will follow the same shape and live in sibling
`crates/stardust-*/` directories.

## Status

POC — v0 of a real instrument. Supports enough SFZ to load and play
basic real-world instruments (one-shot sample regions, multi-key
mapping, velocity layers); intentionally skips envelopes, filters,
LFOs, round-robin, looping, and groups for now.

Supported opcodes inside `<region>`:

- `sample=<relative path>` — relative to the `.sfz` file's directory.
  Forward and backslash separators both work.
- `pitch_keycenter=<0..127>` — note the sample was recorded at.
- `lokey` / `hikey` — MIDI note range (inclusive).
- `key=N` — shorthand for `lokey=N hikey=N pitch_keycenter=N`.
- `lovel` / `hivel` — velocity range (inclusive).
- `volume=<db>` — region gain in decibels.

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

## Future work

- CLAP state extension (persist loaded SFZ in patch files)
- Plugin GUI with file picker
- Envelopes (ampeg_attack, ampeg_release, etc.)
- Round-robin (`seq_position`, `seq_length`)
- Looping (`loop_mode`, `loop_start`, `loop_end`)
- Note names in opcode values (`c4` rather than `60`)
- Groups + globals with opcode inheritance
- Streaming sample playback for libraries that don't fit in RAM
