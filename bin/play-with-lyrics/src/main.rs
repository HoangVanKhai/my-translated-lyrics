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

use clap::{Parser, ValueEnum};
use derive_more::Display;
use fuzzy_select::fuzzy::{ResolveError, resolve_unique};
use fuzzy_select::selection::Searchable;
use lyrics_core::video_descriptor::Language;
use play_with_lyrics::catalog::{Video, language_label, load};
use play_with_lyrics::library::{
    VideoLookupError, available_subtitles, find_video_file, subtitle_path,
};
use play_with_lyrics::player::{Player, SubtitleFormat};
use play_with_lyrics_tui::{select_one, select_video};
use std::io::{self, IsTerminal};
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

    /// Pre-select the subtitle language. Must be available for the video.
    #[clap(long, short = 'l')]
    language: Option<LanguageArg>,

    /// Pre-select the subtitle format. Must be available for the language.
    #[clap(long, short = 'f')]
    format: Option<FormatArg>,

    /// Pre-select the media player.
    #[clap(long, short = 'p')]
    player: Option<PlayerArg>,
}

/// The media player chosen on the command line.
#[derive(Debug, Clone, Copy, ValueEnum)]
enum PlayerArg {
    Mpv,
    Celluloid,
}

impl From<PlayerArg> for Player {
    fn from(arg: PlayerArg) -> Self {
        match arg {
            PlayerArg::Mpv => Player::Mpv,
            PlayerArg::Celluloid => Player::Celluloid,
        }
    }
}

/// The subtitle language chosen on the command line. The variants mirror
/// [`Language`]. On the command line the two-letter codes are the canonical
/// values, with the three-letter codes and the full names as aliases.
#[derive(Debug, Clone, Copy, ValueEnum)]
enum LanguageArg {
    #[value(name = "en", aliases = ["eng", "english"])]
    English,
    #[value(name = "vi", aliases = ["vie", "vietnamese"])]
    Vietnamese,
    #[value(name = "zh", aliases = ["zho", "chinese"])]
    Chinese,
}

impl From<LanguageArg> for Language {
    fn from(arg: LanguageArg) -> Self {
        match arg {
            LanguageArg::English => Language::English,
            LanguageArg::Vietnamese => Language::Vietnamese,
            LanguageArg::Chinese => Language::Chinese,
        }
    }
}

/// The subtitle format chosen on the command line. The variants mirror
/// [`SubtitleFormat`]. On the command line the file extensions are the
/// canonical values, with the full format names as aliases.
#[derive(Debug, Clone, Copy, ValueEnum)]
enum FormatArg {
    #[value(name = "srt", alias = "sub-rip")]
    SubRip,
    #[value(name = "vtt", alias = "web-vtt")]
    WebVtt,
}

impl From<FormatArg> for SubtitleFormat {
    fn from(arg: FormatArg) -> Self {
        match arg {
            FormatArg::SubRip => SubtitleFormat::SubRip,
            FormatArg::WebVtt => SubtitleFormat::WebVtt,
        }
    }
}

/// A non-success outcome of [`run`]. Each variant carries the data needed
/// to print a message through its [`Display`] implementation and to choose
/// a process exit code through [`Failure::exit_code`].
#[derive(Debug, Display)]
enum Failure {
    #[display("error: No videos found in source directory {_0:?}.")]
    NoVideos(PathBuf),

    #[display("error: No subtitles for {video_title:?} were found in {collection_dir:?}.")]
    NoSubtitles {
        video_title: String,
        collection_dir: PathBuf,
    },

    #[display("error: --title {query:?}: {error}.")]
    UnresolvedTitle { query: String, error: ResolveError },

    #[display(
        "error: no {requested} subtitle is available for this video (available: {available})."
    )]
    LanguageUnavailable {
        requested: Language,
        available: String,
    },

    #[display(
        "error: no {requested} subtitle is available in {language} (available: {available})."
    )]
    FormatUnavailable {
        language: Language,
        requested: SubtitleFormat,
        available: String,
    },

    #[display(
        "error: {_0} must be selected interactively, but stdin is not a terminal. Provide the corresponding flag instead."
    )]
    NotInteractive(&'static str),

    #[display("error: {_0}")]
    VideoLookup(VideoLookupError),

    #[display("Cancelled.")]
    Cancelled,

    #[display("the media player exited with status {_0}.")]
    PlayerExited(u8),
}

