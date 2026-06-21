//! Shared fixtures for the selector page tests. Each group of related tests
//! is its own sibling module and pulls the helpers it needs from here.

use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use lyrics_core::video_descriptor::{Language, VideoDesc, Visibility};
use play_with_lyrics::catalog::Video;
use std::collections::{HashMap, VecDeque};
use std::io;
use std::sync::Mutex;
use test_utils::video_desc;

/// A video with both an English and a Vietnamese title, so the table can be
/// sorted by either column to a different order.
pub(super) fn bilingual_video(english: &str, vietnamese: &str) -> Video {
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
pub(super) fn press(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
}

/// A key press combined with the Control modifier.
pub(super) fn control(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::CONTROL))
}

/// A key press combined with the Shift modifier.
pub(super) fn shift(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::SHIFT))
}

/// A left-button click at screen `column` and `row`.
pub(super) fn click_at(column: u16, row: u16) -> Event {
    Event::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column,
        row,
        modifiers: KeyModifiers::NONE,
    })
}

/// A left-button click in the first column of screen `row`.
pub(super) fn click(row: u16) -> Event {
    click_at(0, row)
}

/// A pointer movement to `column`, `row`, with no button held.
pub(super) fn hover_at(column: u16, row: u16) -> Event {
    Event::Mouse(MouseEvent {
        kind: MouseEventKind::Moved,
        column,
        row,
        modifiers: KeyModifiers::NONE,
    })
}

/// A scroll-wheel-down event.
pub(super) fn scroll_down() -> Event {
    Event::Mouse(MouseEvent {
        kind: MouseEventKind::ScrollDown,
        column: 0,
        row: 0,
        modifiers: KeyModifiers::NONE,
    })
}

/// A scroll-wheel-up event.
pub(super) fn scroll_up() -> Event {
    Event::Mouse(MouseEvent {
        kind: MouseEventKind::ScrollUp,
        column: 0,
        row: 0,
        modifiers: KeyModifiers::NONE,
    })
}

/// Pops the next scripted event from a test's own queue, reporting an error
/// if the loop reads past the end of the script it was given.
pub(super) fn pop_scripted(queue: &Mutex<VecDeque<Event>>) -> io::Result<Event> {
    queue.lock().unwrap().pop_front().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "the event script is exhausted",
        )
    })
}

pub(super) fn label_list(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| value.to_string()).collect()
}

pub(super) fn video(title: &str) -> Video {
    Video {
        desc: video_desc("Touhou Hero of Ice Fairy", title, Visibility::Visible),
    }
}

/// A video whose English title is `title`, so rows are distinguishable by
/// the English column in rendered output. The `video_desc` helper gives every
/// video the same placeholder titles, which would render identically.
pub(super) fn english_video(title: &str) -> Video {
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
pub(super) fn standard_size() -> io::Result<(u16, u16)> {
    Ok((80, 24))
}
