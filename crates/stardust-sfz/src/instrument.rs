//! Build a loaded, ready-to-play instrument from a parsed `.sfz` file.
//!
//! Multiple regions can share the same sample (different velocity layers
//! of the same note, for example). The loader deduplicates by path so
//! the audio engine ends up with `regions.len() <= samples.len()` and
//! every region carries an index into the sample bank.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::sample::{Sample, SampleError};
use crate::sfz::{Region, SfzFile};

/// A region paired with an index into [`Instrument::samples`].
#[derive(Debug, Clone)]
pub struct InstrumentRegion {
    /// The parsed region (key/velocity range, pitch, volume).
    pub region: Region,
    /// Index of this region's sample in the bank.
    pub sample_index: usize,
}

/// A loaded instrument — regions + the unique sample bank they reference.
#[derive(Debug, Clone)]
pub struct Instrument {
    /// Regions in original SFZ order.
    pub regions: Vec<InstrumentRegion>,
    /// Unique samples referenced by at least one region.
    pub samples: Vec<Sample>,
}

/// Aggregate error: which files we couldn't load and why. Loading
/// continues past individual failures so a missing or corrupt sample
/// doesn't make the whole instrument unusable.
#[derive(Debug, Default)]
pub struct LoadReport {
    /// Successfully loaded instrument.
    pub instrument: Instrument,
    /// Samples we tried to load but couldn't, paired with the reason.
    /// Regions that referenced these samples are silently dropped from
    /// the instrument.
    pub errors: Vec<(PathBuf, String)>,
}

/// Read an `.sfz` file from disk, parse it, and load every sample it
/// references. Returns a [`LoadReport`] — never fails outright at the
/// `LoadReport` level; per-sample failures are collected and the
/// instrument is built from whatever did load.
pub fn load_sfz(path: &Path) -> Result<LoadReport, std::io::Error> {
    let text = std::fs::read_to_string(path)?;
    let base = path
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));
    let parsed = SfzFile::parse(&text, &base);
    Ok(build_load_report(parsed))
}

/// Pure-data variant of [`load_sfz`] that takes an already-parsed file.
/// Useful in tests and when the SFZ text comes from a non-file source.
pub fn build_load_report(parsed: SfzFile) -> LoadReport {
    let mut report = LoadReport::default();
    let mut path_to_index: HashMap<PathBuf, usize> = HashMap::new();
    for region in parsed.regions {
        let entry = path_to_index.get(&region.sample).copied();
        let sample_index = match entry {
            Some(i) => i,
            None => match Sample::load_wav(&region.sample) {
                Ok(sample) => {
                    let i = report.instrument.samples.len();
                    report.instrument.samples.push(sample);
                    path_to_index.insert(region.sample.clone(), i);
                    i
                }
                Err(e) => {
                    report.errors.push((region.sample.clone(), format!("{e}")));
                    continue;
                }
            },
        };
        report
            .instrument
            .regions
            .push(InstrumentRegion { region, sample_index });
    }
    report
}

/// Convenience helper: build an [`Instrument`] without reporting errors.
/// Use in tests.
pub fn build_instrument(parsed: SfzFile) -> Instrument {
    build_load_report(parsed).instrument
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_sample_files_collect_as_errors() {
        let sfz = SfzFile::parse(
            "
            <region> sample=does_not_exist.wav pitch_keycenter=60
            <region> sample=also_missing.wav pitch_keycenter=72
            ",
            Path::new("/tmp/nope"),
        );
        let r = build_load_report(sfz);
        assert_eq!(r.errors.len(), 2);
        assert_eq!(r.instrument.regions.len(), 0);
        assert_eq!(r.instrument.samples.len(), 0);
    }
}
