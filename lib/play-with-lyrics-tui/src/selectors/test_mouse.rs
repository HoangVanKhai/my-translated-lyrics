use crate::Navigation;
use crate::host::{Clock, ReadEvent, WindowSize};
use crate::render::HEADER_ROW;
use crate::selectors::_test_utils::{
    bilingual_video, click, click_at, control, english_video, hover_at, label_list, pop_scripted,
    press, scroll_down, scroll_up, standard_size, video,
};
use crate::selectors::list::select_one_loop;
use crate::selectors::video::select_video_loop;
use crossterm::event::{Event, KeyCode};
use play_with_lyrics::catalog::Video;
use pretty_assertions::assert_eq;
use std::collections::VecDeque;
use std::io;
use std::sync::Mutex;
use std::time::{Duration, SystemTime};

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

/// Scrolling the wheel up moves the cursor toward the top of the list.
#[test]
fn select_one_scroll_up_moves_the_cursor() {
    static EVENTS: Mutex<VecDeque<Event>> = Mutex::new(VecDeque::new());
    struct Scripted;
    impl ReadEvent for Scripted {
        fn read_event() -> io::Result<Event> {
            pop_scripted(&EVENTS)
        }
    }
    impl Clock for Scripted {
        fn now() -> SystemTime {
            SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000)
        }
    }
    impl WindowSize for Scripted {
        fn window_size() -> io::Result<(u16, u16)> {
            standard_size()
        }
    }
    let labels = label_list(&["alpha", "beta"]);
    // Starting on the second row, a wheel-up returns the cursor to the first.
    EVENTS
        .lock()
        .unwrap()
        .extend([scroll_up(), press(KeyCode::Enter)]);
    let chosen = select_one_loop::<Scripted>(&mut Vec::new(), "pick", &labels, 1).unwrap();
    assert_eq!(chosen, Navigation::Selected(0));
}

/// Scrolling up moves the table cursor toward the top, and a pointer movement
/// in between only updates the hover highlight.
#[test]
fn select_video_scroll_up_moves_the_cursor() {
    static EVENTS: Mutex<VecDeque<Event>> = Mutex::new(VecDeque::new());
    struct Scripted;
    impl ReadEvent for Scripted {
        fn read_event() -> io::Result<Event> {
            pop_scripted(&EVENTS)
        }
    }
    impl Clock for Scripted {
        fn now() -> SystemTime {
            SystemTime::UNIX_EPOCH + Duration::from_secs(1_705_000_000)
        }
    }
    impl WindowSize for Scripted {
        fn window_size() -> io::Result<(u16, u16)> {
            standard_size()
        }
    }
    let videos = vec![video("Alpha"), video("Beta")];
    // Start on the second row; a hover only highlights, then a wheel-up returns
    // the cursor to the first row, which Enter selects.
    EVENTS
        .lock()
        .unwrap()
        .extend([hover_at(10, 5), scroll_up(), press(KeyCode::Enter)]);
    let chosen =
        select_video_loop::<Scripted>(&mut Vec::new(), &videos, &mut String::new(), Some(1))
            .unwrap();
    assert_eq!(chosen, Navigation::Selected(0));
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

/// A click on the help line below a full window of rows selects nothing, not
/// the first item scrolled off the bottom.
#[test]
fn clicking_below_the_visible_rows_selects_nothing() {
    static EVENTS: Mutex<VecDeque<Event>> = Mutex::new(VecDeque::new());
    struct Scripted;
    impl ReadEvent for Scripted {
        fn read_event() -> io::Result<Event> {
            pop_scripted(&EVENTS)
        }
    }
    impl Clock for Scripted {
        fn now() -> SystemTime {
            SystemTime::UNIX_EPOCH + Duration::from_secs(1_708_111_222)
        }
    }
    impl WindowSize for Scripted {
        fn window_size() -> io::Result<(u16, u16)> {
            standard_size()
        }
    }
    // 25 zero-padded names sort in their listed order; a 24-row terminal shows
    // 20 of them (rows 3..23), with the help line at row 23.
    let videos: Vec<Video> = (0..25)
        .map(|n| english_video(&format!("Item {n:02}")))
        .collect();
    // Clicking row 23 (the help line) maps past the last visible row; it must
    // not focus item 20, so Enter still selects the top item.
    EVENTS
        .lock()
        .unwrap()
        .extend([click(23), press(KeyCode::Enter)]);
    let chosen =
        select_video_loop::<Scripted>(&mut Vec::new(), &videos, &mut String::new(), None).unwrap();
    assert_eq!(chosen, Navigation::Selected(0));
}

/// A sort between two clicks on the same row is not a double click, because the
/// row now shows a different item. Clicking row 3 (Alpha), then the Vietnamese
/// header (which reorders so Charlie is first), then row 3 again, must not
/// select: the two clicks landed on different videos.
#[test]
fn a_sort_between_clicks_is_not_a_double_click() {
    static EVENTS: Mutex<VecDeque<Event>> = Mutex::new(VecDeque::new());
    struct Scripted;
    impl ReadEvent for Scripted {
        fn read_event() -> io::Result<Event> {
            pop_scripted(&EVENTS)
        }
    }
    impl Clock for Scripted {
        // A fixed time makes both clicks fall inside the double-click window, so
        // only the item check can keep them from forming a double click.
        fn now() -> SystemTime {
            SystemTime::UNIX_EPOCH + Duration::from_secs(1_709_222_333)
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
    EVENTS.lock().unwrap().extend([
        click(3),
        click_at(30, HEADER_ROW),
        click(3),
        control(KeyCode::Char('q')),
    ]);
    let chosen =
        select_video_loop::<Scripted>(&mut Vec::new(), &videos, &mut String::new(), None).unwrap();
    assert_eq!(chosen, Navigation::Quit);
}

/// On a terminal too short to show every label, a click on the help line does
/// not reach the label hidden beneath it.
#[test]
fn select_one_ignores_a_click_on_the_help_line() {
    static EVENTS: Mutex<VecDeque<Event>> = Mutex::new(VecDeque::new());
    struct Scripted;
    impl ReadEvent for Scripted {
        fn read_event() -> io::Result<Event> {
            pop_scripted(&EVENTS)
        }
    }
    impl Clock for Scripted {
        fn now() -> SystemTime {
            SystemTime::UNIX_EPOCH + Duration::from_secs(1_710_333_444)
        }
    }
    impl WindowSize for Scripted {
        fn window_size() -> io::Result<(u16, u16)> {
            // Five rows: top bar at 0, labels at 1..4, help line at row 4.
            Ok((80, 5))
        }
    }
    let labels = label_list(&["alpha", "beta", "gamma", "delta"]);
    // Row 4 is the help line, which hides the fourth label drawn under it.
    // Two clicks there must not select; the following Ctrl-Q ends the loop.
    EVENTS
        .lock()
        .unwrap()
        .extend([click(4), click(4), control(KeyCode::Char('q'))]);
    let chosen = select_one_loop::<Scripted>(&mut Vec::new(), "pick", &labels, 0).unwrap();
    assert_eq!(chosen, Navigation::Quit);
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
