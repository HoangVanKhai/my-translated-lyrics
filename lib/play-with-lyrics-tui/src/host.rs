//! The capability seams the selectors read the terminal through, and the
//! production provider that backs them with the real terminal.

use crossterm::event::{Event, read};
use crossterm::terminal::size;
use std::io;
use std::time::SystemTime;

/// Reads the next input event from the terminal.
pub(crate) trait ReadEvent {
    fn read_event() -> io::Result<Event>;
}

/// Reports the terminal size as `(columns, rows)`.
pub(crate) trait WindowSize {
    fn window_size() -> io::Result<(u16, u16)>;
}

/// Reports the current time.
pub(crate) trait Clock {
    fn now() -> SystemTime;
}

/// The production provider: it reads from the real terminal.
pub(crate) struct Host;

impl ReadEvent for Host {
    fn read_event() -> io::Result<Event> {
        read()
    }
}

impl WindowSize for Host {
    fn window_size() -> io::Result<(u16, u16)> {
        size()
    }
}

impl Clock for Host {
    fn now() -> SystemTime {
        SystemTime::now()
    }
}
