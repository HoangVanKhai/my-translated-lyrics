use crate::video_descriptor::Language;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const LINE_MARKERS_CONFIG_FILE_NAME: &str = "line-markers.toml";

/// Parsed contents of a `line-markers.toml` file.
///
/// Each source directory may contain a `line-markers.toml` file that
/// declares the short prefix codes appearing at the start of lines in the
/// song's lyric text files.
#[derive(Default, Deserialize, Serialize)]
pub struct LineMarkersDesc {
    /// Every cue-producing prefix code this song uses. Intended for a
    /// future generator to validate that `.txt` lines carry no unknown
    /// prefixes and that no entry here is unused. Universal control
    /// markers such as `clr` and `eov` are excluded because they produce
    /// no cue output.
    #[serde(default)]
    pub cues: Vec<String>,
    /// Per-code voice attribution. Each entry maps a prefix code to the
    /// voice-element inner text rendered as `<v …>…</v>` in VTT cues.
    #[serde(default)]
    pub voices: BTreeMap<String, BTreeMap<Language, String>>,
    /// Per-code class-name mapping for prefix codes that wrap the whole
    /// line as `<c.className>…</c>`. Markers like `cre` whose output
    /// classes (`creditRole`, `creditName`, `creditSpecial`) wrap only
    /// individual columns are not represented here; their behavior is
    /// inferred from `credits.yaml` and the bracketing convention in the
    /// source text.
    #[serde(default)]
    pub classes: BTreeMap<String, String>,
}
