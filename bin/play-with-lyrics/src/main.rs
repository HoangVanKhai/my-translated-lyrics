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

mod catalog;
mod fuzzy;
mod library;
mod player;
mod selection;
mod tui;

use crate::catalog::{Video, language_label, language_search_keys, load};
use crate::fuzzy::resolve_unique;
use crate::library::{available_subtitles, find_video_file, subtitle_path};
use crate::player::{Player, SubtitleFormat};
use clap::Parser;
use lyrics_core::video_descriptor::Language;
use std::io::{self, IsTerminal, Write};
use std::path::PathBuf;
use std::process::{Command, ExitCode};
use strum::VariantArray;

#[derive(Debug, Parser)]
#[clap(about = "Play a local video with its translated subtitle")]
struct Args {
    /// Source directory of per-video `video.toml` descriptors, in the same
    /// layout `install-local-lyrics` reads.
    source: PathBuf,

    /// Media library directory holding the video files and the installed
    /// subtitles, the same directory `install-local-lyrics` writes to.
    target: PathBuf,

    /// Pre-select the video by fuzzily matching its English, Vietnamese,
    /// Chinese, or raw video title. Must match exactly one video.
    #[clap(long, short = 't')]
    title: Option<String>,

    /// Pre-select the subtitle language. Must match exactly one of the
    /// languages available for the chosen video.
    #[clap(long, short = 'l')]
    language: Option<String>,

    /// Pre-select the subtitle format. Must match exactly one of the
    /// formats available for the chosen language.
    #[clap(long, short = 'f')]
    format: Option<String>,

    /// Pre-select the media player (mpv or celluloid).
    #[clap(long, short = 'p')]
    player: Option<String>,

    /// Print the resolved command instead of launching the player.
    #[clap(long, short = 'n')]
    dry_run: bool,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(code) => code,
    }
}

/// Runs the command, returning the process exit code through the `Err`
/// variant so that no path calls [`std::process::exit`] directly.
fn run() -> Result<(), ExitCode> {
    let args = Args::parse();

    let catalog = load(&args.source);
    if catalog.is_empty() {
        eprintln!(
            "error: No videos found in source directory {:?}.",
            args.source,
        );
        return Err(ExitCode::FAILURE);
    }

    let video = resolve_video(&args, &catalog)?;
    let collection_dir = args.target.join(&*video.desc.collection);
    let video_title = video.desc.video_title.as_ref();

    let available = available_subtitles(&collection_dir, video_title);
    if available.is_empty() {
        eprintln!("error: No subtitles for {video_title:?} were found in {collection_dir:?}.");
        return Err(ExitCode::FAILURE);
    }

    let language = resolve_language(&args, &available)?;
    let formats: Vec<SubtitleFormat> = available
        .iter()
        .filter(|(candidate, _)| *candidate == language)
        .map(|(_, format)| *format)
        .collect();
    let format = resolve_format(&args, &formats)?;
    let player = resolve_player(&args)?;

    let video_file = match find_video_file(&collection_dir, video_title) {
        Ok(path) => path,
        Err(error) => {
            eprintln!("error: {error}");
            return Err(ExitCode::FAILURE);
        }
    };
    let subtitle_file = subtitle_path(&collection_dir, video_title, language, format);

    let mut command = player.command(&video_file, &subtitle_file);
    if args.dry_run {
        print_command(&command);
        return Ok(());
    }
    launch(&mut command, player)
}

/// Resolves the video from `--title` or through the interactive table.
fn resolve_video<'a>(args: &Args, catalog: &'a [Video]) -> Result<&'a Video, ExitCode> {
    match &args.title {
        Some(query) => resolve_unique(query, catalog, Video::search_keys_for)
            .map_err(|error| unresolved("--title", query, error)),
        None => {
            require_terminal("a video title")?;
            match tui::select_video(catalog).expect("interactive selection failed") {
                Some(index) => Ok(&catalog[index]),
                None => Err(cancelled()),
            }
        }
    }
}

