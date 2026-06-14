//! The capability seams the selectors read the terminal through, and the
//! production provider that backs them with the real terminal.

use crossterm::event::{Event, read};
use crossterm::terminal::size;
use std::io;
use std::time::SystemTime;

/// Reads the next input event from the terminal.
///
/// A dependency-injection seam: production reads from the real terminal with
/// [`Host`], while a test replays a scripted sequence of events, so the
/// otherwise unreachable event handling is testable without a TTY.
pub(crate) trait ReadEvent {
    fn read_event() -> io::Result<Event>;
}

/// Reports the terminal size as `(columns, rows)`.
///
/// A dependency-injection seam: a test reports a chosen size so the width- and
/// height-dependent layout can be asserted deterministically, without a real
/// terminal.
pub(crate) trait WindowSize {
    fn window_size() -> io::Result<(u16, u16)>;
}

/// Reports the current time, for measuring the gap between two clicks when
/// detecting a double click.
///
/// A dependency-injection seam: a test reports a fixed time so double-click
/// detection does not depend on how fast the clicks are processed. `SystemTime`
/// is used rather than `Instant` because it can be built at compile time from
/// the `UNIX_EPOCH` constant, which lets a fake return a fixed moment without a
/// real clock; the time is read only to compare two clicks moments apart, where
/// `SystemTime`'s lack of monotonicity does not matter in practice.
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
