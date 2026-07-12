#![cfg_attr(dylint_lib = "perfectionist", feature(register_tool))]
#![cfg_attr(dylint_lib = "perfectionist", register_tool(perfectionist))]

//! Play a local video with one of its translated subtitles.
//!
//! The command reads a `source` directory of `video.toml` descriptors to
//! build a table of titles, lets the user pick a video, a subtitle
//! language, a subtitle format, and a media player, then launches the
//! player against the matching files in the `target` media library. Each
//! choice can be pre-selected with a command-line flag; any choice left
//! unset is made through an interactive selector.

mod cli;
mod failure;
mod host;
mod resolve;

use crate::cli::Args;
use crate::failure::{Failure, NoSubtitles, NoVideos, Termination};
use crate::host::Host;
use crate::resolve::{Resolution, resolve_format, resolve_language, resolve_player, resolve_video};
use clap::Parser;
use lyrics_core::video_descriptor::Language;
use play_with_lyrics::catalog::load;
use play_with_lyrics::library::{available_subtitles, find_video_file, subtitle_path};
use play_with_lyrics::player::{Player, SubtitleFormat};
use std::process::{Command, ExitCode};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(termination) => {
            eprintln!("{termination}");
            termination.exit_code()
        }
    }
}

/// One step in the resolution sequence. A history of the steps that were
/// resolved interactively lets the user go back through them. The order of the
/// variants is the order of the pages, which [`step_back`] compares against.
#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
enum Stage {
    Video,
    Language,
    Format,
    Player,
}

/// Returns the page to revisit when the user goes back, after erasing the
/// state of the pages ahead of it so a later forward pass chooses them afresh.
/// Aborts the command when no earlier interactive page exists.
fn step_back(
    history: &mut Vec<Stage>,
    language: &mut Option<Language>,
    format: &mut Option<SubtitleFormat>,
) -> Result<Stage, Termination> {
    let target = history.pop().ok_or(Termination::Cancelled)?;
    if target < Stage::Language {
        *language = None;
    }
    if target < Stage::Format {
        *format = None;
    }
    Ok(target)
}

/// Runs the command, reporting any non-success outcome as a [`Termination`].
///
/// The four choices are resolved in order. Each interactive page can return
/// to the previous interactive page, so the sequence is driven as a small
/// state machine rather than a straight line, with `history` recording the
/// pages the user can step back through.
fn run() -> Result<(), Termination> {
    let args = Args::parse();

    let catalog = load(&args.source);
    if catalog.is_empty() {
        return Err(Failure::NoVideos(NoVideos {
            source: args.source.clone(),
        })
        .into());
    }

    let mut stage = Stage::Video;
    let mut history: Vec<Stage> = Vec::new();
    let mut video_query = String::new();
    let mut video_index: Option<usize> = None;
    let mut available: Vec<(Language, SubtitleFormat)> = Vec::new();
    let mut language: Option<Language> = None;
    let mut format: Option<SubtitleFormat> = None;

    let player = loop {
        match stage {
            Stage::Video => {
                match resolve_video::<Host>(&args, &catalog, &mut video_query, video_index)? {
                    Resolution::Auto(chosen) => {
                        video_index = Some(chosen);
                        stage = Stage::Language;
                    }
                    Resolution::Chosen(chosen) => {
                        video_index = Some(chosen);
                        history.push(Stage::Video);
                        stage = Stage::Language;
                    }
                    Resolution::Back => {
                        stage = step_back(&mut history, &mut language, &mut format)?
                    }
                }
            }
            Stage::Language => {
                let chosen_video =
                    &catalog[video_index.expect("the video is resolved before the language")];
                let collection_dir = args.target.join(&*chosen_video.desc.collection);
                let video_title = chosen_video.desc.video_title.as_ref();
                available = available_subtitles(&collection_dir, video_title);
                if available.is_empty() {
                    return Err(Failure::NoSubtitles(NoSubtitles {
                        video_title: video_title.to_string(),
                        collection_dir,
                    })
                    .into());
                }
                match resolve_language::<Host>(&args, &available, language)? {
                    Resolution::Auto(chosen) => {
                        language = Some(chosen);
                        stage = Stage::Format;
                    }
                    Resolution::Chosen(chosen) => {
                        language = Some(chosen);
                        history.push(Stage::Language);
                        stage = Stage::Format;
                    }
                    Resolution::Back => {
                        stage = step_back(&mut history, &mut language, &mut format)?
                    }
                }
            }
            Stage::Format => {
                let chosen_language = language.expect("the language is resolved before the format");
                let formats: Vec<SubtitleFormat> = available
                    .iter()
                    .filter(|(candidate, _)| *candidate == chosen_language)
                    .map(|(_, format)| *format)
                    .collect();
                match resolve_format::<Host>(&args, chosen_language, &formats, format)? {
                    Resolution::Auto(chosen) => {
                        format = Some(chosen);
                        stage = Stage::Player;
                    }
                    Resolution::Chosen(chosen) => {
                        format = Some(chosen);
                        history.push(Stage::Format);
                        stage = Stage::Player;
                    }
                    Resolution::Back => {
                        stage = step_back(&mut history, &mut language, &mut format)?
                    }
                }
            }
            Stage::Player => match resolve_player::<Host>(&args)? {
                Resolution::Auto(chosen) | Resolution::Chosen(chosen) => break chosen,
                Resolution::Back => stage = step_back(&mut history, &mut language, &mut format)?,
            },
        }
    };

    let video = &catalog[video_index.expect("the video is resolved")];
    let language = language.expect("the language is resolved");
    let format = format.expect("the format is resolved");

    let collection_dir = args.target.join(&*video.desc.collection);
    let video_title = video.desc.video_title.as_ref();
    let video_file = find_video_file(&collection_dir, video_title).map_err(Failure::VideoLookup)?;
    let subtitle_file = subtitle_path(&collection_dir, video_title, language, format);

    launch(player.command(&video_file, &subtitle_file), player)
}

