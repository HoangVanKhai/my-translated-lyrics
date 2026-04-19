use crate::video_descriptor::Language;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const LINE_MARKERS_CONFIG_FILE_NAME: &str = "line-markers.toml";

/// Parsed contents of a `line-markers.toml` file.
///
/// A _prefix code_ is a short token written at the start of a line in
/// the song's lyric text files (for example `LTY`, `cre`, `ttl`).
/// Universal control markers such as `clr` and `eov` produce no cue
/// output and are handled as keywords by the generator; they are not
/// represented here. Every other prefix code the song uses is listed
/// in [`cues`]; codes that additionally map to a styled element in the
/// generated VTT output also appear in [`voices`] or [`classes`].
///
/// [`cues`]: Self::cues
/// [`voices`]: Self::voices
/// [`classes`]: Self::classes
#[derive(Default, Deserialize, Serialize)]
pub struct LineMarkersDesc {
    /// Exhaustive inventory of cue-producing prefix codes used by this
    /// song. A future generator will reject lines in `lyrics.*.txt`
    /// whose prefix is not in this list, and will warn on entries
    /// declared here that never appear in the `.txt` files.
    #[serde(default)]
    pub cues: Vec<String>,
    /// Prefix codes that map to `<v ...>...</v>` voice elements in the
    /// generated VTT. Each entry records the voice name per language,
    /// emitted as the inner text of the `<v>` element.
    #[serde(default)]
    pub voices: BTreeMap<String, BTreeMap<Language, String>>,
    /// Prefix codes that map to `<c.className>...</c>` wrapping the whole
    /// line. The value is the class name applied to the wrapping
    /// element.
    ///
    /// Markers such as `cre`, whose output decomposes into multiple
    /// inner classes (`creditRole`, `creditName`, `creditSpecial`)
    /// driven by the column layout and the bracketing convention in the
    /// source text, are not represented here. Their behavior is
    /// inferred from `credits.yaml` and the source line itself.
    #[serde(default)]
    pub classes: BTreeMap<String, String>,
}
