//! The capability seams the selectors read the terminal through, and the
//! production provider that backs them with the real terminal.

use crossterm::event::{Event, read};
use crossterm::terminal::size;
use std::io;
use std::time::SystemTime;
use terminal_screen::{Height, Width};

/// Reads the next input event from the terminal.
pub(crate) trait ReadEvent {
    fn read_event() -> io::Result<Event>;
}

/// Reports the terminal size.
pub(crate) trait WindowSize {
    fn window_size() -> io::Result<(Width, Height)>;
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
    fn window_size() -> io::Result<(Width, Height)> {
        let (width, height) = size()?;
        Ok((Width::new(width), Height::new(height)))
    }
}

impl Clock for Host {
    fn now() -> SystemTime {
        SystemTime::now()
    }
}
