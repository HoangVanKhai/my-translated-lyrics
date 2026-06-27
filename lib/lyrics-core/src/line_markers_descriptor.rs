use crate::video_descriptor::Language;
use derive_more::Display;
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
/// groups them by the rendering role they play. The roles are
/// voice, named class, credit block, and plain pass-through. The
/// groups are consumed by the `generate-subtitles` crate and its
/// submodules; see its `render_vtt` module for how each group is
/// wrapped in the output, and its `styles` module for the shared
/// presentation palette.
#[derive(Default, Deserialize, Serialize)]
pub struct LineMarkersDesc {
    /// Exhaustive inventory of markers used by this song, in the
    /// order the style block should emit per-marker rules.
    #[serde(default)]
    pub markers: Vec<String>,
    /// Markers that name a voice. Each value maps a language code to
    /// the voice name to emit for that language.
    #[serde(default)]
    pub voices: BTreeMap<String, BTreeMap<Language, VoiceName>>,
    /// Markers that name a class. The mapped value is the class name
    /// applied to the wrapping element.
    #[serde(default)]
    pub classes: BTreeMap<String, CssClassName>,
    /// Markers that open a credit block. The cue body is parsed
    /// line-by-line against the `credit-roles` entries in the song's
    /// `credits.yaml`; the companion `credit-names` entries are not
    /// consumed by this path and are tracked separately.
    #[serde(default)]
    pub credits: Vec<String>,
}

/// A CSS class name that is safe to splat into a `::cue(c.{name})`
/// selector and a `<c.{name}>` tag without escaping.
///
/// The permitted shape is `[A-Za-z_][A-Za-z0-9_-]*`. The pattern
/// is the common subset of the CSS identifier grammar and the HTML
/// class-name rules. It excludes whitespace, quotes, dots, braces,
/// and anything outside basic ASCII, all of which would break the
/// STYLE block or the inline tag if interpolated raw.
#[derive(Debug, Display, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
#[serde(try_from = "String", into = "String")]
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
        for char in chars {
            if !is_class_name_continue(char) {
                return Err(InvalidCssClassName::InvalidCharacter(char));
            }
        }
        Ok(CssClassName(source))
    }

    /// The underlying class-name text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for CssClassName {
    type Error = InvalidCssClassName;

    fn try_from(source: String) -> Result<Self, Self::Error> {
        CssClassName::new(source)
    }
}

impl From<CssClassName> for String {
    fn from(value: CssClassName) -> Self {
        value.0
    }
}

fn is_class_name_start(char: char) -> bool {
    char.is_ascii_alphabetic() || char == '_'
}

fn is_class_name_continue(char: char) -> bool {
    char.is_ascii_alphanumeric() || char == '-' || char == '_'
}

/// A speaker label. Populates the WebVTT `<v {name}>` cue tag and
/// the `::cue(v[voice="{name}"])` attribute selector that styles
/// it.
///
/// The permitted shape is any non-empty string whose characters are
/// none of `<`, `>`, `"`, `\`, `U+2028`, `U+2029`, and which
/// contains no ASCII or Unicode control character. This reject list
/// captures every character that would terminate the HTML-like cue
/// tag or the CSS attribute string. The shape is otherwise
/// permissive, and in particular accepts CJK text, accented Latin,
/// and embedded spaces, the three categories that already appear in
/// `sources/*/line-markers.toml`.
///
/// [`VoiceName`] deliberately does not implement `Display`. The two
/// destination contexts, the WebVTT cue tag and the CSS attribute
/// selector, have incompatible quoting rules, and a single
/// `Display` impl could only be correct in one of them. Rendering
/// therefore goes through context-specific wrappers in the WebVTT
/// renderer that name the destination grammar; each wrapper
/// produces one of the two output shapes so the call site cannot
/// cross them up.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
#[serde(try_from = "String", into = "String")]
pub struct VoiceName(String);

impl VoiceName {
    /// Wraps `source` if and only if it satisfies the voice-name
    /// shape above.
    pub fn new(source: String) -> Result<Self, InvalidVoiceName> {
        if source.is_empty() {
            return Err(InvalidVoiceName::Empty);
        }
        for char in source.chars() {
            if is_forbidden_voice_char(char) {
                return Err(InvalidVoiceName::ForbiddenCharacter(char));
            }
        }
        Ok(VoiceName(source))
    }

    /// The underlying voice-name text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for VoiceName {
    type Error = InvalidVoiceName;

    fn try_from(source: String) -> Result<Self, Self::Error> {
        VoiceName::new(source)
    }
}

impl From<VoiceName> for String {
    fn from(value: VoiceName) -> Self {
        value.0
    }
}

fn is_forbidden_voice_char(char: char) -> bool {
    matches!(char, '<' | '>' | '"' | '\\' | '\u{2028}' | '\u{2029}') || char.is_control()
}

#[derive(Debug, Display, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum InvalidVoiceName {
    #[display("voice name must not be empty")]
    Empty,
    #[display(
        r#"voice name must not contain {_0:?}; `<`, `>`, `"`, `\`, line separators, and control characters are reserved by WebVTT and CSS"#
    )]
    ForbiddenCharacter(char),
}

#[derive(Debug, Display, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum InvalidCssClassName {
    #[display("class name must not be empty")]
    Empty,
    #[display("class name must begin with an ASCII letter or `_`, got {_0:?}")]
    InvalidLeadingCharacter(char),
    #[display("class name must contain only ASCII letters, digits, `-`, and `_`, got {_0:?}")]
    InvalidCharacter(char),
}

#[cfg(test)]
mod tests;
