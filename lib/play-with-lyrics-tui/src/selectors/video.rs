//! The fuzzy table of titles. This page enters the terminal, then drives a
//! loop that reads events and redraws the table after each event that changes
//! what is shown.

use crate::Navigation;
use crate::host::{Clock, Host, ReadEvent, WindowSize};
use crate::render::{
    Button, COLUMN_SEPARATOR, FIRST_DATA_ROW, HEADER_ROW, SEARCH_ROW, button_at, column_at,
    column_spans, columns_line, columns_line_highlighted, draw_highlighted_line, fit,
    is_double_click, render_top_bar, scroll_offset, visible_rows,
};
use crate::selectors::row_style;
use crate::terminal::TerminalGuard;
use column_sort::{ColumnSort, Direction};
use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind};
use fuzzy_select::fuzzy::match_mask;
use fuzzy_select::selection::{ItemIndex, Selector};
use lyrics_core::video_descriptor::Language;
use play_with_lyrics::catalog::{Video, language_label};
use std::cmp::Ordering;
use std::io::{self, Write};
use std::time::SystemTime;
use terminal_screen::{Buffer, Column, Height, Row, Screen, Style, Width};

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
    selector.set_query(query.clone());
    // The table sorts by English title first, then Vietnamese, then Chinese.
    // Clicking a column header changes the priority and direction.
    let mut sort = ColumnSort::new([Language::English, Language::Vietnamese, Language::Chinese]);
    selector.set_order(video_order(sort.clone()));
    if let Some(index) = selected {
        selector.focus(ItemIndex::new(index));
    }
    let mut last_click: Option<(SystemTime, ItemIndex)> = None;
    let mut hover: Option<(Column, Row)> = None;
    let mut screen = Screen::new();
    // Draw once up front, then redraw after any event that changes what is
    // shown, including a mouse movement that changes the hover highlight. The
    // double-buffered screen sends only the cells that differ, so redrawing this
    // often stays cheap.
    render_table::<Sys>(&mut screen, output, &selector, videos, &sort, hover)?;
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
                    Some(index) => break Navigation::Selected(index.get()),
                    None => continue,
                },
                _ => continue,
            },
            Event::Mouse(mouse) => {
                // Track the pointer so the hovered button and row are highlighted
                // on the redraw that follows.
                hover = Some((Column::new(mouse.column), Row::new(mouse.row)));
                match mouse.kind {
                    MouseEventKind::ScrollUp => selector.move_up(),
                    MouseEventKind::ScrollDown => selector.move_down(),
                    // A single click highlights the video on the clicked row; a
                    // double click on the same item also selects it.
                    MouseEventKind::Down(MouseButton::Left) => {
                        let (columns, rows) = Sys::window_size().unwrap_or((80, 24));
                        let width = Width::new(columns);
                        let clicked_row = Row::new(mouse.row);
                        let clicked_column = Column::new(mouse.column);
                        // A click on the top bar acts on the button under the
                        // pointer, where "Forward" matches pressing Enter.
                        if clicked_row == Row::TOP {
                            match button_at(width, clicked_column) {
                                Some(Button::Exit) => break Navigation::Quit,
                                // Go back is disabled on the first page.
                                Some(Button::Back) | None => {}
                                Some(Button::Forward) => {
                                    if let Some(index) = selector.selected_index() {
                                        break Navigation::Selected(index.get());
                                    }
                                }
                            }
                        } else if clicked_row == HEADER_ROW {
                            // A click on a column header re-sorts the table by
                            // that column.
                            if let Some(column) = column_at(width, clicked_column) {
                                let language = COLUMN_LANGUAGES[column.position()];
                                sort.click(language);
                                selector.set_order(video_order(sort.clone()));
                            }
                        } else {
                            let visible = visible_rows(Height::new(rows));
                            let offset = scroll_offset(selector.cursor(), visible);
                            // Only the data rows are clickable; a click on the
                            // help line below them must not reach a scrolled-off
                            // item.
                            let clicked = clicked_row
                                .rows_below(FIRST_DATA_ROW)
                                .filter(|&rows_down| rows_down < visible)
                                .and_then(|rows_down| {
                                    selector.item_at(offset.down_by(usize::from(rows_down)))
                                });
                            if let Some(index) = clicked {
                                let now = Sys::now();
                                let confirm = is_double_click(last_click, now, index);
                                last_click = Some((now, index));
                                selector.focus(index);
                                if confirm {
                                    break Navigation::Selected(index.get());
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
        render_table::<Sys>(&mut screen, output, &selector, videos, &sort, hover)?;
    };
    // Hand the final query back so the caller can restore it on a later visit.
    *query = selector.query().to_string();
    Ok(outcome)
}

/// The table columns, left to right, naming the language each one shows.
const COLUMN_LANGUAGES: [Language; 3] =
    [Language::English, Language::Vietnamese, Language::Chinese];

/// A comparator over videos for the given column sort, reading each video's
/// title in the column's language.
fn video_order(sort: ColumnSort<Language>) -> impl Fn(&Video, &Video) -> Ordering {
    move |left, right| {
        sort.compare(
            |language| left.title(language).unwrap_or(""),
            |language| right.title(language).unwrap_or(""),
        )
    }
}

/// Draws the search bar at [`SEARCH_ROW`]: a dimmed magnifier, the italic
/// "Search:" label, and the typed `query` in bold.
pub(super) fn render_search_bar(buffer: &mut Buffer, columns: Width, query: &str) {
    // Chain off each segment's end column so the layout matches the buffer's own
    // widths; measuring separately would disagree on the magnifier's width.
    let after_magnifier = buffer.set_string(Column::LEFT, SEARCH_ROW, "🔍︎", Style::DIM);
    let after_label = buffer.set_string(after_magnifier, SEARCH_ROW, " Search: ", Style::ITALIC);
    let shown = fit(query, columns.remaining_from(after_label));
    buffer.set_string(after_label, SEARCH_ROW, &shown, Style::BOLD);
}

/// Draws the clickable column header at [`HEADER_ROW`]. The column the table is
/// sorted by is marked with an arrow for its direction, and the column under
/// the pointer drops the dim to read brighter.
pub(super) fn render_header(
    buffer: &mut Buffer,
    columns: Width,
    sort: &ColumnSort<Language>,
    hover: Option<(Column, Row)>,
) {
    let primary = sort.order().first().copied();
    let label = |language: Language| -> String {
        let mut text = language_label(language).to_string();
        if let Some((column, direction)) = primary
            && column == language
        {
            text.push(' ');
            text.push(match direction {
                Direction::Ascending => '▲',
                Direction::Descending => '▼',
            });
        }
        text
    };
    let labels = COLUMN_LANGUAGES.map(label);
    let header = columns_line(&labels[0], &labels[1], &labels[2], columns);
    let spans = column_spans(columns);
    // The headers are bold and dimmed.
    buffer.set_string(
        Column::LEFT,
        HEADER_ROW,
        &header,
        Style::BOLD.with(Style::DIM),
    );
    // The separators between the headers are bold but not dimmed.
    for span in &spans[..2] {
        buffer.set_string(span.end(), HEADER_ROW, COLUMN_SEPARATOR, Style::BOLD);
    }
    // The column under the pointer drops the dim.
    if let Some((hover_column, hover_row)) = hover
        && hover_row == HEADER_ROW
        && let Some(column) = column_at(columns, hover_column)
    {
        let span = spans[column.position()];
        let fitted = fit(&labels[column.position()], span.width());
        buffer.set_string(span.start(), HEADER_ROW, &fitted, Style::BOLD);
    }
}

fn render_table<Sys>(
    screen: &mut Screen,
    output: &mut impl Write,
    selector: &Selector<Video>,
    videos: &[Video],
    sort: &ColumnSort<Language>,
    hover: Option<(Column, Row)>,
) -> io::Result<()>
where
    Sys: WindowSize,
{
    let (width, height) = Sys::window_size().unwrap_or((80, 24));
    let columns = Width::new(width);
    let rows = Height::new(height);
    let buffer = screen.begin(columns, rows, output)?;

    // The top bar names the page; the table is the first page, so going back is
    // not available here.
    render_top_bar(buffer, columns, "Select a Video", false, hover);

    render_search_bar(buffer, columns, selector.query());

    render_header(buffer, columns, sort, hover);

    let selected = selector.selected_index();
    let visible = visible_rows(rows);
    let offset = scroll_offset(selector.cursor(), visible);

    let query = selector.query();
    let window = selector
        .filtered()
        .iter()
        .skip(offset.get())
        .take(usize::from(visible));
    for (screen_y, &item) in FIRST_DATA_ROW.downwards().zip(window) {
        let video = &videos[item];
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
        // The highlighted row is the one showing the selected item, which
        // stays correct however the window is scrolled or the table sorted.
        let base = row_style(Some(item) == selected, hover, screen_y);
        draw_highlighted_line(buffer, screen_y, &line, base);
    }

    let help = "↑/↓ move · type to search · ⌫ delete · ^⌫ back · ⏎ select · Esc/^Q quit";
    buffer.set_string(
        Column::LEFT,
        Row::new(height.saturating_sub(1)),
        &fit(help, columns),
        Style::DIM,
    );

    screen.flush(output)
}
