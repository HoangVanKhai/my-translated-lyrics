//! The layout primitives the selector pages draw with: fitting text to a
//! column budget, laying out the three-column title line, drawing a highlighted
//! line into the frame buffer, and the small geometry helpers the pages share
//! with the click handling.
//!
//! Three unrelated counts meet in this module, and none of them is
//! interchangeable with the others: a display width measured through
//! `unicode-width`, an index into a collection, and a number of screen rows.
//! Screen positions and display widths are the `terminal_screen` geometry
//! types, so they match the buffer a page draws into rather than being
//! remeasured in a different integer. Indices are [`ItemIndex`],
//! [`FilteredIndex`], and [`TitleColumn`], each naming the collection it
//! indexes. Row counts are [`Height`].

use fuzzy_select::selection::{FilteredIndex, ItemIndex};
use pipe_trait::Pipe;
use std::time::{Duration, SystemTime};
use terminal_screen::{Buffer, Column, Height, Row, Style, Width};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// The screen row of the search bar, below the top bar.
pub(crate) const SEARCH_ROW: Row = Row::new(1);

/// The screen row of the column header, below the top bar and the search
/// prompt. The header is clickable, so the renderer and the click handling
/// share this.
pub(crate) const HEADER_ROW: Row = Row::new(2);

/// The screen row of the first title in the table, below the top bar, the
/// search prompt, and the column header. Shared by the renderer and the click
/// handling so they agree on where the rows are.
pub(crate) const FIRST_DATA_ROW: Row = Row::new(3);

/// The screen row of the first item in a list, directly below the top bar,
/// whose centered title names the page.
pub(crate) const FIRST_LIST_ROW: Row = Row::new(1);

/// How close together two clicks on the same row must be to count as a double
/// click, which confirms the choice.
const DOUBLE_CLICK_WINDOW: Duration = Duration::from_millis(500);

/// The display width of `text`, saturating at the widest a `u16` can measure
/// rather than wrapping around on absurdly long input.
fn text_width(text: &str) -> Width {
    text.width().pipe(saturating_width)
}

/// The display width of `character`, counting a character with no column of
/// its own, such as a combining mark, as no columns at all.
fn char_width(character: char) -> Width {
    character.width().unwrap_or(0).pipe(saturating_width)
}

/// A column count measured by `unicode-width`, narrowed to the integer the
/// frame buffer addresses its cells with.
fn saturating_width(columns: usize) -> Width {
    u16::try_from(columns).unwrap_or(u16::MAX).pipe(Width::new)
}

/// Unhighlighted blanks filling `width` columns.
fn blanks(width: Width) -> impl Iterator<Item = (char, bool)> {
    std::iter::repeat_n((' ', false), usize::from(width))
}

/// Pads or truncates `text` to exactly `width` display columns, pairing each
/// resulting character with whether it is highlighted. The `mask` is aligned
/// with `text.chars()`; an out-of-range or missing entry counts as not
/// highlighted, and the ellipsis and padding are never highlighted. Column
/// counts follow the Unicode display width, so a wide glyph such as a CJK
/// ideograph counts as two columns.
pub(crate) fn fit_chars(text: &str, mask: &[bool], width: Width) -> Vec<(char, bool)> {
    let characters: Vec<char> = text.chars().collect();
    let full_width = text_width(text);
    let mut result: Vec<(char, bool)> = Vec::new();
    if full_width <= width {
        for (index, &character) in characters.iter().enumerate() {
            result.push((character, mask.get(index).copied().unwrap_or(false)));
        }
        result.extend(blanks(width.saturating_sub(full_width)));
        return result;
    }
    if width == Width::ZERO {
        return result;
    }
    // Keep whole characters until the next one would not leave room for the
    // one-column ellipsis, then pad the column a wide glyph could not fill.
    let room = width.saturating_sub(Width::ONE);
    let mut used = Width::ZERO;
    for (index, &character) in characters.iter().enumerate() {
        let character_width = char_width(character);
        if used + character_width > room {
            break;
        }
        result.push((character, mask.get(index).copied().unwrap_or(false)));
        used += character_width;
    }
    result.push(('…', false));
    result.extend(blanks(room.saturating_sub(used)));
    result
}

/// Pads or truncates `text` to exactly `width` display columns, appending an
/// ellipsis when it has to cut the text short.
pub(crate) fn fit(text: &str, width: Width) -> String {
    fit_chars(text, &[], width)
        .into_iter()
        .map(|(character, _)| character)
        .collect()
}

/// The separator drawn between the three title cells.
pub(crate) const COLUMN_SEPARATOR: &str = " │ ";

