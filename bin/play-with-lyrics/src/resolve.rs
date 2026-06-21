//! Resolving each choice from a command-line flag, automatically when only
//! one option exists, or through an interactive selector.

use crate::cli::Args;
use crate::failure::{
    Failure, FormatUnavailable, LanguageUnavailable, NotInteractive, Termination, UnresolvedTitle,
};
use fuzzy_select::fuzzy::resolve_unique;
use fuzzy_select::selection::Searchable;
use into_deduped::IntoDeduped;
use itertools::Itertools;
use lyrics_core::video_descriptor::Language;
use pipe_trait::Pipe;
use play_with_lyrics::catalog::{Video, language_label};
use play_with_lyrics::player::{Player, SubtitleFormat};
use play_with_lyrics_tui::{Navigation, select_one, select_video};
use std::fmt::Display;
use std::io::{self, IsTerminal};
use strum::VariantArray;

/// How a choice was resolved.
pub(crate) enum Resolution<Value> {
    /// Resolved without interaction, from a flag or because only one option
    /// exists. Such a step is not a place the user can return to.
    Auto(Value),
    /// Chosen through an interactive page, which the user can return to.
    Chosen(Value),
    /// The user asked to go back from the interactive page.
    Back,
}

/// Maps the outcome of an interactive selector to a [`Resolution`]. The
/// `value` closure turns the chosen index into the resolved value. A request
/// to quit becomes [`Termination::Cancelled`].
fn from_selection<Value>(
    selection: io::Result<Navigation>,
    value: impl FnOnce(usize) -> Value,
) -> Result<Resolution<Value>, Termination> {
    match selection.expect("interactive selection failed") {
        Navigation::Selected(index) => index.pipe(value).pipe(Resolution::Chosen).pipe(Ok),
        Navigation::Back => Ok(Resolution::Back),
        Navigation::Quit => Err(Termination::Cancelled),
    }
}

/// Resolves the video from `--title` or through the interactive table.
pub(crate) fn resolve_video(
    args: &Args,
    catalog: &[Video],
    query: &mut String,
    previous: Option<usize>,
) -> Result<Resolution<usize>, Termination> {
    if let Some(title) = &args.title {
        let video = resolve_unique(title, catalog, <Video as Searchable>::search_keys).map_err(
            |error| {
                Failure::UnresolvedTitle(UnresolvedTitle {
                    query: title.clone(),
                    error,
                })
            },
        )?;
        return catalog
            .iter()
            .position(|candidate| std::ptr::eq(candidate, video))
            .expect("the resolved video belongs to the catalog")
            .pipe(Resolution::Auto)
            .pipe(Ok);
    }
    require_terminal("a video title")?;
    from_selection(select_video(catalog, query, previous), |index| index)
}

/// Resolves the subtitle language from `--language`, automatically when
/// only one is available, or through an interactive selector.
pub(crate) fn resolve_language(
    args: &Args,
    available: &[(Language, SubtitleFormat)],
    previous: Option<Language>,
) -> Result<Resolution<Language>, Termination> {
    let languages = available
        .iter()
        .map(|(language, _)| *language)
        .collect::<Vec<Language>>()
        .into_deduped();

    if let Some(arg) = args.language {
        let requested = Language::from(arg);
        if languages.contains(&requested) {
            return Ok(Resolution::Auto(requested));
        }
        let error = LanguageUnavailable {
            requested,
            available: join_display(&languages),
        };
        return error
            .pipe(Failure::LanguageUnavailable)
            .pipe(Termination::from)
            .pipe(Err);
    }
    if let [only] = languages.as_slice() {
        return Ok(Resolution::Auto(*only));
    }
    require_terminal("a subtitle language")?;
    let labels: Vec<String> = languages
        .iter()
        .map(|language| format!("{} ({language})", language_label(*language)))
        .collect();
    let start = previous
        .and_then(|previous| languages.iter().position(|&language| language == previous))
        .unwrap_or(0);
    from_selection(select_one("Select a Language", &labels, start), |index| {
        languages[index]
    })
}

/// Resolves the subtitle format from `--format`, automatically when only
/// one is available, or through an interactive selector.
pub(crate) fn resolve_format(
    args: &Args,
    language: Language,
    formats: &[SubtitleFormat],
    previous: Option<SubtitleFormat>,
) -> Result<Resolution<SubtitleFormat>, Termination> {
    if let Some(arg) = args.format {
        let requested = SubtitleFormat::from(arg);
        if formats.contains(&requested) {
            return Ok(Resolution::Auto(requested));
        }
        let error = FormatUnavailable {
            language,
            requested,
            available: join_display(formats),
        };
        return error
            .pipe(Failure::FormatUnavailable)
            .pipe(Termination::from)
            .pipe(Err);
    }
    if let [only] = formats {
        return Ok(Resolution::Auto(*only));
    }
    require_terminal("a subtitle format")?;
    let labels: Vec<String> = formats
        .iter()
        .map(|format| format!("{} ({format})", format.full_name()))
        .collect();
    let start = previous
        .and_then(|previous| formats.iter().position(|&format| format == previous))
        .unwrap_or(0);
    from_selection(
        select_one("Select a Subtitle Format", &labels, start),
        |index| formats[index],
    )
}

/// Resolves the media player from `--player` or through an interactive
/// selector.
pub(crate) fn resolve_player(args: &Args) -> Result<Resolution<Player>, Termination> {
    if let Some(arg) = args.player {
        return Ok(Resolution::Auto(Player::from(arg)));
    }
    require_terminal("a media player")?;
    let labels: Vec<String> = Player::VARIANTS.iter().map(ToString::to_string).collect();
    // The player is the last page, so it is never returned to and starts at the
    // top each time.
    from_selection(select_one("Select a Media Player", &labels, 0), |index| {
        Player::VARIANTS[index]
    })
}

/// Returns a [`Failure::NotInteractive`] when an interactive selection is
/// required but standard input is not a terminal.
fn require_terminal(what: &'static str) -> Result<(), Termination> {
    io::stdin()
        .is_terminal()
        .then_some(())
        .ok_or(NotInteractive { what })
        .map_err(Failure::NotInteractive)
        .map_err(Termination::Failed)
}

/// Joins the displayed forms of `items` with commas, for an error message
/// that lists the available choices.
fn join_display<Item>(items: &[Item]) -> String
where
    Item: Display,
{
    items.iter().join(", ")
}
