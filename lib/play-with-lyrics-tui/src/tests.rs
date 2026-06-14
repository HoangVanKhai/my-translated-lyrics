use crate::{
    Navigation, ReadEvent, WindowSize, columns_line, fit, scroll_offset, select_one_loop,
    select_video_loop, visible_rows,
};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use lyrics_core::video_descriptor::{Language, VideoDesc, Visibility};
use play_with_lyrics::catalog::Video;
use pretty_assertions::assert_eq;
use std::collections::{HashMap, VecDeque};
use std::io;
use std::sync::Mutex;
use test_utils::video_desc;
use unicode_width::UnicodeWidthStr;

#[test]
fn fit_pads_short_text() {
    assert_eq!(fit("ab", 5), "ab   ");
}

#[test]
fn fit_truncates_with_an_ellipsis() {
    assert_eq!(fit("abcdef", 4), "abc…");
}

#[test]
fn fit_handles_zero_width() {
    assert_eq!(fit("abc", 0), "");
}

/// Width is measured in display columns: an accented letter is one column,
/// a CJK ideograph is two.
#[test]
fn fit_measures_display_width() {
    // "café" is four single-column characters.
    assert_eq!(fit("café", 4), "café");
    // Each ideograph occupies two columns, so "示例" fills four exactly.
    assert_eq!(fit("示例", 4), "示例");
    // Padding accounts for the double-width glyphs.
    assert_eq!(fit("示例", 6), "示例  ");
}

/// Truncation counts each glyph's width, never overflows the budget, and
/// pads the column a wide glyph could not fill before the ellipsis.
#[test]
fn fit_truncates_wide_characters_to_the_column_budget() {
    // "示例例" is six columns; in four, one ideograph and the ellipsis fit
    // and a single padding column fills the rest.
    assert_eq!(fit("示例例", 4), "示… ");
}

#[test]
fn columns_line_splits_the_width_three_ways() {
    let line = columns_line("alpha", "beta", "gamma", 30);
    // The line fills the full width and keeps the two column separators.
    assert_eq!(line.chars().count(), 30);
    assert_eq!(line.matches('│').count(), 2);
    let cells: Vec<&str> = line.split('│').map(str::trim).collect();
    assert_eq!(cells, vec!["alpha", "beta", "gamma"]);
}

/// Cells with wide glyphs are measured by display width, so the line still
/// fills exactly `total` columns rather than overrunning the terminal.
#[test]
fn columns_line_aligns_wide_characters() {
    // cspell:locale en vi
    let line = columns_line("中文", "Tiếng Việt", "示例歌曲", 30);
    assert_eq!(line.width(), 30);
}

#[test]
fn scroll_offset_keeps_the_cursor_on_screen() {
    // The cursor fits within the first page, so no scrolling.
    assert_eq!(scroll_offset(2, 5), 0);
    // The cursor sits past the page, so the window scrolls to show it.
    assert_eq!(scroll_offset(7, 5), 3);
}

// The interactive loops read their events through the `ReadEvent` seam, so a
// test runs them with a fake event source instead of a terminal. Following
// the dependency-injection pattern, each test below defines its own fake and
// its own scripted queue inside the test body, so the tests share no state.
// The small stateless helpers below carry no state and so stay at module
// scope.

/// A key press with no modifiers.
fn press(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
}

/// A key press combined with the Control modifier.
fn control(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::CONTROL))
}

/// A key press combined with the Shift modifier.
fn shift(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::SHIFT))
}

/// Pops the next scripted event from a test's own queue, reporting an error
/// if the loop reads past the end of the script it was given.
fn pop_scripted(queue: &Mutex<VecDeque<Event>>) -> io::Result<Event> {
    queue.lock().unwrap().pop_front().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "the event script is exhausted",
        )
    })
}

fn label_list(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| value.to_string()).collect()
}

fn video(title: &str) -> Video {
    Video {
        desc: video_desc("Touhou Hero of Ice Fairy", title, Visibility::Visible),
    }
}

