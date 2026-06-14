#![cfg_attr(dylint_lib = "perfectionist", feature(register_tool))]
#![cfg_attr(dylint_lib = "perfectionist", register_tool(perfectionist))]

//! The interactive terminal selectors built on `crossterm`.
//!
//! This crate renders the fuzzy table of titles and the simple list
//! selectors for the player, language, and subtitle format. It reads key
//! events and drives a [`Selector`] for the table. Column widths follow the
//! Unicode display width, so a wide glyph such as a CJK ideograph occupies
//! two columns and the columns stay aligned.
//!
//! All drawing goes to standard error, so standard output stays clean. The
//! commands are sent through the `QueueableCommand` and `ExecutableCommand`
//! trait methods rather than the `queue!` and `execute!` macros.

use crossterm::cursor::{Hide, MoveTo, Show};
use crossterm::event::{
    DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers,
    KeyboardEnhancementFlags, MouseButton, MouseEventKind, PopKeyboardEnhancementFlags,
    PushKeyboardEnhancementFlags, read,
};
use crossterm::style::{Attribute, Print, SetAttribute};
use crossterm::terminal::{
    Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
    enable_raw_mode, size, supports_keyboard_enhancement,
};
use crossterm::{ExecutableCommand, QueueableCommand};
use fuzzy_select::fuzzy::match_mask;
use fuzzy_select::selection::Selector;
use lyrics_core::video_descriptor::Language;
use play_with_lyrics::catalog::{Video, language_label};
use std::io::{self, Stderr, Write};
use std::time::{Duration, Instant};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// The outcome of an interactive selector.
#[derive(Debug, PartialEq, Eq)]
pub enum Navigation {
    /// The user chose the item at this index.
    Selected(usize),
    /// The user asked to return to the previous page.
    Back,
    /// The user asked to quit the program.
    Quit,
}

/// Reads the next input event from the terminal.
///
/// This trait is the dependency-injection seam for the interactive loops.
/// Production code runs them with [`Host`], which blocks on the real
/// terminal, while a test runs them with a fake that replays a scripted
/// sequence of key events. That makes the otherwise unreachable event
/// handling testable without a TTY.
trait ReadEvent {
    fn read_event() -> io::Result<Event>;
}

/// Reports the terminal size as `(columns, rows)`.
///
/// This is injected alongside [`ReadEvent`] so a test can render at a chosen
/// size and exercise the width- and height-dependent layout deterministically,
/// without a real terminal.
trait WindowSize {
    fn window_size() -> io::Result<(u16, u16)>;
}

/// Reports the current instant, for measuring the gap between two clicks when
/// detecting a double click.
///
/// Injected alongside [`ReadEvent`] and [`WindowSize`] so a test can freeze
/// time and make double-click detection deterministic, rather than depending
/// on how far apart a throttled machine happens to process the two clicks.
trait Clock {
    fn now() -> Instant;
}

/// The production provider: it reads from the real terminal.
struct Host;

impl ReadEvent for Host {
    fn read_event() -> io::Result<Event> {
        read()
    }
}

impl WindowSize for Host {
    fn window_size() -> io::Result<(u16, u16)> {
        size()
    }
}

impl Clock for Host {
    fn now() -> Instant {
        Instant::now()
    }
}

/// Restores the terminal to its normal state when dropped, even if the
/// caller returns early or panics.
struct TerminalGuard {
    output: Stderr,
    /// Whether the keyboard enhancement protocol was enabled, so it is only
    /// popped when it was pushed.
    enhanced: bool,
}

impl TerminalGuard {
    fn enter() -> io::Result<Self> {
        enable_raw_mode()?;
        let mut output = io::stderr();
        output
            .execute(EnterAlternateScreen)?
            .execute(Hide)?
            .execute(EnableMouseCapture)?;
        // Request the keyboard enhancement protocol so modified keys such as
        // Ctrl-Backspace arrive with their modifier. Terminals that do not
        // support it are left untouched, and Ctrl-Backspace simply has no
        // effect there.
        let enhanced = matches!(supports_keyboard_enhancement(), Ok(true));
        if enhanced {
            output.execute(PushKeyboardEnhancementFlags(
                KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES,
            ))?;
        }
        Ok(TerminalGuard { output, enhanced })
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        // Best effort: there is nothing useful to do if restoring fails.
        if self.enhanced {
            let _ = self.output.execute(PopKeyboardEnhancementFlags);
        }
        let _ = self.output.execute(DisableMouseCapture);
        let _ = self.output.execute(Show);
        let _ = self.output.execute(LeaveAlternateScreen);
        let _ = disable_raw_mode();
    }
}

