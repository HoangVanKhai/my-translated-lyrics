use super::{Color, InvalidColor};
use pretty_assertions::assert_eq;

/// Each form is accepted and kept verbatim, interior spaces included.
#[test]
fn accepts_hex_keyword_and_functional_colors() {
    for value in ["#FFD966", "white", "rgb(0, 0, 0)"] {
        assert_eq!(
            Color::new(value.to_string()).as_ref().map(Color::as_str),
            Ok(value),
            "{value:?} should be accepted verbatim",
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
