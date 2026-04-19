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
/// * Cues declared in [`voices`] wrap the line in `<v ...>...</v>`,
///   with the voice name given per language.
/// * Cues declared in [`classes`] wrap the line in
///   `<c.className>...</c>`, with the class name given as the mapped
///   value.
/// * Cues declared in [`credits`] invoke the credit-block renderer.
///   The renderer splits the line by column layout into
///   `<c.creditRole>` and `<c.creditName>` segments and validates
///   each against `credits.yaml`. Names wrapped in brackets in the
///   source become `<c.creditSpecial>` instead of `<c.creditName>`.
/// * Cues declared only in [`cues`] emit the line content as plain
///   unwrapped text.
///
/// Universal control keywords such as `clr` and `eov` produce no
/// output and are handled by the generator directly. They are not
/// represented in this struct.
///
/// # Mixed-kind cues
///
/// A single timed cue in a `lyrics.*.txt` file may combine lines of
/// different kinds. When a continuation line carries its own cue
/// token, that token dispatches its own renderer for the line
/// instead of inheriting from the line that opened the block. The
/// convention was introduced for Cloudside Dreams, whose title and
/// opening credit share one timed block:
///
/// ```text
/// 00:00:10.080 ttl: Vân Biên Mộng Thoại
///              cre: Điều phối sản xuất âm nhạc  WOVOP
/// ```
///
/// The two lines appear in the same rendered cue but are produced by
/// different renderers. A cue whose lines all share one renderer
/// should continue to omit prefixes on continuation lines so that
/// inheritance applies.
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
    /// Cues that invoke the credit-block renderer. Columns of the
    /// line become `<c.creditRole>` and `<c.creditName>` segments,
    /// each validated against `credits.yaml`. Names wrapped in
    /// brackets in the source become `<c.creditSpecial>` instead of
    /// `<c.creditName>`.
    #[serde(default)]
    pub credits: Vec<String>,
}
