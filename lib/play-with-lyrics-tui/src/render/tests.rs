use super::{
    Button, button_at, button_bar, columns_line, columns_line_highlighted, fit, fit_chars,
    is_double_click, scroll_offset, visible_rows,
};
use pretty_assertions::assert_eq;
use std::time::{Duration, SystemTime};
use unicode_width::UnicodeWidthStr;

/// A second click on the same row within the window is a double click; a
/// different row or a late click is not.
#[test]
fn is_double_click_needs_the_same_row_within_the_window() {
    let first = SystemTime::UNIX_EPOCH + Duration::from_secs(1_590_373_467);
    assert!(is_double_click(
        Some((first, 3)),
        first + Duration::from_millis(100),
        3,
    ));
    // Too long after the first click.
    assert!(!is_double_click(
        Some((first, 3)),
        first + Duration::from_millis(600),
        3,
    ));
    // A different row.
    assert!(!is_double_click(
        Some((first, 2)),
        first + Duration::from_millis(100),
        3,
    ));
    // No previous click to pair with.
    assert!(!is_double_click(None, first, 3));
}

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

/// Each output character carries its highlight flag; the padding does not.
#[test]
fn fit_chars_pairs_characters_with_their_highlight() {
    let cells = fit_chars("abc", &[false, true, false], 5);
    assert_eq!(
        cells,
        vec![
            ('a', false),
            ('b', true),
            ('c', false),
            (' ', false),
            (' ', false),
        ],
    );
}

/// The column separators are never highlighted, only the cell characters the
/// mask marks.
#[test]
fn columns_line_highlighted_marks_only_cell_characters() {
    let line = columns_line_highlighted([("ab", &[false, true]), ("", &[]), ("", &[])], 30);
    let marked: String = line
        .iter()
        .filter(|&&(_, on)| on)
        .map(|&(character, _)| character)
        .collect();
    assert_eq!(marked, "b");
}

/// The height-dependent count of title rows reserves the prompt, header,
/// help, and button lines and never drops below one.
#[test]
fn visible_rows_reserves_the_chrome_lines() {
    assert_eq!(visible_rows(24), 20);
    assert_eq!(visible_rows(6), 2);
    assert_eq!(visible_rows(5), 1);
    // A terminal too short for any title row still reports one.
    assert_eq!(visible_rows(4), 1);
    assert_eq!(visible_rows(0), 1);
}

/// The button bar draws each button bracketed, separated by a two-space gap.
#[test]
fn button_bar_brackets_each_button() {
    assert_eq!(button_bar(), "[ Exit ]  [ Go back ]  [ Forward ]");
}

/// A column lands on the button drawn there, and a column in a gap or past the
/// last button lands on none.
#[test]
fn button_at_maps_a_column_to_its_button() {
    // "[ Exit ]" spans columns 0..8.
    assert_eq!(button_at(0), Some(Button::Exit));
    assert_eq!(button_at(7), Some(Button::Exit));
    // The two-column gap before "[ Go back ]" lands on no button.
    assert_eq!(button_at(8), None);
    assert_eq!(button_at(9), None);
    // "[ Go back ]" spans columns 10..21.
    assert_eq!(button_at(10), Some(Button::Back));
    assert_eq!(button_at(20), Some(Button::Back));
    // "[ Forward ]" spans columns 23..34.
    assert_eq!(button_at(23), Some(Button::Forward));
    assert_eq!(button_at(33), Some(Button::Forward));
    // Past the last button lands on none.
    assert_eq!(button_at(34), None);
}
