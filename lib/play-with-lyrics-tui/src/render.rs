//! The layout and printing primitives the selector pages draw with: fitting
//! text to a column budget, laying out the three-column title line, printing a
//! highlighted line, and the small geometry helpers the pages share with the
//! click handling.

use crossterm::QueueableCommand;
use crossterm::cursor::MoveTo;
use crossterm::style::{Attribute, Print, SetAttribute};
use std::io::{self, Write};
use std::ops::Range;
use std::time::{Duration, SystemTime};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// The title of the program, shown in the center of the top bar.
pub(crate) const PROGRAM_TITLE: &str = "Play with Lyrics";

/// The screen row of the first title in the table, below the top bar, the
/// search prompt, and the column header. Shared by the renderer and the click
/// handling so they agree on where the rows are.
pub(crate) const DATA_ROW_OFFSET: usize = 3;

/// The screen row of the first item in a list, below the top bar and the
/// single prompt line.
pub(crate) const LIST_ROW_OFFSET: usize = 2;

/// How close together two clicks on the same row must be to count as a double
/// click, which confirms the choice.
const DOUBLE_CLICK_WINDOW: Duration = Duration::from_millis(500);

/// Pads or truncates `text` to exactly `width` display columns, pairing each
/// resulting character with whether it is highlighted. The `mask` is aligned
/// with `text.chars()`; an out-of-range or missing entry counts as not
/// highlighted, and the ellipsis and padding are never highlighted. Column
/// counts follow the Unicode display width, so a wide glyph such as a CJK
/// ideograph counts as two columns.
pub(crate) fn fit_chars(text: &str, mask: &[bool], width: usize) -> Vec<(char, bool)> {
    let characters: Vec<char> = text.chars().collect();
    let text_width = text.width();
    let mut result: Vec<(char, bool)> = Vec::new();
    if text_width <= width {
        for (index, &character) in characters.iter().enumerate() {
            result.push((character, mask.get(index).copied().unwrap_or(false)));
        }
        result.extend(std::iter::repeat_n((' ', false), width - text_width));
        return result;
    }
    if width == 0 {
        return result;
    }
    // Keep whole characters until the next one would not leave room for the
    // one-column ellipsis, then pad the column a wide glyph could not fill.
    let mut used = 0;
    for (index, &character) in characters.iter().enumerate() {
        let character_width = character.width().unwrap_or(0);
        if used + character_width > width - 1 {
            break;
        }
        result.push((character, mask.get(index).copied().unwrap_or(false)));
        used += character_width;
    }
    result.push(('…', false));
    result.extend(std::iter::repeat_n((' ', false), width - used - 1));
    result
}

/// Pads or truncates `text` to exactly `width` display columns, appending an
/// ellipsis when it has to cut the text short.
pub(crate) fn fit(text: &str, width: usize) -> String {
    fit_chars(text, &[], width)
        .into_iter()
        .map(|(character, _)| character)
        .collect()
}

/// Lays out three highlighted cells into one line of `total` columns, pairing
/// each character with whether it is highlighted. Separators and padding are
/// never highlighted.
pub(crate) fn columns_line_highlighted(
    cells: [(&str, &[bool]); 3],
    total: usize,
) -> Vec<(char, bool)> {
    let separator = " │ ";
    let available = total.saturating_sub(separator.width() * 2);
    let each = (available / 3).max(1);
    let mut line: Vec<(char, bool)> = Vec::new();
    for (index, (text, mask)) in cells.into_iter().enumerate() {
        if index > 0 {
            line.extend(separator.chars().map(|character| (character, false)));
        }
        line.extend(fit_chars(text, mask, each));
    }
    line
}

/// Lays out three cells into a single line that fits `total` columns.
pub(crate) fn columns_line(english: &str, vietnamese: &str, chinese: &str, total: usize) -> String {
    columns_line_highlighted([(english, &[]), (vietnamese, &[]), (chinese, &[])], total)
        .into_iter()
        .map(|(character, _)| character)
        .collect()
}

/// Whether a left click at `row` and `now` completes a double click that began
/// at `previous` (the time and row of the last click), so the same row was
/// clicked twice within the double-click window.
pub(crate) fn is_double_click(
    previous: Option<(SystemTime, u16)>,
    now: SystemTime,
    row: u16,
) -> bool {
    previous.is_some_and(|(when, last_row)| {
        // A backward clock step between the two clicks reads as "not a double
        // click", which is the safe outcome.
        last_row == row
            && now
                .duration_since(when)
                .is_ok_and(|gap| gap <= DOUBLE_CLICK_WINDOW)
    })
}

/// The first row offset that keeps `cursor` visible within `visible` rows.
pub(crate) fn scroll_offset(cursor: usize, visible: usize) -> usize {
    cursor.saturating_sub(visible.saturating_sub(1))
}