/// Launches the player, reporting a non-zero exit status as a
/// [`Termination::PlayerExited`].
fn launch(mut command: Command, player: Player) -> Result<(), Termination> {
    let status = command
        .status()
        .unwrap_or_else(|error| panic!("error: Failed to launch {player}: {error}"));
    status
        .success()
        .then_some(())
        .ok_or_else(|| status.code())
        .map_err(|code| code.unwrap_or(1))
        .map_err(u8::try_from)
        .map_err(|code| code.unwrap_or(1))
        .map_err(Termination::PlayerExited)
}

#[cfg(test)]
mod tests {
    use super::{Stage, step_back};
    use lyrics_core::video_descriptor::Language;
    use play_with_lyrics::player::SubtitleFormat;

    /// Backing to the language page clears the format chosen later, but keeps
    /// the language so it can be restored.
    #[test]
    fn stepping_back_to_the_language_page_erases_only_the_format() {
        let mut history = vec![Stage::Video, Stage::Language];
        let mut language = Some(Language::Vietnamese);
        let mut format = Some(SubtitleFormat::SubRip);
        let target = step_back(&mut history, &mut language, &mut format).unwrap();
        assert!(matches!(target, Stage::Language));
        assert_eq!(language, Some(Language::Vietnamese));
        assert_eq!(format, None);
    }

    /// Backing all the way to the song page clears both the language and the
    /// format, so the next forward pass chooses them afresh.
    #[test]
    fn stepping_back_to_the_song_page_erases_the_language_and_format() {
        let mut history = vec![Stage::Video];
        let mut language = Some(Language::Vietnamese);
        let mut format = Some(SubtitleFormat::SubRip);
        let target = step_back(&mut history, &mut language, &mut format).unwrap();
        assert!(matches!(target, Stage::Video));
        assert_eq!(language, None);
        assert_eq!(format, None);
    }

    /// Backing with no earlier interactive page cancels the command.
    #[test]
    fn stepping_back_with_no_history_cancels() {
        let mut history: Vec<Stage> = Vec::new();
        let mut language = Some(Language::Vietnamese);
        let mut format = Some(SubtitleFormat::SubRip);
        assert!(step_back(&mut history, &mut language, &mut format).is_err());
    }
}
