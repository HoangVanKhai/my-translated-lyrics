use crate::video_descriptor::Language;
use core::fmt;
use derive_more::{Display, Error};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
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
    pub classes: BTreeMap<String, CssClassName>,
    /// Markers that open a credit block. The cue body is parsed
    /// line-by-line with the song's `credits.yaml` vocabulary.
    #[serde(default)]
    pub credits: Vec<String>,
}

/// A CSS class name that is safe to splat into a `::cue(c.{name})`
/// selector and a `<c.{name}>` tag without escaping.
///
/// The permitted shape is `[A-Za-z_][A-Za-z0-9_-]*` — the common
/// subset of the CSS identifier grammar and the HTML class-name
/// rules. This excludes whitespace, quotes, dots, braces, and
/// anything outside basic ASCII, all of which would break the
/// STYLE block or the inline tag if interpolated raw.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CssClassName(String);

impl CssClassName {
    /// Wraps `source` if and only if it satisfies the class-name
    /// shape above.
    pub fn new(source: String) -> Result<Self, InvalidCssClassName> {
        let mut chars = source.chars();
        let Some(first) = chars.next() else {
            return Err(InvalidCssClassName::Empty);
        };
        if !is_class_name_start(first) {
            return Err(InvalidCssClassName::InvalidLeadingCharacter(first));
        }
        for ch in chars {
            if !is_class_name_continue(ch) {
                return Err(InvalidCssClassName::InvalidCharacter(ch));
            }
        }
        Ok(CssClassName(source))
    }

    /// The underlying class-name text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for CssClassName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for CssClassName {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let source = String::deserialize(deserializer)?;
        CssClassName::new(source).map_err(serde::de::Error::custom)
    }
}

impl Serialize for CssClassName {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

fn is_class_name_start(ch: char) -> bool {
    ch.is_ascii_alphabetic() || ch == '_'
}

fn is_class_name_continue(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '-' || ch == '_'
}

#[derive(Debug, Display, Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum InvalidCssClassName {
    #[display("class name must not be empty")]
    Empty,
    #[display("class name must begin with an ASCII letter or `_`, got {_0:?}")]
    InvalidLeadingCharacter(#[error(not(source))] char),
    #[display("class name must contain only ASCII letters, digits, `-`, and `_`, got {_0:?}")]
    InvalidCharacter(#[error(not(source))] char),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_simple_ascii_names() {
        assert_eq!(
            CssClassName::new("title".to_string()).unwrap().as_str(),
            "title",
        );
        assert_eq!(
            CssClassName::new("creditRole".to_string())
                .unwrap()
                .as_str(),
            "creditRole",
        );
        assert_eq!(
            CssClassName::new("_hidden".to_string()).unwrap().as_str(),
            "_hidden",
        );
        assert_eq!(
            CssClassName::new("kebab-name_42".to_string())
                .unwrap()
                .as_str(),
            "kebab-name_42",
        );
    }

    #[test]
    fn rejects_empty() {
        assert!(matches!(
            CssClassName::new(String::new()),
            Err(InvalidCssClassName::Empty),
        ));
    }

    #[test]
    fn rejects_leading_digit_hyphen_or_non_ascii() {
        assert_eq!(
            CssClassName::new("1name".to_string()).unwrap_err(),
            InvalidCssClassName::InvalidLeadingCharacter('1'),
        );
        assert_eq!(
            CssClassName::new("-name".to_string()).unwrap_err(),
            InvalidCssClassName::InvalidLeadingCharacter('-'),
        );
        assert!(matches!(
            CssClassName::new("名字".to_string()),
            Err(InvalidCssClassName::InvalidLeadingCharacter(_)),
        ));
    }

    #[test]
    fn rejects_unsafe_continue_characters() {
        assert_eq!(
            CssClassName::new("bad name".to_string()).unwrap_err(),
            InvalidCssClassName::InvalidCharacter(' '),
        );
        assert_eq!(
            CssClassName::new("bad.name".to_string()).unwrap_err(),
            InvalidCssClassName::InvalidCharacter('.'),
        );
        assert_eq!(
            CssClassName::new("bad\"name".to_string()).unwrap_err(),
            InvalidCssClassName::InvalidCharacter('"'),
        );
    }
}
