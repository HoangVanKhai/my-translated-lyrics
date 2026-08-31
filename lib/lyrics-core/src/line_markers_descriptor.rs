use crate::video_descriptor::Language;
use derive_more::Display;
use serde::{Deserialize, Serialize};
use std::borrow::Borrow;
use std::collections::BTreeMap;
use strum::{AsRefStr, EnumString, VariantArray};

pub const LINE_MARKERS_CONFIG_FILE_NAME: &str = "line-markers.toml";

/// A marker whose meaning the parser fixes.
///
/// A reserved marker names no rendering role, so a song must not
/// declare it in its `line-markers.toml`; [`MarkerName`] rejects
/// every one of them at the deserialization boundary.
#[derive(AsRefStr, Clone, Copy, Debug, strum::Display, EnumString, Eq, PartialEq, VariantArray)]
pub enum ReservedMarker {
    /// An annotation. Lines that start with this marker carry
    /// commentary about the cue part above them. They take no
    /// timestamp of their own and are ignored by both renderers.
    #[strum(serialize = "ann")]
    Annotation,
    /// Cue clearing. Lines that start with this marker cause the
    /// previously opened cue to end at the marker's timestamp and
    /// produce no visible text of their own.
    #[strum(serialize = "clr")]
    Clear,
    /// The end-of-video sentinel. It marks the point at which no
    /// further subtitle activity occurs.
    #[strum(serialize = "eov")]
    EndOfVideo,
}

impl ReservedMarker {
    /// Whether the reserved marker is a control marker.
    pub fn is_control(self) -> bool {
        match self {
            ReservedMarker::Annotation => false,
            ReservedMarker::Clear => true,
            ReservedMarker::EndOfVideo => true,
        }
    }
}

/// Parsed contents of a `line-markers.toml` file.
///
/// A _marker_ is the short token (for example `LTY`, `cre`, `ttl`,
/// `LRC`) at the start of each line in a song's `lyrics.*.txt`
/// files. This descriptor catalogs every marker the song uses and
/// groups them by the rendering role they play. A marker whose
/// meaning the parser fixes is not declared here, because it names no
/// rendering role. The roles are voice, named class, credit block,
/// and plain pass-through. The groups are consumed by the
/// `generate-subtitles` crate and its submodules; see its
/// `render_vtt` module for how each group is wrapped in the output,
/// and its `styles` module for the shared presentation palette.
#[derive(Default, Deserialize, Serialize)]
pub struct LineMarkersDesc {
    /// Exhaustive inventory of markers used by this song, in the
    /// order the style block should emit per-marker rules.
    #[serde(default)]
    pub markers: Vec<MarkerName>,
    /// Markers that name a voice. Each value maps a language code to
    /// the voice name to emit for that language.
    #[serde(default)]
    pub voices: BTreeMap<MarkerName, BTreeMap<Language, VoiceName>>,
    /// Markers that name a class. The mapped value is the class name
    /// applied to the wrapping element.
    #[serde(default)]
    pub classes: BTreeMap<MarkerName, CssClassName>,
    /// Markers that open a credit block. The cue body is parsed
    /// line-by-line against the `credit-roles` entries in the song's
    /// `credits.yaml`; the companion `credit-names` entries are not
    /// consumed by this path and are tracked separately.
    #[serde(default)]
    pub credits: Vec<MarkerName>,
}

impl LineMarkersDesc {
    /// Whether `marker` opens a credit block in this song. The
    /// argument is the plain token a parsed cue part carries.
    pub fn is_credit(&self, marker: &str) -> bool {
        self.credits.iter().any(|credit| credit.as_str() == marker)
    }
}

/// A marker name that a song declares in its `line-markers.toml`.
///
/// The name must not be a [`ReservedMarker`]; no further shape is
/// imposed.
#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(try_from = "String", into = "String")]
pub struct MarkerName(String);

impl MarkerName {
    /// Wraps `source` if and only if it names no [`ReservedMarker`].
    pub fn new(source: String) -> Result<Self, InvalidMarkerName> {
        match source.parse::<ReservedMarker>() {
            Ok(reserved) => Err(InvalidMarkerName::Reserved(reserved)),
            Err(strum::ParseError::VariantNotFound) => Ok(MarkerName(source)),
        }
    }

    /// The underlying marker text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for MarkerName {
    type Error = InvalidMarkerName;

    fn try_from(source: String) -> Result<Self, Self::Error> {
        MarkerName::new(source)
    }
}

impl From<MarkerName> for String {
    fn from(value: MarkerName) -> Self {
        value.0
    }
}

// Enables lookups into a map keyed by `MarkerName` with the plain
// token a parsed cue part carries.
impl Borrow<str> for MarkerName {
    fn borrow(&self) -> &str {
        &self.0
    }
}

/// A CSS class name that is safe to splat into a `::cue(c.{name})`
/// selector and a `<c.{name}>` tag without escaping.
///
/// The permitted shape is `[A-Za-z_][A-Za-z0-9_-]*`. The pattern
/// is the common subset of the CSS identifier grammar and the HTML
/// class-name rules. It excludes whitespace, quotes, dots, braces,
/// and anything outside basic ASCII, all of which would break the
/// STYLE block or the inline tag if interpolated raw.
#[derive(Clone, Debug, Deserialize, Display, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
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
#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
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

#[derive(Clone, Debug, Display, Eq, PartialEq)]
#[non_exhaustive]
pub enum InvalidVoiceName {
    #[display("voice name must not be empty")]
    Empty,
    #[display(
        r#"voice name must not contain {_0:?}; `<`, `>`, `"`, `\`, line separators, and control characters are reserved by WebVTT and CSS"#
    )]
    ForbiddenCharacter(char),
}

#[derive(Clone, Debug, Display, Eq, PartialEq)]
#[non_exhaustive]
pub enum InvalidCssClassName {
    #[display("class name must not be empty")]
    Empty,
    #[display("class name must begin with an ASCII letter or `_`, got {_0:?}")]
    InvalidLeadingCharacter(char),
    #[display("class name must contain only ASCII letters, digits, `-`, and `_`, got {_0:?}")]
    InvalidCharacter(char),
}

#[derive(Clone, Copy, Debug, Display, Eq, PartialEq)]
#[non_exhaustive]
pub enum InvalidMarkerName {
    #[display("marker name `{_0}` is reserved by the parser and must not be declared")]
    Reserved(ReservedMarker),
}

#[cfg(test)]
mod tests;
