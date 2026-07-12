//! Shared fixtures for the SRT and VTT renderer tests. Both renderers
//! exercise the same format-agnostic scaffolding: a credits descriptor,
//! a line-markers descriptor that triggers credit rendering, a [`Color`]
//! constructor wrapper, and a [`StylePalette`]. Each renderer's test
//! module pulls the pieces it needs from here rather than defining its
//! own copy, so a change to a shared type is made in one place instead
//! of two that can silently drift apart.

use crate::styles::{Color, CreditPalette, Style, StylePalette};
use lyrics_core::credits_descriptor::CreditsDesc;
use lyrics_core::line_markers_descriptor::LineMarkersDesc;
use lyrics_core::video_descriptor::Language;
use maplit::btreemap;
use pipe_trait::Pipe;
use std::collections::BTreeMap;

/// A credits descriptor declaring a single Vietnamese role, `role-a`.
pub(crate) fn credits_with_one_role() -> CreditsDesc {
    CreditsDesc {
        credit_roles: vec![btreemap! { Language::Vietnamese => "role-a".to_string() }],
        ..Default::default()
    }
}

/// A line-markers descriptor whose `cre` marker triggers credit
/// rendering.
pub(crate) fn markers_with_credit_trigger() -> LineMarkersDesc {
    LineMarkersDesc {
        credits: vec!["cre".to_string()],
        ..Default::default()
    }
}

/// Wraps a color string that the fixtures know to be valid.
pub(crate) fn color(value: String) -> Color {
    value
        .pipe(Color::new)
        .expect("test fixture passes the color validator")
}

/// A style palette carrying the standard credit colors together with
/// the given voice table. The credit colors and the empty class table
/// are identical across both renderers; each renderer's test module
/// supplies whatever voice entries it needs.
pub(crate) fn style_palette(voices: BTreeMap<String, Style>) -> StylePalette {
    StylePalette {
        credit: CreditPalette {
            role: color("#AAAA22".to_owned()),
            name: color("#AAAAAA".to_owned()),
            special: color("#55ABCD".to_owned()),
        },
        voices,
        classes: btreemap! {},
    }
}
