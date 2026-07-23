use crate::host::{Clock, ReadEvent, WindowSize};
use crate::render::{HEADER_ROW, SEARCH_ROW, column_spans};
use crate::selectors::_test_utils::{
    control, english_video, hover_at, label_list, pop_scripted, press, standard_size, video,
};
use crate::selectors::list::select_one_loop;
use crate::selectors::video::{render_header, render_search_bar, select_video_loop};
use column_sort::ColumnSort;
use crossterm::event::{Event, KeyCode};
use lyrics_core::video_descriptor::Language;
use pretty_assertions::assert_eq;
use std::collections::VecDeque;
use std::io;
use std::sync::Mutex;
use std::time::{Duration, SystemTime};
use terminal_screen::{Buffer, Style};

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
    impl Clock for Scripted {
        fn now() -> SystemTime {
            SystemTime::UNIX_EPOCH + Duration::from_secs(1_668_062_428)
        }
    }
    impl WindowSize for Scripted {
        fn window_size() -> io::Result<(u16, u16)> {
            Ok((80, 24))
        }
    }
    let videos = vec![video("Alpha".to_owned())];
    // Quit right after the first frame is drawn.
    EVENTS.lock().unwrap().extend([control(KeyCode::Char('q'))]);
    let mut buffer = Vec::new();
    select_video_loop::<Scripted>(&mut buffer, &videos, &mut String::new(), None).unwrap();
    let rendered = str::from_utf8(&buffer).unwrap();
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
    impl Clock for Scripted {
        fn now() -> SystemTime {
            SystemTime::UNIX_EPOCH + Duration::from_secs(1_628_105_030)
        }
    }
    impl WindowSize for Scripted {
        fn window_size() -> io::Result<(u16, u16)> {
            // Each of the three columns gets about six display columns.
            Ok((24, 24))
        }
    }
    let videos = vec![video("Alpha".to_owned())];
    EVENTS.lock().unwrap().extend([control(KeyCode::Char('q'))]);
    let mut buffer = Vec::new();
    select_video_loop::<Scripted>(&mut buffer, &videos, &mut String::new(), None).unwrap();
    let rendered = str::from_utf8(&buffer).unwrap();
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
    impl Clock for Scripted {
        fn now() -> SystemTime {
            SystemTime::UNIX_EPOCH + Duration::from_secs(1_722_259_165)
        }
    }
    impl WindowSize for Scripted {
        fn window_size() -> io::Result<(u16, u16)> {
            // Six rows leave room for two title rows, after the prompt, header,
            // help, and button lines.
            Ok((80, 6))
        }
    }
    // The names are alphabetical, so the default English sort keeps this order.
    let videos = vec![
        english_video("Alpha"),
        english_video("Bravo"),
        english_video("Charlie"),
        english_video("Delta"),
    ];
    EVENTS.lock().unwrap().extend([control(KeyCode::Char('q'))]);
    let mut buffer = Vec::new();
    select_video_loop::<Scripted>(&mut buffer, &videos, &mut String::new(), None).unwrap();
    let rendered = str::from_utf8(&buffer).unwrap();
    assert!(rendered.contains("Alpha"), "{rendered}");
    assert!(rendered.contains("Bravo"), "{rendered}");
    assert!(!rendered.contains("Charlie"), "{rendered}");
    assert!(!rendered.contains("Delta"), "{rendered}");
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
    impl Clock for Scripted {
        fn now() -> SystemTime {
            SystemTime::UNIX_EPOCH + Duration::from_secs(1_728_497_133)
        }
    }
    impl WindowSize for Scripted {
        fn window_size() -> io::Result<(u16, u16)> {
            Err(io::Error::other("the terminal size is unavailable"))
        }
    }
    let videos = vec![video("Alpha".to_owned())];
    EVENTS.lock().unwrap().extend([control(KeyCode::Char('q'))]);
    let mut buffer = Vec::new();
    select_video_loop::<Scripted>(&mut buffer, &videos, &mut String::new(), None).unwrap();
    let rendered = str::from_utf8(&buffer).unwrap();
    // The 80-column fallback is wide enough to show the native header.
    assert!(rendered.contains("Tiếng Việt"), "{rendered}");
}

