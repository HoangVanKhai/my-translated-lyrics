use super::require_terminal;
use crate::failure::{Failure, NotInteractive, Termination};
use crate::host::Stdin;

/// When standard input is a terminal, the interactive selection is allowed to
/// proceed, so the check succeeds.
#[test]
fn require_terminal_allows_an_interactive_terminal() {
    struct Interactive;
    impl Stdin for Interactive {
        fn is_terminal() -> bool {
            true
        }
    }
    assert!(require_terminal::<Interactive>("a video title").is_ok());
}

/// When standard input is not a terminal, the program cannot prompt, so the
/// check fails with a [`Failure::NotInteractive`] naming what was needed.
#[test]
fn require_terminal_rejects_a_non_terminal() {
    struct NonInteractive;
    impl Stdin for NonInteractive {
        fn is_terminal() -> bool {
            false
        }
    }
    let error = require_terminal::<NonInteractive>("a video title").unwrap_err();
    assert!(matches!(
        error,
        Termination::Failed(Failure::NotInteractive(NotInteractive {
            what: "a video title"
        }))
    ));
}
