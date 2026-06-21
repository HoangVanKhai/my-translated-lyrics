use crate::Navigation;
use crate::host::{Clock, ReadEvent, WindowSize};
use crate::selectors::_test_utils::{
    control, label_list, pop_scripted, press, shift, standard_size,
};
use crate::selectors::list::select_one_loop;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use pretty_assertions::assert_eq;
use std::collections::VecDeque;
use std::io;
use std::sync::Mutex;
use std::time::{Duration, SystemTime};

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

/// An unhandled key and a resize only prompt a redraw, so the loop reads on
/// until a key it recognizes, here Escape.
#[test]
fn select_one_ignores_unhandled_keys_and_resizes() {
    static EVENTS: Mutex<VecDeque<Event>> = Mutex::new(VecDeque::new());
    struct Scripted;
    impl ReadEvent for Scripted {
        fn read_event() -> io::Result<Event> {
            pop_scripted(&EVENTS)
        }
    }
    impl Clock for Scripted {
        fn now() -> SystemTime {
            SystemTime::UNIX_EPOCH + Duration::from_secs(1_672_531_200)
        }
    }
    impl WindowSize for Scripted {
        fn window_size() -> io::Result<(u16, u16)> {
            standard_size()
        }
    }
    let labels = label_list(&["alpha", "beta"]);
    // Left is not bound on a list page, and a resize only changes the layout.
    EVENTS.lock().unwrap().extend([
        press(KeyCode::Left),
        Event::Resize(100, 30),
        press(KeyCode::Esc),
    ]);
    let chosen = select_one_loop::<Scripted>(&mut Vec::new(), "pick", &labels, 0).unwrap();
    assert_eq!(chosen, Navigation::Back);
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
