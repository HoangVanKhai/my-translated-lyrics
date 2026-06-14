use crate::{columns_line, fit, scroll_offset};
use pretty_assertions::assert_eq;
use unicode_width::UnicodeWidthStr;

#[test]
fn fit_pads_short_text() {
    assert_eq!(fit("ab", 5), "ab   ");
}

#[test]
fn fit_truncates_with_an_ellipsis() {
    assert_eq!(fit("abcdef", 4), "abc…");
}

#[test]
fn fit_handles_zero_width() {
    assert_eq!(fit("abc", 0), "");
}

/// Width is measured in display columns: an accented letter is one column,
/// a CJK ideograph is two.
#[test]
fn fit_measures_display_width() {
    // "café" is four single-column characters.
    assert_eq!(fit("café", 4), "café");
    // Each ideograph occupies two columns, so "示例" fills four exactly.
    assert_eq!(fit("示例", 4), "示例");
    // Padding accounts for the double-width glyphs.
    assert_eq!(fit("示例", 6), "示例  ");
}

/// Truncation counts each glyph's width, never overflows the budget, and
/// pads the column a wide glyph could not fill before the ellipsis.
#[test]
fn fit_truncates_wide_characters_to_the_column_budget() {
    // "示例例" is six columns; in four, one ideograph and the ellipsis fit
    // and a single padding column fills the rest.
    assert_eq!(fit("示例例", 4), "示… ");
}

#[test]
fn columns_line_splits_the_width_three_ways() {
    let line = columns_line("alpha", "beta", "gamma", 30);
    // The line fills the full width and keeps the two column separators.
    assert_eq!(line.chars().count(), 30);
    assert_eq!(line.matches('│').count(), 2);
    let cells: Vec<&str> = line.split('│').map(str::trim).collect();
    assert_eq!(cells, vec!["alpha", "beta", "gamma"]);
}

/// Cells with wide glyphs are measured by display width, so the line still
/// fills exactly `total` columns rather than overrunning the terminal.
#[test]
fn columns_line_aligns_wide_characters() {
    // cspell:locale en vi
    let line = columns_line("中文", "Tiếng Việt", "示例歌曲", 30);
    assert_eq!(line.width(), 30);
}

#[test]
fn scroll_offset_keeps_the_cursor_on_screen() {
    // The cursor fits within the first page, so no scrolling.
    assert_eq!(scroll_offset(2, 5), 0);
    // The cursor sits past the page, so the window scrolls to show it.
    assert_eq!(scroll_offset(7, 5), 3);
}