/// Typing a query that matches a title underlines the matched characters,
/// which crossterm emits as the SGR underline escape.
#[test]
fn select_video_underlines_matched_characters() {
    static EVENTS: Mutex<VecDeque<Event>> = Mutex::new(VecDeque::new());
    struct Scripted;
    impl ReadEvent for Scripted {
        fn read_event() -> io::Result<Event> {
            pop_scripted(&EVENTS)
        }
    }
    impl Clock for Scripted {
        fn now() -> SystemTime {
            SystemTime::UNIX_EPOCH + Duration::from_secs(1_761_568_740)
        }
    }
    impl WindowSize for Scripted {
        fn window_size() -> io::Result<(u16, u16)> {
            standard_size()
        }
    }
    let videos = vec![english_video("Alpha")];
    // "al" matches the start of "Alpha", so those characters are underlined.
    EVENTS.lock().unwrap().extend([
        press(KeyCode::Char('a')),
        press(KeyCode::Char('l')),
        control(KeyCode::Char('q')),
    ]);
    let mut buffer = Vec::new();
    select_video_loop::<Scripted>(&mut buffer, &videos, &mut String::new(), None).unwrap();
    let rendered = str::from_utf8(&buffer).unwrap();
    assert!(rendered.contains("\u{1b}[4m"), "{rendered:?}");
}

/// With no query typed, nothing is underlined.
#[test]
fn select_video_does_not_underline_without_a_query() {
    static EVENTS: Mutex<VecDeque<Event>> = Mutex::new(VecDeque::new());
    struct Scripted;
    impl ReadEvent for Scripted {
        fn read_event() -> io::Result<Event> {
            pop_scripted(&EVENTS)
        }
    }
    impl Clock for Scripted {
        fn now() -> SystemTime {
            SystemTime::UNIX_EPOCH + Duration::from_secs(1_757_755_084)
        }
    }
    impl WindowSize for Scripted {
        fn window_size() -> io::Result<(u16, u16)> {
            standard_size()
        }
    }
    let videos = vec![english_video("Alpha")];
    EVENTS.lock().unwrap().extend([control(KeyCode::Char('q'))]);
    let mut buffer = Vec::new();
    select_video_loop::<Scripted>(&mut buffer, &videos, &mut String::new(), None).unwrap();
    let rendered = str::from_utf8(&buffer).unwrap();
    assert!(!rendered.contains("\u{1b}[4m"), "{rendered:?}");
}

/// The typing-page footer shows Backspace as delete and Ctrl-Backspace as the
/// way back.
#[test]
fn select_video_footer_shows_delete_and_back() {
    static EVENTS: Mutex<VecDeque<Event>> = Mutex::new(VecDeque::new());
    struct Scripted;
    impl ReadEvent for Scripted {
        fn read_event() -> io::Result<Event> {
            pop_scripted(&EVENTS)
        }
    }
    impl Clock for Scripted {
        fn now() -> SystemTime {
            SystemTime::UNIX_EPOCH + Duration::from_secs(1_641_715_796)
        }
    }
    impl WindowSize for Scripted {
        fn window_size() -> io::Result<(u16, u16)> {
            standard_size()
        }
    }
    let videos = vec![english_video("Alpha")];
    EVENTS.lock().unwrap().extend([control(KeyCode::Char('q'))]);
    let mut buffer = Vec::new();
    select_video_loop::<Scripted>(&mut buffer, &videos, &mut String::new(), None).unwrap();
    let rendered = str::from_utf8(&buffer).unwrap();
    assert!(rendered.contains("⌫ delete"), "{rendered:?}");
    assert!(rendered.contains("^⌫ back"), "{rendered:?}");
}

