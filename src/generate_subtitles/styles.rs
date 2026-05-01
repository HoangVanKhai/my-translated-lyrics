//! Hardcoded presentation palette.
//!
//! The two songs currently in the repository share a single palette
//! for their class and voice markers. Rather than duplicate that
//! palette in every `line-markers.toml`, both subtitle renderers
//! consult the tables in this module to map a class name or a voice
//! marker to its color and text decoration.

/// Color used for the role cell of every credit line.
pub(super) const CREDIT_ROLE_COLOR: &str = "#AAAA22";
/// Color used for the name cell of every credit line.
pub(super) const CREDIT_NAME_COLOR: &str = "#AAAAAA";
/// Color used for a bracketed highlight (`【...】`, `[...]`, or
/// `(...)`) inside a credit name.
pub(super) const CREDIT_SPECIAL_COLOR: &str = "#55ABCD";

/// Presentation attributes applied to a run of cue text.
#[derive(Clone, Copy)]
pub struct Style {
    /// CSS color value, such as `"#FFD966"` or `"white"`.
    pub color: Option<&'static str>,
    /// Render the run in italics.
    pub italic: bool,
    /// Render the run in bold.
    pub bold: bool,
}

impl Style {
    const fn plain(color: &'static str) -> Self {
        Style {
            color: Some(color),
            italic: false,
            bold: false,
        }
    }

    const fn italic(color: &'static str) -> Self {
        Style {
            color: Some(color),
            italic: true,
            bold: false,
        }
    }

    const fn bold(color: &'static str) -> Self {
        Style {
            color: Some(color),
            italic: false,
            bold: true,
        }
    }
}

/// Looks up the style for a class name. Class names come from the
/// `[classes]` section of a song's `line-markers.toml`.
pub fn class_style(class_name: &str) -> Option<Style> {
    match class_name {
        "title" => Some(Style::bold("#FFD966")),
        "expo" => Some(Style::italic("#CCCCCC")),
        _ => None,
    }
}

/// Looks up the style for a voice marker. Voice markers are the
/// per-character abbreviations such as `LTY`, `lty`, `YZL`, `yzl`,
/// and `Y+L` that appear in the `[voices]` section of a song's
/// `line-markers.toml`. Lowercase marker variants signal a spoken
/// word break, which the repository renders as italic in addition
/// to the character color.
pub fn voice_style(marker: &str) -> Option<Style> {
    match marker {
        "LTY" => Some(Style::plain("#66CCFF")),
        "lty" => Some(Style::italic("#66CCFF")),
        "YZL" => Some(Style::plain("#EE0000")),
        "yzl" => Some(Style::italic("#EE0000")),
        "Y+L" => Some(Style::plain("#9966CC")),
        _ => None,
    }
}