/// A video whose English title is `title`, so rows are distinguishable by
/// the English column in rendered output. The `video_desc` helper gives every
/// video the same placeholder titles, which would render identically.
fn english_video(title: &str) -> Video {
    Video {
        desc: VideoDesc {
            collection: "Touhou Hero of Ice Fairy".to_string().try_into().unwrap(),
            video_title: title.to_string().try_into().unwrap(),
            song_titles: HashMap::from([(Language::English, title.to_string())]),
            visibility: Visibility::Visible,
        },
    }
}

/// The terminal size a size-agnostic test renders at: wide and tall enough
/// that neither truncation nor scrolling occurs.
fn standard_size() -> io::Result<(u16, u16)> {
    Ok((80, 24))
}

/// Enter returns the highlighted row after the cursor has moved down to it.
#[test]
fn select_one_returns_the_highlighted_row() {
    static EVENTS: Mutex<VecDeque<Event>> = Mutex::new(VecDeque::new());
    struct Scripted;
    impl ReadEvent for Scripted {
        fn read_event() -> io::Result<Event> {
            pop_scripted(&EVENTS)
        }
    }
    impl WindowSize for Scripted {
        fn window_size() -> io::Result<(u16, u16)> {
            standard_size()
        }
    }
    let labels = label_list(&["alpha", "beta", "gamma"]);
    EVENTS.lock().unwrap().extend([
        press(KeyCode::Down),
        press(KeyCode::Down),
        press(KeyCode::Enter),
    ]);
    let chosen = select_one_loop::<Scripted>(&mut Vec::new(), "pick", &labels).unwrap();
    assert_eq!(chosen, Navigation::Selected(2));
}

/// Escape cancels the list selector.
#[test]
fn select_one_cancels_on_escape() {
    static EVENTS: Mutex<VecDeque<Event>> = Mutex::new(VecDeque::new());
    struct Scripted;
    impl ReadEvent for Scripted {
        fn read_event() -> io::Result<Event> {
            pop_scripted(&EVENTS)
        }
    }
    impl WindowSize for Scripted {
        fn window_size() -> io::Result<(u16, u16)> {
            standard_size()
        }
    }
    let labels = label_list(&["alpha", "beta"]);
    EVENTS.lock().unwrap().extend([press(KeyCode::Esc)]);
    let chosen = select_one_loop::<Scripted>(&mut Vec::new(), "pick", &labels).unwrap();
    assert_eq!(chosen, Navigation::Quit);
}

/// Ctrl-C cancels the list selector.
#[test]
fn select_one_cancels_on_ctrl_c() {
    static EVENTS: Mutex<VecDeque<Event>> = Mutex::new(VecDeque::new());
    struct Scripted;
    impl ReadEvent for Scripted {
        fn read_event() -> io::Result<Event> {
            pop_scripted(&EVENTS)
        }
    }
    impl WindowSize for Scripted {
        fn window_size() -> io::Result<(u16, u16)> {
            standard_size()
        }
    }
    let labels = label_list(&["alpha", "beta"]);
    EVENTS.lock().unwrap().extend([control(KeyCode::Char('c'))]);
    let chosen = select_one_loop::<Scripted>(&mut Vec::new(), "pick", &labels).unwrap();
    assert_eq!(chosen, Navigation::Quit);
}

/// The cursor never moves above the first row or below the last.
#[test]
fn select_one_keeps_the_cursor_within_bounds() {
    static EVENTS: Mutex<VecDeque<Event>> = Mutex::new(VecDeque::new());
    struct Scripted;
    impl ReadEvent for Scripted {
        fn read_event() -> io::Result<Event> {
            pop_scripted(&EVENTS)
        }
    }
    impl WindowSize for Scripted {
        fn window_size() -> io::Result<(u16, u16)> {
            standard_size()
        }
    }
    let labels = label_list(&["alpha", "beta"]);
    // Up at the top holds the first row; three Downs cannot pass the last.
    EVENTS.lock().unwrap().extend([
        press(KeyCode::Up),
        press(KeyCode::Down),
        press(KeyCode::Down),
        press(KeyCode::Down),
        press(KeyCode::Enter),
    ]);
    let chosen = select_one_loop::<Scripted>(&mut Vec::new(), "pick", &labels).unwrap();
    assert_eq!(chosen, Navigation::Selected(1));
}

