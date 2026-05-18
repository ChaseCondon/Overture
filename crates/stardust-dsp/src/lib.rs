//! # stardust-dsp
//!
//! Audio-thread DSP primitives for stardust-core. POC scope: a polyphonic
//! sine synth with linear ADSR so we have *something* to verify the full
//! MIDI → engine → audio pipeline before plugin hosting lands.
//!
//! ## RT-safety
//!
//! Everything here is allocation-free after construction:
//!
//! - [`Synth::new`] pre-allocates the voice pool.
//! - [`Synth::note_on`] / [`Synth::note_off`] / [`Synth::all_notes_off`]
//!   mutate fixed-size state. No locks, no allocs, no syscalls.
//! - [`Synth::render`] writes into a caller-supplied interleaved `f32`
//!   buffer. No allocs.
//!
//! The synth is **queue-agnostic** — the integration layer drains a
//! lock-free SPSC (see `stardust-rt`) and feeds events in via `note_on`
//! / `note_off`. That keeps the synth easy to test standalone and lets
//! us swap it for a plugin in later phases without touching the bridge.

#![doc(html_root_url = "https://docs.rs/stardust-dsp/0.0.1")]
#![warn(missing_docs)]

use std::f32::consts::TAU;
use stardust_midi::MidiMessage;

/// Master output gain. -12 dB leaves headroom when many voices stack.
const MASTER_GAIN: f32 = 0.25;

/// Default ADSR shape — short attack, modest decay, mid sustain, longer
/// release. Tunable per-patch eventually; constant for POC.
const ATTACK_S: f32 = 0.005;
const DECAY_S: f32 = 0.050;
const SUSTAIN_LEVEL: f32 = 0.7;
const RELEASE_S: f32 = 0.200;

/// Convert a MIDI note number to frequency in Hz (A4 = 440 Hz = note 69).
#[inline]
fn note_hz(note: u8) -> f32 {
    440.0 * 2.0f32.powf((note as f32 - 69.0) / 12.0)
}

/// Polyphonic sine synth.
///
/// Constructed with a fixed polyphony cap; allocates that many voices once
/// and never resizes. Voice stealing is "release first, then oldest" — if
/// the pool is exhausted and a new note arrives, the voice closest to
/// silence loses its slot.
pub struct Synth {
    sample_rate: f32,
    voices: Vec<Voice>,
    /// Monotonic counter to time-order voice allocations for stealing.
    age_counter: u64,
    /// Per-sample envelope rates, recomputed at construction.
    attack_per_sample: f32,
    decay_per_sample: f32,
    release_per_sample: f32,
}

impl Synth {
    /// Create a synth with `polyphony` simultaneous voices at the given
    /// sample rate. Pre-allocates all voice state.
    pub fn new(sample_rate: f32, polyphony: usize) -> Self {
        debug_assert!(sample_rate > 0.0);
        debug_assert!(polyphony > 0);
        let voices = (0..polyphony).map(|_| Voice::default()).collect();
        Self {
            sample_rate,
            voices,
            age_counter: 0,
            attack_per_sample: 1.0 / (sample_rate * ATTACK_S),
            decay_per_sample: (1.0 - SUSTAIN_LEVEL) / (sample_rate * DECAY_S),
            release_per_sample: SUSTAIN_LEVEL / (sample_rate * RELEASE_S),
        }
    }

    /// Convenience: dispatch a parsed [`MidiMessage`]. CCs and pitch bend
    /// are ignored in this POC engine.
    pub fn process_midi(&mut self, message: MidiMessage) {
        match message {
            MidiMessage::NoteOn { note, velocity, .. } => self.note_on(note, velocity),
            MidiMessage::NoteOff { note, .. } => self.note_off(note),
            _ => {}
        }
    }

    /// Start a note. Allocates the next-best voice (idle preferred, else
    /// stealing the most-released or oldest).
    pub fn note_on(&mut self, note: u8, velocity: u8) {
        self.age_counter = self.age_counter.wrapping_add(1);
        let age = self.age_counter;
        let idx = self.pick_voice_for_steal();
        self.voices[idx].start(note, velocity, age, self.sample_rate);
    }

    /// Release any voice currently playing this note. No-op if none.
    pub fn note_off(&mut self, note: u8) {
        for v in &mut self.voices {
            if v.note == Some(note) && v.state != EnvState::Released {
                v.release();
            }
        }
    }

