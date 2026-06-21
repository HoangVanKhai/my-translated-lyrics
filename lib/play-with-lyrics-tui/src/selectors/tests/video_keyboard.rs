use super::{control, pop_scripted, press, standard_size, video};
use crate::Navigation;
use crate::host::{Clock, ReadEvent, WindowSize};
use crate::selectors::video::select_video_loop;
use crossterm::event::{Event, KeyCode};
use pretty_assertions::assert_eq;
use std::collections::VecDeque;
use std::io;
use std::sync::Mutex;
use std::time::{Duration, SystemTime};

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
