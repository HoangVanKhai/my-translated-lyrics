use crate::video_descriptor::Language;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const LINE_MARKERS_CONFIG_FILE_NAME: &str = "line-markers.toml";

/// Built-in marker name for cue clearing. Lines that start with this
/// marker cause the previously opened cue to end at the `clr`
/// timestamp and produce no visible text of their own.
pub const CLEAR_MARKER: &str = "clr";

/// Built-in marker name for the end-of-video sentinel. Lines that
/// start with this marker are ignored entirely by the parser: they
/// open no cue and close no cue. The marker exists as a convention
/// so that source files can record, for human readers, the point at
/// which no further subtitle activity occurs. Every cue must still
/// be closed by a following cue or by a `clr` marker; reaching an
/// `eov` line with an open cue is not treated as a cue boundary.
pub const END_OF_VIDEO_MARKER: &str = "eov";

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
/// Presentation styles, that is, colors, bolding, and italics, are
/// not carried in this struct. They are derived from the class name
/// or the voice marker name by a central table maintained in
/// [`crate::build_subtitles::styles`], so that the repository's
/// shared palette stays consistent across every song.
///
/// Universal control keywords such as `clr` and `eov` produce no
/// output and are handled by the generator directly. They are not
/// represented in this struct.
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