/// The number of title rows that fit in a terminal `rows` rows tall, after
/// reserving the top bar, the prompt line, the header line, and the help line.
/// At least one row is always reported, so the table never collapses to
/// nothing.
pub(crate) fn visible_rows(rows: usize) -> usize {
    rows.saturating_sub(4).max(1)
}

/// Prints a line of `(character, highlighted)` pairs, underlining the
/// highlighted characters. When `reverse` is set the whole line is drawn in
/// reverse video, for the row under the cursor; the underline composes with
/// it. A single reset at the end clears both attributes.
pub(crate) fn print_highlighted_line(
    output: &mut impl Write,
    line: &[(char, bool)],
    reverse: bool,
) -> io::Result<()> {
    if reverse {
        output.queue(SetAttribute(Attribute::Reverse))?;
    }
    let mut underlined = false;
    let mut run = String::new();
    for &(character, highlight) in line {
        if highlight != underlined {
            if !run.is_empty() {
                output.queue(Print(std::mem::take(&mut run)))?;
            }
            underlined = highlight;
            let attribute = if underlined {
                Attribute::Underlined
            } else {
                Attribute::NoUnderline
            };
            output.queue(SetAttribute(attribute))?;
        }
        run.push(character);
    }
    if !run.is_empty() {
        output.queue(Print(run))?;
    }
    output.queue(SetAttribute(Attribute::Reset))?;
    Ok(())
}

/// A clickable button shown in the top bar, paired with the action a click on
/// it performs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Button {
    /// Quit the program.
    Exit,
    /// Return to the previous page.
    Back,
    /// Confirm the current item, the same as pressing Enter.
    Forward,
}

/// The gap, in columns, between the Back and Forward buttons on the left.
const BUTTON_GAP: usize = 2;

impl Button {
    /// The text shown inside the button's brackets, led by a symbol.
    pub(crate) fn label(self) -> &'static str {
        match self {
            Button::Exit => "✕ Exit",
            Button::Back => "← Go back",
            Button::Forward => "→ Forward",
        }
    }

    /// The number of columns the drawn button occupies, counting the brackets
    /// and the single space of padding on each side of the label.
    fn width(self) -> usize {
        self.label().width() + "[  ]".width()
    }

    /// The button drawn as `[ label ]`.
    fn draw(self) -> String {
        format!("[ {} ]", self.label())
    }
}

/// The screen columns each top-bar button spans, as half-open `[start, end)`
/// ranges, for a bar `width` columns wide. Back and Forward sit on the left;
/// Exit is right-aligned. The renderer and the click handling share this, so
/// they agree on where each button sits.
pub(crate) fn button_columns(width: usize) -> [(Button, Range<usize>); 3] {
    let back = 0..Button::Back.width();
    let forward_start = back.end + BUTTON_GAP;
    let forward = forward_start..forward_start + Button::Forward.width();
    let exit_start = width.saturating_sub(Button::Exit.width());
    let exit = exit_start..width;
    [
        (Button::Back, back),
        (Button::Forward, forward),
        (Button::Exit, exit),
    ]
}

/// The top-bar button drawn at screen `column`, if any, for a bar `width`
/// columns wide. A click between or past the buttons lands on none of them.
pub(crate) fn button_at(width: usize, column: usize) -> Option<Button> {
    button_columns(width)
        .into_iter()
        .find_map(|(button, range)| range.contains(&column).then_some(button))
}

/// Draws the top bar at the first row: the Back and Forward buttons on the
/// left, the Exit button on the right, and `title` centered between them. When
/// `back_enabled` is false the Back button is disabled, drawn dimmed to show
/// that there is no previous page to return to.
pub(crate) fn render_top_bar(
    output: &mut impl Write,
    width: usize,
    title: &str,
    back_enabled: bool,
) -> io::Result<()> {
    let columns = button_columns(width);
    for (button, range) in &columns {
        output.queue(MoveTo(range.start as u16, 0))?;
        if matches!(button, Button::Back) && !back_enabled {
            output
                .queue(SetAttribute(Attribute::Dim))?
                .queue(Print(button.draw()))?
                .queue(SetAttribute(Attribute::Reset))?;
        } else {
            output.queue(Print(button.draw()))?;
        }
    }
    // Center the title in the space between the Forward and Exit buttons.
    let gap_start = columns[1].1.end;
    let gap_end = columns[2].1.start;
    if gap_end > gap_start {
        let region = gap_end - gap_start;
        let title_width = title.width();
        let centered = if title_width >= region {
            fit(title, region)
        } else {
            format!("{}{title}", " ".repeat((region - title_width) / 2))
        };
        output
            .queue(MoveTo(gap_start as u16, 0))?
            .queue(Print(centered))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
