//! The capability seams the resolver reads standard input and runs the
//! interactive selectors through, and the production provider that backs them
//! with the real terminal.

use play_with_lyrics::catalog::Video;
use play_with_lyrics_tui::{Navigation, select_one, select_video};
use std::io::{self, IsTerminal};

/// Reports whether standard input is connected to an interactive terminal.
///
/// A dependency-injection seam: production consults the real standard input
/// with [`Host`], while a test supplies a fake that returns a chosen value, so
/// both branches of [`require_terminal`] are covered without a real terminal.
///
/// [`require_terminal`]: crate::resolve
pub(crate) trait Stdin {
    fn is_terminal() -> bool;
}

/// Runs an interactive selector and reports the user's [`Navigation`].
///
/// A dependency-injection seam: production opens the real terminal selectors
/// with [`Host`], while a test returns a scripted outcome, so the interactive
/// branches of the resolvers are covered without a TTY.
pub(crate) trait Select {
    /// Presents the fuzzy table of videos.
    fn select_video(
        videos: &[Video],
        query: &mut String,
        selected: Option<usize>,
    ) -> io::Result<Navigation>;

    /// Presents a single-column list of `labels` under `title`.
    fn select_one(title: &str, labels: &[String], start: usize) -> io::Result<Navigation>;
}

/// The production provider: it reads the real standard input and opens the real
/// terminal selectors.
pub(crate) struct Host;

impl Stdin for Host {
    fn is_terminal() -> bool {
        io::stdin().is_terminal()
    }
}

impl Select for Host {
    fn select_video(
        videos: &[Video],
        query: &mut String,
        selected: Option<usize>,
    ) -> io::Result<Navigation> {
        select_video(videos, query, selected)
    }

    fn select_one(title: &str, labels: &[String], start: usize) -> io::Result<Navigation> {
        select_one(title, labels, start)
    }
}
