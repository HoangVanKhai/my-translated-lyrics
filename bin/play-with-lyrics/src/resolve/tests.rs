use super::{Resolution, from_selection, require_terminal};
use crate::failure::{Failure, NotInteractive, Termination};
use crate::host::Stdin;
use play_with_lyrics_tui::Navigation;

/// A chosen row becomes [`Resolution::Chosen`], with the closure mapping the
/// index to the resolved value.
#[test]
fn from_selection_maps_a_choice_to_chosen() {
    let resolution = from_selection(Navigation::Selected(2), |index| index * 10);
    assert!(matches!(resolution, Ok(Resolution::Chosen(20))));
}

/// A request to go back becomes [`Resolution::Back`], so the caller can return
/// to the previous page.
#[test]
fn from_selection_maps_back() {
    let resolution = from_selection(Navigation::Back, |index| index);
    assert!(matches!(resolution, Ok(Resolution::Back)));
}

/// A request to quit becomes [`Termination::Cancelled`].
#[test]
fn from_selection_maps_quit_to_cancelled() {
    let resolution = from_selection(Navigation::Quit, |index| index);
    assert!(matches!(resolution, Err(Termination::Cancelled)));
}

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
