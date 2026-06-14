//! Resolving each choice from a command-line flag, automatically when only
//! one option exists, or through an interactive selector.

use crate::cli::Args;
use crate::failure::{
    Failure, FormatUnavailable, LanguageUnavailable, NotInteractive, Termination, UnresolvedTitle,
};
use fuzzy_select::fuzzy::resolve_unique;
use fuzzy_select::selection::Searchable;
use lyrics_core::video_descriptor::Language;
use pipe_trait::Pipe;
use play_with_lyrics::catalog::{Video, language_label};
use play_with_lyrics::player::{Player, SubtitleFormat};
use play_with_lyrics_tui::{select_one, select_video};
use std::io::{self, IsTerminal};
use strum::VariantArray;

/// Resolves the video from `--title` or through the interactive table.
pub(crate) fn resolve_video<'a>(
    args: &Args,
    catalog: &'a [Video],
) -> Result<&'a Video, Termination> {
    match &args.title {
        Some(query) => {
            resolve_unique(query, catalog, <Video as Searchable>::search_keys).map_err(|error| {
                Failure::UnresolvedTitle(UnresolvedTitle {
                    query: query.clone(),
                    error,
                })
                .into()
            })
        }
        None => {
            require_terminal("a video title")?;
            match select_video(catalog).expect("interactive selection failed") {
                Some(index) => Ok(&catalog[index]),
                None => Err(Termination::Cancelled),
            }
        }
    }
}

/// Resolves the subtitle language from `--language`, automatically when
/// only one is available, or through an interactive selector.
pub(crate) fn resolve_language(
    args: &Args,
    available: &[(Language, SubtitleFormat)],
) -> Result<Language, Termination> {
    let mut languages: Vec<Language> = available.iter().map(|(language, _)| *language).collect();
    languages.dedup();

    if let Some(arg) = args.language {
        let requested = Language::from(arg);
        return if languages.contains(&requested) {
            Ok(requested)
        } else {
            Err(Failure::LanguageUnavailable(LanguageUnavailable {
                requested,
                available: join_display(&languages),
            })
            .into())
        };
    }
    if let [only] = languages.as_slice() {
        return Ok(*only);
    }
    require_terminal("a subtitle language")?;
    let labels: Vec<String> = languages
        .iter()
        .map(|language| format!("{} ({language})", language_label(*language)))
        .collect();
    match select_one("Select subtitle language", &labels).expect("interactive selection failed") {
        Some(index) => Ok(languages[index]),
        None => Err(Termination::Cancelled),
    }
}

/// Resolves the subtitle format from `--format`, automatically when only
/// one is available, or through an interactive selector.
pub(crate) fn resolve_format(
    args: &Args,
    language: Language,
    formats: &[SubtitleFormat],
) -> Result<SubtitleFormat, Termination> {
    if let Some(arg) = args.format {
        let requested = SubtitleFormat::from(arg);
        if formats.contains(&requested) {
            return Ok(requested);
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
        return Ok(*only);
    }
    require_terminal("a subtitle format")?;
    let labels: Vec<String> = formats
        .iter()
        .map(|format| format!("{} ({format})", format.full_name()))
        .collect();
    match select_one("Select subtitle format", &labels).expect("interactive selection failed") {
        Some(index) => Ok(formats[index]),
        None => Err(Termination::Cancelled),
    }
}

/// Resolves the media player from `--player` or through an interactive
/// selector.
pub(crate) fn resolve_player(args: &Args) -> Result<Player, Termination> {
    if let Some(arg) = args.player {
        return Ok(Player::from(arg));
    }
    require_terminal("a media player")?;
    let labels: Vec<String> = Player::VARIANTS.iter().map(ToString::to_string).collect();
    match select_one("Select media player", &labels).expect("interactive selection failed") {
        Some(index) => Ok(Player::VARIANTS[index]),
        None => Err(Termination::Cancelled),
    }
}

/// Returns a [`Failure::NotInteractive`] when an interactive selection is
/// required but standard input is not a terminal.
fn require_terminal(what: &'static str) -> Result<(), Termination> {
    if io::stdin().is_terminal() {
        Ok(())
    } else {
        Err(Failure::NotInteractive(NotInteractive { what }).into())
    }
}

/// Joins the displayed forms of `items` with commas, for an error message
/// that lists the available choices.
fn join_display<Item: std::fmt::Display>(items: &[Item]) -> String {
    items
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}