/// The width of each of the three title cells in a line `total` columns wide,
/// once the two separators have taken their share. A cell always keeps at
/// least one column, so a very narrow terminal still shows the layout.
fn cell_width(total: Width) -> Width {
    let separator = text_width(COLUMN_SEPARATOR);
    let available = total.saturating_sub(separator + separator);
    Width::new((available.get() / 3).max(1))
}

/// Lays out three highlighted cells into one line of `total` columns, pairing
/// each character with whether it is highlighted. Separators and padding are
/// never highlighted.
pub(crate) fn columns_line_highlighted(
    cells: [(&str, &[bool]); 3],
    total: Width,
) -> Vec<(char, bool)> {
    let each = cell_width(total);
    let mut line: Vec<(char, bool)> = Vec::new();
    for (index, (text, mask)) in cells.into_iter().enumerate() {
        if index > 0 {
            line.extend(COLUMN_SEPARATOR.chars().map(|character| (character, false)));
        }
        line.extend(fit_chars(text, mask, each));
    }
    line
}

/// Lays out three cells into a single line that fits `total` columns.
pub(crate) fn columns_line(english: &str, vietnamese: &str, chinese: &str, total: Width) -> String {
    columns_line_highlighted([(english, &[]), (vietnamese, &[]), (chinese, &[])], total)
        .into_iter()
        .map(|(character, _)| character)
        .collect()
}

/// A run of screen columns: where it begins and how wide it is. The layout
/// helpers report their runs this way so the renderer and the click handling
/// agree on both ends without either one measuring them again.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ColumnSpan {
    start: Column,
    width: Width,
}

impl ColumnSpan {
    /// The run of `width` columns beginning at `start`.
    pub(crate) fn new(start: Column, width: Width) -> ColumnSpan {
        ColumnSpan { start, width }
    }

    /// The run from `start` up to but not including `end`, which is empty when
    /// `end` is not past `start`.
    fn between(start: Column, end: Column) -> ColumnSpan {
        ColumnSpan::new(start, end.columns_after(start).unwrap_or(Width::ZERO))
    }

    /// The run's leftmost column.
    pub(crate) fn start(self) -> Column {
        self.start
    }

    /// The column just past the run's rightmost one.
    pub(crate) fn end(self) -> Column {
        self.start + self.width
    }

    /// How many columns the run spans.
    pub(crate) fn width(self) -> Width {
        self.width
    }

    /// Whether `column` falls inside the run.
    pub(crate) fn contains(self, column: Column) -> bool {
        self.start <= column && column < self.end()
    }
}

/// The screen columns each of the three title cells spans, matching the layout
/// of [`columns_line`]. The header renderer and the header click handling share
/// this so they agree on where each column sits.
pub(crate) fn column_spans(total: Width) -> [ColumnSpan; 3] {
    let each = cell_width(total);
    let step = each + text_width(COLUMN_SEPARATOR);
    [
        ColumnSpan::new(Column::LEFT, each),
        ColumnSpan::new(Column::LEFT + step, each),
        ColumnSpan::new(Column::LEFT + step + step, each),
    ]
}

/// Which of the three title columns is meant, counting from the left: English,
/// then Vietnamese, then Chinese.
///
/// This indexes the table's own layout rather than the screen columns that
/// layout occupies, so it is deliberately not a [`Column`]. Only [`column_at`]
/// produces one, and only from a screen column that landed inside a cell, so
/// every value names a real column of the table.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct TitleColumn(usize);

impl TitleColumn {
    /// The column's place among the three, for indexing a per-column array
    /// such as the header labels or their spans.
    pub(crate) fn position(self) -> usize {
        self.0
    }
}

/// The title column at screen `column`, for a header `total` columns wide, or
/// `None` for a click on a separator or past the columns.
pub(crate) fn column_at(total: Width, column: Column) -> Option<TitleColumn> {
    total
        .pipe(column_spans)
        .iter()
        .position(|span| span.contains(column))
        .map(TitleColumn)
}

/// Whether a left click on the item at `index` at time `now` completes a double
/// click that began at `previous` (the time and item index of the last click),
/// so the same item was clicked twice within the double-click window. Keying on
/// the item rather than the screen row means a sort or scroll that moves an item
/// between two clicks does not read as a double click.
pub(crate) fn is_double_click(
    previous: Option<(SystemTime, ItemIndex)>,
    now: SystemTime,
    index: ItemIndex,
) -> bool {
    previous.is_some_and(|(when, last_index)| {
        // A backward clock step between the two clicks reads as "not a double
        // click", which is the safe outcome.
        last_index == index
            && now
                .duration_since(when)
                .is_ok_and(|gap| gap <= DOUBLE_CLICK_WINDOW)
    })
}

