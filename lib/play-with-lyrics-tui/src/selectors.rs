//! The interactive selector pages: the fuzzy table of titles and the simple
//! single-column list. Each page enters the terminal, then drives a loop that
//! reads events and redraws only after a change, so a mouse movement does not
//! make the screen flicker.

use crate::Navigation;
use crate::host::{Clock, Host, ReadEvent, WindowSize};
use crate::render::{
    Button, DATA_ROW_OFFSET, LIST_ROW_OFFSET, button_at, columns_line, columns_line_highlighted,
    draw_highlighted_line, fit, is_double_click, render_top_bar, scroll_offset, visible_rows,
};
use crate::terminal::TerminalGuard;
use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind};
use fuzzy_select::fuzzy::match_mask;
use fuzzy_select::selection::Selector;
use lyrics_core::video_descriptor::Language;
use play_with_lyrics::catalog::{Video, language_label};
use std::io::{self, Write};
use std::time::SystemTime;
use terminal_screen::{Screen, Style};

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
pub(crate) fn select_video_loop<Sys>(
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
    let mut last_click: Option<(SystemTime, u16)> = None;
    let mut hover: Option<(u16, u16)> = None;
    let mut screen = Screen::new();
    // Draw once up front, then redraw only after an event that changes what is
    // shown, including a mouse movement that changes the hover highlight. The
    // double-buffered screen sends only the cells that change, so a redraw never
    // clears the whole screen and the display does not flicker.
    render_table::<Sys>(&mut screen, output, &selector, videos, hover)?;
    let outcome = loop {
        match Sys::read_event()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                // The table is the first page, so Escape quits here. The later
                // list pages, in `select_one_loop`, treat Escape as "go back"
                // instead.
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
                KeyCode::Enter => match selector.selected_index() {
                    Some(index) => break Navigation::Selected(index),
                    None => continue,
                },
                _ => continue,
            },
            Event::Mouse(mouse) => {
                // Track the pointer so the hovered button and row are highlighted
                // on the redraw that follows.
                hover = Some((mouse.column, mouse.row));
                match mouse.kind {
                    MouseEventKind::ScrollUp => selector.move_up(),
                    MouseEventKind::ScrollDown => selector.move_down(),
                    // A single click highlights the video on the clicked row; a
                    // double click on the same row also selects it.
                    MouseEventKind::Down(MouseButton::Left) => {
                        let (columns, rows) = Sys::window_size().unwrap_or((80, 24));
                        // A click on the top bar acts on the button under the
                        // pointer, where "Forward" matches pressing Enter.
                        if mouse.row == 0 {
                            match button_at(columns as usize, mouse.column as usize) {
                                Some(Button::Exit) => break Navigation::Quit,
                                // Go back is disabled on the first page.
                                Some(Button::Back) | None => {}
                                Some(Button::Forward) => {
                                    if let Some(index) = selector.selected_index() {
                                        break Navigation::Selected(index);
                                    }
                                }
                            }
                        } else {
                            let visible = visible_rows(rows as usize);
                            let offset = scroll_offset(selector.cursor(), visible);
                            let clicked = (mouse.row as usize)
                                .checked_sub(DATA_ROW_OFFSET)
                                .and_then(|screen_index| {
                                    selector.filtered().get(offset + screen_index).copied()
                                });
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
                    }
                    // Any other mouse event, such as a movement, only updates the
                    // hover highlight, which the redraw below applies.
                    _ => {}
                }
            }
            // A resize changes the layout, so redraw; any other event does not.
            Event::Resize(..) => {}
            _ => continue,
        }
        render_table::<Sys>(&mut screen, output, &selector, videos, hover)?;
    };
    // Hand the final query back so the caller can restore it on a later visit.
    *query = selector.query().to_string();
    Ok(outcome)
}

/// The base style for a selectable row at screen `row`: reverse video when it
/// is the current selection, with bold added when the pointer hovers over it.
fn row_style(selected: bool, hover: Option<(u16, u16)>, row: u16) -> Style {
    let mut style = if selected {
        Style::REVERSE
    } else {
        Style::PLAIN
    };
    if hover.is_some_and(|(_, hovered_row)| hovered_row == row) {
        style = style.with(Style::BOLD);
    }
    style
}

fn render_table<Sys>(
    screen: &mut Screen,
    output: &mut impl Write,
    selector: &Selector<Video>,
    videos: &[Video],
    hover: Option<(u16, u16)>,
) -> io::Result<()>
where
    Sys: WindowSize,
{
    let (width, height) = Sys::window_size().unwrap_or((80, 24));
    let buffer = screen.begin(width, height, output)?;
    let columns = width as usize;
    let rows = height as usize;

    // The top bar names the page; the table is the first page, so going back is
    // not available here.
    render_top_bar(buffer, columns, "Select a Video", false, hover);

    let prompt = format!("Search: {}", selector.query());
    buffer.set_string(0, 1, &fit(&prompt, columns), Style::PLAIN);

    let header = columns_line(
        language_label(Language::English),
        language_label(Language::Vietnamese),
        language_label(Language::Chinese),
        columns,
    );
    buffer.set_string(0, 2, &header, Style::BOLD);

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
        let base = row_style(filtered_position == cursor, hover, screen_y);
        draw_highlighted_line(buffer, screen_y, &line, base);
    }

    let help = "↑/↓ move · type to search · ⌫ delete · ^⌫ back · ⏎ select · Esc/^Q quit";
    buffer.set_string(0, height.saturating_sub(1), &fit(help, columns), Style::DIM);

    screen.flush(output)
}

