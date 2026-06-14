use super::Buffer;
use crate::style::Style;
use pretty_assertions::assert_eq;

/// A fresh buffer reads back as blank.
#[test]
fn an_empty_buffer_is_blank() {
    let buffer = Buffer::new(4, 1);
    assert_eq!(buffer.row_text(0), "    ");
}

/// Writing a string places each character in its own cell.
#[test]
fn set_string_places_each_character() {
    let mut buffer = Buffer::new(8, 1);
    buffer.set_string(2, 0, "hi", Style::PLAIN);
    assert_eq!(buffer.row_text(0), "  hi    ");
}

/// A double-width glyph occupies two columns: its own and the trailing one.
#[test]
fn a_wide_glyph_spans_two_columns() {
    let mut buffer = Buffer::new(4, 1);
    buffer.set_string(0, 0, "中x", Style::PLAIN);
    // 中 sits in columns 0 and 1, so x lands in column 2.
    assert_eq!(buffer.row_text(0), "中 x ");
}

/// Text written past the right edge is clipped rather than wrapping.
#[test]
fn set_string_clips_at_the_right_edge() {
    let mut buffer = Buffer::new(3, 1);
    buffer.set_string(0, 0, "abcdef", Style::PLAIN);
    assert_eq!(buffer.row_text(0), "abc");
}
