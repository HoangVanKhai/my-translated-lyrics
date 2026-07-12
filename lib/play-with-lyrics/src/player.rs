//! The media players and subtitle formats this command understands, plus
//! the construction of the player invocation.

use command_extra::CommandExtra;
use pipe_trait::Pipe;
use std::ffi::OsString;
use std::path::Path;
use std::process::Command;
use strum::{AsRefStr, Display, EnumString, VariantArray};

/// A media player that can load an external subtitle file.
#[derive(AsRefStr, Clone, Copy, Debug, Display, EnumString, Eq, PartialEq, VariantArray)]
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
        // Build the flag as an `OsString` so the subtitle path passes through
        // verbatim, without requiring it to be valid UTF-8.
        let mut subtitle_flag = OsString::from(match self {
            Player::Mpv => "--sub-file=",
            Player::Celluloid => "--mpv-sub-file=",
        });
        subtitle_flag.push(subtitle);
        self.pipe_as_ref(Command::new)
            .with_arg(subtitle_flag)
            .with_arg(video)
    }
}

/// A subtitle file format, also referred to as the subtitle "type".
#[derive(
    AsRefStr,
    Clone,
    Copy,
    Debug,
    Display,
    EnumString,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd,
    VariantArray,
)]
pub enum SubtitleFormat {
    #[strum(serialize = "srt")]
    SubRip,
    #[strum(serialize = "vtt")]
    WebVtt,
}

impl SubtitleFormat {
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
