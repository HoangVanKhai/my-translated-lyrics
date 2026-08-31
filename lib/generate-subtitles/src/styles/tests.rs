use super::{Color, InvalidColor, StylePalette};
use crate::_test_utils::{marker_name, style_palette};
use lyrics_core::line_markers_descriptor::CssClassName;
use pipe_trait::Pipe;
use pretty_assertions::assert_eq;
use std::collections::BTreeMap;
use text_block_macros::text_block_fnl;

#[test]
fn accepts_hex_keyword_and_functional_colors() {
    for value in ["#FFD966", "white", "rgb(0, 0, 0)"] {
        assert!(
            Color::new(value.to_string()).is_ok(),
            "{value:?} should be accepted",
        );
    }
}

#[test]
fn rejects_empty() {
    assert_eq!(Color::new(String::new()), Err(InvalidColor::Empty));
}

#[test]
fn rejects_surrounding_whitespace() {
    for value in ["   ", " white", "white "] {
        assert_eq!(
            Color::new(value.to_string()),
            Err(InvalidColor::SurroundingWhitespace),
            "{value:?} should be rejected for surrounding whitespace",
        );
    }
}

#[test]
fn rejects_css_or_html_terminators() {
    for ch in ['<', '>', '"', '\\', '{', '}', ';'] {
        let value = format!("re{ch}d");
        assert_eq!(
            Color::new(value),
            Err(InvalidColor::ForbiddenCharacter(ch)),
            "a color containing {ch:?} should be rejected",
        );
    }
}

/// A voice marker the palette does not style names itself in the message,
/// quoted as a song's `line-markers.toml` spells it rather than as the
/// `Debug` form of the type that carries it.
#[test]
fn a_missing_voice_style_names_the_marker() {
    let error = BTreeMap::new()
        .pipe(style_palette)
        .voice_style(&marker_name("vca"))
        .expect_err("the fixture palette styles no voice");
    assert_eq!(
        error.to_string(),
        r#"no style is defined for voice marker "vca" in the palette"#,
    );
}

/// A class the palette does not style names itself the same way.
#[test]
fn a_missing_class_style_names_the_class() {
    let class_name = "title"
        .to_owned()
        .pipe(CssClassName::new)
        .expect("test fixture passes the class-name validator");
    let error = BTreeMap::new()
        .pipe(style_palette)
        .class_style(&class_name)
        .expect_err("the fixture palette styles no class");
    assert_eq!(
        error.to_string(),
        r#"no style is defined for class "title" in the palette"#,
    );
}

/// The `[voices]` table is keyed by the same type a song declares its voices
/// with, so a palette that styles a reserved marker is rejected when the file
/// is read. Such an entry could never be reached, because the parser fixes
/// the meaning of those tokens and refuses them as cue markers.
#[test]
fn a_voice_style_for_a_reserved_marker_is_rejected() {
    let source = text_block_fnl! {
        "[credit]"
        r#"role = "white""#
        r#"name = "white""#
        r#"special = "white""#
        ""
        "[voices.ann]"
        r#"color = "white""#
    };
    let error = source
        .pipe(toml::from_str::<StylePalette>)
        .expect_err("a reserved marker must not be declarable as a voice");
    assert!(
        error.to_string().contains("`ann` is reserved"),
        "unexpected message: {error}",
    );
}
