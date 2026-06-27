use super::{
    Resolution, from_selection, require_terminal, resolve_format, resolve_language, resolve_player,
    resolve_video,
};
use crate::cli::Args;
use crate::failure::{Failure, NotInteractive, Termination};
use crate::host::{Select, Stdin};
use clap::Parser;
use lyrics_core::video_descriptor::Language;
use play_with_lyrics::catalog::Video;
use play_with_lyrics::player::{Player, SubtitleFormat};
use play_with_lyrics_tui::Navigation;
use std::io;

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

/// Without `--title`, the video is chosen through the interactive table, and
/// the chosen row becomes the resolved index.
#[test]
fn resolve_video_uses_the_interactive_table() {
    struct Pick;
    impl Stdin for Pick {
        fn is_terminal() -> bool {
            true
        }
    }
    impl Select for Pick {
        fn select_video(_: &[Video], _: &mut String, _: Option<usize>) -> io::Result<Navigation> {
            Ok(Navigation::Selected(2))
        }
        fn select_one(_: &str, _: &[String], _: usize) -> io::Result<Navigation> {
            unreachable!("the video page uses the table, not the list")
        }
    }
    let args = Args::parse_from(["play-with-lyrics", "src", "tgt"]);
    let resolution = resolve_video::<Pick>(&args, &[], &mut String::new(), None);
    assert!(matches!(resolution, Ok(Resolution::Chosen(2))));
}

/// With more than one language and no `--language`, the language is chosen
/// through the interactive list. A `previous` language seeds the starting row.
#[test]
fn resolve_language_uses_the_interactive_list() {
    struct Pick;
    impl Stdin for Pick {
        fn is_terminal() -> bool {
            true
        }
    }
    impl Select for Pick {
        fn select_video(_: &[Video], _: &mut String, _: Option<usize>) -> io::Result<Navigation> {
            unreachable!("the language page uses the list, not the table")
        }
        fn select_one(_: &str, _: &[String], _: usize) -> io::Result<Navigation> {
            Ok(Navigation::Selected(0))
        }
    }
    let args = Args::parse_from(["play-with-lyrics", "src", "tgt"]);
    let available = [
        (Language::English, SubtitleFormat::SubRip),
        (Language::Vietnamese, SubtitleFormat::SubRip),
    ];
    let resolution = resolve_language::<Pick>(&args, &available, Some(Language::Vietnamese));
    assert!(matches!(
        resolution,
        Ok(Resolution::Chosen(Language::English))
    ));
}

/// Without a previous language, the interactive list starts at the top.
#[test]
fn resolve_language_starts_at_the_top_without_a_previous() {
    struct Pick;
    impl Stdin for Pick {
        fn is_terminal() -> bool {
            true
        }
    }
    impl Select for Pick {
        fn select_video(_: &[Video], _: &mut String, _: Option<usize>) -> io::Result<Navigation> {
            unreachable!("the language page uses the list, not the table")
        }
        fn select_one(_: &str, _: &[String], _: usize) -> io::Result<Navigation> {
            Ok(Navigation::Selected(1))
        }
    }
    let args = Args::parse_from(["play-with-lyrics", "src", "tgt"]);
    let available = [
        (Language::English, SubtitleFormat::SubRip),
        (Language::Vietnamese, SubtitleFormat::SubRip),
    ];
    let resolution = resolve_language::<Pick>(&args, &available, None);
    assert!(matches!(
        resolution,
        Ok(Resolution::Chosen(Language::Vietnamese))
    ));
}

/// With more than one format and no `--format`, the format is chosen through
/// the interactive list. A `previous` format seeds the starting row.
#[test]
fn resolve_format_uses_the_interactive_list() {
    struct Pick;
    impl Stdin for Pick {
        fn is_terminal() -> bool {
            true
        }
    }
    impl Select for Pick {
        fn select_video(_: &[Video], _: &mut String, _: Option<usize>) -> io::Result<Navigation> {
            unreachable!("the format page uses the list, not the table")
        }
        fn select_one(_: &str, _: &[String], _: usize) -> io::Result<Navigation> {
            Ok(Navigation::Selected(0))
        }
    }
    let args = Args::parse_from(["play-with-lyrics", "src", "tgt"]);
    let formats = [SubtitleFormat::SubRip, SubtitleFormat::WebVtt];
    let resolution = resolve_format::<Pick>(
        &args,
        Language::Vietnamese,
        &formats,
        Some(SubtitleFormat::WebVtt),
    );
    assert!(matches!(
        resolution,
        Ok(Resolution::Chosen(SubtitleFormat::SubRip))
    ));
}

/// Without a previous format, the interactive list starts at the top.
#[test]
fn resolve_format_starts_at_the_top_without_a_previous() {
    struct Pick;
    impl Stdin for Pick {
        fn is_terminal() -> bool {
            true
        }
    }
    impl Select for Pick {
        fn select_video(_: &[Video], _: &mut String, _: Option<usize>) -> io::Result<Navigation> {
            unreachable!("the format page uses the list, not the table")
        }
        fn select_one(_: &str, _: &[String], _: usize) -> io::Result<Navigation> {
            Ok(Navigation::Selected(1))
        }
    }
    let args = Args::parse_from(["play-with-lyrics", "src", "tgt"]);
    let formats = [SubtitleFormat::SubRip, SubtitleFormat::WebVtt];
    let resolution = resolve_format::<Pick>(&args, Language::Vietnamese, &formats, None);
    assert!(matches!(
        resolution,
        Ok(Resolution::Chosen(SubtitleFormat::WebVtt))
    ));
}

/// Without `--player`, the player is chosen through the interactive list.
#[test]
fn resolve_player_uses_the_interactive_list() {
    struct Pick;
    impl Stdin for Pick {
        fn is_terminal() -> bool {
            true
        }
    }
    impl Select for Pick {
        fn select_video(_: &[Video], _: &mut String, _: Option<usize>) -> io::Result<Navigation> {
            unreachable!("the player page uses the list, not the table")
        }
        fn select_one(_: &str, _: &[String], _: usize) -> io::Result<Navigation> {
            Ok(Navigation::Selected(0))
        }
    }
    let args = Args::parse_from(["play-with-lyrics", "src", "tgt"]);
    let resolution = resolve_player::<Pick>(&args);
    assert!(matches!(resolution, Ok(Resolution::Chosen(Player::Mpv))));
}
