use crate::video_descriptor::Language;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const LINE_MARKERS_CONFIG_FILE_NAME: &str = "line-markers.toml";

/// Parsed contents of a `line-markers.toml` file.
///
/// A _marker_ is a short token (for example `LTY`, `cre`, `ttl`, `LRC`)
/// that appears at the start of a line in the song's lyric text files
/// and controls how the line is rendered into VTT. The [`markers`]
/// field holds the exhaustive inventory of markers the song uses.
/// Each marker in [`markers`] falls into one of four categories:
///
/// * Markers declared in [`voices`] wrap the line in `<v ...>...</v>`,
///   with the voice name given per language.
/// * Markers declared in [`classes`] wrap the line in
///   `<c.className>...</c>`, with the class name given as the mapped
///   value.
/// * Markers declared in [`credits`] invoke the credit-block
///   renderer. The renderer splits the line by column layout into
///   `<c.creditRole>` and `<c.creditName>` segments and validates
///   each against `credits.yaml`. Names wrapped in brackets in the
///   source become `<c.creditSpecial>` instead of `<c.creditName>`.
/// * Markers absent from [`voices`], [`classes`], and [`credits`]
///   emit the line content as plain unwrapped text.
///
/// Universal control keywords such as `clr` and `eov` produce no
/// output and are handled by the generator directly. They are not
/// represented in this struct.
///
/// # Per-line markers
///
/// A cue in a `lyrics.*.txt` file, opened by a timestamp, may
/// contain continuation lines that carry their own markers instead
/// of inheriting the marker of the line that opened the block. The
/// cue then combines renderings from different markers in a single
/// timed output. For example, a cue whose first line is a song title
/// and whose second line is an opening credit is written as:
///
/// ```text
/// 00:00:10.080 ttl: <song title>
///              cre: <credit role>  <credit name>
/// ```
///
/// The two lines appear in the same cue but are produced by
/// different renderers. A cue whose lines all share one renderer
/// should continue to omit markers on continuation lines so that
/// inheritance applies.
///
/// [`markers`]: Self::markers
/// [`voices`]: Self::voices
/// [`classes`]: Self::classes
/// [`credits`]: Self::credits
#[derive(Default, Deserialize, Serialize)]
pub struct LineMarkersDesc {
    /// Exhaustive inventory of markers used by this song. A future
    /// generator will use this list to validate the leading tokens of
    /// `lyrics.*.txt` lines, rejecting unknown markers and warning on
    /// declared markers that never appear.
    #[serde(default)]
    pub markers: Vec<String>,
    /// Markers that wrap the line in `<v ...>...</v>`. Each entry
    /// gives the voice name per language, emitted as the inner text
    /// of the `<v>` element.
    #[serde(default)]
    pub voices: BTreeMap<String, BTreeMap<Language, String>>,
    /// Markers that wrap the line in `<c.className>...</c>`. The
    /// value is the class name applied to the wrapping element.
    #[serde(default)]
    pub classes: BTreeMap<String, String>,
    /// Markers that invoke the credit-block renderer. Columns of the
    /// line become `<c.creditRole>` and `<c.creditName>` segments,
    /// each validated against `credits.yaml`. Names wrapped in
    /// brackets in the source become `<c.creditSpecial>` instead of
    /// `<c.creditName>`.
    #[serde(default)]
    pub credits: Vec<String>,
}
