//! WAV sample loading via [`hound`].
//!
//! Samples are decoded fully into memory at load time. That's fine for
//! the POC and many real instruments (felt piano = tens of MB), and
//! keeps the audio thread allocation-free. Streaming from disk for
//! larger libraries lands in a later phase.

use std::path::{Path, PathBuf};

use hound::{SampleFormat, WavReader};
use thiserror::Error;

/// Errors returned when loading a sample.
#[derive(Debug, Error)]
pub enum SampleError {
    /// The file couldn't be opened or read.
    #[error("failed to open sample at {path}: {source}")]
    Open {
        /// Path we tried to open.
        path: PathBuf,
        /// The underlying I/O / decoder error.
        #[source]
        source: hound::Error,
    },
    /// We don't yet handle the sample's bit depth / format combination.
    #[error("unsupported sample format at {path}: {format:?} {bits} bit")]
    UnsupportedFormat {
        /// Path we tried to load.
        path: PathBuf,
        /// hound's sample format tag (PCM int / IEEE float).
        format: SampleFormat,
        /// Bits per sample reported by the WAV header.
        bits: u16,
    },
}

/// An in-memory PCM sample, normalised to interleaved f32 in `[-1, 1]`.
#[derive(Debug, Clone)]
pub struct Sample {
    /// Interleaved frames: `[L0, R0, L1, R1, ...]` for stereo,
    /// `[S0, S1, ...]` for mono.
    pub data: Vec<f32>,
    /// Number of channels (1 = mono, 2 = stereo, etc.).
    pub channels: u16,
    /// Sample rate the file was recorded at, in Hz.
    pub sample_rate: u32,
}

impl Sample {
    /// Number of frames in the sample (= `data.len() / channels`).
    pub fn frames(&self) -> usize {
        if self.channels == 0 {
            0
        } else {
            self.data.len() / self.channels as usize
        }
    }

    /// Load a sample from a WAV file on disk.
    ///
    /// Supports the common pairs: PCM 16-bit / 24-bit / 32-bit signed
    /// integer, and IEEE 32-bit float. Anything else returns
    /// [`SampleError::UnsupportedFormat`].
    pub fn load_wav(path: &Path) -> Result<Self, SampleError> {
        let mut reader = WavReader::open(path).map_err(|e| SampleError::Open {
            path: path.to_path_buf(),
            source: e,
        })?;
        let spec = reader.spec();

        let data: Vec<f32> = match (spec.sample_format, spec.bits_per_sample) {
            (SampleFormat::Int, 16) => reader
                .samples::<i16>()
                .map(|s| s.map(|v| v as f32 / i16::MAX as f32))
                .collect::<Result<_, _>>()
                .map_err(|e| SampleError::Open {
                    path: path.to_path_buf(),
                    source: e,
                })?,
            (SampleFormat::Int, 24) => reader
                .samples::<i32>()
                // 24-bit WAV stores in i32 left-aligned; divide by 2^23.
                .map(|s| s.map(|v| v as f32 / 8_388_608.0))
                .collect::<Result<_, _>>()
                .map_err(|e| SampleError::Open {
                    path: path.to_path_buf(),
                    source: e,
                })?,
            (SampleFormat::Int, 32) => reader
                .samples::<i32>()
                .map(|s| s.map(|v| v as f32 / i32::MAX as f32))
                .collect::<Result<_, _>>()
                .map_err(|e| SampleError::Open {
                    path: path.to_path_buf(),
                    source: e,
                })?,
            (SampleFormat::Float, 32) => reader
                .samples::<f32>()
                .collect::<Result<_, _>>()
                .map_err(|e| SampleError::Open {
                    path: path.to_path_buf(),
                    source: e,
                })?,
            (format, bits) => {
                return Err(SampleError::UnsupportedFormat {
                    path: path.to_path_buf(),
                    format,
                    bits,
                });
            }
        };

        Ok(Self {
            data,
            channels: spec.channels,
            sample_rate: spec.sample_rate,
        })
    }

    /// Read one frame at integer sample index `i`, returning (L, R).
    /// Mono samples duplicate to both channels. Out-of-range returns
    /// silence so the caller doesn't have to bounds-check every read.
    #[inline]
    pub fn frame(&self, i: usize) -> (f32, f32) {
        if i >= self.frames() {
            return (0.0, 0.0);
        }
        match self.channels {
            1 => {
                let s = self.data[i];
                (s, s)
            }
            // For >=2 channels read the first two as left/right and
            // ignore extras. Surround panning is out of scope.
            _ => {
                let base = i * self.channels as usize;
                (self.data[base], self.data[base + 1])
            }
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mono_frame_duplicates_to_both_channels() {
        let s = Sample {
            data: vec![0.5, -0.25, 0.1],
            channels: 1,
            sample_rate: 48_000,
        };
        assert_eq!(s.frame(0), (0.5, 0.5));
        assert_eq!(s.frame(2), (0.1, 0.1));
    }

    #[test]
    fn stereo_frame_returns_l_r_pair() {
        let s = Sample {
            data: vec![0.1, 0.2, 0.3, 0.4],
            channels: 2,
            sample_rate: 48_000,
        };
        assert_eq!(s.frame(0), (0.1, 0.2));
        assert_eq!(s.frame(1), (0.3, 0.4));
    }

    #[test]
    fn out_of_range_frame_is_silent() {
        let s = Sample {
            data: vec![0.5],
            channels: 1,
            sample_rate: 48_000,
        };
        assert_eq!(s.frame(99), (0.0, 0.0));
    }

    #[test]
    fn frames_count_uses_channel_count() {
        let s = Sample {
            data: vec![0.0; 1024],
            channels: 2,
            sample_rate: 44_100,
        };
        assert_eq!(s.frames(), 512);
    }
}