/// Pads or truncates `text` to exactly `width` display columns, pairing each
/// resulting character with whether it is highlighted. The `mask` is aligned
/// with `text.chars()`; an out-of-range or missing entry counts as not
/// highlighted, and the ellipsis and padding are never highlighted. Column
/// counts follow the Unicode display width, so a wide glyph such as a CJK
/// ideograph counts as two columns.
fn fit_chars(text: &str, mask: &[bool], width: usize) -> Vec<(char, bool)> {
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
fn fit(text: &str, width: usize) -> String {
    fit_chars(text, &[], width)
        .into_iter()
        .map(|(character, _)| character)
        .collect()
}

/// Lays out three highlighted cells into one line of `total` columns, pairing
/// each character with whether it is highlighted. Separators and padding are
/// never highlighted.
fn columns_line_highlighted(cells: [(&str, &[bool]); 3], total: usize) -> Vec<(char, bool)> {
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
fn columns_line(english: &str, vietnamese: &str, chinese: &str, total: usize) -> String {
    columns_line_highlighted([(english, &[]), (vietnamese, &[]), (chinese, &[])], total)
        .into_iter()
        .map(|(character, _)| character)
        .collect()
}

/// The screen row of the first title in the table, below the search prompt
/// and the column header. Shared by the renderer and the click handling so
/// they agree on where the rows are.
const DATA_ROW_OFFSET: usize = 2;

/// The screen row of the first item in a list, below its single prompt line.
const LIST_ROW_OFFSET: usize = 1;

/// How close together two clicks on the same row must be to count as a double
/// click, which confirms the choice.
const DOUBLE_CLICK_WINDOW: Duration = Duration::from_millis(500);

/// Whether a left click at `row` and `now` completes a double click that began
/// at `previous` (the time and row of the last click), so the same row was
/// clicked twice within the double-click window.
fn is_double_click(previous: Option<(Instant, u16)>, now: Instant, row: u16) -> bool {
    previous.is_some_and(|(when, last_row)| {
        last_row == row && now.duration_since(when) <= DOUBLE_CLICK_WINDOW
    })
}

/// The first row offset that keeps `cursor` visible within `visible` rows.
fn scroll_offset(cursor: usize, visible: usize) -> usize {
    cursor.saturating_sub(visible.saturating_sub(1))
}

/// The number of title rows that fit in a terminal `rows` rows tall, after
/// reserving the prompt line, the header line, and the help line. At least
/// one row is always reported, so the table never collapses to nothing.
fn visible_rows(rows: usize) -> usize {
    rows.saturating_sub(3).max(1)
}

/// Prints a line of `(character, highlighted)` pairs, underlining the
/// highlighted characters. When `reverse` is set the whole line is drawn in
/// reverse video, for the row under the cursor; the underline composes with
/// it. A single reset at the end clears both attributes.
fn print_highlighted_line(
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

/// Presents the fuzzy table of titles and reports the chosen row, a request
/// to go back, or a request to quit. This is the first page, so going back
/// from an empty query is the way out of it.
pub fn select_video(
    videos: &[Video],
    query: &mut String,
    selected: Option<usize>,
) -> io::Result<Navigation> {
    let mut guard = TerminalGuard::enter()?;
    select_video_loop::<Host>(&mut guard.output, videos, query, selected)
}

/// Drives the fuzzy-table selector, reading events from `Sys`. Splitting
/// this out from [`select_video`] lets a test replay scripted events and
/// render to a buffer, exercising the loop without a terminal. `query` seeds
/// the search box and receives the final text, and `selected` is the original
/// index to highlight at first, so a previous visit can be restored.
fn select_video_loop<Sys>(
    output: &mut impl Write,
    videos: &[Video],
    query: &mut String,
    selected: Option<usize>,
) -> io::Result<Navigation>
where
    Sys: ReadEvent + WindowSize + Clock,
{
    let mut selector = Selector::new(videos);
    selector.set_query(query);
    if let Some(index) = selected {
        selector.focus(index);
    }
    let mut last_click: Option<(Instant, u16)> = None;
    let outcome = loop {
        render_table::<Sys>(output, &selector, videos)?;
        match Sys::read_event()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                KeyCode::Esc => break Navigation::Quit,
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    break Navigation::Quit;
                }
                // Ctrl-Q quits. Both cases are matched so that Shift or Caps
                // Lock, which we cannot reliably tell apart, never change this.
                KeyCode::Char('q' | 'Q') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    break Navigation::Quit;
                }
                KeyCode::Up => selector.move_up(),
                KeyCode::Down => selector.move_down(),
                // Ctrl-Backspace goes back. Plain Backspace only deletes, so
                // holding it to clear the search box never exits the page.
                KeyCode::Backspace if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    break Navigation::Back;
                }
                KeyCode::Backspace => selector.pop_char(),
                KeyCode::Char(char) => selector.push_char(char),
                KeyCode::Enter => {
                    if let Some(index) = selector.selected_index() {
                        break Navigation::Selected(index);
                    }
                }
                _ => {}
            },
            Event::Mouse(mouse) => match mouse.kind {
                MouseEventKind::ScrollUp => selector.move_up(),
                MouseEventKind::ScrollDown => selector.move_down(),
                // A single click highlights the video on the clicked row; a
                // double click on the same row also selects it.
                MouseEventKind::Down(MouseButton::Left) => {
                    let (_, rows) = Sys::window_size().unwrap_or((80, 24));
                    let visible = visible_rows(rows as usize);
                    let offset = scroll_offset(selector.cursor(), visible);
                    let clicked = (mouse.row as usize).checked_sub(DATA_ROW_OFFSET).and_then(
                        |screen_index| selector.filtered().get(offset + screen_index).copied(),
                    );
                    if let Some(index) = clicked {
                        let now = Sys::now();
                        let confirm = is_double_click(last_click, now, mouse.row);
                        last_click = Some((now, mouse.row));
                        selector.focus(index);
                        if confirm {
                            break Navigation::Selected(index);
                        }
                    }
                }
                _ => {}
            },
            _ => {}
        }
    };
    // Hand the final query back so the caller can restore it on a later visit.
    *query = selector.query().to_string();
    Ok(outcome)
}