/// Events that are not key presses, such as key releases, are ignored.
#[test]
fn select_one_ignores_non_press_events() {
    static EVENTS: Mutex<VecDeque<Event>> = Mutex::new(VecDeque::new());
    struct Scripted;
    impl ReadEvent for Scripted {
        fn read_event() -> io::Result<Event> {
            pop_scripted(&EVENTS)
        }
    }
    impl WindowSize for Scripted {
        fn window_size() -> io::Result<(u16, u16)> {
            standard_size()
        }
    }
    let labels = label_list(&["alpha", "beta"]);
    let release = Event::Key(KeyEvent::new_with_kind(
        KeyCode::Down,
        KeyModifiers::NONE,
        KeyEventKind::Release,
    ));
    // The release does not move the cursor; only the press does.
    EVENTS
        .lock()
        .unwrap()
        .extend([release, press(KeyCode::Down), press(KeyCode::Enter)]);
    let chosen = select_one_loop::<Scripted>(&mut Vec::new(), "pick", &labels).unwrap();
    assert_eq!(chosen, Navigation::Selected(1));
}

/// Enter does nothing while the list is empty, so the loop keeps reading.
#[test]
fn select_one_enter_is_a_no_op_for_an_empty_list() {
    static EVENTS: Mutex<VecDeque<Event>> = Mutex::new(VecDeque::new());
    struct Scripted;
    impl ReadEvent for Scripted {
        fn read_event() -> io::Result<Event> {
            pop_scripted(&EVENTS)
        }
    }
    impl WindowSize for Scripted {
        fn window_size() -> io::Result<(u16, u16)> {
            standard_size()
        }
    }
    let labels: Vec<String> = Vec::new();
    EVENTS
        .lock()
        .unwrap()
        .extend([press(KeyCode::Enter), press(KeyCode::Esc)]);
    let chosen = select_one_loop::<Scripted>(&mut Vec::new(), "pick", &labels).unwrap();
    assert_eq!(chosen, Navigation::Quit);
}

/// Typing narrows the table, and Enter returns the index, into the original
/// slice, of the row that stays highlighted.
#[test]
fn select_video_filters_then_selects() {
    static EVENTS: Mutex<VecDeque<Event>> = Mutex::new(VecDeque::new());
    struct Scripted;
    impl ReadEvent for Scripted {
        fn read_event() -> io::Result<Event> {
            pop_scripted(&EVENTS)
        }
    }
    impl WindowSize for Scripted {
        fn window_size() -> io::Result<(u16, u16)> {
            standard_size()
        }
    }
    let videos = vec![video("Alpha"), video("Beta")];
    // Type "beta" so only the second row matches, then select it.
    EVENTS.lock().unwrap().extend([
        press(KeyCode::Char('b')),
        press(KeyCode::Char('e')),
        press(KeyCode::Char('t')),
        press(KeyCode::Char('a')),
        press(KeyCode::Enter),
    ]);
    let chosen = select_video_loop::<Scripted>(&mut Vec::new(), &videos).unwrap();
    assert_eq!(chosen, Navigation::Selected(1));
}

/// Backspace widens the query again after it has filtered everything out.
#[test]
fn select_video_backspace_widens_the_query() {
    static EVENTS: Mutex<VecDeque<Event>> = Mutex::new(VecDeque::new());
    struct Scripted;
    impl ReadEvent for Scripted {
        fn read_event() -> io::Result<Event> {
            pop_scripted(&EVENTS)
        }
    }
    impl WindowSize for Scripted {
        fn window_size() -> io::Result<(u16, u16)> {
            standard_size()
        }
    }
    let videos = vec![video("Alpha"), video("Beta")];
    // "bz" matches nothing; deleting the "z" leaves "b", which matches Beta.
    EVENTS.lock().unwrap().extend([
        press(KeyCode::Char('b')),
        press(KeyCode::Char('z')),
        press(KeyCode::Backspace),
        press(KeyCode::Enter),
    ]);
    let chosen = select_video_loop::<Scripted>(&mut Vec::new(), &videos).unwrap();
    assert_eq!(chosen, Navigation::Selected(1));
}