/// Presents a simple single-column list of `labels` and reports the chosen
/// item, a request to go back to the previous page, or a request to quit. The
/// `title` names the page in the top bar.
pub fn select_one(title: &str, labels: &[String], start: usize) -> io::Result<Navigation> {
    let mut guard = TerminalGuard::enter()?;
    select_one_loop::<Host>(&mut guard.output, title, labels, start)
}

/// Drives the single-column list selector, reading events from `Sys`.
/// Splitting this out from [`select_one`] lets a test replay scripted
/// events and render to a buffer, exercising the loop without a terminal.
/// `start` is the row to highlight at first, to restore a previous choice.
pub(crate) fn select_one_loop<Sys>(
    output: &mut impl Write,
    title: &str,
    labels: &[String],
    start: usize,
) -> io::Result<Navigation>
where
    Sys: ReadEvent + WindowSize + Clock,
{
    let mut cursor = start.min(labels.len().saturating_sub(1));
    let mut last_click: Option<(SystemTime, u16)> = None;
    let mut hover: Option<(u16, u16)> = None;
    let mut screen = Screen::new();
    // Draw once up front, then redraw only after an event that changes what is
    // shown, including a mouse movement that changes the hover highlight. The
    // double-buffered screen sends only the cells that change, so a redraw never
    // clears the whole screen and the display does not flicker.
    render_list::<Sys>(&mut screen, output, title, labels, cursor, hover)?;
    loop {
        match Sys::read_event()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                // A list page is never the first page, so Escape goes back to
                // the previous page, the same as Ctrl-Backspace. Only the video
                // table, the first page, quits on Escape.
                KeyCode::Esc => return Ok(Navigation::Back),
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
                    continue;
                }
                _ => continue,
            },
            Event::Mouse(mouse) => {
                // Track the pointer so the hovered button and label are
                // highlighted on the redraw that follows.
                hover = Some((mouse.column, mouse.row));
                match mouse.kind {
                    MouseEventKind::ScrollUp => cursor = cursor.saturating_sub(1),
                    MouseEventKind::ScrollDown => {
                        if cursor + 1 < labels.len() {
                            cursor += 1;
                        }
                    }
                    // A single click highlights the label on the clicked row; a
                    // double click on the same row also selects it.
                    MouseEventKind::Down(MouseButton::Left) => {
                        let (columns, _) = Sys::window_size().unwrap_or((80, 24));
                        // A click on the top bar acts on the button under the
                        // pointer, where "Forward" matches pressing Enter.
                        if mouse.row == 0 {
                            match button_at(columns as usize, mouse.column as usize) {
                                Some(Button::Exit) => return Ok(Navigation::Quit),
                                Some(Button::Back) => return Ok(Navigation::Back),
                                Some(Button::Forward) => {
                                    if !labels.is_empty() {
                                        return Ok(Navigation::Selected(cursor));
                                    }
                                }
                                None => {}
                            }
                        } else if let Some(index) = (mouse.row as usize)
                            .checked_sub(LIST_ROW_OFFSET)
                            .filter(|&index| index < labels.len())
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
                    // Any other mouse event, such as a movement, only updates the
                    // hover highlight, which the redraw below applies.
                    _ => {}
                }
            }
            // A resize changes the layout, so redraw; any other event does not.
            Event::Resize(..) => {}
            _ => continue,
        }
        render_list::<Sys>(&mut screen, output, title, labels, cursor, hover)?;
    }
}

fn render_list<Sys>(
    screen: &mut Screen,
    output: &mut impl Write,
    title: &str,
    labels: &[String],
    cursor: usize,
    hover: Option<(u16, u16)>,
) -> io::Result<()>
where
    Sys: WindowSize,
{
    let (width, height) = Sys::window_size().unwrap_or((80, 24));
    let buffer = screen.begin(width, height, output)?;
    let columns = width as usize;

    // The top bar names the page; a list page always follows an earlier page,
    // so going back is available.
    render_top_bar(buffer, columns, title, true, hover);

    for (index, label) in labels.iter().enumerate() {
        let screen_y = (index + LIST_ROW_OFFSET) as u16;
        let line = fit(label, columns);
        let style = row_style(index == cursor, hover, screen_y);
        buffer.set_string(0, screen_y, &line, style);
    }

    let help = "↑/↓ move · ⌫/Esc back · ⏎/␣ select · ^Q quit";
    buffer.set_string(0, height.saturating_sub(1), &fit(help, columns), Style::DIM);

    screen.flush(output)
}

#[cfg(test)]
mod tests;
