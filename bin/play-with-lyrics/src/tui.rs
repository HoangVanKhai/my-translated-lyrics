//! The interactive terminal selectors built on `crossterm`.
//!
//! This module renders the fuzzy table of titles and the simple list
//! selectors for the player, language, and subtitle format. It reads key
//! events and drives a [`Selector`] for the table. The rendering uses plain
//! character counts for column widths, so wide CJK glyphs may not align
//! perfectly; the goal here is a readable, navigable list rather than
//! pixel-perfect columns.
//!
//! All drawing goes to standard error, leaving standard output free for the
//! resolved command that `--dry-run` prints.

use crate::catalog::Video;
use crate::selection::Selector;
use crossterm::cursor::{Hide, MoveTo, Show};
use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers, read};
use crossterm::style::{Attribute, Print, SetAttribute};
use crossterm::terminal::{
    Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
    enable_raw_mode, size,
};
use crossterm::{execute, queue};
use lyrics_core::video_descriptor::Language;
use std::io::{self, Stderr, Write};

/// Restores the terminal to its normal state when dropped, even if the
/// caller returns early or panics.
struct TerminalGuard {
    out: Stderr,
}

impl TerminalGuard {
    fn enter() -> io::Result<Self> {
        enable_raw_mode()?;
        let mut out = io::stderr();
        execute!(out, EnterAlternateScreen, Hide)?;
        Ok(TerminalGuard { out })
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        // Best effort: there is nothing useful to do if restoring fails.
        let _ = execute!(self.out, Show, LeaveAlternateScreen);
        let _ = disable_raw_mode();
    }
}

/// Pads or truncates `text` to exactly `width` characters, appending an
/// ellipsis when it has to cut the text short.
fn fit(text: &str, width: usize) -> String {
    let characters: Vec<char> = text.chars().collect();
    if characters.len() <= width {
        let mut padded = text.to_string();
        padded.extend(std::iter::repeat_n(' ', width - characters.len()));
        return padded;
    }
    if width == 0 {
        return String::new();
    }
    let mut truncated: String = characters[..width - 1].iter().collect();
    truncated.push('…');
    truncated
}

/// Lays out three cells into a single line that fits `total` columns.
fn columns_line(english: &str, vietnamese: &str, chinese: &str, total: usize) -> String {
    let separator = " │ ";
    let available = total.saturating_sub(separator.chars().count() * 2);
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
        render_table(&mut guard.out, &selector, videos)?;
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
            KeyCode::Char(character) => selector.push_char(character),
            KeyCode::Enter => {
                if let Some(index) = selector.selected_index() {
                    return Ok(Some(index));
                }
            }
            _ => {}
        }
    }
}

fn render_table(out: &mut Stderr, selector: &Selector<Video>, videos: &[Video]) -> io::Result<()> {
    let (columns, rows) = size().unwrap_or((80, 24));
    let columns = columns as usize;
    let rows = rows as usize;

    queue!(out, Clear(ClearType::All))?;

    let prompt = format!("Search: {}", selector.query());
    queue!(out, MoveTo(0, 0), Print(fit(&prompt, columns)))?;

    let header = columns_line("English", "Vietnamese", "Chinese", columns);
    queue!(
        out,
        MoveTo(0, 1),
        SetAttribute(Attribute::Bold),
        Print(header),
        SetAttribute(Attribute::Reset),
    )?;

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
        let y = (screen_index + 2) as u16;
        queue!(out, MoveTo(0, y))?;
        if filtered_position == cursor {
            queue!(
                out,
                SetAttribute(Attribute::Reverse),
                Print(line),
                SetAttribute(Attribute::Reset),
            )?;
        } else {
            queue!(out, Print(line))?;
        }
    }

    let help = "↑/↓ move · type to filter · Enter select · Esc cancel";
    queue!(
        out,
        MoveTo(0, rows.saturating_sub(1) as u16),
        SetAttribute(Attribute::Dim),
        Print(fit(help, columns)),
        SetAttribute(Attribute::Reset),
    )?;

    out.flush()
}

/// Presents a simple single-column list of `labels` under `prompt` and
/// returns the index of the chosen item. Returns `None` when the user
/// cancels.
pub fn select_one(prompt: &str, labels: &[String]) -> io::Result<Option<usize>> {
    let mut guard = TerminalGuard::enter()?;
    let mut cursor = 0;
    loop {
        render_list(&mut guard.out, prompt, labels, cursor)?;
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

#[cfg(test)]
mod tests;

fn render_list(out: &mut Stderr, prompt: &str, labels: &[String], cursor: usize) -> io::Result<()> {
    let (columns, _) = size().unwrap_or((80, 24));
    let columns = columns as usize;

    queue!(out, Clear(ClearType::All))?;
    queue!(
        out,
        MoveTo(0, 0),
        SetAttribute(Attribute::Bold),
        Print(fit(prompt, columns)),
        SetAttribute(Attribute::Reset),
    )?;

    for (index, label) in labels.iter().enumerate() {
        let y = (index + 1) as u16;
        let line = fit(label, columns);
        queue!(out, MoveTo(0, y))?;
        if index == cursor {
            queue!(
                out,
                SetAttribute(Attribute::Reverse),
                Print(line),
                SetAttribute(Attribute::Reset),
            )?;
        } else {
            queue!(out, Print(line))?;
        }
    }

    out.flush()
}