fn render_table<Sys>(
    output: &mut impl Write,
    selector: &Selector<Video>,
    videos: &[Video],
) -> io::Result<()>
where
    Sys: WindowSize,
{
    let (columns, rows) = Sys::window_size().unwrap_or((80, 24));
    let columns = columns as usize;
    let rows = rows as usize;

    output.queue(Clear(ClearType::All))?;

    let prompt = format!("Search: {}", selector.query());
    output
        .queue(MoveTo(0, 0))?
        .queue(Print(fit(&prompt, columns)))?;

    let header = columns_line(
        language_label(Language::English),
        language_label(Language::Vietnamese),
        language_label(Language::Chinese),
        columns,
    );
    output
        .queue(MoveTo(0, 1))?
        .queue(SetAttribute(Attribute::Bold))?
        .queue(Print(header))?
        .queue(SetAttribute(Attribute::Reset))?;

    let filtered = selector.filtered();
    let cursor = selector.cursor();
    let visible = visible_rows(rows);
    let offset = scroll_offset(cursor, visible);

    let query = selector.query();
    for (screen_index, filtered_position) in
        (offset..filtered.len().min(offset + visible)).enumerate()
    {
        let video = &videos[filtered[filtered_position]];
        let english = video.title(Language::English).unwrap_or("");
        let vietnamese = video.title(Language::Vietnamese).unwrap_or("");
        let chinese = video.title(Language::Chinese).unwrap_or("");
        let line = columns_line_highlighted(
            [
                (english, &match_mask(english, query)),
                (vietnamese, &match_mask(vietnamese, query)),
                (chinese, &match_mask(chinese, query)),
            ],
            columns,
        );
        let screen_y = (screen_index + DATA_ROW_OFFSET) as u16;
        output.queue(MoveTo(0, screen_y))?;
        print_highlighted_line(output, &line, filtered_position == cursor)?;
    }

    let help = "↑/↓ move · type to search · ⌫ delete · ^⌫ back · ⏎ select · Esc/^Q quit";
    output
        .queue(MoveTo(0, rows.saturating_sub(1) as u16))?
        .queue(SetAttribute(Attribute::Dim))?
        .queue(Print(fit(help, columns)))?
        .queue(SetAttribute(Attribute::Reset))?;

    output.flush()
}

