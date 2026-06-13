//! The media players and subtitle formats this command understands, plus
//! the construction of the player invocation.

use command_extra::CommandExtra;
use std::path::Path;
use std::process::Command;
use strum::{AsRefStr, Display, EnumString, VariantArray};

/// A media player that can load an external subtitle file.
#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, AsRefStr, EnumString, VariantArray)]
pub enum Player {
    #[strum(serialize = "mpv")]
    Mpv,
    #[strum(serialize = "celluloid")]
    Celluloid,
}

impl Player {
    /// Builds the command that plays `video` with `subtitle` loaded as an
    /// external subtitle track.
    ///
    /// mpv accepts the subtitle through `--sub-file=`. Celluloid forwards
    /// any `--mpv-...` option to its embedded mpv instance, so the same
    /// subtitle is passed through `--mpv-sub-file=`.
    pub fn command(self, video: &Path, subtitle: &Path) -> Command {
        let mut subtitle_flag = match self {
            Player::Mpv => "--sub-file=".to_string(),
            Player::Celluloid => "--mpv-sub-file=".to_string(),
        };
        subtitle_flag.push_str(&subtitle.to_string_lossy());
        Command::new(self.as_ref())
            .with_arg(subtitle_flag)
            .with_arg(video)
    }

    /// The strings a command-line `--player` value is fuzzily matched
    /// against.
    pub fn search_keys(&self) -> Vec<&'static str> {
        match self {
            Player::Mpv => vec!["mpv"],
            Player::Celluloid => vec!["celluloid"],
        }
    }
}

/// A subtitle file format, also referred to as the subtitle "type".
#[derive(
    Debug,
    Display,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    AsRefStr,
    EnumString,
    VariantArray,
)]
pub enum SubtitleFormat {
    #[strum(serialize = "srt")]
    SubRip,
    #[strum(serialize = "vtt")]
    WebVtt,
}

impl SubtitleFormat {
    /// The strings a command-line `--format` value is fuzzily matched
    /// against: both the file extension and the full format name.
    pub fn search_keys(&self) -> Vec<&'static str> {
        match self {
            SubtitleFormat::SubRip => vec!["srt", "subrip"],
            SubtitleFormat::WebVtt => vec!["vtt", "webvtt"],
        }
    }

    /// The full, human-readable name of the format, shown in the format
    /// selector.
    pub fn full_name(&self) -> &'static str {
        match self {
            SubtitleFormat::SubRip => "SubRip",
            SubtitleFormat::WebVtt => "WebVTT",
        }
    }
}

#[cfg(test)]
mod tests;
