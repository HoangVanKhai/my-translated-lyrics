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
/// A _marker_ is the short token (for example `LTY`, `cre`, `ttl`,
/// `LRC`) at the start of each line in a song's `lyrics.*.txt`
/// files. This descriptor catalogs every marker the song uses and
/// groups them by the rendering role they play — voice, named
/// class, credit block, or plain pass-through. The groups are
/// consumed by [`crate::generate_subtitles`] and its submodules;
/// see [`crate::generate_subtitles::render_vtt`] for how each group
/// is wrapped in the output and
/// [`crate::generate_subtitles::styles`] for the shared presentation
/// palette.
#[derive(Default, Deserialize, Serialize)]
pub struct LineMarkersDesc {
    /// Exhaustive inventory of markers used by this song, in the
    /// order the style block should emit per-marker rules.
    #[serde(default)]
    pub markers: Vec<String>,
    /// Markers that name a voice. Each value maps a language code to
    /// the voice name to emit for that language.
    #[serde(default)]
    pub voices: BTreeMap<String, BTreeMap<Language, String>>,
    /// Markers that name a class. The mapped value is the class name
    /// applied to the wrapping element.
    #[serde(default)]
    pub classes: BTreeMap<String, String>,
    /// Markers that open a credit block. The cue body is parsed
    /// line-by-line with the song's `credits.yaml` vocabulary.
    #[serde(default)]
    pub credits: Vec<String>,
}