#[cfg(test)]
mod tests;

/// Presents a simple single-column list of `labels` under `prompt` and
/// reports the chosen item, a request to go back to the previous page, or a
/// request to quit.
pub fn select_one(prompt: &str, labels: &[String], start: usize) -> io::Result<Navigation> {
    let mut guard = TerminalGuard::enter()?;
    select_one_loop::<Host>(&mut guard.output, prompt, labels, start)
}

/// Drives the single-column list selector, reading events from `Sys`.
/// Splitting this out from [`select_one`] lets a test replay scripted
/// events and render to a buffer, exercising the loop without a terminal.
/// `start` is the row to highlight at first, to restore a previous choice.
fn select_one_loop<Sys>(
    output: &mut impl Write,
    prompt: &str,
    labels: &[String],
    start: usize,
) -> io::Result<Navigation>
where
    Sys: ReadEvent + WindowSize + Clock,
{
    let mut cursor = start.min(labels.len().saturating_sub(1));
    let mut last_click: Option<(Instant, u16)> = None;
    loop {
        render_list::<Sys>(output, prompt, labels, cursor)?;
        match Sys::read_event()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                KeyCode::Esc => return Ok(Navigation::Quit),
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    return Ok(Navigation::Quit);
                }
                // This list has no text entry, so a bare Q quits as well as
                // Ctrl-Q. Both cases match, so neither Shift nor Caps Lock
                // changes the behavior.
                KeyCode::Char('q' | 'Q') => return Ok(Navigation::Quit),
                // With nothing to type, Backspace goes back to the previous page.
                KeyCode::Backspace => return Ok(Navigation::Back),
                KeyCode::Up => cursor = cursor.saturating_sub(1),
                KeyCode::Down => {
                    if cursor + 1 < labels.len() {
                        cursor += 1;
                    }
                }
                // With no text to type, Space confirms the choice like Enter.
                KeyCode::Enter | KeyCode::Char(' ') => {
                    if !labels.is_empty() {
                        return Ok(Navigation::Selected(cursor));
                    }
                }
                _ => {}
            },
            Event::Mouse(mouse) => match mouse.kind {
                MouseEventKind::ScrollUp => cursor = cursor.saturating_sub(1),
                MouseEventKind::ScrollDown => {
                    if cursor + 1 < labels.len() {
                        cursor += 1;
                    }
                }
                // A single click highlights the label on the clicked row; a
                // double click on the same row also selects it.
                MouseEventKind::Down(MouseButton::Left) => {
                    let clicked = (mouse.row as usize).checked_sub(LIST_ROW_OFFSET);
                    if let Some(index) = clicked
                        && index < labels.len()
                    {
                        let now = Sys::now();
                        let confirm = is_double_click(last_click, now, mouse.row);
                        last_click = Some((now, mouse.row));
                        cursor = index;
                        if confirm {
                            return Ok(Navigation::Selected(index));
                        }
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }
}

fn render_list<Sys>(
    output: &mut impl Write,
    prompt: &str,
    labels: &[String],
    cursor: usize,
) -> io::Result<()>
where
    Sys: WindowSize,
{
    let (columns, rows) = Sys::window_size().unwrap_or((80, 24));
    let columns = columns as usize;

    output.queue(Clear(ClearType::All))?;
    output
        .queue(MoveTo(0, 0))?
        .queue(SetAttribute(Attribute::Bold))?
        .queue(Print(fit(prompt, columns)))?
        .queue(SetAttribute(Attribute::Reset))?;

    for (index, label) in labels.iter().enumerate() {
        let screen_y = (index + LIST_ROW_OFFSET) as u16;
        let line = fit(label, columns);
        output.queue(MoveTo(0, screen_y))?;
        if index == cursor {
            output
                .queue(SetAttribute(Attribute::Reverse))?
                .queue(Print(line))?
                .queue(SetAttribute(Attribute::Reset))?;
        } else {
            output.queue(Print(line))?;
        }
    }

    let help = "↑/↓ move · ⌫ back · ⏎/Space select · Esc/^Q quit";
    output
        .queue(MoveTo(0, rows.saturating_sub(1)))?
        .queue(SetAttribute(Attribute::Dim))?
        .queue(Print(fit(help, columns)))?
        .queue(SetAttribute(Attribute::Reset))?;

    output.flush()
}