impl Failure {
    /// The process exit code this failure maps to.
    fn exit_code(&self) -> ExitCode {
        match self {
            // 130 is the conventional code for an action cancelled at the
            // terminal (128 + SIGINT).
            Failure::Cancelled => ExitCode::from(130),
            Failure::PlayerExited(code) => ExitCode::from(*code),
            _ => ExitCode::FAILURE,
        }
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(failure) => {
            eprintln!("{failure}");
            failure.exit_code()
        }
    }
}

/// Runs the command, reporting any non-success outcome as a [`Failure`].
fn run() -> Result<(), Failure> {
    let args = Args::parse();

    let catalog = load(&args.source);
    if catalog.is_empty() {
        return Err(Failure::NoVideos(args.source.clone()));
    }

    let video = resolve_video(&args, &catalog)?;
    let collection_dir = args.target.join(&*video.desc.collection);
    let video_title = video.desc.video_title.as_ref();

    let available = available_subtitles(&collection_dir, video_title);
    if available.is_empty() {
        return Err(Failure::NoSubtitles {
            video_title: video_title.to_string(),
            collection_dir,
        });
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

    let mut command = player.command(&video_file, &subtitle_file);
    launch(&mut command, player)
}

/// Resolves the video from `--title` or through the interactive table.
fn resolve_video<'a>(args: &Args, catalog: &'a [Video]) -> Result<&'a Video, Failure> {
    match &args.title {
        Some(query) => {
            resolve_unique(query, catalog, <Video as Searchable>::search_keys).map_err(|error| {
                Failure::UnresolvedTitle {
                    query: query.clone(),
                    error,
                }
            })
        }
        None => {
            require_terminal("a video title")?;
            match select_video(catalog).expect("interactive selection failed") {
                Some(index) => Ok(&catalog[index]),
                None => Err(Failure::Cancelled),
            }
        }
    }
}

/// Resolves the subtitle language from `--language`, automatically when
/// only one is available, or through an interactive selector.
fn resolve_language(
    args: &Args,
    available: &[(Language, SubtitleFormat)],
) -> Result<Language, Failure> {
    let mut languages: Vec<Language> = available.iter().map(|(language, _)| *language).collect();
    languages.dedup();

    if let Some(arg) = args.language {
        let requested = Language::from(arg);
        return if languages.contains(&requested) {
            Ok(requested)
        } else {
            Err(Failure::LanguageUnavailable {
                requested,
                available: join_display(&languages),
            })
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
        None => Err(Failure::Cancelled),
    }
}

/// Resolves the subtitle format from `--format`, automatically when only
/// one is available, or through an interactive selector.
fn resolve_format(
    args: &Args,
    language: Language,
    formats: &[SubtitleFormat],
) -> Result<SubtitleFormat, Failure> {
    if let Some(arg) = args.format {
        let requested = SubtitleFormat::from(arg);
        return if formats.contains(&requested) {
            Ok(requested)
        } else {
            Err(Failure::FormatUnavailable {
                language,
                requested,
                available: join_display(formats),
            })
        };
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
        None => Err(Failure::Cancelled),
    }
}

/// Resolves the media player from `--player` or through an interactive
/// selector.
fn resolve_player(args: &Args) -> Result<Player, Failure> {
    if let Some(arg) = args.player {
        return Ok(Player::from(arg));
    }
    require_terminal("a media player")?;
    let labels: Vec<String> = Player::VARIANTS.iter().map(ToString::to_string).collect();
    match select_one("Select media player", &labels).expect("interactive selection failed") {
        Some(index) => Ok(Player::VARIANTS[index]),
        None => Err(Failure::Cancelled),
    }
}

/// Launches the player, reporting a non-zero exit status as a [`Failure`].
fn launch(command: &mut Command, player: Player) -> Result<(), Failure> {
    let status = command
        .status()
        .unwrap_or_else(|error| panic!("error: Failed to launch {player}: {error}"));
    if status.success() {
        Ok(())
    } else {
        let code = u8::try_from(status.code().unwrap_or(1)).unwrap_or(1);
        Err(Failure::PlayerExited(code))
    }
}

/// Returns a [`Failure`] when an interactive selection is required but
/// standard input is not a terminal.
fn require_terminal(what: &'static str) -> Result<(), Failure> {
    if io::stdin().is_terminal() {
        Ok(())
    } else {
        Err(Failure::NotInteractive(what))
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