/// The list page names itself in the top bar with the title it is given.
#[test]
fn select_one_shows_the_page_title_in_the_top_bar() {
    static EVENTS: Mutex<VecDeque<Event>> = Mutex::new(VecDeque::new());
    struct Scripted;
    impl ReadEvent for Scripted {
        fn read_event() -> io::Result<Event> {
            pop_scripted(&EVENTS)
        }
    }
    impl Clock for Scripted {
        fn now() -> SystemTime {
            SystemTime::UNIX_EPOCH + Duration::from_secs(1_701_234_567)
        }
    }
    impl WindowSize for Scripted {
        fn window_size() -> io::Result<(u16, u16)> {
            standard_size()
        }
    }
    let labels = label_list(&["alpha", "beta"]);
    EVENTS.lock().unwrap().extend([press(KeyCode::Esc)]);
    let mut buffer = Vec::new();
    select_one_loop::<Scripted>(&mut buffer, "Select a Language", &labels, 0).unwrap();
    let rendered = str::from_utf8(&buffer).unwrap();
    assert!(rendered.contains("Select a Language"), "{rendered}");
}

/// Moving the pointer over a label draws it bold, the hover effect for a
/// selectable item, and the movement alone triggers the redraw.
#[test]
fn select_one_bolds_the_label_under_the_pointer() {
    static EVENTS: Mutex<VecDeque<Event>> = Mutex::new(VecDeque::new());
    struct Scripted;
    impl ReadEvent for Scripted {
        fn read_event() -> io::Result<Event> {
            pop_scripted(&EVENTS)
        }
    }
    impl Clock for Scripted {
        fn now() -> SystemTime {
            SystemTime::UNIX_EPOCH + Duration::from_secs(1_690_123_456)
        }
    }
    impl WindowSize for Scripted {
        fn window_size() -> io::Result<(u16, u16)> {
            standard_size()
        }
    }
    let labels = label_list(&["alpha", "beta", "gamma"]);
    // "beta" is at row 2 (index 1, below the top bar). The cursor starts on
    // "alpha", so a bold run can only come from the hover.
    EVENTS
        .lock()
        .unwrap()
        .extend([hover_at(0, 2), press(KeyCode::Esc)]);
    let mut buffer = Vec::new();
    select_one_loop::<Scripted>(&mut buffer, "Select a Language", &labels, 0).unwrap();
    let rendered = str::from_utf8(&buffer).unwrap();
    assert!(rendered.contains("\u{1b}[1m"), "{rendered:?}");
}

/// Moving the pointer over a control button draws it in reverse video.
#[test]
fn select_one_reverses_the_button_under_the_pointer() {
    static EVENTS: Mutex<VecDeque<Event>> = Mutex::new(VecDeque::new());
    struct Scripted;
    impl ReadEvent for Scripted {
        fn read_event() -> io::Result<Event> {
            pop_scripted(&EVENTS)
        }
    }
    impl Clock for Scripted {
        fn now() -> SystemTime {
            SystemTime::UNIX_EPOCH + Duration::from_secs(1_695_678_901)
        }
    }
    impl WindowSize for Scripted {
        fn window_size() -> io::Result<(u16, u16)> {
            standard_size()
        }
    }
    // An empty list has no cursor row to reverse, so the only reverse run can
    // come from the hovered button.
    let labels: Vec<String> = Vec::new();
    EVENTS
        .lock()
        .unwrap()
        .extend([hover_at(5, 0), control(KeyCode::Char('c'))]);
    let mut buffer = Vec::new();
    select_one_loop::<Scripted>(&mut buffer, "Select a Language", &labels, 0).unwrap();
    let rendered = str::from_utf8(&buffer).unwrap();
    assert!(rendered.contains("\u{1b}[7m"), "{rendered:?}");
}

