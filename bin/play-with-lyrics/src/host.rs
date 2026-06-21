//! The capability seam the resolver reads standard input through, and the
//! production provider that backs it with the real standard input.

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

/// The production provider: it consults the real standard input.
pub(crate) struct Host;

impl Stdin for Host {
    fn is_terminal() -> bool {
        io::stdin().is_terminal()
    }
}
