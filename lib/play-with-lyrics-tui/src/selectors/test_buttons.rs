use crate::Navigation;
use crate::host::{Clock, ReadEvent, WindowSize};
use crate::selectors::_test_utils::{
    click_at, control, english_video, label_list, pop_scripted, press, standard_size,
};
use crate::selectors::list::select_one_loop;
use crate::selectors::video::select_video_loop;
use crossterm::event::{Event, KeyCode};
use pretty_assertions::assert_eq;
use std::collections::VecDeque;
use std::io;
use std::sync::Mutex;
use std::time::{Duration, SystemTime};

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