/// The header marks the sorted column with a direction arrow and draws the
/// column under the pointer bold and reversed.
#[test]
fn render_header_marks_the_sorted_and_hovered_columns() {
    let sort = ColumnSort::new([Language::English, Language::Vietnamese, Language::Chinese]);
    let mut buffer = Buffer::new(80, 3);
    // Hover the English header, at column 5 on the header row.
    render_header(&mut buffer, 80, &sort, Some((5, HEADER_ROW)));
    let header = buffer.row_text(HEADER_ROW);
    // English is the default sort column, ascending, so it carries the ▲ arrow.
    assert!(header.contains("English ▲"), "{header}");
    // The hovered English header is bold without the dim.
    assert_eq!(buffer.style_at(0, HEADER_ROW), Style::BOLD);
    // A column the pointer is not over is bold and dimmed.
    let vietnamese_start = column_spans(80)[1].start as u16;
    assert_eq!(
        buffer.style_at(vietnamese_start, HEADER_ROW),
        Style::BOLD.with(Style::DIM),
    );
    // The separator bar between the headers is bold but not dimmed.
    let separator_bar = column_spans(80)[0].end as u16 + 1;
    assert_eq!(buffer.style_at(separator_bar, HEADER_ROW), Style::BOLD);
}

/// The search bar dims the magnifier, italicizes the "Search:" label, and bolds
/// the typed query.
#[test]
fn render_search_bar_styles_the_magnifier_label_and_query() {
    let mut buffer = Buffer::new(40, 2);
    render_search_bar(&mut buffer, 40, "abc");
    let row = buffer.row_text(SEARCH_ROW);
    assert!(row.contains("Search:"), "{row}");
    assert!(row.contains("abc"), "{row}");
    // The magnifier is dimmed, not italic.
    assert_eq!(buffer.style_at(0, SEARCH_ROW), Style::DIM);
    // The "Search:" label is italic. The magnifier spans columns 0-1 and column
    // 2 is the label's leading space, so column 3 is its first letter.
    assert_eq!(buffer.style_at(3, SEARCH_ROW), Style::ITALIC);
    // The typed query is bold. The magnifier (2) and " Search: " (9) take eleven
    // columns, so the query begins at column 11.
    assert_eq!(buffer.style_at(11, SEARCH_ROW), Style::BOLD);
}

/// The search bar shows a magnifier with the italic "Search:" label and the
/// typed query in bold.
#[test]
fn the_search_bar_shows_a_magnifier_with_styled_parts() {
    static EVENTS: Mutex<VecDeque<Event>> = Mutex::new(VecDeque::new());
    struct Scripted;
    impl ReadEvent for Scripted {
        fn read_event() -> io::Result<Event> {
            pop_scripted(&EVENTS)
        }
    }
    impl Clock for Scripted {
        fn now() -> SystemTime {
            SystemTime::UNIX_EPOCH + Duration::from_secs(1_701_987_654)
        }
    }
    impl WindowSize for Scripted {
        fn window_size() -> io::Result<(u16, u16)> {
            standard_size()
        }
    }
    let videos = vec![english_video("Alpha")];
    EVENTS.lock().unwrap().extend([control(KeyCode::Char('q'))]);
    let mut buffer = Vec::new();
    let mut query = "alpha".to_string();
    select_video_loop::<Scripted>(&mut buffer, &videos, &mut query, None).unwrap();
    let rendered = str::from_utf8(&buffer).unwrap();
    // The magnifier is sent with its text-presentation variation selector.
    assert!(rendered.contains("🔍︎"), "{rendered:?}");
    // The label keeps a space before the typed query, the one-column gap.
    assert!(rendered.contains("Search: "), "{rendered:?}");
    // The italic attribute (SGR 3) is applied to the label.
    assert!(rendered.contains("\u{1b}[3m"), "{rendered:?}");
    // The bold attribute (SGR 1) precedes the typed query.
    let bold = "\u{1b}[1m";
    assert!(rendered.contains(&format!("{bold}alpha")), "{rendered:?}");
}
