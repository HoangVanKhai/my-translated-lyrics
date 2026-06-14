use super::{render_header, select_one_loop, select_video_loop};
use crate::Navigation;
use crate::host::{Clock, ReadEvent, WindowSize};
use crate::render::{HEADER_ROW, column_spans};
use column_sort::ColumnSort;
use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use lyrics_core::video_descriptor::{Language, VideoDesc, Visibility};
use play_with_lyrics::catalog::Video;
use pretty_assertions::assert_eq;
use std::collections::{HashMap, VecDeque};
use std::io;
use std::sync::Mutex;
use std::time::{Duration, SystemTime};
use terminal_screen::{Buffer, Style};
use test_utils::video_desc;

/// A video with both an English and a Vietnamese title, so the table can be
/// sorted by either column to a different order.
fn bilingual_video(english: &str, vietnamese: &str) -> Video {
    Video {
        desc: VideoDesc {
            collection: "Touhou Hero of Ice Fairy".to_string().try_into().unwrap(),
            video_title: english.to_string().try_into().unwrap(),
            song_titles: HashMap::from([
                (Language::English, english.to_string()),
                (Language::Vietnamese, vietnamese.to_string()),
            ]),
            visibility: Visibility::Visible,
        },
    }
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

/// A left-button click at screen `column` and `row`.
fn click_at(column: u16, row: u16) -> Event {
    Event::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column,
        row,
        modifiers: KeyModifiers::NONE,
    })
}

/// A left-button click in the first column of screen `row`.
fn click(row: u16) -> Event {
    click_at(0, row)
}

/// A pointer movement to `column`, `row`, with no button held.
fn hover_at(column: u16, row: u16) -> Event {
    Event::Mouse(MouseEvent {
        kind: MouseEventKind::Moved,
        column,
        row,
        modifiers: KeyModifiers::NONE,
    })
}

