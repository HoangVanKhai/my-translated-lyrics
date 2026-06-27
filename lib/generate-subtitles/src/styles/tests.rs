use super::{Color, InvalidColor};
use pretty_assertions::assert_eq;

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