    /// Cut every voice instantly. Useful for panic / patch change.
    pub fn all_notes_off(&mut self) {
        for v in &mut self.voices {
            v.reset();
        }
    }

    /// Render `buf` (interleaved f32, `channels` channels per frame).
    /// Existing buffer contents are overwritten.
    pub fn render(&mut self, buf: &mut [f32], channels: usize) {
        debug_assert!(channels > 0);
        for frame in buf.chunks_exact_mut(channels) {
            let mut mix = 0.0f32;
            for v in &mut self.voices {
                mix += v.tick(
                    self.sample_rate,
                    self.attack_per_sample,
                    self.decay_per_sample,
                    self.release_per_sample,
                );
            }
            let sample = mix * MASTER_GAIN;
            for ch in frame.iter_mut() {
                *ch = sample;
            }
        }
    }

    /// Returns the number of voices currently producing sound.
    pub fn active_voice_count(&self) -> usize {
        self.voices.iter().filter(|v| v.state != EnvState::Idle).count()
    }

    fn pick_voice_for_steal(&self) -> usize {
        // 1) Idle voice if any.
        if let Some((i, _)) = self
            .voices
            .iter()
            .enumerate()
            .find(|(_, v)| v.state == EnvState::Idle)
        {
            return i;
        }
        // 2) Released voice with lowest envelope (closest to silence).
        if let Some((i, _)) = self
            .voices
            .iter()
            .enumerate()
            .filter(|(_, v)| v.state == EnvState::Released)
            .min_by(|a, b| a.1.env_level.partial_cmp(&b.1.env_level).unwrap())
        {
            return i;
        }
        // 3) Otherwise oldest active voice (smallest age wins).
        self.voices
            .iter()
            .enumerate()
            .min_by_key(|(_, v)| v.age)
            .map(|(i, _)| i)
            .unwrap_or(0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EnvState {
    Idle,
    Attack,
    Decay,
    Sustain,
    Released,
}

#[derive(Clone)]
struct Voice {
    note: Option<u8>,
    /// Allocation order — older voices are stolen first when pool is full.
    age: u64,
    phase: f32,
    /// Phase increment per sample for the current note.
    phase_inc: f32,
    velocity: f32, // 0.0..=1.0
    state: EnvState,
    env_level: f32,
}

impl Default for Voice {
    fn default() -> Self {
        Self {
            note: None,
            age: 0,
            phase: 0.0,
            phase_inc: 0.0,
            velocity: 0.0,
            state: EnvState::Idle,
            env_level: 0.0,
        }
    }
}

impl Voice {
    fn start(&mut self, note: u8, velocity: u8, age: u64, sample_rate: f32) {
        self.note = Some(note);
        self.age = age;
        self.phase = 0.0;
        self.phase_inc = note_hz(note) * TAU / sample_rate;
        self.velocity = (velocity as f32 / 127.0).clamp(0.0, 1.0);
        self.state = EnvState::Attack;
        // Don't reset env_level — re-triggering an already-playing voice
        // starts the new attack from wherever the envelope was, which
        // avoids clicks.
    }

    fn release(&mut self) {
        if self.state != EnvState::Idle && self.state != EnvState::Released {
            self.state = EnvState::Released;
        }
    }

    fn reset(&mut self) {
        self.note = None;
        self.phase = 0.0;
        self.phase_inc = 0.0;
        self.velocity = 0.0;
        self.state = EnvState::Idle;
        self.env_level = 0.0;
    }

    #[inline]
    fn tick(
        &mut self,
        _sample_rate: f32,
        attack_inc: f32,
        decay_inc: f32,
        release_inc: f32,
    ) -> f32 {
        // Advance envelope.
        match self.state {
            EnvState::Idle => return 0.0,
            EnvState::Attack => {
                self.env_level += attack_inc;
                if self.env_level >= 1.0 {
                    self.env_level = 1.0;
                    self.state = EnvState::Decay;
                }
            }
            EnvState::Decay => {
                self.env_level -= decay_inc;
                if self.env_level <= SUSTAIN_LEVEL {
                    self.env_level = SUSTAIN_LEVEL;
                    self.state = EnvState::Sustain;
                }
            }
            EnvState::Sustain => {
                // Hold.
            }
            EnvState::Released => {
                self.env_level -= release_inc;
                if self.env_level <= 0.0 {
                    self.reset();
                    return 0.0;
                }
            }
        }

        // Oscillator.
        let sample = self.phase.sin() * self.env_level * self.velocity;
        self.phase += self.phase_inc;
        if self.phase >= TAU {
            self.phase -= TAU;
        }
        sample
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn new_synth() -> Synth {
        Synth::new(48_000.0, 8)
    }

    #[test]
    fn new_creates_polyphony_voices() {
        let s = new_synth();
        assert_eq!(s.voices.len(), 8);
        assert_eq!(s.active_voice_count(), 0);
    }

    #[test]
    fn note_on_activates_one_voice() {
        let mut s = new_synth();
        s.note_on(60, 100);
        assert_eq!(s.active_voice_count(), 1);
    }

    #[test]
    fn note_off_starts_release() {
        let mut s = new_synth();
        s.note_on(60, 100);
        // Render briefly so attack ramps up.
        let mut buf = vec![0.0f32; 256 * 2];
        s.render(&mut buf, 2);
        assert!(s.voices.iter().any(|v| v.state != EnvState::Idle));

        s.note_off(60);
        assert!(s.voices.iter().any(|v| v.state == EnvState::Released));
    }

    #[test]
    fn render_produces_non_silent_output_when_playing() {
        let mut s = new_synth();
        s.note_on(69, 127); // A4 fortissimo
        let mut buf = vec![0.0f32; 4096 * 2];
        s.render(&mut buf, 2);
        let peak = buf.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
        assert!(peak > 0.05, "expected audible output, got peak {peak}");
    }

    #[test]
    fn render_falls_silent_after_release_completes() {
        let mut s = new_synth();
        s.note_on(69, 127);
        // Render enough to pass attack+decay
        let mut buf = vec![0.0f32; 4800 * 2]; // 100ms
        s.render(&mut buf, 2);
        s.note_off(69);
        // Render past full release (200ms + a bit of slack)
        let mut buf2 = vec![0.0f32; 48_000 * 2 / 2]; // 500ms
        s.render(&mut buf2, 2);
        assert_eq!(s.active_voice_count(), 0);
        // Tail samples should be silent.
        let tail_peak = buf2[buf2.len() - 64..]
            .iter()
            .map(|s| s.abs())
            .fold(0.0f32, f32::max);
        assert!(tail_peak < 1e-6, "expected silence after release, got {tail_peak}");
    }

    #[test]
    fn all_notes_off_silences_immediately() {
        let mut s = new_synth();
        s.note_on(60, 100);
        s.note_on(64, 100);
        s.note_on(67, 100);
        assert_eq!(s.active_voice_count(), 3);
        s.all_notes_off();
        assert_eq!(s.active_voice_count(), 0);
    }

    #[test]
    fn polyphony_cap_steals_voice() {
        let mut s = Synth::new(48_000.0, 2);
        s.note_on(60, 100);
        s.note_on(62, 100);
        assert_eq!(s.active_voice_count(), 2);
        s.note_on(64, 100); // pool full — must steal
        assert_eq!(s.active_voice_count(), 2);
        // The newest note (64) must be present.
        assert!(s.voices.iter().any(|v| v.note == Some(64)));
    }

    #[test]
    fn process_midi_dispatches_note_on_off() {
        let mut s = new_synth();
        s.process_midi(MidiMessage::NoteOn { channel: 0, note: 60, velocity: 100 });
        assert_eq!(s.active_voice_count(), 1);
        s.process_midi(MidiMessage::NoteOff { channel: 0, note: 60, velocity: 0 });
        assert!(s.voices.iter().any(|v| v.state == EnvState::Released));
    }

    #[test]
    fn process_midi_ignores_cc_and_pitch_bend() {
        let mut s = new_synth();
        s.process_midi(MidiMessage::ControlChange { channel: 0, cc: 1, value: 64 });
        s.process_midi(MidiMessage::PitchBend { channel: 0, value: 1000 });
        assert_eq!(s.active_voice_count(), 0);
    }

    #[test]
    fn note_hz_a4_is_440() {
        assert!((note_hz(69) - 440.0).abs() < 0.001);
    }

    #[test]
    fn note_hz_one_octave_doubles() {
        let a4 = note_hz(69);
        let a5 = note_hz(81);
        assert!((a5 - 2.0 * a4).abs() < 0.001);
    }
}