/// Escape cancels the table without choosing a row.
#[test]
fn select_video_cancels_on_escape() {
    static EVENTS: Mutex<VecDeque<Event>> = Mutex::new(VecDeque::new());
    struct Scripted;
    impl ReadEvent for Scripted {
        fn read_event() -> io::Result<Event> {
            pop_scripted(&EVENTS)
        }
    }
    impl WindowSize for Scripted {
        fn window_size() -> io::Result<(u16, u16)> {
            standard_size()
        }
    }
    let videos = vec![video("Alpha")];
    EVENTS.lock().unwrap().extend([press(KeyCode::Esc)]);
    let chosen = select_video_loop::<Scripted>(&mut Vec::new(), &videos).unwrap();
    assert_eq!(chosen, Navigation::Quit);
}

/// Ctrl-Q quits the table.
#[test]
fn select_video_quits_on_ctrl_q() {
    static EVENTS: Mutex<VecDeque<Event>> = Mutex::new(VecDeque::new());
    struct Scripted;
    impl ReadEvent for Scripted {
        fn read_event() -> io::Result<Event> {
            pop_scripted(&EVENTS)
        }
    }
    impl WindowSize for Scripted {
        fn window_size() -> io::Result<(u16, u16)> {
            standard_size()
        }
    }
    let videos = vec![video("Alpha")];
    EVENTS.lock().unwrap().extend([control(KeyCode::Char('q'))]);
    let chosen = select_video_loop::<Scripted>(&mut Vec::new(), &videos).unwrap();
    assert_eq!(chosen, Navigation::Quit);
}

/// Ctrl-Q quits the table even when the character arrives upper-cased, as it
/// would under Shift or Caps Lock.
#[test]
fn select_video_quits_on_ctrl_q_upper_case() {
    static EVENTS: Mutex<VecDeque<Event>> = Mutex::new(VecDeque::new());
    struct Scripted;
    impl ReadEvent for Scripted {
        fn read_event() -> io::Result<Event> {
            pop_scripted(&EVENTS)
        }
    }
    impl WindowSize for Scripted {
        fn window_size() -> io::Result<(u16, u16)> {
            standard_size()
        }
    }
    let videos = vec![video("Alpha")];
    EVENTS.lock().unwrap().extend([control(KeyCode::Char('Q'))]);
    let chosen = select_video_loop::<Scripted>(&mut Vec::new(), &videos).unwrap();
    assert_eq!(chosen, Navigation::Quit);
}

/// In the table a bare "q" is a search character, not a quit, because the
/// user is typing a filter there.
#[test]
fn select_video_treats_a_bare_q_as_a_filter_character() {
    static EVENTS: Mutex<VecDeque<Event>> = Mutex::new(VecDeque::new());
    struct Scripted;
    impl ReadEvent for Scripted {
        fn read_event() -> io::Result<Event> {
            pop_scripted(&EVENTS)
        }
    }
    impl WindowSize for Scripted {
        fn window_size() -> io::Result<(u16, u16)> {
            standard_size()
        }
    }
    let videos = vec![video("Quartz"), video("Beta")];
    // "q" filters down to the only title that contains it, then Enter picks
    // it; the loop does not treat the "q" as a quit.
    EVENTS
        .lock()
        .unwrap()
        .extend([press(KeyCode::Char('q')), press(KeyCode::Enter)]);
    let chosen = select_video_loop::<Scripted>(&mut Vec::new(), &videos).unwrap();
    assert_eq!(chosen, Navigation::Selected(0));
}

/// In the list selector a bare "q" quits, since there is no text entry.
#[test]
fn select_one_quits_on_q() {
    static EVENTS: Mutex<VecDeque<Event>> = Mutex::new(VecDeque::new());
    struct Scripted;
    impl ReadEvent for Scripted {
        fn read_event() -> io::Result<Event> {
            pop_scripted(&EVENTS)
        }
    }
    impl WindowSize for Scripted {
        fn window_size() -> io::Result<(u16, u16)> {
            standard_size()
        }
    }
    let labels = label_list(&["alpha", "beta"]);
    EVENTS.lock().unwrap().extend([press(KeyCode::Char('q'))]);
    let chosen = select_one_loop::<Scripted>(&mut Vec::new(), "pick", &labels).unwrap();
    assert_eq!(chosen, Navigation::Quit);
}

