//! Polyphonic sample-playback engine.
//!
//! Audio-thread design:
//!
//! - Pre-allocated voice pool sized at construction (POLYPHONY).
//! - Per-voice state is `Copy` — no heap allocation per note.
//! - Voice steal strategy: prefer Idle, then Releasing (closest to
//!   silence), else oldest active.
//! - Pitch shift via linear interpolation between two consecutive
//!   source samples — cheap, audibly clean for non-extreme intervals.
//!   Higher-order interpolation can swap in later without touching the
//!   note routing.
//!
//! Channel routing: the engine renders interleaved stereo. Mono source
//! samples get duplicated to both channels. The host is responsible for
//! providing a stereo output port (the plugin declares stereo only).

use crate::instrument::Instrument;
use std::sync::Arc;

/// Default per-voice release time in seconds. SFZ files can override
/// per-region eventually; for the POC every voice uses this.
const RELEASE_SECONDS: f32 = 0.150;

/// Maximum polyphony. 32 is plenty for a piano-like instrument and
/// keeps memory predictable.
pub const POLYPHONY: usize = 32;

/// One playing note.
#[derive(Copy, Clone)]
struct Voice {
    /// Index into the instrument's `regions`. `usize::MAX` = inactive.
    region_index: usize,
    /// MIDI key — used for matching note-offs.
    key: u8,
    /// Channel — also matched for note-offs.
    channel: u8,
    /// Fractional read position in the source sample's frames.
    position: f32,
    /// Per-source-frame increment driven by pitch ratio.
    increment: f32,
    /// Linear amplitude after velocity + volume_db. Held constant per voice.
    gain: f32,
    /// Envelope: 1.0 while held, linearly ramps to 0 when releasing.
    env: f32,
    /// Per-sample env decrement once releasing. 0 while held.
    env_decrement: f32,
    /// Monotonic allocation counter for stealing the oldest active voice.
    age: u64,
    /// One of [`VoiceState`].
    state: VoiceState,
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum VoiceState {
    Idle,
    Playing,
    Releasing,
}

impl Default for Voice {
    fn default() -> Self {
        Self {
            region_index: usize::MAX,
            key: 0,
            channel: 0,
            position: 0.0,
            increment: 1.0,
            gain: 0.0,
            env: 0.0,
            env_decrement: 0.0,
            age: 0,
            state: VoiceState::Idle,
        }
    }
}

/// The sample-playback engine. Constructed once per audio session; never
/// reallocates.
pub struct Engine {
    instrument: Arc<Instrument>,
    voices: [Voice; POLYPHONY],
    sample_rate: f32,
    age_counter: u64,
    release_decrement: f32,
}

impl Engine {
    /// Construct an engine for an instrument + output sample rate.
    pub fn new(instrument: Arc<Instrument>, sample_rate: f32) -> Self {
        Self {
            instrument,
            voices: [Voice::default(); POLYPHONY],
            sample_rate,
            age_counter: 0,
            release_decrement: 1.0 / (RELEASE_SECONDS * sample_rate),
        }
    }

    /// Number of voices currently producing sound.
    pub fn active_voice_count(&self) -> usize {
        self.voices.iter().filter(|v| v.state != VoiceState::Idle).count()
    }

    /// True when no voice is producing audio — host can sleep the
    /// processor in that case.
    pub fn is_idle(&self) -> bool {
        self.active_voice_count() == 0
    }

