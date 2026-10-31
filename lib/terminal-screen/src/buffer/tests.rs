use super::Buffer;
use crate::geometry::{Column, Height, Row, Width};
use crate::style::Style;
use pretty_assertions::assert_eq;

/// A fresh buffer reads back as blank.
#[test]
fn an_empty_buffer_is_blank() {
    let buffer = Buffer::new(Width::new(4), Height::new(1));
    assert_eq!(buffer.row_text(Row::TOP), "    ");
}

/// Writing a string places each character in its own cell.
#[test]
fn set_string_places_each_character() {
    let mut buffer = Buffer::new(Width::new(8), Height::new(1));
    buffer.set_string(Column::new(2), Row::TOP, "hi", Style::PLAIN);
    assert_eq!(buffer.row_text(Row::TOP), "  hi    ");
}

/// A double-width glyph occupies two columns: its own and the trailing one.
#[test]
fn a_wide_glyph_spans_two_columns() {
    let mut buffer = Buffer::new(Width::new(4), Height::new(1));
    buffer.set_string(Column::LEFT, Row::TOP, "中x", Style::PLAIN);
    // 中 sits in columns 0 and 1, so x lands in column 2.
    assert_eq!(buffer.row_text(Row::TOP), "中 x ");
}

/// Text written past the right edge is clipped rather than wrapping.
#[test]
fn set_string_clips_at_the_right_edge() {
    let mut buffer = Buffer::new(Width::new(3), Height::new(1));
    buffer.set_string(Column::LEFT, Row::TOP, "abcdef", Style::PLAIN);
    assert_eq!(buffer.row_text(Row::TOP), "abc");
}

/// A variation selector changes a symbol's form but not its width, matching
/// how terminals render it, so a selected glyph keeps its base column span.
#[test]
fn a_variation_selected_glyph_keeps_its_base_width() {
    let mut buffer = Buffer::new(Width::new(4), Height::new(1));
    // 🔍 is two columns; the text variation selector (U+FE0E) does not narrow it.
    buffer.set_string(Column::LEFT, Row::TOP, "🔍︎x", Style::PLAIN);
    // 🔍 spans columns 0 and 1, so x lands in column 2.
    assert_eq!(buffer.row_text(Row::TOP), "🔍 x ");
}

/// A wide glyph that would run past the right edge is clipped, not written with
/// its trailing column off-buffer.
#[test]
fn a_wide_glyph_at_the_right_edge_is_clipped() {
    let mut buffer = Buffer::new(Width::new(3), Height::new(1));
    // 中 is two columns; at column 2 it would overrun the width-3 buffer.
    buffer.set_string(Column::LEFT, Row::TOP, "ab中", Style::PLAIN);
    assert_eq!(buffer.row_text(Row::TOP), "ab ");
}

/// A zero-width character claims no column: `set_glyph` writes nothing and
/// reports a zero advance, so a combining mark cannot shift the grid.
#[test]
fn set_glyph_ignores_a_zero_width_character() {
    let mut buffer = Buffer::new(Width::new(4), Height::new(1));
    // U+0301 is a combining acute accent, which has no column of its own.
    let advance = buffer.set_glyph(Column::LEFT, Row::TOP, '\u{0301}', Style::PLAIN);
    assert_eq!(advance, Width::ZERO);
    assert_eq!(buffer.row_text(Row::TOP), "    ");
}

/// A glyph placed on a row outside the buffer writes nothing yet still reports
/// its width, so a caller laying out off-screen rows keeps advancing correctly.
#[test]
fn set_glyph_outside_the_buffer_writes_nothing() {
    let mut buffer = Buffer::new(Width::new(4), Height::new(1));
    let advance = buffer.set_glyph(Column::LEFT, Row::new(5), 'a', Style::PLAIN);
    assert_eq!(advance, Width::ONE);
    assert_eq!(buffer.row_text(Row::TOP), "    ");
}

/// `set_string` skips a leading zero-width character, so the text after it
/// still starts at the left edge rather than being shifted along.
#[test]
fn set_string_skips_a_leading_zero_width_character() {
    let mut buffer = Buffer::new(Width::new(4), Height::new(1));
    // U+0301 is a combining acute accent, which has no column of its own.
    buffer.set_string(Column::LEFT, Row::TOP, "\u{0301}x", Style::PLAIN);
    assert_eq!(buffer.row_text(Row::TOP), "x   ");
}

/// `style_at` reports a glyph's style, but the plain style for a trailing
/// column, an empty cell, or a position outside the buffer.
#[test]
fn style_at_is_plain_for_non_glyph_cells() {
    let mut buffer = Buffer::new(Width::new(4), Height::new(1));
    buffer.set_string(Column::LEFT, Row::TOP, "中", Style::BOLD);
    assert_eq!(buffer.style_at(Column::LEFT, Row::TOP), Style::BOLD);
    assert_eq!(buffer.style_at(Column::new(1), Row::TOP), Style::PLAIN);
    assert_eq!(buffer.style_at(Column::new(3), Row::TOP), Style::PLAIN);
    assert_eq!(buffer.style_at(Column::new(9), Row::TOP), Style::PLAIN);
}
