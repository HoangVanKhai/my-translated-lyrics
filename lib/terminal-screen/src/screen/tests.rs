use super::Screen;
use crate::style::Style;

/// The first frame clears the terminal and draws the whole frame.
#[test]
fn the_first_frame_clears_and_draws() {
    let mut screen = Screen::new();
    let mut output = Vec::new();
    screen
        .begin(10, 1, &mut output)
        .unwrap()
        .set_string(0, 0, "hello", Style::PLAIN);
    screen.flush(&mut output).unwrap();
    let rendered = String::from_utf8(output).unwrap();
    // The clear-screen sequence precedes the drawn text.
    assert!(rendered.contains("\u{1b}[2J"), "{rendered:?}");
    assert!(rendered.contains("hello"), "{rendered:?}");
}

/// A later frame at the same size sends only the cells that changed, without
/// clearing the screen or reprinting the unchanged text.
#[test]
fn a_later_frame_sends_only_the_changed_cells() {
    let mut screen = Screen::new();
    let mut output = Vec::new();
    screen
        .begin(10, 1, &mut output)
        .unwrap()
        .set_string(0, 0, "world", Style::PLAIN);
    screen.flush(&mut output).unwrap();

    output.clear();
    // "world" and "would" differ only in the third character.
    screen
        .begin(10, 1, &mut output)
        .unwrap()
        .set_string(0, 0, "would", Style::PLAIN);
    screen.flush(&mut output).unwrap();
    let rendered = String::from_utf8(output).unwrap();
    // Only the changed character is sent, so the unchanged text is not redrawn
    // and the screen is not cleared.
    assert!(!rendered.contains("\u{1b}[2J"), "{rendered:?}");
    assert!(!rendered.contains("world"), "{rendered:?}");
    assert!(rendered.contains('u'), "{rendered:?}");
}

/// Erasing a character writes a blank over it rather than leaving it on screen.
#[test]
fn clearing_a_cell_overwrites_it_with_a_blank() {
    let mut screen = Screen::new();
    let mut output = Vec::new();
    screen
        .begin(4, 1, &mut output)
        .unwrap()
        .set_string(0, 0, "ab", Style::PLAIN);
    screen.flush(&mut output).unwrap();

    output.clear();
    // The second frame draws only "a", so the "b" cell becomes blank.
    screen
        .begin(4, 1, &mut output)
        .unwrap()
        .set_string(0, 0, "a", Style::PLAIN);
    screen.flush(&mut output).unwrap();
    let rendered = String::from_utf8(output).unwrap();
    assert!(rendered.contains(' '), "{rendered:?}");
    assert!(!rendered.contains('b'), "{rendered:?}");
}

/// A variation selector is sent to the terminal along with its glyph, so a
/// symbol keeps its requested text or emoji form.
#[test]
fn a_variation_selector_is_sent_with_its_glyph() {
    let mut screen = Screen::new();
    let mut output = Vec::new();
    screen
        .begin(4, 1, &mut output)
        .unwrap()
        .set_string(0, 0, "🔍\u{FE0E}", Style::PLAIN);
    screen.flush(&mut output).unwrap();
    let rendered = String::from_utf8(output).unwrap();
    assert!(rendered.contains("🔍\u{FE0E}"), "{rendered:?}");
}
