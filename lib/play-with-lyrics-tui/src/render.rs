//! The layout and printing primitives the selector pages draw with: fitting
//! text to a column budget, laying out the three-column title line, printing a
//! highlighted line, and the small geometry helpers the pages share with the
//! click handling.

use crossterm::QueueableCommand;
use crossterm::style::{Attribute, Print, SetAttribute};
use std::io::{self, Write};
use std::time::{Duration, SystemTime};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// The screen row of the first title in the table, below the search prompt
/// and the column header. Shared by the renderer and the click handling so
/// they agree on where the rows are.
pub(crate) const DATA_ROW_OFFSET: usize = 2;

/// The screen row of the first item in a list, below its single prompt line.
pub(crate) const LIST_ROW_OFFSET: usize = 1;

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
/// reserving the prompt line, the header line, the help line, and the button
/// line. At least one row is always reported, so the table never collapses to
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

/// A clickable button shown in the footer, paired with the action a click on
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

/// The footer buttons in the order they are shown, from left to right.
pub(crate) const FOOTER_BUTTONS: [Button; 3] = [Button::Exit, Button::Back, Button::Forward];

/// The gap, in columns, between adjacent footer buttons.
const BUTTON_GAP: usize = 2;

impl Button {
    /// The text shown inside the button's brackets.
    pub(crate) fn label(self) -> &'static str {
        match self {
            Button::Exit => "Exit",
            Button::Back => "Go back",
            Button::Forward => "Forward",
        }
    }

    /// The number of columns the drawn button occupies, counting the brackets
    /// and the single space of padding on each side of the label.
    fn width(self) -> usize {
        self.label().width() + "[  ]".width()
    }
}

/// The footer button bar, drawn as bracketed buttons separated by a gap. The
/// click handling locates a click within it with [`button_at`], so the two
/// stay in step through the shared [`FOOTER_BUTTONS`] order and widths.
pub(crate) fn button_bar() -> String {
    FOOTER_BUTTONS
        .iter()
        .map(|button| format!("[ {} ]", button.label()))
        .collect::<Vec<_>>()
        .join(&" ".repeat(BUTTON_GAP))
}

/// The footer button drawn at screen `column`, if any. A click between the
/// buttons, in a gap, lands on none of them and returns `None`.
pub(crate) fn button_at(column: usize) -> Option<Button> {
    let mut start = 0;
    for (index, &button) in FOOTER_BUTTONS.iter().enumerate() {
        if index > 0 {
            start += BUTTON_GAP;
        }
        let end = start + button.width();
        if (start..end).contains(&column) {
            return Some(button);
        }
        start = end;
    }
    None
}

#[cfg(test)]
mod tests;
