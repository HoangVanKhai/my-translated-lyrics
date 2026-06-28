//! Command-line argument parsing.
//!
//! The flag value enums are defined here as binary-local types so the
//! library crates do not depend on clap. Each one converts to the
//! corresponding domain type.

use clap::{Parser, ValueEnum};
use lyrics_core::video_descriptor::Language;
use play_with_lyrics::player::{Player, SubtitleFormat};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[clap(about = "Play a local video with its translated subtitle")]
pub(crate) struct Args {
    /// Source directory of per-video video.toml descriptors, in the same
    /// layout install-local-lyrics reads.
    pub(crate) source: PathBuf,

    /// Media library directory holding the video files and the installed
    /// subtitles, the same directory install-local-lyrics writes to.
    pub(crate) target: PathBuf,

    /// Pre-select the video by fuzzily matching its English, Vietnamese,
    /// Chinese, or raw video title. Must match exactly one video.
    #[clap(long, short = 't')]
    pub(crate) title: Option<String>,

    /// Pre-select the subtitle language. Must be available for the video.
    #[clap(long, short = 'l')]
    pub(crate) language: Option<LanguageArg>,

    /// Pre-select the subtitle format. Must be available for the language.
    #[clap(long, short = 'f')]
    pub(crate) format: Option<FormatArg>,

    /// Pre-select the media player.
    #[clap(long, short = 'p')]
    pub(crate) player: Option<PlayerArg>,
}

/// The media player chosen on the command line.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub(crate) enum PlayerArg {
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
pub(crate) enum LanguageArg {
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
pub(crate) enum FormatArg {
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
