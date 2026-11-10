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
    // Compared whole, so that the clear-screen sequence is required to
    // precede the drawn text. Two `contains` checks would also pass for
    // a frame that drew the text and then wiped it.
    assert_eq!(rendered, "\u{1b}[2J\u{1b}[1;1Hhello", "{rendered:?}");
}

/// The default screen is an empty one, so its first frame clears and draws
/// like a freshly constructed screen.
#[test]
fn the_default_screen_draws_its_first_frame() {
    let mut screen = Screen::default();
    let mut output = Vec::new();
    screen
        .begin(4, 1, &mut output)
        .unwrap()
        .set_string(0, 0, "hi", Style::PLAIN);
    screen.flush(&mut output).unwrap();
    let rendered = String::from_utf8(output).unwrap();
    assert!(rendered.contains("\u{1b}[2J"), "{rendered:?}");
    assert!(rendered.contains("hi"), "{rendered:?}");
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
    // One cursor move to the third column and the one changed character,
    // and nothing else: no clear, and none of the unchanged text around
    // it. The previous frame's `world` never appears in a repaint of
    // this frame, so looking for its absence passed however much of
    // `would` was redrawn.
    assert_eq!(rendered, "\u{1b}[1;3Hu", "{rendered:?}");
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
    // The cursor moves to the erased cell before the blank is printed.
    // A bare search for a space says nothing about where it landed.
    assert_eq!(rendered, "\u{1b}[1;2H ", "{rendered:?}");
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
        .set_string(0, 0, "🔍︎", Style::PLAIN);
    screen.flush(&mut output).unwrap();
    let rendered = String::from_utf8(output).unwrap();
    assert!(rendered.contains("🔍︎"), "{rendered:?}");
}