/// Shift-Q quits the list selector too: the Shift state does not change it.
#[test]
fn select_one_quits_on_shift_q() {
    static EVENTS: Mutex<VecDeque<Event>> = Mutex::new(VecDeque::new());
    struct Scripted;
    impl ReadEvent for Scripted {
        fn read_event() -> io::Result<Event> {
            pop_scripted(&EVENTS)
        }
    }
    impl WindowSize for Scripted {
        fn window_size() -> io::Result<(u16, u16)> {
            standard_size()
        }
    }
    let labels = label_list(&["alpha", "beta"]);
    EVENTS.lock().unwrap().extend([shift(KeyCode::Char('Q'))]);
    let chosen = select_one_loop::<Scripted>(&mut Vec::new(), "pick", &labels).unwrap();
    assert_eq!(chosen, Navigation::Quit);
}

/// Ctrl-Q quits the list selector as well.
#[test]
fn select_one_quits_on_ctrl_q() {
    static EVENTS: Mutex<VecDeque<Event>> = Mutex::new(VecDeque::new());
    struct Scripted;
    impl ReadEvent for Scripted {
        fn read_event() -> io::Result<Event> {
            pop_scripted(&EVENTS)
        }
    }
    impl WindowSize for Scripted {
        fn window_size() -> io::Result<(u16, u16)> {
            standard_size()
        }
    }
    let labels = label_list(&["alpha", "beta"]);
    EVENTS.lock().unwrap().extend([control(KeyCode::Char('q'))]);
    let chosen = select_one_loop::<Scripted>(&mut Vec::new(), "pick", &labels).unwrap();
    assert_eq!(chosen, Navigation::Quit);
}

/// The height-dependent count of title rows reserves the prompt, header, and
/// help lines and never drops below one.
#[test]
fn visible_rows_reserves_the_chrome_lines() {
    assert_eq!(visible_rows(24), 21);
    assert_eq!(visible_rows(5), 2);
    assert_eq!(visible_rows(4), 1);
    // A terminal too short for any title row still reports one.
    assert_eq!(visible_rows(3), 1);
    assert_eq!(visible_rows(0), 1);
}

/// The table header labels each column with its language's own name. Driving
/// the loop with an injected size makes the rendered output deterministic.
#[test]
fn select_video_header_shows_native_language_names() {
    // cspell:locale en vi
    static EVENTS: Mutex<VecDeque<Event>> = Mutex::new(VecDeque::new());
    struct Scripted;
    impl ReadEvent for Scripted {
        fn read_event() -> io::Result<Event> {
            pop_scripted(&EVENTS)
        }
    }
    impl WindowSize for Scripted {
        fn window_size() -> io::Result<(u16, u16)> {
            Ok((80, 24))
        }
    }
    let videos = vec![video("Alpha")];
    // Quit right after the first frame is drawn.
    EVENTS.lock().unwrap().extend([control(KeyCode::Char('q'))]);
    let mut buffer = Vec::new();
    select_video_loop::<Scripted>(&mut buffer, &videos).unwrap();
    let rendered = String::from_utf8_lossy(&buffer);
    assert!(rendered.contains("English"), "{rendered}");
    assert!(rendered.contains("Tiếng Việt"), "{rendered}");
    assert!(rendered.contains("中文"), "{rendered}");
}

/// A terminal too narrow for a header label truncates it with an ellipsis
/// rather than overrunning the column.
#[test]
fn select_video_header_truncates_in_a_narrow_terminal() {
    // cspell:locale en vi
    static EVENTS: Mutex<VecDeque<Event>> = Mutex::new(VecDeque::new());
    struct Scripted;
    impl ReadEvent for Scripted {
        fn read_event() -> io::Result<Event> {
            pop_scripted(&EVENTS)
        }
    }
    impl WindowSize for Scripted {
        fn window_size() -> io::Result<(u16, u16)> {
            // Each of the three columns gets about six display columns.
            Ok((24, 24))
        }
    }
    let videos = vec![video("Alpha")];
    EVENTS.lock().unwrap().extend([control(KeyCode::Char('q'))]);
    let mut buffer = Vec::new();
    select_video_loop::<Scripted>(&mut buffer, &videos).unwrap();
    let rendered = String::from_utf8_lossy(&buffer);
    // "Tiếng Việt" is ten columns wide and cannot survive intact in six.
    assert!(!rendered.contains("Tiếng Việt"), "{rendered}");
    assert!(rendered.contains('…'), "{rendered}");
}

