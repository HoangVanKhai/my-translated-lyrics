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
use std::process::{Command, exit};
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

fn main() {
    let args = Args::parse();

    let catalog = load(&args.source);
    if catalog.is_empty() {
        eprintln!(
            "error: No videos found in source directory {:?}.",
            args.source,
        );
        exit(1);
    }

    let video = resolve_video(&args, &catalog);
    let collection_dir = args.target.join(&*video.desc.collection);
    let video_title = video.desc.video_title.as_ref();

    let available = available_subtitles(&collection_dir, video_title);
    if available.is_empty() {
        eprintln!("error: No subtitles for {video_title:?} were found in {collection_dir:?}.");
        exit(1);
    }

    let language = resolve_language(&args, &available);
    let formats: Vec<SubtitleFormat> = available
        .iter()
        .filter(|(candidate, _)| *candidate == language)
        .map(|(_, format)| *format)
        .collect();
    let format = resolve_format(&args, &formats);
    let player = resolve_player(&args);

    let video_file = find_video_file(&collection_dir, video_title).unwrap_or_else(|error| {
        eprintln!("error: {error}");
        exit(1);
    });
    let subtitle_file = subtitle_path(&collection_dir, video_title, language, format);

    let mut command = player.command(&video_file, &subtitle_file);
    if args.dry_run {
        print_command(&command);
    } else {
        launch(&mut command, player);
    }
}

/// Resolves the video from `--title` or through the interactive table.
fn resolve_video<'a>(args: &Args, catalog: &'a [Video]) -> &'a Video {
    match &args.title {
        Some(query) => match resolve_unique(query, catalog, Video::search_keys_for) {
            Ok(video) => video,
            Err(error) => exit_unresolved("--title", query, error),
        },
        None => {
            require_terminal("a video title");
            match tui::select_video(catalog).expect("interactive selection failed") {
                Some(index) => &catalog[index],
                None => cancelled(),
            }
        }
    }
}

/// Resolves the subtitle language from `--language`, automatically when
/// only one is available, or through an interactive selector.
fn resolve_language(args: &Args, available: &[(Language, SubtitleFormat)]) -> Language {
    let mut languages: Vec<Language> = available.iter().map(|(language, _)| *language).collect();
    languages.dedup();

    if let Some(query) = &args.language {
        return match resolve_unique(query, &languages, |language| {
            language_search_keys(*language)
        }) {
            Ok(language) => *language,
            Err(error) => exit_unresolved("--language", query, error),
        };
    }
    if let [only] = languages.as_slice() {
        return *only;
    }
    require_terminal("a subtitle language");
    let labels: Vec<String> = languages
        .iter()
        .map(|language| format!("{} ({language})", language_label(*language)))
        .collect();
    match tui::select_one("Select subtitle language", &labels)
        .expect("interactive selection failed")
    {
        Some(index) => languages[index],
        None => cancelled(),
    }
}

/// Resolves the subtitle format from `--format`, automatically when only
/// one is available, or through an interactive selector.
fn resolve_format(args: &Args, formats: &[SubtitleFormat]) -> SubtitleFormat {
    if let Some(query) = &args.format {
        return match resolve_unique(query, formats, |format| format.search_keys()) {
            Ok(format) => *format,
            Err(error) => exit_unresolved("--format", query, error),
        };
    }
    if let [only] = formats {
        return *only;
    }
    require_terminal("a subtitle format");
    let labels: Vec<String> = formats
        .iter()
        .map(|format| format!("{} ({format})", format.full_name()))
        .collect();
    match tui::select_one("Select subtitle format", &labels).expect("interactive selection failed")
    {
        Some(index) => formats[index],
        None => cancelled(),
    }
}

/// Resolves the media player from `--player` or through an interactive
/// selector.
fn resolve_player(args: &Args) -> Player {
    if let Some(query) = &args.player {
        return match resolve_unique(query, Player::VARIANTS, |player| player.search_keys()) {
            Ok(player) => *player,
            Err(error) => exit_unresolved("--player", query, error),
        };
    }
    require_terminal("a media player");
    let labels: Vec<String> = Player::VARIANTS.iter().map(ToString::to_string).collect();
    match tui::select_one("Select media player", &labels).expect("interactive selection failed") {
        Some(index) => Player::VARIANTS[index],
        None => cancelled(),
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

/// Launches the player and propagates a non-zero exit status.
fn launch(command: &mut Command, player: Player) {
    let status = command
        .status()
        .unwrap_or_else(|error| panic!("error: Failed to launch {player}: {error}"));
    if !status.success() {
        exit(status.code().unwrap_or(1));
    }
}

/// Prints a resolution failure for a flag and exits.
fn exit_unresolved(flag: &str, query: &str, error: fuzzy::ResolveError) -> ! {
    eprintln!("error: {flag} {query:?}: {error}.");
    exit(1);
}

/// Exits when an interactive selection is required but standard input is
/// not a terminal.
fn require_terminal(what: &str) {
    if !io::stdin().is_terminal() {
        eprintln!(
            "error: {what} must be selected interactively, but stdin is not a terminal. Provide the corresponding flag instead.",
        );
        exit(1);
    }
}

/// Exits cleanly after the user cancels an interactive selection.
fn cancelled() -> ! {
    eprintln!("Cancelled.");
    exit(130);
}

impl Video {
    /// Free-function form of [`selection::Searchable::search_keys`] usable
    /// as a function pointer for [`resolve_unique`].
    fn search_keys_for(&self) -> Vec<&str> {
        crate::selection::Searchable::search_keys(self)
    }
}