/// A scroll-wheel-down event.
fn scroll_down() -> Event {
    Event::Mouse(MouseEvent {
        kind: MouseEventKind::ScrollDown,
        column: 0,
        row: 0,
        modifiers: KeyModifiers::NONE,
    })
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
    impl Clock for Scripted {
        fn now() -> SystemTime {
            SystemTime::UNIX_EPOCH + Duration::from_secs(1_669_457_355)
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
    let chosen = select_one_loop::<Scripted>(&mut Vec::new(), "pick", &labels, 0).unwrap();
    assert_eq!(chosen, Navigation::Selected(2));
}

/// Escape goes back from a list page, which is never the first page.
#[test]
fn select_one_goes_back_on_escape() {
    static EVENTS: Mutex<VecDeque<Event>> = Mutex::new(VecDeque::new());
    struct Scripted;
    impl ReadEvent for Scripted {
        fn read_event() -> io::Result<Event> {
            pop_scripted(&EVENTS)
        }
    }
    impl Clock for Scripted {
        fn now() -> SystemTime {
            SystemTime::UNIX_EPOCH + Duration::from_secs(1_617_613_607)
        }
    }
    impl WindowSize for Scripted {
        fn window_size() -> io::Result<(u16, u16)> {
            standard_size()
        }
    }
    let labels = label_list(&["alpha", "beta"]);
    EVENTS.lock().unwrap().extend([press(KeyCode::Esc)]);
    let chosen = select_one_loop::<Scripted>(&mut Vec::new(), "pick", &labels, 0).unwrap();
    assert_eq!(chosen, Navigation::Back);
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
    impl Clock for Scripted {
        fn now() -> SystemTime {
            SystemTime::UNIX_EPOCH + Duration::from_secs(1_684_364_767)
        }
    }
    impl WindowSize for Scripted {
        fn window_size() -> io::Result<(u16, u16)> {
            standard_size()
        }
    }
    let labels = label_list(&["alpha", "beta"]);
    EVENTS.lock().unwrap().extend([control(KeyCode::Char('c'))]);
    let chosen = select_one_loop::<Scripted>(&mut Vec::new(), "pick", &labels, 0).unwrap();
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
    impl Clock for Scripted {
        fn now() -> SystemTime {
            SystemTime::UNIX_EPOCH + Duration::from_secs(1_587_858_250)
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
    let chosen = select_one_loop::<Scripted>(&mut Vec::new(), "pick", &labels, 0).unwrap();
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
    impl Clock for Scripted {
        fn now() -> SystemTime {
            SystemTime::UNIX_EPOCH + Duration::from_secs(1_632_765_214)
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
    let chosen = select_one_loop::<Scripted>(&mut Vec::new(), "pick", &labels, 0).unwrap();
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
    impl Clock for Scripted {
        fn now() -> SystemTime {
            SystemTime::UNIX_EPOCH + Duration::from_secs(1_660_150_297)
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
    let chosen = select_one_loop::<Scripted>(&mut Vec::new(), "pick", &labels, 0).unwrap();
    // The empty Enter did not select; the following Escape ends the loop by
    // going back.
    assert_eq!(chosen, Navigation::Back);
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
    impl Clock for Scripted {
        fn now() -> SystemTime {
            SystemTime::UNIX_EPOCH + Duration::from_secs(1_762_678_056)
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
    let chosen =
        select_video_loop::<Scripted>(&mut Vec::new(), &videos, &mut String::new(), None).unwrap();
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
    impl Clock for Scripted {
        fn now() -> SystemTime {
            SystemTime::UNIX_EPOCH + Duration::from_secs(1_688_012_705)
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
    let chosen =
        select_video_loop::<Scripted>(&mut Vec::new(), &videos, &mut String::new(), None).unwrap();
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
    impl Clock for Scripted {
        fn now() -> SystemTime {
            SystemTime::UNIX_EPOCH + Duration::from_secs(1_646_430_706)
        }
    }
    impl WindowSize for Scripted {
        fn window_size() -> io::Result<(u16, u16)> {
            standard_size()
        }
    }
    let videos = vec![video("Alpha")];
    EVENTS.lock().unwrap().extend([press(KeyCode::Esc)]);
    let chosen =
        select_video_loop::<Scripted>(&mut Vec::new(), &videos, &mut String::new(), None).unwrap();
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
    impl Clock for Scripted {
        fn now() -> SystemTime {
            SystemTime::UNIX_EPOCH + Duration::from_secs(1_645_457_949)
        }
    }
    impl WindowSize for Scripted {
        fn window_size() -> io::Result<(u16, u16)> {
            standard_size()
        }
    }
    let videos = vec![video("Alpha")];
    EVENTS.lock().unwrap().extend([control(KeyCode::Char('q'))]);
    let chosen =
        select_video_loop::<Scripted>(&mut Vec::new(), &videos, &mut String::new(), None).unwrap();
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
    impl Clock for Scripted {
        fn now() -> SystemTime {
            SystemTime::UNIX_EPOCH + Duration::from_secs(1_611_722_613)
        }
    }
    impl WindowSize for Scripted {
        fn window_size() -> io::Result<(u16, u16)> {
            standard_size()
        }
    }
    let videos = vec![video("Alpha")];
    EVENTS.lock().unwrap().extend([control(KeyCode::Char('Q'))]);
    let chosen =
        select_video_loop::<Scripted>(&mut Vec::new(), &videos, &mut String::new(), None).unwrap();
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
    impl Clock for Scripted {
        fn now() -> SystemTime {
            SystemTime::UNIX_EPOCH + Duration::from_secs(1_680_338_900)
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
    let chosen =
        select_video_loop::<Scripted>(&mut Vec::new(), &videos, &mut String::new(), None).unwrap();
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
    impl Clock for Scripted {
        fn now() -> SystemTime {
            SystemTime::UNIX_EPOCH + Duration::from_secs(1_602_298_170)
        }
    }
    impl WindowSize for Scripted {
        fn window_size() -> io::Result<(u16, u16)> {
            standard_size()
        }
    }
    let labels = label_list(&["alpha", "beta"]);
    EVENTS.lock().unwrap().extend([press(KeyCode::Char('q'))]);
    let chosen = select_one_loop::<Scripted>(&mut Vec::new(), "pick", &labels, 0).unwrap();
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
    impl Clock for Scripted {
        fn now() -> SystemTime {
            SystemTime::UNIX_EPOCH + Duration::from_secs(1_684_603_475)
        }
    }
    impl WindowSize for Scripted {
        fn window_size() -> io::Result<(u16, u16)> {
            standard_size()
        }
    }
    let labels = label_list(&["alpha", "beta"]);
    EVENTS.lock().unwrap().extend([shift(KeyCode::Char('Q'))]);
    let chosen = select_one_loop::<Scripted>(&mut Vec::new(), "pick", &labels, 0).unwrap();
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
    impl Clock for Scripted {
        fn now() -> SystemTime {
            SystemTime::UNIX_EPOCH + Duration::from_secs(1_724_331_079)
        }
    }
    impl WindowSize for Scripted {
        fn window_size() -> io::Result<(u16, u16)> {
            standard_size()
        }
    }
    let labels = label_list(&["alpha", "beta"]);
    EVENTS.lock().unwrap().extend([control(KeyCode::Char('q'))]);
    let chosen = select_one_loop::<Scripted>(&mut Vec::new(), "pick", &labels, 0).unwrap();
    assert_eq!(chosen, Navigation::Quit);
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
    let videos = vec![video("Alpha")];
    // Quit right after the first frame is drawn.
    EVENTS.lock().unwrap().extend([control(KeyCode::Char('q'))]);
    let mut buffer = Vec::new();
    select_video_loop::<Scripted>(&mut buffer, &videos, &mut String::new(), None).unwrap();
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
    let videos = vec![video("Alpha")];
    EVENTS.lock().unwrap().extend([control(KeyCode::Char('q'))]);
    let mut buffer = Vec::new();
    select_video_loop::<Scripted>(&mut buffer, &videos, &mut String::new(), None).unwrap();
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
    let rendered = String::from_utf8_lossy(&buffer);
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
    let videos = vec![video("Alpha")];
    EVENTS.lock().unwrap().extend([control(KeyCode::Char('q'))]);
    let mut buffer = Vec::new();
    select_video_loop::<Scripted>(&mut buffer, &videos, &mut String::new(), None).unwrap();
    let rendered = String::from_utf8_lossy(&buffer);
    // The 80-column fallback is wide enough to show the native header.
    assert!(rendered.contains("Tiếng Việt"), "{rendered}");
}

/// On the song page, Ctrl-Backspace goes back, which on this first page is
/// the way out.
#[test]
fn select_video_ctrl_backspace_goes_back() {
    static EVENTS: Mutex<VecDeque<Event>> = Mutex::new(VecDeque::new());
    struct Scripted;
    impl ReadEvent for Scripted {
        fn read_event() -> io::Result<Event> {
            pop_scripted(&EVENTS)
        }
    }
    impl Clock for Scripted {
        fn now() -> SystemTime {
            SystemTime::UNIX_EPOCH + Duration::from_secs(1_762_452_348)
        }
    }
    impl WindowSize for Scripted {
        fn window_size() -> io::Result<(u16, u16)> {
            standard_size()
        }
    }
    let videos = vec![video("Alpha")];
    EVENTS.lock().unwrap().extend([control(KeyCode::Backspace)]);
    let chosen =
        select_video_loop::<Scripted>(&mut Vec::new(), &videos, &mut String::new(), None).unwrap();
    assert_eq!(chosen, Navigation::Back);
}

/// On the song page, plain Backspace only deletes, so an empty query plus
/// Backspace does not go back; clearing the box by holding it never exits.
#[test]
fn select_video_plain_backspace_does_not_go_back() {
    static EVENTS: Mutex<VecDeque<Event>> = Mutex::new(VecDeque::new());
    struct Scripted;
    impl ReadEvent for Scripted {
        fn read_event() -> io::Result<Event> {
            pop_scripted(&EVENTS)
        }
    }
    impl Clock for Scripted {
        fn now() -> SystemTime {
            SystemTime::UNIX_EPOCH + Duration::from_secs(1_705_233_553)
        }
    }
    impl WindowSize for Scripted {
        fn window_size() -> io::Result<(u16, u16)> {
            standard_size()
        }
    }
    let videos = vec![video("Alpha")];
    // Backspace on the empty box is a no-op; only the following quit ends it.
    EVENTS
        .lock()
        .unwrap()
        .extend([press(KeyCode::Backspace), control(KeyCode::Char('q'))]);
    let chosen =
        select_video_loop::<Scripted>(&mut Vec::new(), &videos, &mut String::new(), None).unwrap();
    assert_eq!(chosen, Navigation::Quit);
}

/// On a non-typing list page, Space confirms the highlighted row just like
/// Enter.
#[test]
fn select_one_selects_on_space() {
    static EVENTS: Mutex<VecDeque<Event>> = Mutex::new(VecDeque::new());
    struct Scripted;
    impl ReadEvent for Scripted {
        fn read_event() -> io::Result<Event> {
            pop_scripted(&EVENTS)
        }
    }
    impl Clock for Scripted {
        fn now() -> SystemTime {
            SystemTime::UNIX_EPOCH + Duration::from_secs(1_648_099_508)
        }
    }
    impl WindowSize for Scripted {
        fn window_size() -> io::Result<(u16, u16)> {
            standard_size()
        }
    }
    let labels = label_list(&["alpha", "beta"]);
    EVENTS
        .lock()
        .unwrap()
        .extend([press(KeyCode::Down), press(KeyCode::Char(' '))]);
    let chosen = select_one_loop::<Scripted>(&mut Vec::new(), "pick", &labels, 0).unwrap();
    assert_eq!(chosen, Navigation::Selected(1));
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
    impl Clock for Scripted {
        fn now() -> SystemTime {
            SystemTime::UNIX_EPOCH + Duration::from_secs(1_715_598_825)
        }
    }
    impl WindowSize for Scripted {
        fn window_size() -> io::Result<(u16, u16)> {
            standard_size()
        }
    }
    let labels = label_list(&["alpha", "beta"]);
    EVENTS.lock().unwrap().extend([press(KeyCode::Backspace)]);
    let chosen = select_one_loop::<Scripted>(&mut Vec::new(), "pick", &labels, 0).unwrap();
    assert_eq!(chosen, Navigation::Back);
}

/// Ctrl-Backspace, the universal back key, also goes back on a list page.
#[test]
fn select_one_ctrl_backspace_goes_back() {
    static EVENTS: Mutex<VecDeque<Event>> = Mutex::new(VecDeque::new());
    struct Scripted;
    impl ReadEvent for Scripted {
        fn read_event() -> io::Result<Event> {
            pop_scripted(&EVENTS)
        }
    }
    impl Clock for Scripted {
        fn now() -> SystemTime {
            SystemTime::UNIX_EPOCH + Duration::from_secs(1_661_191_560)
        }
    }
    impl WindowSize for Scripted {
        fn window_size() -> io::Result<(u16, u16)> {
            standard_size()
        }
    }
    let labels = label_list(&["alpha", "beta"]);
    EVENTS.lock().unwrap().extend([control(KeyCode::Backspace)]);
    let chosen = select_one_loop::<Scripted>(&mut Vec::new(), "pick", &labels, 0).unwrap();
    assert_eq!(chosen, Navigation::Back);
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
    let rendered = String::from_utf8_lossy(&buffer);
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
    let rendered = String::from_utf8_lossy(&buffer);
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
    let rendered = String::from_utf8_lossy(&buffer);
    assert!(rendered.contains("⌫ delete"), "{rendered:?}");
    assert!(rendered.contains("^⌫ back"), "{rendered:?}");
}

/// The list starts with the cursor on `start`, to restore a prior choice; an
/// immediate Enter then confirms that row without moving.
#[test]
fn select_one_starts_on_the_given_row() {
    static EVENTS: Mutex<VecDeque<Event>> = Mutex::new(VecDeque::new());
    struct Scripted;
    impl ReadEvent for Scripted {
        fn read_event() -> io::Result<Event> {
            pop_scripted(&EVENTS)
        }
    }
    impl Clock for Scripted {
        fn now() -> SystemTime {
            SystemTime::UNIX_EPOCH + Duration::from_secs(1_682_507_444)
        }
    }
    impl WindowSize for Scripted {
        fn window_size() -> io::Result<(u16, u16)> {
            standard_size()
        }
    }
    let labels = label_list(&["alpha", "beta", "gamma"]);
    EVENTS.lock().unwrap().extend([press(KeyCode::Enter)]);
    let chosen = select_one_loop::<Scripted>(&mut Vec::new(), "pick", &labels, 1).unwrap();
    assert_eq!(chosen, Navigation::Selected(1));
}

/// A `start` past the last row is clamped rather than leaving an unselectable
/// cursor.
#[test]
fn select_one_clamps_an_out_of_range_start() {
    static EVENTS: Mutex<VecDeque<Event>> = Mutex::new(VecDeque::new());
    struct Scripted;
    impl ReadEvent for Scripted {
        fn read_event() -> io::Result<Event> {
            pop_scripted(&EVENTS)
        }
    }
    impl Clock for Scripted {
        fn now() -> SystemTime {
            SystemTime::UNIX_EPOCH + Duration::from_secs(1_721_678_388)
        }
    }
    impl WindowSize for Scripted {
        fn window_size() -> io::Result<(u16, u16)> {
            standard_size()
        }
    }
    let labels = label_list(&["alpha", "beta"]);
    EVENTS.lock().unwrap().extend([press(KeyCode::Enter)]);
    let chosen = select_one_loop::<Scripted>(&mut Vec::new(), "pick", &labels, 9).unwrap();
    assert_eq!(chosen, Navigation::Selected(1));
}

/// The table starts with the cursor on the given video, to restore a prior
/// choice; an immediate Enter then confirms it.
#[test]
fn select_video_starts_on_the_selected_video() {
    static EVENTS: Mutex<VecDeque<Event>> = Mutex::new(VecDeque::new());
    struct Scripted;
    impl ReadEvent for Scripted {
        fn read_event() -> io::Result<Event> {
            pop_scripted(&EVENTS)
        }
    }
    impl Clock for Scripted {
        fn now() -> SystemTime {
            SystemTime::UNIX_EPOCH + Duration::from_secs(1_737_077_732)
        }
    }
    impl WindowSize for Scripted {
        fn window_size() -> io::Result<(u16, u16)> {
            standard_size()
        }
    }
    let videos = vec![
        english_video("First"),
        english_video("Second"),
        english_video("Third"),
    ];
    EVENTS.lock().unwrap().extend([press(KeyCode::Enter)]);
    let chosen =
        select_video_loop::<Scripted>(&mut Vec::new(), &videos, &mut String::new(), Some(2))
            .unwrap();
    assert_eq!(chosen, Navigation::Selected(2));
}

/// The table starts with the given query already applied, to restore a prior
/// search.
#[test]
fn select_video_starts_with_the_given_query() {
    static EVENTS: Mutex<VecDeque<Event>> = Mutex::new(VecDeque::new());
    struct Scripted;
    impl ReadEvent for Scripted {
        fn read_event() -> io::Result<Event> {
            pop_scripted(&EVENTS)
        }
    }
    impl Clock for Scripted {
        fn now() -> SystemTime {
            SystemTime::UNIX_EPOCH + Duration::from_secs(1_696_011_440)
        }
    }
    impl WindowSize for Scripted {
        fn window_size() -> io::Result<(u16, u16)> {
            standard_size()
        }
    }
    let videos = vec![english_video("Alpha"), english_video("Beta")];
    // "beta" filters to the second video; Enter then selects it.
    let mut query = String::from("beta");
    EVENTS.lock().unwrap().extend([press(KeyCode::Enter)]);
    let chosen = select_video_loop::<Scripted>(&mut Vec::new(), &videos, &mut query, None).unwrap();
    assert_eq!(chosen, Navigation::Selected(1));
}

/// The final query is written back, so the caller can restore it next time.
#[test]
fn select_video_writes_back_the_final_query() {
    static EVENTS: Mutex<VecDeque<Event>> = Mutex::new(VecDeque::new());
    struct Scripted;
    impl ReadEvent for Scripted {
        fn read_event() -> io::Result<Event> {
            pop_scripted(&EVENTS)
        }
    }
    impl Clock for Scripted {
        fn now() -> SystemTime {
            SystemTime::UNIX_EPOCH + Duration::from_secs(1_762_767_185)
        }
    }
    impl WindowSize for Scripted {
        fn window_size() -> io::Result<(u16, u16)> {
            standard_size()
        }
    }
    let videos = vec![english_video("Alpha")];
    let mut query = String::new();
    EVENTS
        .lock()
        .unwrap()
        .extend([press(KeyCode::Char('a')), press(KeyCode::Enter)]);
    select_video_loop::<Scripted>(&mut Vec::new(), &videos, &mut query, None).unwrap();
    assert_eq!(query, "a");
}

/// A single click highlights the clicked label without confirming, so a
/// following Enter selects that row.
#[test]
fn select_one_single_click_highlights_the_clicked_row() {
    static EVENTS: Mutex<VecDeque<Event>> = Mutex::new(VecDeque::new());
    struct Scripted;
    impl ReadEvent for Scripted {
        fn read_event() -> io::Result<Event> {
            pop_scripted(&EVENTS)
        }
    }
    impl Clock for Scripted {
        fn now() -> SystemTime {
            SystemTime::UNIX_EPOCH + Duration::from_secs(1_616_532_322)
        }
    }
    impl WindowSize for Scripted {
        fn window_size() -> io::Result<(u16, u16)> {
            standard_size()
        }
    }
    let labels = label_list(&["alpha", "beta", "gamma"]);
    // Labels render at rows 1, 2, 3, directly below the top bar; clicking row 2
    // highlights "beta", then Enter confirms it.
    EVENTS
        .lock()
        .unwrap()
        .extend([click(2), press(KeyCode::Enter)]);
    let chosen = select_one_loop::<Scripted>(&mut Vec::new(), "pick", &labels, 0).unwrap();
    assert_eq!(chosen, Navigation::Selected(1));
}

/// A single click on its own does not select; it only moves the highlight.
#[test]
fn select_one_single_click_does_not_select() {
    static EVENTS: Mutex<VecDeque<Event>> = Mutex::new(VecDeque::new());
    struct Scripted;
    impl ReadEvent for Scripted {
        fn read_event() -> io::Result<Event> {
            pop_scripted(&EVENTS)
        }
    }
    impl Clock for Scripted {
        fn now() -> SystemTime {
            SystemTime::UNIX_EPOCH + Duration::from_secs(1_682_756_276)
        }
    }
    impl WindowSize for Scripted {
        fn window_size() -> io::Result<(u16, u16)> {
            standard_size()
        }
    }
    let labels = label_list(&["alpha", "beta"]);
    EVENTS
        .lock()
        .unwrap()
        .extend([click(2), press(KeyCode::Esc)]);
    let chosen = select_one_loop::<Scripted>(&mut Vec::new(), "pick", &labels, 0).unwrap();
    // The single click only moved the highlight; the following Escape ends the
    // loop by going back.
    assert_eq!(chosen, Navigation::Back);
}

/// A double click on a label row selects it.
#[test]
fn select_one_double_click_selects_the_clicked_row() {
    static EVENTS: Mutex<VecDeque<Event>> = Mutex::new(VecDeque::new());
    struct Scripted;
    impl ReadEvent for Scripted {
        fn read_event() -> io::Result<Event> {
            pop_scripted(&EVENTS)
        }
    }
    impl Clock for Scripted {
        fn now() -> SystemTime {
            SystemTime::UNIX_EPOCH + Duration::from_secs(1_726_468_386)
        }
    }
    impl WindowSize for Scripted {
        fn window_size() -> io::Result<(u16, u16)> {
            standard_size()
        }
    }
    let labels = label_list(&["alpha", "beta", "gamma"]);
    EVENTS.lock().unwrap().extend([click(2), click(2)]);
    let chosen = select_one_loop::<Scripted>(&mut Vec::new(), "pick", &labels, 0).unwrap();
    assert_eq!(chosen, Navigation::Selected(1));
}

/// The scroll wheel moves the list cursor.
#[test]
fn select_one_scroll_moves_the_cursor() {
    static EVENTS: Mutex<VecDeque<Event>> = Mutex::new(VecDeque::new());
    struct Scripted;
    impl ReadEvent for Scripted {
        fn read_event() -> io::Result<Event> {
            pop_scripted(&EVENTS)
        }
    }
    impl Clock for Scripted {
        fn now() -> SystemTime {
            SystemTime::UNIX_EPOCH + Duration::from_secs(1_752_709_849)
        }
    }
    impl WindowSize for Scripted {
        fn window_size() -> io::Result<(u16, u16)> {
            standard_size()
        }
    }
    let labels = label_list(&["alpha", "beta"]);
    EVENTS
        .lock()
        .unwrap()
        .extend([scroll_down(), press(KeyCode::Enter)]);
    let chosen = select_one_loop::<Scripted>(&mut Vec::new(), "pick", &labels, 0).unwrap();
    assert_eq!(chosen, Navigation::Selected(1));
}

/// A single click highlights the clicked video without confirming, so a
/// following Enter selects it.
#[test]
fn select_video_single_click_highlights_the_clicked_row() {
    static EVENTS: Mutex<VecDeque<Event>> = Mutex::new(VecDeque::new());
    struct Scripted;
    impl ReadEvent for Scripted {
        fn read_event() -> io::Result<Event> {
            pop_scripted(&EVENTS)
        }
    }
    impl Clock for Scripted {
        fn now() -> SystemTime {
            SystemTime::UNIX_EPOCH + Duration::from_secs(1_715_200_158)
        }
    }
    impl WindowSize for Scripted {
        fn window_size() -> io::Result<(u16, u16)> {
            standard_size()
        }
    }
    let videos = vec![
        english_video("First"),
        english_video("Second"),
        english_video("Third"),
    ];
    // Data rows render at 3, 4, 5; clicking row 4 highlights the second video,
    // then Enter confirms it.
    EVENTS
        .lock()
        .unwrap()
        .extend([click(4), press(KeyCode::Enter)]);
    let chosen =
        select_video_loop::<Scripted>(&mut Vec::new(), &videos, &mut String::new(), None).unwrap();
    assert_eq!(chosen, Navigation::Selected(1));
}

/// A single click on its own does not select a video; it only moves the
/// highlight.
#[test]
fn select_video_single_click_does_not_select() {
    static EVENTS: Mutex<VecDeque<Event>> = Mutex::new(VecDeque::new());
    struct Scripted;
    impl ReadEvent for Scripted {
        fn read_event() -> io::Result<Event> {
            pop_scripted(&EVENTS)
        }
    }
    impl Clock for Scripted {
        fn now() -> SystemTime {
            SystemTime::UNIX_EPOCH + Duration::from_secs(1_599_322_043)
        }
    }
    impl WindowSize for Scripted {
        fn window_size() -> io::Result<(u16, u16)> {
            standard_size()
        }
    }
    let videos = vec![english_video("First"), english_video("Second")];
    EVENTS
        .lock()
        .unwrap()
        .extend([click(4), control(KeyCode::Char('q'))]);
    let chosen =
        select_video_loop::<Scripted>(&mut Vec::new(), &videos, &mut String::new(), None).unwrap();
    assert_eq!(chosen, Navigation::Quit);
}

/// A double click on a table row selects the video there.
#[test]
fn select_video_double_click_selects_the_clicked_row() {
    static EVENTS: Mutex<VecDeque<Event>> = Mutex::new(VecDeque::new());
    struct Scripted;
    impl ReadEvent for Scripted {
        fn read_event() -> io::Result<Event> {
            pop_scripted(&EVENTS)
        }
    }
    impl Clock for Scripted {
        fn now() -> SystemTime {
            SystemTime::UNIX_EPOCH + Duration::from_secs(1_758_333_602)
        }
    }
    impl WindowSize for Scripted {
        fn window_size() -> io::Result<(u16, u16)> {
            standard_size()
        }
    }
    let videos = vec![
        english_video("First"),
        english_video("Second"),
        english_video("Third"),
    ];
    EVENTS.lock().unwrap().extend([click(4), click(4)]);
    let chosen =
        select_video_loop::<Scripted>(&mut Vec::new(), &videos, &mut String::new(), None).unwrap();
    assert_eq!(chosen, Navigation::Selected(1));
}

/// A click above the first title, on the prompt or header, selects nothing.
/// The top bar, where the buttons sit, is tested separately.
#[test]
fn select_video_click_above_the_rows_does_nothing() {
    static EVENTS: Mutex<VecDeque<Event>> = Mutex::new(VecDeque::new());
    struct Scripted;
    impl ReadEvent for Scripted {
        fn read_event() -> io::Result<Event> {
            pop_scripted(&EVENTS)
        }
    }
    impl Clock for Scripted {
        fn now() -> SystemTime {
            SystemTime::UNIX_EPOCH + Duration::from_secs(1_767_187_724)
        }
    }
    impl WindowSize for Scripted {
        fn window_size() -> io::Result<(u16, u16)> {
            standard_size()
        }
    }
    let videos = vec![english_video("First")];
    // Row 2 is the header, above the first title row; the top bar is row 0.
    EVENTS
        .lock()
        .unwrap()
        .extend([click(2), control(KeyCode::Char('q'))]);
    let chosen =
        select_video_loop::<Scripted>(&mut Vec::new(), &videos, &mut String::new(), None).unwrap();
    assert_eq!(chosen, Navigation::Quit);
}

/// The scroll wheel moves the table cursor.
#[test]
fn select_video_scroll_moves_the_cursor() {
    static EVENTS: Mutex<VecDeque<Event>> = Mutex::new(VecDeque::new());
    struct Scripted;
    impl ReadEvent for Scripted {
        fn read_event() -> io::Result<Event> {
            pop_scripted(&EVENTS)
        }
    }
    impl Clock for Scripted {
        fn now() -> SystemTime {
            SystemTime::UNIX_EPOCH + Duration::from_secs(1_745_793_916)
        }
    }
    impl WindowSize for Scripted {
        fn window_size() -> io::Result<(u16, u16)> {
            standard_size()
        }
    }
    let videos = vec![english_video("First"), english_video("Second")];
    EVENTS
        .lock()
        .unwrap()
        .extend([scroll_down(), press(KeyCode::Enter)]);
    let chosen =
        select_video_loop::<Scripted>(&mut Vec::new(), &videos, &mut String::new(), None).unwrap();
    assert_eq!(chosen, Navigation::Selected(1));
}

/// Clicking the "Exit" top-bar button quits the list selector.
#[test]
fn select_one_exit_button_quits() {
    static EVENTS: Mutex<VecDeque<Event>> = Mutex::new(VecDeque::new());
    struct Scripted;
    impl ReadEvent for Scripted {
        fn read_event() -> io::Result<Event> {
            pop_scripted(&EVENTS)
        }
    }
    impl Clock for Scripted {
        fn now() -> SystemTime {
            SystemTime::UNIX_EPOCH + Duration::from_secs(1_755_728_654)
        }
    }
    impl WindowSize for Scripted {
        fn window_size() -> io::Result<(u16, u16)> {
            standard_size()
        }
    }
    let labels = label_list(&["alpha", "beta"]);
    // The top bar is row 0; column 75 falls on the right-aligned "Exit".
    EVENTS.lock().unwrap().extend([click_at(75, 0)]);
    let chosen = select_one_loop::<Scripted>(&mut Vec::new(), "pick", &labels, 0).unwrap();
    assert_eq!(chosen, Navigation::Quit);
}

/// Clicking the "Go back" top-bar button returns from the list selector.
#[test]
fn select_one_back_button_goes_back() {
    static EVENTS: Mutex<VecDeque<Event>> = Mutex::new(VecDeque::new());
    struct Scripted;
    impl ReadEvent for Scripted {
        fn read_event() -> io::Result<Event> {
            pop_scripted(&EVENTS)
        }
    }
    impl Clock for Scripted {
        fn now() -> SystemTime {
            SystemTime::UNIX_EPOCH + Duration::from_secs(1_684_773_973)
        }
    }
    impl WindowSize for Scripted {
        fn window_size() -> io::Result<(u16, u16)> {
            standard_size()
        }
    }
    let labels = label_list(&["alpha", "beta"]);
    // Column 5 falls on "Go back", at the left of the top bar.
    EVENTS.lock().unwrap().extend([click_at(5, 0)]);
    let chosen = select_one_loop::<Scripted>(&mut Vec::new(), "pick", &labels, 0).unwrap();
    assert_eq!(chosen, Navigation::Back);
}

/// Clicking the "Forward" top-bar button selects the highlighted row, the same
/// as pressing Enter.
#[test]
fn select_one_forward_button_selects_the_highlighted_row() {
    static EVENTS: Mutex<VecDeque<Event>> = Mutex::new(VecDeque::new());
    struct Scripted;
    impl ReadEvent for Scripted {
        fn read_event() -> io::Result<Event> {
            pop_scripted(&EVENTS)
        }
    }
    impl Clock for Scripted {
        fn now() -> SystemTime {
            SystemTime::UNIX_EPOCH + Duration::from_secs(1_710_695_619)
        }
    }
    impl WindowSize for Scripted {
        fn window_size() -> io::Result<(u16, u16)> {
            standard_size()
        }
    }
    let labels = label_list(&["alpha", "beta", "gamma"]);
    // Move the highlight down, then click "Forward" at column 20.
    EVENTS
        .lock()
        .unwrap()
        .extend([press(KeyCode::Down), click_at(20, 0)]);
    let chosen = select_one_loop::<Scripted>(&mut Vec::new(), "pick", &labels, 0).unwrap();
    assert_eq!(chosen, Navigation::Selected(1));
}

/// Clicking the "Exit" top-bar button quits the table.
#[test]
fn select_video_exit_button_quits() {
    static EVENTS: Mutex<VecDeque<Event>> = Mutex::new(VecDeque::new());
    struct Scripted;
    impl ReadEvent for Scripted {
        fn read_event() -> io::Result<Event> {
            pop_scripted(&EVENTS)
        }
    }
    impl Clock for Scripted {
        fn now() -> SystemTime {
            SystemTime::UNIX_EPOCH + Duration::from_secs(1_705_629_322)
        }
    }
    impl WindowSize for Scripted {
        fn window_size() -> io::Result<(u16, u16)> {
            standard_size()
        }
    }
    let videos = vec![english_video("First"), english_video("Second")];
    // Column 75 falls on the right-aligned "Exit" in the top bar.
    EVENTS.lock().unwrap().extend([click_at(75, 0)]);
    let chosen =
        select_video_loop::<Scripted>(&mut Vec::new(), &videos, &mut String::new(), None).unwrap();
    assert_eq!(chosen, Navigation::Quit);
}

/// The table is the first page, so its "Go back" button is disabled: clicking
/// it does nothing, and a following Ctrl-Q is what ends the loop.
#[test]
fn select_video_back_button_is_disabled() {
    static EVENTS: Mutex<VecDeque<Event>> = Mutex::new(VecDeque::new());
    struct Scripted;
    impl ReadEvent for Scripted {
        fn read_event() -> io::Result<Event> {
            pop_scripted(&EVENTS)
        }
    }
    impl Clock for Scripted {
        fn now() -> SystemTime {
            SystemTime::UNIX_EPOCH + Duration::from_secs(1_739_444_713)
        }
    }
    impl WindowSize for Scripted {
        fn window_size() -> io::Result<(u16, u16)> {
            standard_size()
        }
    }
    let videos = vec![english_video("First"), english_video("Second")];
    // Column 5 falls on the dimmed "Go back" button, which is a no-op here.
    EVENTS
        .lock()
        .unwrap()
        .extend([click_at(5, 0), control(KeyCode::Char('q'))]);
    let chosen =
        select_video_loop::<Scripted>(&mut Vec::new(), &videos, &mut String::new(), None).unwrap();
    assert_eq!(chosen, Navigation::Quit);
}

/// Clicking the "Forward" top-bar button selects the highlighted video, the
/// same as pressing Enter.
#[test]
fn select_video_forward_button_selects_the_highlighted_video() {
    static EVENTS: Mutex<VecDeque<Event>> = Mutex::new(VecDeque::new());
    struct Scripted;
    impl ReadEvent for Scripted {
        fn read_event() -> io::Result<Event> {
            pop_scripted(&EVENTS)
        }
    }
    impl Clock for Scripted {
        fn now() -> SystemTime {
            SystemTime::UNIX_EPOCH + Duration::from_secs(1_663_808_390)
        }
    }
    impl WindowSize for Scripted {
        fn window_size() -> io::Result<(u16, u16)> {
            standard_size()
        }
    }
    let videos = vec![english_video("First"), english_video("Second")];
    // Move the highlight down to the second video, then click "Forward" at
    // column 20.
    EVENTS
        .lock()
        .unwrap()
        .extend([press(KeyCode::Down), click_at(20, 0)]);
    let chosen =
        select_video_loop::<Scripted>(&mut Vec::new(), &videos, &mut String::new(), None).unwrap();
    assert_eq!(chosen, Navigation::Selected(1));
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
    let rendered = String::from_utf8_lossy(&buffer);
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
    let rendered = String::from_utf8_lossy(&buffer);
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
    let rendered = String::from_utf8_lossy(&buffer);
    assert!(rendered.contains("\u{1b}[7m"), "{rendered:?}");
}

/// Clicking a column header re-sorts the table by that column. Sorting by
/// Vietnamese brings the video with the first Vietnamese title to the top.
#[test]
fn clicking_a_column_header_re_sorts_by_that_column() {
    static EVENTS: Mutex<VecDeque<Event>> = Mutex::new(VecDeque::new());
    struct Scripted;
    impl ReadEvent for Scripted {
        fn read_event() -> io::Result<Event> {
            pop_scripted(&EVENTS)
        }
    }
    impl Clock for Scripted {
        fn now() -> SystemTime {
            SystemTime::UNIX_EPOCH + Duration::from_secs(1_703_456_789)
        }
    }
    impl WindowSize for Scripted {
        fn window_size() -> io::Result<(u16, u16)> {
            standard_size()
        }
    }
    let videos = vec![
        bilingual_video("Alpha", "Zulu"),
        bilingual_video("Bravo", "Yankee"),
        bilingual_video("Charlie", "Xray"),
    ];
    // The default English sort shows Alpha, Bravo, Charlie. Column 30 on the
    // header row falls on the Vietnamese header; clicking it sorts by
    // Vietnamese, putting Charlie (Xray) first. Clicking the first data row,
    // then Enter, selects it: Charlie is item index 2.
    EVENTS
        .lock()
        .unwrap()
        .extend([click_at(30, HEADER_ROW), click(3), press(KeyCode::Enter)]);
    let chosen =
        select_video_loop::<Scripted>(&mut Vec::new(), &videos, &mut String::new(), None).unwrap();
    assert_eq!(chosen, Navigation::Selected(2));
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
    // The hovered English header is drawn bold and reversed.
    assert_eq!(
        buffer.style_at(0, HEADER_ROW),
        Style::BOLD.with(Style::REVERSE),
    );
    // A column the pointer is not over stays plain bold.
    let vietnamese_start = column_spans(80)[1].start as u16;
    assert_eq!(buffer.style_at(vietnamese_start, HEADER_ROW), Style::BOLD);
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
    let rendered = String::from_utf8_lossy(&buffer);
    assert!(rendered.contains('🔍'), "{rendered:?}");
    assert!(rendered.contains("Search:"), "{rendered:?}");
    // The italic attribute (SGR 3) is applied to the label.
    assert!(rendered.contains("\u{1b}[3m"), "{rendered:?}");
    // The bold attribute (SGR 1) precedes the typed query.
    let bold = "\u{1b}[1m";
    assert!(rendered.contains(&format!("{bold}alpha")), "{rendered:?}");
}