    /// Start a note. Picks the best-matching region; silently drops the
    /// event if no region claims this (key, velocity) pair.
    pub fn note_on(&mut self, channel: u8, key: u8, velocity: u8) {
        let Some((region_index, region)) = self
            .instrument
            .regions
            .iter()
            .enumerate()
            .rev() // last-match-wins for overlapping regions, per SFZ convention
            .find(|(_, r)| r.matches(key, velocity))
        else {
            return;
        };

        let sample = &self.instrument.samples[region.sample_index];
        // Pitch ratio = source_rate / dest_rate * 2^((key - center) / 12).
        // The first factor compensates for sample-rate mismatch; the
        // second is the musical pitch shift.
        let sr_ratio = sample.sample_rate as f32 / self.sample_rate;
        let pitch = (key as f32 - region.pitch_keycenter as f32) / 12.0;
        let increment = sr_ratio * 2.0f32.powf(pitch);

        // Velocity (0-127, linear) → gain. Multiply in volume_db.
        let vel_gain = velocity as f32 / 127.0;
        let db_gain = 10.0f32.powf(region.volume_db / 20.0);
        let gain = vel_gain * db_gain;

        let idx = self.pick_voice();
        self.age_counter = self.age_counter.wrapping_add(1);
        self.voices[idx] = Voice {
            region_index,
            key,
            channel,
            position: 0.0,
            increment,
            gain,
            env: 1.0,
            env_decrement: 0.0,
            age: self.age_counter,
            state: VoiceState::Playing,
        };
    }

    /// Release any voice currently playing the given (channel, key).
    pub fn note_off(&mut self, channel: u8, key: u8) {
        for v in &mut self.voices {
            if v.state == VoiceState::Playing && v.key == key && v.channel == channel {
                v.state = VoiceState::Releasing;
                v.env_decrement = self.release_decrement;
            }
        }
    }

    /// Force every voice silent immediately — host calls this on panic,
    /// patch change, or stop_processing.
    pub fn all_notes_off(&mut self) {
        for v in &mut self.voices {
            v.state = VoiceState::Idle;
            v.region_index = usize::MAX;
        }
    }

    /// Render `frames` interleaved stereo samples into `out` (length
    /// must be `frames * 2`). Caller is expected to zero the buffer
    /// first; we ADD into it so multiple engines could share an output.
    pub fn render_into_stereo(&mut self, out: &mut [f32]) {
        let frames = out.len() / 2;
        for f in 0..frames {
            let mut left = 0.0f32;
            let mut right = 0.0f32;
            for v in &mut self.voices {
                if v.state == VoiceState::Idle {
                    continue;
                }
                if v.region_index == usize::MAX {
                    v.state = VoiceState::Idle;
                    continue;
                }
                let region = &self.instrument.regions[v.region_index];
                let sample = &self.instrument.samples[region.sample_index];

                let total_frames = sample.frames();
                if v.position as usize + 1 >= total_frames {
                    // Sample ran out — silence and free the voice.
                    v.state = VoiceState::Idle;
                    continue;
                }

                let idx = v.position as usize;
                let frac = v.position - idx as f32;
                let (l0, r0) = sample.frame(idx);
                let (l1, r1) = sample.frame(idx + 1);
                let l = l0 + (l1 - l0) * frac;
                let r = r0 + (r1 - r0) * frac;
                let amp = v.gain * v.env;
                left += l * amp;
                right += r * amp;

                v.position += v.increment;

                if v.state == VoiceState::Releasing {
                    v.env -= v.env_decrement;
                    if v.env <= 0.0 {
                        v.state = VoiceState::Idle;
                    }
                }
            }
            out[f * 2] += left;
            out[f * 2 + 1] += right;
        }
    }

