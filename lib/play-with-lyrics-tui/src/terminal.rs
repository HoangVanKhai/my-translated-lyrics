//! The terminal guard that switches into the alternate screen for a selector
//! and restores the normal terminal state on the way out.

use crossterm::ExecutableCommand;
use crossterm::cursor::{Hide, Show};
use crossterm::event::{
    DisableMouseCapture, EnableMouseCapture, KeyboardEnhancementFlags, PopKeyboardEnhancementFlags,
    PushKeyboardEnhancementFlags,
};
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
    supports_keyboard_enhancement,
};
use std::io::{self, Stderr};

/// Restores the terminal to its normal state when dropped, even if the
/// caller returns early or panics.
pub(crate) struct TerminalGuard {
    pub(crate) output: Stderr,
    /// Whether the keyboard enhancement protocol was enabled, so it is only
    /// popped when it was pushed.
    enhanced: bool,
}

impl TerminalGuard {
    pub(crate) fn enter() -> io::Result<Self> {
        enable_raw_mode()?;
        let mut output = io::stderr();
        output
            .execute(EnterAlternateScreen)?
            .execute(Hide)?
            .execute(EnableMouseCapture)?;
        // Request the keyboard enhancement protocol so modified keys such as
        // Ctrl-Backspace arrive with their modifier. Terminals that do not
        // support it are left untouched, and Ctrl-Backspace simply has no
        // effect there.
        let enhanced = matches!(supports_keyboard_enhancement(), Ok(true));
        if enhanced {
            output.execute(PushKeyboardEnhancementFlags(
                KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES,
            ))?;
        }
        Ok(TerminalGuard { output, enhanced })
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        // Best effort: there is nothing useful to do if restoring fails.
        if self.enhanced {
            let _ = self.output.execute(PopKeyboardEnhancementFlags);
        }
        let _ = self.output.execute(DisableMouseCapture);
        let _ = self.output.execute(Show);
        let _ = self.output.execute(LeaveAlternateScreen);
        let _ = disable_raw_mode();
    }
}
