//! The simple single-column list selector used for the player, language, and
//! subtitle format pages. It enters the terminal, then drives a loop that
//! reads events and redraws the list after each event that changes what is
//! shown.

use crate::Navigation;
use crate::host::{Clock, Host, ReadEvent, WindowSize};
use crate::render::{Button, FIRST_LIST_ROW, button_at, fit, is_double_click, render_top_bar};
use crate::selectors::row_style;
use crate::terminal::TerminalGuard;
use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind};
use fuzzy_select::selection::ItemIndex;
use std::io::{self, Write};
use std::time::SystemTime;
use terminal_screen::{Column, Height, Row, Screen, Style, Width};

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
    let mut cursor = ItemIndex::new(start.min(labels.len().saturating_sub(1)));
    let mut last_click: Option<(SystemTime, ItemIndex)> = None;
    let mut hover: Option<(Column, Row)> = None;
    let mut screen = Screen::new();
    // Draw once up front, then redraw after any event that changes what is
    // shown, including a mouse movement that changes the hover highlight. The
    // double-buffered screen sends only the cells that differ, so redrawing this
    // often stays cheap.
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
                KeyCode::Up => cursor = cursor.previous(),
                KeyCode::Down => {
                    let next = cursor.next();
                    if next.get() < labels.len() {
                        cursor = next;
                    }
                }
                // With no text to type, Space confirms the choice like Enter.
                KeyCode::Enter | KeyCode::Char(' ') => {
                    if !labels.is_empty() {
                        return Ok(Navigation::Selected(cursor.get()));
                    }
                    continue;
                }
                _ => continue,
            },
            Event::Mouse(mouse) => {
                // Track the pointer so the hovered button and label are
                // highlighted on the redraw that follows.
                hover = Some((Column::new(mouse.column), Row::new(mouse.row)));
                match mouse.kind {
                    MouseEventKind::ScrollUp => cursor = cursor.previous(),
                    MouseEventKind::ScrollDown => {
                        let next = cursor.next();
                        if next.get() < labels.len() {
                            cursor = next;
                        }
                    }
                    // A single click highlights the label on the clicked row; a
                    // double click on the same item also selects it.
                    MouseEventKind::Down(MouseButton::Left) => {
                        let (columns, rows) = Sys::window_size().unwrap_or((80, 24));
                        let clicked_row = Row::new(mouse.row);
                        // A click on the top bar acts on the button under the
                        // pointer, where "Forward" matches pressing Enter.
                        if clicked_row == Row::TOP {
                            match button_at(Width::new(columns), Column::new(mouse.column)) {
                                Some(Button::Exit) => return Ok(Navigation::Quit),
                                Some(Button::Back) => return Ok(Navigation::Back),
                                Some(Button::Forward) => {
                                    if !labels.is_empty() {
                                        return Ok(Navigation::Selected(cursor.get()));
                                    }
                                }
                                None => {}
                            }
                        } else if clicked_row < Row::new(rows.saturating_sub(1))
                            && let Some(index) = clicked_row
                                .rows_below(FIRST_LIST_ROW)
                                .map(|rows_down| ItemIndex::new(usize::from(rows_down)))
                                .filter(|index| index.get() < labels.len())
                        {
                            let now = Sys::now();
                            let confirm = is_double_click(last_click, now, index);
                            last_click = Some((now, index));
                            cursor = index;
                            if confirm {
                                return Ok(Navigation::Selected(index.get()));
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
    cursor: ItemIndex,
    hover: Option<(Column, Row)>,
) -> io::Result<()>
where
    Sys: WindowSize,
{
    let (width, height) = Sys::window_size().unwrap_or((80, 24));
    let columns = Width::new(width);
    let buffer = screen.begin(columns, Height::new(height), output)?;

    // The top bar names the page; a list page always follows an earlier page,
    // so going back is available.
    render_top_bar(buffer, columns, title, true, hover);

    for ((index, label), screen_y) in labels.iter().enumerate().zip(FIRST_LIST_ROW.downwards()) {
        let line = fit(label, columns);
        let style = row_style(ItemIndex::new(index) == cursor, hover, screen_y);
        buffer.set_string(Column::LEFT, screen_y, &line, style);
    }

    let help = "↑/↓ move · ⌫/Esc back · ⏎/␣ select · ^Q quit";
    buffer.set_string(
        Column::LEFT,
        Row::new(height.saturating_sub(1)),
        &fit(help, columns),
        Style::DIM,
    );

    screen.flush(output)
}
