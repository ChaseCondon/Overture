//! Minimal SFZ format parser.
//!
//! Only the subset Stardust needs for first-pass sample playback:
//!
//! - `<region>` headers (groups, globals, and other section headers are
//!   tolerated but ignored — opcodes inside them don't inherit yet).
//! - Opcodes per region: `sample`, `lokey`, `hikey`, `pitch_keycenter`,
//!   `lovel`, `hivel`, `volume`. Anything else is silently dropped.
//! - Numeric note values (0–127) only. Note names (`c4`, `f#3`) come
//!   later — easy to add but not required for the POC instruments we're
//!   testing against.
//! - `//` line comments. SFZ also has `/* */` block comments which a
//!   real engine should handle; rare enough in practice that we skip.
//!
//! The parser is intentionally forgiving — unknown opcodes don't error,
//! malformed values fall back to defaults. The reasoning is that we
//! want to load real-world SFZ files even when they use opcodes we
//! don't understand, and just produce sound for the regions we do.

use std::path::{Path, PathBuf};

/// A single `<region>` block from an SFZ file.
#[derive(Debug, Clone, PartialEq)]
pub struct Region {
    /// Resolved absolute path to the sample file. The SFZ `sample=`
    /// opcode is relative to the .sfz file's directory; the parser
    /// joins it with the file's parent here.
    pub sample: PathBuf,
    /// Lowest MIDI note this region responds to (inclusive).
    pub lokey: u8,
    /// Highest MIDI note this region responds to (inclusive).
    pub hikey: u8,
    /// MIDI note number whose pitch the sample was recorded at. Other
    /// notes are pitch-shifted relative to this.
    pub pitch_keycenter: u8,
    /// Lowest MIDI velocity this region responds to (inclusive).
    pub lovel: u8,
    /// Highest MIDI velocity this region responds to (inclusive).
    pub hivel: u8,
    /// Volume in decibels relative to unity. 0.0 == unchanged.
    pub volume_db: f32,
}

impl Default for Region {
    fn default() -> Self {
        Self {
            sample: PathBuf::new(),
            lokey: 0,
            hikey: 127,
            pitch_keycenter: 60,
            lovel: 0,
            hivel: 127,
            volume_db: 0.0,
        }
    }
}

impl Region {
    /// True if this region should sound for the given (key, velocity).
    pub fn matches(&self, key: u8, velocity: u8) -> bool {
        key >= self.lokey
            && key <= self.hikey
            && velocity >= self.lovel
            && velocity <= self.hivel
    }
}

/// A parsed SFZ instrument — just the regions, for now.
#[derive(Debug, Clone, Default)]
pub struct SfzFile {
    /// Every `<region>` block in declaration order. Order is preserved
    /// because some SFZ files rely on last-region-wins resolution for
    /// overlapping ranges.
    pub regions: Vec<Region>,
}

impl SfzFile {
    /// Parse an SFZ string. `base_dir` is the directory `sample=` paths
    /// resolve against (typically the directory the .sfz file lives in).
    pub fn parse(text: &str, base_dir: &Path) -> Self {
        let mut file = SfzFile::default();
        let mut current: Option<Region> = None;
        let mut in_region = false;

        for raw_line in text.lines() {
            // Strip `//` line comments before tokenising.
            let line = match raw_line.find("//") {
                Some(idx) => &raw_line[..idx],
                None => raw_line,
            };
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            // Headers can sit mid-line in real-world SFZ files (e.g.
            // "<region> sample=foo.wav"). Walk left-to-right splitting
            // on whitespace into either headers or opcodes.
            for token in split_sfz_tokens(trimmed) {
                if let Some(header) = token.strip_prefix('<').and_then(|s| s.strip_suffix('>')) {
                    // Flush whatever region was being built.
                    if let Some(r) = current.take() {
                        if !r.sample.as_os_str().is_empty() {
                            file.regions.push(r);
                        }
                    }
                    if header.eq_ignore_ascii_case("region") {
                        current = Some(Region::default());
                        in_region = true;
                    } else {
                        // Group / global / control / master / curve — not
                        // yet supported. Stop accumulating opcodes into a
                        // region until we see the next <region>.
                        in_region = false;
                    }
                    continue;
                }

                if !in_region {
                    continue;
                }
                let Some((k, v)) = token.split_once('=') else {
                    continue;
                };
                let Some(region) = current.as_mut() else {
                    continue;
                };
                apply_opcode(region, k.trim(), v.trim(), base_dir);
            }
        }

        // Final region in the file.
        if let Some(r) = current {
            if !r.sample.as_os_str().is_empty() {
                file.regions.push(r);
            }
        }

        file
    }
}

