use crate::video_descriptor::Language;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const LINE_MARKERS_CONFIG_FILE_NAME: &str = "line-markers.toml";

/// Parsed contents of a `line-markers.toml` file.
///
/// A _cue_ is a short token (for example `LTY`, `cre`, `ttl`, `LRC`)
/// that appears at the start of a line in the song's lyric text files
/// and controls how the line is rendered into VTT. The [`cues`] field
/// holds the exhaustive inventory of cues the song uses. Each cue in
/// [`cues`] falls into one of four categories:
///
/// * declared in [`voices`] — wraps the line in `<v ...>...</v>`,
///   with the voice name given per language;
/// * declared in [`classes`] — wraps the line in
///   `<c.className>...</c>`, with the class name given as the mapped
///   value;
/// * declared in [`credits`] — invokes the credit-block renderer,
///   which splits the line into `<c.creditRole>` and `<c.creditName>`
///   (or `<c.creditSpecial>` for bracket-wrapped entries) segments
///   based on the column layout of the source text and validates each
///   segment against `credits.yaml`;
/// * declared only in [`cues`] — emits the line content as plain
///   unwrapped text.
///
/// Universal control keywords such as `clr` and `eov` produce no
/// output and are handled by the generator directly; they are not
/// represented in this struct.
///
/// [`cues`]: Self::cues
/// [`voices`]: Self::voices
/// [`classes`]: Self::classes
/// [`credits`]: Self::credits
#[derive(Default, Deserialize, Serialize)]
pub struct LineMarkersDesc {
    /// Exhaustive inventory of cues used by this song. A future
    /// generator will use this list to validate the leading tokens of
    /// `lyrics.*.txt` lines, rejecting unknown cues and warning on
    /// declared cues that never appear.
    #[serde(default)]
    pub cues: Vec<String>,
    /// Cues that wrap the line in `<v ...>...</v>`. Each entry gives
    /// the voice name per language, emitted as the inner text of the
    /// `<v>` element.
    #[serde(default)]
    pub voices: BTreeMap<String, BTreeMap<Language, String>>,
    /// Cues that wrap the line in `<c.className>...</c>`. The value
    /// is the class name applied to the wrapping element.
    #[serde(default)]
    pub classes: BTreeMap<String, String>,
    /// Cues that invoke the credit-block renderer, which splits the
    /// line into per-column `<c.creditRole>` and `<c.creditName>` (or
    /// `<c.creditSpecial>` for bracket-wrapped entries) segments and
    /// validates each segment against `credits.yaml`.
    #[serde(default)]
    pub credits: Vec<String>,
}
