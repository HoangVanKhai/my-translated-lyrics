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
use play_with_lyrics::catalog::Video;
use std::io::{self, Stderr, Write};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

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

/// Presents the fuzzy table of titles and returns the index, into `videos`,
/// of the chosen row. Returns `None` when the user cancels.
pub fn select_video(videos: &[Video]) -> io::Result<Option<usize>> {
    let mut guard = TerminalGuard::enter()?;
    let mut selector = Selector::new(videos);
    loop {
        render_table(&mut guard.output, &selector, videos)?;
        let Event::Key(key) = read()? else {
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

fn render_table(
    output: &mut Stderr,
    selector: &Selector<Video>,
    videos: &[Video],
) -> io::Result<()> {
    let (columns, rows) = size().unwrap_or((80, 24));
    let columns = columns as usize;
    let rows = rows as usize;

    output.queue(Clear(ClearType::All))?;

    let prompt = format!("Search: {}", selector.query());
    output
        .queue(MoveTo(0, 0))?
        .queue(Print(fit(&prompt, columns)))?;

    let header = columns_line("English", "Vietnamese", "Chinese", columns);
    output
        .queue(MoveTo(0, 1))?
        .queue(SetAttribute(Attribute::Bold))?
        .queue(Print(header))?
        .queue(SetAttribute(Attribute::Reset))?;

    let filtered = selector.filtered();
    let cursor = selector.cursor();
    let visible = rows.saturating_sub(3).max(1);
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
    let mut cursor = 0;
    loop {
        render_list(&mut guard.output, prompt, labels, cursor)?;
        let Event::Key(key) = read()? else {
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

fn render_list(
    output: &mut Stderr,
    prompt: &str,
    labels: &[String],
    cursor: usize,
) -> io::Result<()> {
    let (columns, _) = size().unwrap_or((80, 24));
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
