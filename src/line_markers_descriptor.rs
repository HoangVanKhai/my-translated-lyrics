use crate::video_descriptor::Language;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const LINE_MARKERS_CONFIG_FILE_NAME: &str = "line-markers.toml";

/// Parsed contents of a `line-markers.toml` file.
///
/// A _cue_ is a short token (for example `LTY`, `cre`, `ttl`, `LRC`)
/// that appears at the start of a line in the song's lyric text files
/// and controls how the line is rendered into VTT. The [`cues`] field
/// holds the exhaustive inventory of cues the song uses. Cues whose
/// rendering wraps the line in a single `<v ...>...</v>` element
/// additionally appear in [`voices`], and cues whose rendering wraps
/// the line in a single `<c.className>...</c>` element additionally
/// appear in [`classes`].
///
/// A cue that appears in [`cues`] but in neither [`voices`] nor
/// [`classes`] falls into one of two cases:
///
/// * the generator has a hardcoded rendering for that specific cue.
///   For example, `cre` decomposes into multiple class-wrapped
///   segments (`creditRole`, `creditName`, `creditSpecial`) driven by
///   the column layout and the bracketing convention in the source
///   text, with validation against `credits.yaml`.
/// * otherwise, the cue's content is emitted as plain unwrapped text.
///   For example, `LRC` in a song that does not map it to a voice or
///   class falls into this case.
///
/// Universal control keywords such as `clr` and `eov` produce no
/// output and are handled by the generator directly; they are not
/// represented in this struct.
///
/// [`cues`]: Self::cues
/// [`voices`]: Self::voices
/// [`classes`]: Self::classes
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
}
