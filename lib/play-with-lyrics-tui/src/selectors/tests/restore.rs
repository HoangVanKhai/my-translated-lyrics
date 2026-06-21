use super::{english_video, label_list, pop_scripted, press, standard_size};
use crate::Navigation;
use crate::host::{Clock, ReadEvent, WindowSize};
use crate::selectors::list::select_one_loop;
use crate::selectors::video::select_video_loop;
use crossterm::event::{Event, KeyCode};
use pretty_assertions::assert_eq;
use std::collections::VecDeque;
use std::io;
use std::sync::Mutex;
use std::time::{Duration, SystemTime};

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
