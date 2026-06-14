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
use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers, read};
use crossterm::style::{Attribute, Print, SetAttribute};
use crossterm::terminal::{
    Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
    enable_raw_mode, size,
};
use crossterm::{ExecutableCommand, QueueableCommand};
use fuzzy_select::selection::Selector;
use lyrics_core::video_descriptor::Language;
use play_with_lyrics::catalog::{Video, language_label};
use std::io::{self, Stderr, Write};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

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

/// Restores the terminal to its normal state when dropped, even if the
/// caller returns early or panics.
struct TerminalGuard {
    output: Stderr,
}

impl TerminalGuard {
    fn enter() -> io::Result<Self> {
        enable_raw_mode()?;
        let mut output = io::stderr();
        output.execute(EnterAlternateScreen)?.execute(Hide)?;
        Ok(TerminalGuard { output })
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        // Best effort: there is nothing useful to do if restoring fails.
        let _ = self.output.execute(Show);
        let _ = self.output.execute(LeaveAlternateScreen);
        let _ = disable_raw_mode();
    }
}

/// Pads or truncates `text` to exactly `width` display columns, appending
/// an ellipsis when it has to cut the text short. Column counts follow the
/// Unicode display width, so a wide glyph such as a CJK ideograph counts as
/// two columns.
fn fit(text: &str, width: usize) -> String {
    let text_width = text.width();
    if text_width <= width {
        let mut padded = text.to_string();
        padded.extend(std::iter::repeat_n(' ', width - text_width));
        return padded;
    }
    if width == 0 {
        return String::new();
    }
    // Keep whole characters until the next one would not leave room for the
    // one-column ellipsis, then pad the column a wide glyph could not fill.
    let mut truncated = String::new();
    let mut used = 0;
    for character in text.chars() {
        let character_width = character.width().unwrap_or(0);
        if used + character_width > width - 1 {
            break;
        }
        truncated.push(character);
        used += character_width;
    }
    truncated.push('…');
    truncated.extend(std::iter::repeat_n(' ', width - used - 1));
    truncated
}

/// Lays out three cells into a single line that fits `total` columns.
fn columns_line(english: &str, vietnamese: &str, chinese: &str, total: usize) -> String {
    let separator = " │ ";
    let available = total.saturating_sub(separator.width() * 2);
    let each = (available / 3).max(1);
    format!(
        "{}{separator}{}{separator}{}",
        fit(english, each),
        fit(vietnamese, each),
        fit(chinese, each),
    )
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

/// Presents the fuzzy table of titles and returns the index, into `videos`,
/// of the chosen row. Returns `None` when the user cancels.
pub fn select_video(videos: &[Video]) -> io::Result<Option<usize>> {
    let mut guard = TerminalGuard::enter()?;
    select_video_loop::<Host>(&mut guard.output, videos)
}

/// Drives the fuzzy-table selector, reading events from `Sys`. Splitting
/// this out from [`select_video`] lets a test replay scripted events and
/// render to a buffer, exercising the loop without a terminal.
fn select_video_loop<Sys>(output: &mut impl Write, videos: &[Video]) -> io::Result<Option<usize>>
where
    Sys: ReadEvent + WindowSize,
{
    let mut selector = Selector::new(videos);
    loop {
        render_table::<Sys>(output, &selector, videos)?;
        let Event::Key(key) = Sys::read_event()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }
        match key.code {
            KeyCode::Esc => return Ok(None),
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                return Ok(None);
            }
            // Ctrl-Q quits. Both cases are matched so that Shift or Caps
            // Lock, which we cannot reliably tell apart, never change this.
            KeyCode::Char('q' | 'Q') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                return Ok(None);
            }
            KeyCode::Up => selector.move_up(),
            KeyCode::Down => selector.move_down(),
            KeyCode::Backspace => selector.pop_char(),
            KeyCode::Char(char) => selector.push_char(char),
            KeyCode::Enter => {
                if let Some(index) = selector.selected_index() {
                    return Ok(Some(index));
                }
            }
            _ => {}
        }
    }
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

    for (screen_index, filtered_position) in
        (offset..filtered.len().min(offset + visible)).enumerate()
    {
        let video = &videos[filtered[filtered_position]];
        let line = columns_line(
            video.title(Language::English).unwrap_or(""),
            video.title(Language::Vietnamese).unwrap_or(""),
            video.title(Language::Chinese).unwrap_or(""),
            columns,
        );
        let screen_y = (screen_index + 2) as u16;
        output.queue(MoveTo(0, screen_y))?;
        if filtered_position == cursor {
            output
                .queue(SetAttribute(Attribute::Reverse))?
                .queue(Print(line))?
                .queue(SetAttribute(Attribute::Reset))?;
        } else {
            output.queue(Print(line))?;
        }
    }

    let help = "↑/↓ move · type to filter · Enter select · Esc cancel";
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
/// returns the index of the chosen item. Returns `None` when the user
/// cancels.
pub fn select_one(prompt: &str, labels: &[String]) -> io::Result<Option<usize>> {
    let mut guard = TerminalGuard::enter()?;
    select_one_loop::<Host>(&mut guard.output, prompt, labels)
}

/// Drives the single-column list selector, reading events from `Sys`.
/// Splitting this out from [`select_one`] lets a test replay scripted
/// events and render to a buffer, exercising the loop without a terminal.
fn select_one_loop<Sys>(
    output: &mut impl Write,
    prompt: &str,
    labels: &[String],
) -> io::Result<Option<usize>>
where
    Sys: ReadEvent + WindowSize,
{
    let mut cursor = 0;
    loop {
        render_list::<Sys>(output, prompt, labels, cursor)?;
        let Event::Key(key) = Sys::read_event()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }
        match key.code {
            KeyCode::Esc => return Ok(None),
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                return Ok(None);
            }
            // This list has no text entry, so a bare Q quits as well as
            // Ctrl-Q. Both cases match, so neither Shift nor Caps Lock
            // changes the behavior.
            KeyCode::Char('q' | 'Q') => return Ok(None),
            KeyCode::Up => cursor = cursor.saturating_sub(1),
            KeyCode::Down => {
                if cursor + 1 < labels.len() {
                    cursor += 1;
                }
            }
            KeyCode::Enter => {
                if !labels.is_empty() {
                    return Ok(Some(cursor));
                }
            }
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
    let (columns, _) = Sys::window_size().unwrap_or((80, 24));
    let columns = columns as usize;

    output.queue(Clear(ClearType::All))?;
    output
        .queue(MoveTo(0, 0))?
        .queue(SetAttribute(Attribute::Bold))?
        .queue(Print(fit(prompt, columns)))?
        .queue(SetAttribute(Attribute::Reset))?;

    for (index, label) in labels.iter().enumerate() {
        let screen_y = (index + 1) as u16;
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

    output.flush()
}