/// Resolves the subtitle language from `--language`, automatically when
/// only one is available, or through an interactive selector.
fn resolve_language(
    args: &Args,
    available: &[(Language, SubtitleFormat)],
) -> Result<Language, ExitCode> {
    let mut languages: Vec<Language> = available.iter().map(|(language, _)| *language).collect();
    languages.dedup();

    if let Some(query) = &args.language {
        return resolve_unique(query, &languages, |language| {
            language_search_keys(*language)
        })
        .copied()
        .map_err(|error| unresolved("--language", query, error));
    }
    if let [only] = languages.as_slice() {
        return Ok(*only);
    }
    require_terminal("a subtitle language")?;
    let labels: Vec<String> = languages
        .iter()
        .map(|language| format!("{} ({language})", language_label(*language)))
        .collect();
    match tui::select_one("Select subtitle language", &labels)
        .expect("interactive selection failed")
    {
        Some(index) => Ok(languages[index]),
        None => Err(cancelled()),
    }
}

/// Resolves the subtitle format from `--format`, automatically when only
/// one is available, or through an interactive selector.
fn resolve_format(args: &Args, formats: &[SubtitleFormat]) -> Result<SubtitleFormat, ExitCode> {
    if let Some(query) = &args.format {
        return resolve_unique(query, formats, |format| format.search_keys())
            .copied()
            .map_err(|error| unresolved("--format", query, error));
    }
    if let [only] = formats {
        return Ok(*only);
    }
    require_terminal("a subtitle format")?;
    let labels: Vec<String> = formats
        .iter()
        .map(|format| format!("{} ({format})", format.full_name()))
        .collect();
    match tui::select_one("Select subtitle format", &labels).expect("interactive selection failed")
    {
        Some(index) => Ok(formats[index]),
        None => Err(cancelled()),
    }
}

/// Resolves the media player from `--player` or through an interactive
/// selector.
fn resolve_player(args: &Args) -> Result<Player, ExitCode> {
    if let Some(query) = &args.player {
        return resolve_unique(query, Player::VARIANTS, |player| player.search_keys())
            .copied()
            .map_err(|error| unresolved("--player", query, error));
    }
    require_terminal("a media player")?;
    let labels: Vec<String> = Player::VARIANTS.iter().map(ToString::to_string).collect();
    match tui::select_one("Select media player", &labels).expect("interactive selection failed") {
        Some(index) => Ok(Player::VARIANTS[index]),
        None => Err(cancelled()),
    }
}

/// Prints the resolved command, one token per line, so paths that contain
/// spaces stay unambiguous.
fn print_command(command: &Command) {
    let mut stdout = io::stdout().lock();
    writeln!(stdout, "{}", command.get_program().to_string_lossy())
        .expect("failed to write to stdout");
    for argument in command.get_args() {
        writeln!(stdout, "{}", argument.to_string_lossy()).expect("failed to write to stdout");
    }
}

/// Launches the player, returning its exit status as an [`ExitCode`].
fn launch(command: &mut Command, player: Player) -> Result<(), ExitCode> {
    let status = command
        .status()
        .unwrap_or_else(|error| panic!("error: Failed to launch {player}: {error}"));
    if status.success() {
        Ok(())
    } else {
        let code = u8::try_from(status.code().unwrap_or(1)).unwrap_or(1);
        Err(ExitCode::from(code))
    }
}

/// Prints a resolution failure for a flag and returns the failure code.
fn unresolved(flag: &str, query: &str, error: fuzzy::ResolveError) -> ExitCode {
    eprintln!("error: {flag} {query:?}: {error}.");
    ExitCode::FAILURE
}

/// Returns the failure code when an interactive selection is required but
/// standard input is not a terminal.
fn require_terminal(what: &str) -> Result<(), ExitCode> {
    if io::stdin().is_terminal() {
        return Ok(());
    }
    eprintln!(
        "error: {what} must be selected interactively, but stdin is not a terminal. Provide the corresponding flag instead.",
    );
    Err(ExitCode::FAILURE)
}

/// Reports a cancelled interactive selection and returns its exit code.
fn cancelled() -> ExitCode {
    eprintln!("Cancelled.");
    ExitCode::from(130)
}

impl Video {
    /// Free-function form of [`selection::Searchable::search_keys`] usable
    /// as a function pointer for [`resolve_unique`].
    fn search_keys_for(&self) -> Vec<&str> {
        crate::selection::Searchable::search_keys(self)
    }
}