/// Apply a single opcode to a region in-place. Unknown opcodes are
/// ignored so partial-fidelity instruments still load.
fn apply_opcode(region: &mut Region, key: &str, value: &str, base_dir: &Path) {
    match key.to_ascii_lowercase().as_str() {
        "sample" => {
            // SFZ uses backslash separators even on POSIX — normalise.
            let normalised: String = value.replace('\\', "/");
            region.sample = base_dir.join(normalised);
        }
        "lokey" => {
            if let Some(n) = parse_note(value) {
                region.lokey = n;
            }
        }
        "hikey" => {
            if let Some(n) = parse_note(value) {
                region.hikey = n;
            }
        }
        "key" => {
            // `key=N` is shorthand for lokey=N hikey=N pitch_keycenter=N.
            if let Some(n) = parse_note(value) {
                region.lokey = n;
                region.hikey = n;
                region.pitch_keycenter = n;
            }
        }
        "pitch_keycenter" => {
            if let Some(n) = parse_note(value) {
                region.pitch_keycenter = n;
            }
        }
        "lovel" => {
            if let Ok(n) = value.parse::<u8>() {
                region.lovel = n.min(127);
            }
        }
        "hivel" => {
            if let Ok(n) = value.parse::<u8>() {
                region.hivel = n.min(127);
            }
        }
        "volume" => {
            if let Ok(db) = value.parse::<f32>() {
                region.volume_db = db;
            }
        }
        _ => {}
    }
}

/// Parse a note value — for now only decimal numbers (0-127). Real SFZ
/// also accepts note names like `c4` / `f#3`; left as a TODO since the
/// instruments we care about for first testing use numeric values.
fn parse_note(s: &str) -> Option<u8> {
    s.trim().parse::<u8>().ok().map(|n| n.min(127))
}

