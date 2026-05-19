//! Offline render: load an SFZ, play a simple ascending arpeggio, save WAV.
//!
//! Skips the CLAP plumbing entirely — drives the engine directly. Useful
//! for verifying the parser, sample loader, and engine produce sensible
//! audio on a machine without a CLAP host installed.
//!
//! ```text
//! cargo run -p stardust-sfz --example render_to_wav -- path/to/instrument.sfz out.wav
//! ```

use std::env;
use std::path::PathBuf;
use std::sync::Arc;

use stardust_sfz::engine::Engine;
use stardust_sfz::instrument::load_sfz;

fn main() -> anyhow::Result<()> {
    let mut args = env::args().skip(1);
    let sfz_path = PathBuf::from(args.next().ok_or_else(|| {
        anyhow::anyhow!("usage: render_to_wav <input.sfz> <output.wav>")
    })?);
    let out_path = PathBuf::from(args.next().unwrap_or_else(|| "out.wav".to_string()));

    let report = load_sfz(&sfz_path)?;
    println!(
        "Loaded {}: {} regions, {} samples ({} errors)",
        sfz_path.display(),
        report.instrument.regions.len(),
        report.instrument.samples.len(),
        report.errors.len()
    );
    for (p, msg) in &report.errors {
        println!("  ⚠ {} — {msg}", p.display());
    }
    if report.instrument.regions.is_empty() {
        anyhow::bail!("no playable regions in the instrument");
    }

    let sample_rate = 48_000u32;
    let mut engine = Engine::new(Arc::new(report.instrument), sample_rate as f32);

    // Play an ascending C major arpeggio: C E G C, each held for 400ms,
    // then 600ms of release tail.
    let notes: [u8; 4] = [60, 64, 67, 72];
    let hold_frames = sample_rate as usize * 4 / 10; // 400ms
    let tail_frames = sample_rate as usize * 6 / 10; // 600ms
    let total_frames = notes.len() * hold_frames + tail_frames;
    let mut buffer = vec![0.0f32; total_frames * 2];

    let mut cursor = 0usize;
    for &note in &notes {
        engine.note_on(0, note, 100);
        let slice = &mut buffer[cursor * 2..(cursor + hold_frames) * 2];
        engine.render_into_stereo(slice);
        engine.note_off(0, note);
        cursor += hold_frames;
    }
    let tail = &mut buffer[cursor * 2..];
    engine.render_into_stereo(tail);

    let spec = hound::WavSpec {
        channels: 2,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(&out_path, spec)?;
    for sample in &buffer {
        let s = (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
        writer.write_sample(s)?;
    }
    writer.finalize()?;
    println!("Wrote {} ({total_frames} frames)", out_path.display());
    Ok(())
}
