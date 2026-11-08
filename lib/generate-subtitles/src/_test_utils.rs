//! Shared fixtures for the crate's tests.

use crate::parse::LineNumber;
use crate::styles::{Color, CreditPalette, Style, StylePalette};
use lyrics_core::credits_descriptor::CreditsDesc;
use lyrics_core::line_markers_descriptor::{LineMarkersDesc, MarkerName};
use lyrics_core::video_descriptor::Language;
use maplit::btreemap;
use pipe_trait::Pipe;
use std::collections::BTreeMap;
use std::num::NonZeroUsize;

/// A credits descriptor declaring a single Vietnamese role, `role-a`.
pub(crate) fn credits_with_one_role() -> CreditsDesc {
    CreditsDesc {
        credit_roles: vec![btreemap! { Language::Vietnamese => "role-a".to_string().into() }],
        ..Default::default()
    }
}

/// A line-markers descriptor whose `cre` marker triggers credit
/// rendering.
pub(crate) fn markers_with_credit_trigger() -> LineMarkersDesc {
    LineMarkersDesc {
        credits: vec![marker_name("cre")],
        ..Default::default()
    }
}

/// Wraps a marker token that the fixtures know to be declarable.
pub(crate) fn marker_name(name: impl Into<String>) -> MarkerName {
    name.into()
        .pipe(MarkerName::new)
        .expect("test fixture passes the marker-name validator")
}

/// Wraps a line number that the fixtures know to name a real line.
pub(crate) fn line_number(number: usize) -> LineNumber {
    NonZeroUsize::new(number)
        .map(LineNumber::new)
        .expect("test fixture names a line from one upwards")
}

/// Wraps a color string that the fixtures know to be valid.
pub(crate) fn color(value: String) -> Color {
    value
        .pipe(Color::new)
        .expect("test fixture passes the color validator")
}

/// A style palette carrying the standard credit colors together with
/// the given voice table and an empty class table.
pub(crate) fn style_palette(voices: BTreeMap<MarkerName, Style>) -> StylePalette {
    StylePalette {
        credit: CreditPalette {
            role: color("#AAAA22".to_owned()),
            name: color("#AAAAAA".to_owned()),
            special: color("#55ABCD".to_owned()),
        },
        voices,
        classes: BTreeMap::new(),
    }
}