/// Tokenise an SFZ line. Tokens are whitespace-separated, but a `sample=`
/// value may itself contain spaces (e.g. `sample=Felt Piano/A4 v3.wav`).
/// SFZ's rule: a sample path runs until the next `key=` opcode token.
/// We approximate by joining tokens back together when we see one that
/// doesn't contain `=` and the previous token *was* a `sample=`-style
/// opcode.
fn split_sfz_tokens(line: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut in_value_with_spaces = false;
    for word in line.split_whitespace() {
        let is_header = word.starts_with('<') && word.ends_with('>');
        if is_header {
            in_value_with_spaces = false;
            out.push(word.to_string());
            continue;
        }
        if word.contains('=') {
            in_value_with_spaces = word.to_ascii_lowercase().starts_with("sample=");
            out.push(word.to_string());
        } else if in_value_with_spaces {
            // Append the space-containing fragment to the last token.
            if let Some(last) = out.last_mut() {
                last.push(' ');
                last.push_str(word);
            }
        } else {
            // Stray token between opcodes — just push it; apply_opcode
            // will ignore non-`key=value` tokens anyway.
            out.push(word.to_string());
        }
    }
    out
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> PathBuf {
        PathBuf::from(s)
    }

    #[test]
    fn parses_single_region() {
        let sfz = "<region> sample=piano_c4.wav pitch_keycenter=60 lokey=48 hikey=72";
        let f = SfzFile::parse(sfz, &p("/inst"));
        assert_eq!(f.regions.len(), 1);
        let r = &f.regions[0];
        assert_eq!(r.sample, p("/inst/piano_c4.wav"));
        assert_eq!(r.pitch_keycenter, 60);
        assert_eq!(r.lokey, 48);
        assert_eq!(r.hikey, 72);
    }

    #[test]
    fn key_shorthand_sets_all_three() {
        let f = SfzFile::parse("<region> sample=x.wav key=64", &p("/"));
        let r = &f.regions[0];
        assert_eq!(r.lokey, 64);
        assert_eq!(r.hikey, 64);
        assert_eq!(r.pitch_keycenter, 64);
    }

    #[test]
    fn multi_region_with_velocity_layers() {
        let sfz = "
            <region> sample=p.wav pitch_keycenter=60 lovel=0   hivel=63
            <region> sample=f.wav pitch_keycenter=60 lovel=64  hivel=127
        ";
        let f = SfzFile::parse(sfz, &p("/"));
        assert_eq!(f.regions.len(), 2);
        assert!(f.regions[0].matches(60, 30));
        assert!(!f.regions[0].matches(60, 100));
        assert!(f.regions[1].matches(60, 100));
    }

    #[test]
    fn ignores_unknown_opcodes() {
        let sfz = "<region> sample=x.wav loop_mode=loop_continuous ampeg_attack=0.01 pitch_keycenter=60";
        let f = SfzFile::parse(sfz, &p("/"));
        assert_eq!(f.regions.len(), 1);
        assert_eq!(f.regions[0].pitch_keycenter, 60);
    }

    #[test]
    fn ignores_group_sections() {
        let sfz = "
            <group> volume=-6
            <region> sample=a.wav pitch_keycenter=60
            <region> sample=b.wav pitch_keycenter=72
        ";
        let f = SfzFile::parse(sfz, &p("/"));
        // Group opcodes don't inherit (POC scope), but the regions still load.
        assert_eq!(f.regions.len(), 2);
        assert_eq!(f.regions[0].volume_db, 0.0);
    }

    #[test]
    fn line_comments_are_stripped() {
        let sfz = "
            // top comment
            <region> sample=a.wav pitch_keycenter=60  // inline trailing comment
        ";
        let f = SfzFile::parse(sfz, &p("/"));
        assert_eq!(f.regions.len(), 1);
    }

    #[test]
    fn sample_path_with_spaces() {
        let sfz = "<region> sample=Felt Piano/A4 v3.wav pitch_keycenter=69";
        let f = SfzFile::parse(sfz, &p("/inst"));
        assert_eq!(f.regions.len(), 1);
        assert_eq!(f.regions[0].sample, p("/inst/Felt Piano/A4 v3.wav"));
    }

    #[test]
    fn region_match_respects_key_and_velocity() {
        let r = Region {
            sample: p("x.wav"),
            lokey: 60,
            hikey: 72,
            pitch_keycenter: 64,
            lovel: 40,
            hivel: 120,
            volume_db: 0.0,
        };
        assert!(r.matches(64, 80));
        assert!(!r.matches(59, 80));
        assert!(!r.matches(64, 30));
        assert!(!r.matches(64, 121));
    }

    #[test]
    fn region_missing_sample_is_dropped() {
        // `<region>` with no sample= shouldn't make it into the output.
        let sfz = "<region> lokey=0 hikey=127 pitch_keycenter=60";
        let f = SfzFile::parse(sfz, &p("/"));
        assert_eq!(f.regions.len(), 0);
    }

    #[test]
    fn backslash_paths_normalise() {
        let sfz = "<region> sample=subdir\\thing.wav pitch_keycenter=60";
        let f = SfzFile::parse(sfz, &p("/root"));
        assert_eq!(f.regions[0].sample, p("/root/subdir/thing.wav"));
    }

    #[test]
    fn volume_db_parses_negative() {
        let sfz = "<region> sample=a.wav pitch_keycenter=60 volume=-6";
        let f = SfzFile::parse(sfz, &p("/"));
        assert_eq!(f.regions[0].volume_db, -6.0);
    }
}