    fn pick_voice(&self) -> usize {
        if let Some((i, _)) = self
            .voices
            .iter()
            .enumerate()
            .find(|(_, v)| v.state == VoiceState::Idle)
        {
            return i;
        }
        // Steal the releasing voice with the lowest envelope (closest
        // to silence), else the oldest playing voice.
        if let Some((i, _)) = self
            .voices
            .iter()
            .enumerate()
            .filter(|(_, v)| v.state == VoiceState::Releasing)
            .min_by(|a, b| a.1.env.partial_cmp(&b.1.env).unwrap())
        {
            return i;
        }
        self.voices
            .iter()
            .enumerate()
            .min_by_key(|(_, v)| v.age)
            .map(|(i, _)| i)
            .unwrap_or(0)
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instrument::{InstrumentRegion, build_instrument};
    use crate::sample::Sample;
    use crate::sfz::Region;
    use std::path::PathBuf;

    fn ramp_sample(rate: u32, frames: usize) -> Sample {
        // 0.0 → 1.0 linear ramp, mono — predictable for assertions.
        let data: Vec<f32> = (0..frames).map(|i| i as f32 / frames as f32).collect();
        Sample {
            data,
            channels: 1,
            sample_rate: rate,
        }
    }

    fn instrument_with_one_region() -> Arc<Instrument> {
        // Build an instrument directly, bypassing the SFZ parser/loader,
        // so we can assert on engine behaviour in isolation.
        let sample = ramp_sample(48_000, 4_800); // 100ms of ramp
        let region = InstrumentRegion {
            region: Region {
                sample: PathBuf::from("ramp"),
                lokey: 60,
                hikey: 60,
                pitch_keycenter: 60,
                lovel: 0,
                hivel: 127,
                volume_db: 0.0,
            },
            sample_index: 0,
        };
        Arc::new(Instrument {
            regions: vec![region],
            samples: vec![sample],
        })
    }

    #[test]
    fn note_on_off_lifecycle() {
        let mut e = Engine::new(instrument_with_one_region(), 48_000.0);
        assert_eq!(e.active_voice_count(), 0);
        e.note_on(0, 60, 100);
        assert_eq!(e.active_voice_count(), 1);
        e.note_off(0, 60);
        // Voice is now Releasing — still active.
        assert_eq!(e.active_voice_count(), 1);
        // Render past release.
        let mut buf = vec![0.0f32; 48_000];
        e.render_into_stereo(&mut buf);
        assert_eq!(e.active_voice_count(), 0);
    }

    #[test]
    fn render_produces_non_silent_output_for_held_note() {
        let mut e = Engine::new(instrument_with_one_region(), 48_000.0);
        e.note_on(0, 60, 127);
        let mut buf = vec![0.0f32; 2_000 * 2];
        e.render_into_stereo(&mut buf);
        let peak = buf.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
        assert!(peak > 0.05, "expected audible output, got peak {peak}");
    }

    #[test]
    fn note_outside_region_range_is_silent() {
        let mut e = Engine::new(instrument_with_one_region(), 48_000.0);
        e.note_on(0, 72, 100); // outside [60, 60]
        assert_eq!(e.active_voice_count(), 0);
    }

    #[test]
    fn all_notes_off_silences_everything() {
        let mut e = Engine::new(instrument_with_one_region(), 48_000.0);
        e.note_on(0, 60, 100);
        e.all_notes_off();
        assert_eq!(e.active_voice_count(), 0);
    }

    #[test]
    fn note_higher_than_keycenter_pitches_up() {
        // Building two engines is easier than poking voice internals.
        let inst = instrument_with_one_region();
        let mut a = Engine::new(inst.clone(), 48_000.0);
        let mut b = Engine::new(inst.clone(), 48_000.0);
        a.note_on(0, 60, 127); // at keycenter
        b.note_on(0, 72, 127); // one octave up = 2x rate
        // After N output samples, b should have advanced ~2x as far
        // through the source. Render both and check the first sample
        // (both start at position 0 so first sample identical; render
        // for a bit and compare position via second-window energy).
        let mut buf_a = vec![0.0f32; 200 * 2];
        let mut buf_b = vec![0.0f32; 200 * 2];
        a.render_into_stereo(&mut buf_a);
        b.render_into_stereo(&mut buf_b);
        // The ramp goes 0 → 1; the higher-pitched note advances through
        // the ramp faster, so its last sample should be larger than a's.
        let a_last = buf_a[buf_a.len() - 2];
        let b_last = buf_b[buf_b.len() - 2];
        assert!(
            b_last > a_last * 1.5,
            "expected pitched-up render to be further along: a={a_last} b={b_last}"
        );
    }
}
