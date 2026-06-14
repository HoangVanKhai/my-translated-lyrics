//! Declarative presentation palette.
//!
//! Every song shares a single palette for its class and voice markers.
//! Rather than duplicate that palette in every `line-markers.toml`, or
//! hardcode it in Rust, both subtitle renderers consult a
//! [`StylePalette`] loaded from `styles.toml` at startup to map a class
//! name or a voice marker to its color and text decoration.
//!
//! A marker that a song declares as a voice or a class, but that has no
//! entry in the palette, is a [`MissingStyle`] error rather than a
//! silently plain render. Adding a marker style is therefore a data edit
//! in `styles.toml`, and an unrecognized style name is caught at render
//! time.

use derive_more::Display;
use lyrics_core::line_markers_descriptor::CssClassName;
use serde::Deserialize;
use std::collections::BTreeMap;

/// Default file name of the palette, relative to the working directory.
pub const STYLE_PALETTE_FILE_NAME: &str = "styles.toml";

/// The full presentation palette, deserialized from `styles.toml`.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StylePalette {
    /// Colors for the three built-in credit classes.
    pub credit: CreditPalette,
    /// Style for each voice marker, keyed by the marker token used in
    /// the `[voices]` section of a song's `line-markers.toml`, such as
    /// `LTY`, `lty`, or `Y+L`.
    #[serde(default)]
    pub voices: BTreeMap<String, Style>,
    /// Style for each named class, keyed by the class name used in the
    /// `[classes]` section of a song's `line-markers.toml`, such as
    /// `title` or `expo`.
    #[serde(default)]
    pub classes: BTreeMap<CssClassName, Style>,
}

impl StylePalette {
    /// Looks up the style for a voice marker. The marker is expected to
    /// be declared in a song's `[voices]` section; an absent entry is a
    /// configuration error rather than a plain render.
    pub fn voice_style(&self, marker: &str) -> Result<&Style, MissingStyle> {
        self.voices
            .get(marker)
            .ok_or_else(|| MissingStyle::Voice(marker.to_string()))
    }

    /// Looks up the style for a named class. The class is expected to be
    /// the value of an entry in a song's `[classes]` section; an absent
    /// entry is a configuration error rather than a plain render.
    pub fn class_style(&self, class_name: &CssClassName) -> Result<&Style, MissingStyle> {
        self.classes
            .get(class_name)
            .ok_or_else(|| MissingStyle::Class(class_name.as_str().to_string()))
    }
}

/// Colors for the three built-in credit classes. Unlike voice and class
/// styles, credit cells are color-only and carry no decoration.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreditPalette {
    /// Color used for the role cell of every credit line.
    pub role: String,
    /// Color used for the name cell of every credit line.
    pub name: String,
    /// Color used for a bracketed highlight (`【...】`, `[...]`, or
    /// `(...)`) inside a credit name.
    pub special: String,
}

/// Presentation attributes applied to a run of cue text.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Style {
    /// CSS color value, such as `"#FFD966"` or `"white"`.
    #[serde(default)]
    pub color: Option<String>,
    /// Render the run in italics.
    #[serde(default)]
    pub italic: bool,
    /// Render the run in bold.
    #[serde(default)]
    pub bold: bool,
}

impl Style {
    /// A color-only style, with neither italics nor bold. Used to feed
    /// the credit-cell colors through the same rendering helpers as the
    /// voice and class styles.
    pub fn color_only(color: String) -> Self {
        Style {
            color: Some(color),
            italic: false,
            bold: false,
        }
    }
}

/// A marker declared by a song but missing from the palette.
#[derive(Debug, Display, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum MissingStyle {
    #[display("no style is defined for voice marker {_0:?} in the palette")]
    Voice(String),
    #[display("no style is defined for class {_0:?} in the palette")]
    Class(String),
}
