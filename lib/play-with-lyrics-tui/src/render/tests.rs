use super::{
    Button, button_at, columns_line, columns_line_highlighted, fit, fit_chars, is_double_click,
    render_top_bar, scroll_offset, visible_rows,
};
use pretty_assertions::assert_eq;
use std::time::{Duration, SystemTime};
use terminal_screen::{Buffer, Style};
use unicode_width::UnicodeWidthStr;

/// A second click on the same item within the window is a double click; a
/// different item or a late click is not.
#[test]
fn is_double_click_needs_the_same_item_within_the_window() {
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
    // A different item, e.g. because a sort moved a new item under the pointer.
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

/// The top bar draws the three buttons and the centered title.
#[test]
fn top_bar_draws_the_buttons_and_title() {
    let mut buffer = Buffer::new(80, 1);
    render_top_bar(&mut buffer, 80, "Play with Lyrics", true, None);
    let row = buffer.row_text(0);
    assert!(row.contains("[ ← Go back ]"), "{row}");
    assert!(row.contains("[ → Forward ]"), "{row}");
    assert!(row.contains("[ ✕ Exit ]"), "{row}");
    assert!(row.contains("Play with Lyrics"), "{row}");
    // With going back available and no pointer over it, the Back button is
    // drawn plainly.
    assert_eq!(buffer.style_at(0, 0), Style::PLAIN);
}

/// When going back is disabled, the Go back button is drawn dimmed.
#[test]
fn top_bar_dims_the_disabled_go_back_button() {
    let mut buffer = Buffer::new(80, 1);
    render_top_bar(&mut buffer, 80, "Play with Lyrics", false, None);
    // Every cell of the Go back button, here its opening bracket, carries the
    // dim style.
    assert_eq!(buffer.style_at(0, 0), Style::DIM);
}

/// A button under the pointer is drawn in reverse video.
#[test]
fn top_bar_reverses_the_hovered_button() {
    let mut buffer = Buffer::new(80, 1);
    // Column 5 on the top row falls on the Go back button.
    render_top_bar(&mut buffer, 80, "Play with Lyrics", true, Some((5, 0)));
    assert_eq!(buffer.style_at(0, 0), Style::REVERSE);
}

/// The disabled Back button stays dimmed even under the pointer.
#[test]
fn top_bar_keeps_the_disabled_button_dimmed_under_the_pointer() {
    let mut buffer = Buffer::new(80, 1);
    render_top_bar(&mut buffer, 80, "Play with Lyrics", false, Some((5, 0)));
    assert_eq!(buffer.style_at(0, 0), Style::DIM);
}

/// In an 80-column bar, a column lands on the button drawn there. Back and
/// Forward sit on the left; Exit is right-aligned. A column in a gap or past
/// the last button lands on none.
#[test]
fn button_at_maps_a_column_to_its_button() {
    // "[ ← Go back ]" spans columns 0..13.
    assert_eq!(button_at(80, 0), Some(Button::Back));
    assert_eq!(button_at(80, 12), Some(Button::Back));
    // The two-column gap before Forward lands on no button.
    assert_eq!(button_at(80, 13), None);
    assert_eq!(button_at(80, 14), None);
    // "[ → Forward ]" spans columns 15..28.
    assert_eq!(button_at(80, 15), Some(Button::Forward));
    assert_eq!(button_at(80, 27), Some(Button::Forward));
    // The space between Forward and the right-aligned Exit lands on no button.
    assert_eq!(button_at(80, 28), None);
    assert_eq!(button_at(80, 69), None);
    // "[ ✕ Exit ]" is right-aligned, spanning columns 70..80.
    assert_eq!(button_at(80, 70), Some(Button::Exit));
    assert_eq!(button_at(80, 79), Some(Button::Exit));
}