/// Only as many title rows as fit under the prompt, header, and help lines
/// are drawn; the rows past the visible window are not.
#[test]
fn select_video_renders_only_the_visible_window() {
    static EVENTS: Mutex<VecDeque<Event>> = Mutex::new(VecDeque::new());
    struct Scripted;
    impl ReadEvent for Scripted {
        fn read_event() -> io::Result<Event> {
            pop_scripted(&EVENTS)
        }
    }
    impl WindowSize for Scripted {
        fn window_size() -> io::Result<(u16, u16)> {
            // Five rows leave room for two title rows.
            Ok((80, 5))
        }
    }
    let videos = vec![
        english_video("First"),
        english_video("Second"),
        english_video("Third"),
        english_video("Fourth"),
    ];
    EVENTS.lock().unwrap().extend([control(KeyCode::Char('q'))]);
    let mut buffer = Vec::new();
    select_video_loop::<Scripted>(&mut buffer, &videos).unwrap();
    let rendered = String::from_utf8_lossy(&buffer);
    assert!(rendered.contains("First"), "{rendered}");
    assert!(rendered.contains("Second"), "{rendered}");
    assert!(!rendered.contains("Third"), "{rendered}");
    assert!(!rendered.contains("Fourth"), "{rendered}");
}

/// When the terminal size is unavailable, rendering falls back to a usable
/// default rather than failing.
#[test]
fn select_video_renders_with_a_fallback_size_when_size_is_unavailable() {
    // cspell:locale en vi
    static EVENTS: Mutex<VecDeque<Event>> = Mutex::new(VecDeque::new());
    struct Scripted;
    impl ReadEvent for Scripted {
        fn read_event() -> io::Result<Event> {
            pop_scripted(&EVENTS)
        }
    }
    impl WindowSize for Scripted {
        fn window_size() -> io::Result<(u16, u16)> {
            Err(io::Error::other("the terminal size is unavailable"))
        }
    }
    let videos = vec![video("Alpha")];
    EVENTS.lock().unwrap().extend([control(KeyCode::Char('q'))]);
    let mut buffer = Vec::new();
    select_video_loop::<Scripted>(&mut buffer, &videos).unwrap();
    let rendered = String::from_utf8_lossy(&buffer);
    // The 80-column fallback is wide enough to show the native header.
    assert!(rendered.contains("Tiếng Việt"), "{rendered}");
}

/// On the song page, Backspace with an empty query goes back, which on this
/// first page is the way out.
#[test]
fn select_video_backspace_on_an_empty_query_goes_back() {
    static EVENTS: Mutex<VecDeque<Event>> = Mutex::new(VecDeque::new());
    struct Scripted;
    impl ReadEvent for Scripted {
        fn read_event() -> io::Result<Event> {
            pop_scripted(&EVENTS)
        }
    }
    impl WindowSize for Scripted {
        fn window_size() -> io::Result<(u16, u16)> {
            standard_size()
        }
    }
    let videos = vec![video("Alpha")];
    EVENTS.lock().unwrap().extend([press(KeyCode::Backspace)]);
    let chosen = select_video_loop::<Scripted>(&mut Vec::new(), &videos).unwrap();
    assert_eq!(chosen, Navigation::Back);
}

/// On a non-typing list page, Backspace goes back to the previous page.
#[test]
fn select_one_backspace_goes_back() {
    static EVENTS: Mutex<VecDeque<Event>> = Mutex::new(VecDeque::new());
    struct Scripted;
    impl ReadEvent for Scripted {
        fn read_event() -> io::Result<Event> {
            pop_scripted(&EVENTS)
        }
    }
    impl WindowSize for Scripted {
        fn window_size() -> io::Result<(u16, u16)> {
            standard_size()
        }
    }
    let labels = label_list(&["alpha", "beta"]);
    EVENTS.lock().unwrap().extend([press(KeyCode::Backspace)]);
    let chosen = select_one_loop::<Scripted>(&mut Vec::new(), "pick", &labels).unwrap();
    assert_eq!(chosen, Navigation::Back);
}
