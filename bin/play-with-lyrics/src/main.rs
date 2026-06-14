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
mod resolve;

use crate::cli::Args;
use crate::failure::{Failure, NoSubtitles, NoVideos, Termination};
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
/// resolved interactively lets the user go back through them.
#[derive(Clone, Copy)]
enum Stage {
    Video,
    Language,
    Format,
    Player,
}

/// Returns the step to revisit when the user goes back, or aborts the command
/// when no earlier interactive page exists.
fn step_back(history: &mut Vec<Stage>) -> Result<Stage, Termination> {
    history.pop().ok_or(Termination::Cancelled)
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
    let mut video_index: Option<usize> = None;
    let mut available: Vec<(Language, SubtitleFormat)> = Vec::new();
    let mut language: Option<Language> = None;
    let mut format: Option<SubtitleFormat> = None;

    let player = loop {
        match stage {
            Stage::Video => match resolve_video(&args, &catalog, video_index)? {
                Resolution::Auto(chosen) => {
                    video_index = Some(chosen);
                    stage = Stage::Language;
                }
                Resolution::Chosen(chosen) => {
                    video_index = Some(chosen);
                    history.push(Stage::Video);
                    stage = Stage::Language;
                }
                Resolution::Back => stage = step_back(&mut history)?,
            },
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
                match resolve_language(&args, &available, language)? {
                    Resolution::Auto(chosen) => {
                        language = Some(chosen);
                        stage = Stage::Format;
                    }
                    Resolution::Chosen(chosen) => {
                        language = Some(chosen);
                        history.push(Stage::Language);
                        stage = Stage::Format;
                    }
                    Resolution::Back => stage = step_back(&mut history)?,
                }
            }
            Stage::Format => {
                let chosen_language = language.expect("the language is resolved before the format");
                let formats: Vec<SubtitleFormat> = available
                    .iter()
                    .filter(|(candidate, _)| *candidate == chosen_language)
                    .map(|(_, format)| *format)
                    .collect();
                match resolve_format(&args, chosen_language, &formats, format)? {
                    Resolution::Auto(chosen) => {
                        format = Some(chosen);
                        stage = Stage::Player;
                    }
                    Resolution::Chosen(chosen) => {
                        format = Some(chosen);
                        history.push(Stage::Format);
                        stage = Stage::Player;
                    }
                    Resolution::Back => stage = step_back(&mut history)?,
                }
            }
            Stage::Player => match resolve_player(&args)? {
                Resolution::Auto(chosen) | Resolution::Chosen(chosen) => break chosen,
                Resolution::Back => stage = step_back(&mut history)?,
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
