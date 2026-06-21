//! Resolving each choice from a command-line flag, automatically when only
//! one option exists, or through an interactive selector.

use crate::cli::Args;
use crate::failure::{
    Failure, FormatUnavailable, LanguageUnavailable, NotInteractive, Termination, UnresolvedTitle,
};
use crate::host::{Select, Stdin};
use fuzzy_select::fuzzy::resolve_unique;
use fuzzy_select::selection::Searchable;
use into_deduped::IntoDeduped;
use itertools::Itertools;
use lyrics_core::video_descriptor::Language;
use pipe_trait::Pipe;
use play_with_lyrics::catalog::{Video, language_label};
use play_with_lyrics::player::{Player, SubtitleFormat};
use play_with_lyrics_tui::Navigation;
use std::fmt::Display;
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
/// `to_value` closure turns the chosen index into the resolved value. A request
/// to quit becomes [`Termination::Cancelled`].
fn from_selection<Value>(
    selection: Navigation,
    to_value: impl FnOnce(usize) -> Value,
) -> Result<Resolution<Value>, Termination> {
    match selection {
        Navigation::Selected(index) => index.pipe(to_value).pipe(Resolution::Chosen).pipe(Ok),
        Navigation::Back => Ok(Resolution::Back),
        Navigation::Quit => Err(Termination::Cancelled),
    }
}

/// Resolves the video from `--title` or through the interactive table. `Sys`
/// provides the standard-input check and the interactive selector.
pub(crate) fn resolve_video<Sys>(
    args: &Args,
    catalog: &[Video],
    query: &mut String,
    previous: Option<usize>,
) -> Result<Resolution<usize>, Termination>
where
    Sys: Stdin + Select,
{
    if let Some(title) = &args.title {
        let (index, _video) = resolve_unique(title, catalog, <Video as Searchable>::search_keys)
            .map_err(|error| {
                Failure::UnresolvedTitle(UnresolvedTitle {
                    query: title.clone(),
                    error,
                })
            })?;
        return index.pipe(Resolution::Auto).pipe(Ok);
    }
    require_terminal::<Sys>("a video title")?;
    let selection =
        Sys::select_video(catalog, query, previous).expect("interactive selection failed");
    from_selection(selection, |index| index)
}

/// Resolves the subtitle language from `--language`, automatically when
/// only one is available, or through an interactive selector. `Sys` provides
/// the standard-input check and the interactive selector.
pub(crate) fn resolve_language<Sys>(
    args: &Args,
    available: &[(Language, SubtitleFormat)],
    previous: Option<Language>,
) -> Result<Resolution<Language>, Termination>
where
    Sys: Stdin + Select,
{
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
    require_terminal::<Sys>("a subtitle language")?;
    let labels: Vec<String> = languages
        .iter()
        .map(|language| format!("{} ({language})", language_label(*language)))
        .collect();
    let start = previous
        .and_then(|previous| languages.iter().position(|&language| language == previous))
        .unwrap_or(0);
    let selection =
        Sys::select_one("Select a Language", &labels, start).expect("interactive selection failed");
    from_selection(selection, |index| languages[index])
}

/// Resolves the subtitle format from `--format`, automatically when only
/// one is available, or through an interactive selector. `Sys` provides the
/// standard-input check and the interactive selector.
pub(crate) fn resolve_format<Sys>(
    args: &Args,
    language: Language,
    formats: &[SubtitleFormat],
    previous: Option<SubtitleFormat>,
) -> Result<Resolution<SubtitleFormat>, Termination>
where
    Sys: Stdin + Select,
{
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
    require_terminal::<Sys>("a subtitle format")?;
    let labels: Vec<String> = formats
        .iter()
        .map(|format| format!("{} ({format})", format.full_name()))
        .collect();
    let start = previous
        .and_then(|previous| formats.iter().position(|&format| format == previous))
        .unwrap_or(0);
    let selection = Sys::select_one("Select a Subtitle Format", &labels, start)
        .expect("interactive selection failed");
    from_selection(selection, |index| formats[index])
}

/// Resolves the media player from `--player` or through an interactive
/// selector. `Sys` provides the standard-input check and the interactive
/// selector.
pub(crate) fn resolve_player<Sys>(args: &Args) -> Result<Resolution<Player>, Termination>
where
    Sys: Stdin + Select,
{
    if let Some(arg) = args.player {
        return Ok(Resolution::Auto(Player::from(arg)));
    }
    require_terminal::<Sys>("a media player")?;
    let labels: Vec<String> = Player::VARIANTS.iter().map(ToString::to_string).collect();
    // The player is the last page, so it is never returned to and starts at the
    // top each time.
    let selection =
        Sys::select_one("Select a Media Player", &labels, 0).expect("interactive selection failed");
    from_selection(selection, |index| Player::VARIANTS[index])
}

/// Returns a [`Failure::NotInteractive`] when an interactive selection is
/// required but standard input is not a terminal. `Sys` reads standard input;
/// production passes [`Host`].
fn require_terminal<Sys>(what: &'static str) -> Result<(), Termination>
where
    Sys: Stdin,
{
    Sys::is_terminal()
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

#[cfg(test)]
mod tests;