/// The topmost filtered row to draw so that `cursor` stays visible within a
/// window `visible` rows tall.
pub(crate) fn scroll_offset(cursor: FilteredIndex, visible: Height) -> FilteredIndex {
    cursor
        .get()
        .saturating_sub(usize::from(visible).saturating_sub(1))
        .pipe(FilteredIndex::new)
}

/// The rows the table spends on chrome rather than on titles: the top bar, the
/// search prompt, the column header, and the help line.
const TABLE_CHROME_ROWS: Height = Height::new(4);

/// The number of title rows that fit in a terminal `rows` rows tall, after
/// reserving the chrome. At least one row is always reported, so the table
/// never collapses to nothing.
pub(crate) fn visible_rows(rows: Height) -> Height {
    rows.saturating_sub(TABLE_CHROME_ROWS).max(Height::ONE)
}

/// Draws a line of `(character, highlighted)` pairs into `buffer` at `row` over
/// the row's `base` style, underlining the matched characters. The caller sets
/// `base` to reverse video for the row under the cursor and adds bold for a row
/// under the pointer; the underline composes with either.
pub(crate) fn draw_highlighted_line(
    buffer: &mut Buffer,
    row: Row,
    line: &[(char, bool)],
    base: Style,
) {
    let mut col = Column::LEFT;
    for &(character, highlight) in line {
        let style = if highlight {
            base.with(Style::UNDERLINE)
        } else {
            base
        };
        col += buffer.set_glyph(col, row, character, style);
    }
}

/// A clickable button shown in the top bar, paired with the action a click on
/// it performs.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Button {
    /// Quit the program.
    Exit,
    /// Return to the previous page.
    Back,
    /// Confirm the current item, the same as pressing Enter.
    Forward,
}

/// The gap, in columns, between the Back and Forward buttons on the left.
const BUTTON_GAP: Width = Width::new(2);

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
    fn width(self) -> Width {
        text_width(self.label()) + text_width("[  ]")
    }

    /// The button drawn as `[ label ]`.
    fn draw(self) -> String {
        format!("[ {} ]", self.label())
    }
}

/// The screen columns each top-bar button spans, for a bar `width` columns
/// wide. Back and Forward sit on the left; Exit is right-aligned. The renderer
/// and the click handling share this, so they agree on where each button sits.
pub(crate) fn button_columns(width: Width) -> [(Button, ColumnSpan); 3] {
    let back = ColumnSpan::new(Column::LEFT, Button::Back.width());
    let forward = ColumnSpan::new(back.end() + BUTTON_GAP, Button::Forward.width());
    let right_edge = Column::LEFT + width;
    let exit_start = Column::LEFT + width.saturating_sub(Button::Exit.width());
    [
        (Button::Back, back),
        (Button::Forward, forward),
        (Button::Exit, ColumnSpan::between(exit_start, right_edge)),
    ]
}

/// The top-bar button drawn at screen `column`, if any, for a bar `width`
/// columns wide. A click between or past the buttons lands on none of them.
pub(crate) fn button_at(width: Width, column: Column) -> Option<Button> {
    button_columns(width)
        .into_iter()
        .find_map(|(button, span)| span.contains(column).then_some(button))
}

/// Draws the top bar into `buffer` at the first row: the Back and Forward
/// buttons on the left, the Exit button on the right, and `title` centered
/// between them. When `back_enabled` is false the Back button is disabled,
/// drawn dimmed to show that there is no previous page to return to. The button
/// under the pointer at `hover`, given as a column and a row, is drawn in
/// reverse video, except the disabled Back button, which stays dimmed.
pub(crate) fn render_top_bar(
    buffer: &mut Buffer,
    width: Width,
    title: &str,
    back_enabled: bool,
    hover: Option<(Column, Row)>,
) {
    let buttons = button_columns(width);
    for (button, span) in buttons {
        let disabled = matches!(button, Button::Back) && !back_enabled;
        let hovered = hover.is_some_and(|(col, row)| row == Row::TOP && span.contains(col));
        let style = if disabled {
            Style::DIM
        } else if hovered {
            Style::REVERSE
        } else {
            Style::PLAIN
        };
        buffer.set_string(span.start(), Row::TOP, &button.draw(), style);
    }
    // Center the title in the space between the Forward and Exit buttons,
    // truncating it there if it does not fit.
    let gap = ColumnSpan::between(buttons[1].1.end(), buttons[2].1.start());
    if gap.width() > Width::ZERO {
        let title_width = text_width(title);
        let (column, text) = if title_width >= gap.width() {
            (gap.start(), fit(title, gap.width()))
        } else {
            let indent = Width::new((gap.width().get() - title_width.get()) / 2);
            (gap.start() + indent, title.to_string())
        };
        buffer.set_string(column, Row::TOP, &text, Style::PLAIN);
    }
}

#[cfg(test)]
mod tests;
