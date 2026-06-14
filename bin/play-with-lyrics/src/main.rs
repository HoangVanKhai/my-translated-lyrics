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
use crate::resolve::{resolve_format, resolve_language, resolve_player, resolve_video};
use clap::Parser;
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

/// Runs the command, reporting any non-success outcome as a [`Termination`].
fn run() -> Result<(), Termination> {
    let args = Args::parse();

    let catalog = load(&args.source);
    if catalog.is_empty() {
        return Err(Failure::NoVideos(NoVideos {
            source: args.source.clone(),
        })
        .into());
    }

    let video = resolve_video(&args, &catalog)?;
    let collection_dir = args.target.join(&*video.desc.collection);
    let video_title = video.desc.video_title.as_ref();

    let available = available_subtitles(&collection_dir, video_title);
    if available.is_empty() {
        return Err(Failure::NoSubtitles(NoSubtitles {
            video_title: video_title.to_string(),
            collection_dir,
        })
        .into());
    }

    let language = resolve_language(&args, &available)?;
    let formats: Vec<SubtitleFormat> = available
        .iter()
        .filter(|(candidate, _)| *candidate == language)
        .map(|(_, format)| *format)
        .collect();
    let format = resolve_format(&args, language, &formats)?;
    let player = resolve_player(&args)?;

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
