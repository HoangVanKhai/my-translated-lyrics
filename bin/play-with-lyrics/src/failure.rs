//! The ways the command can stop without success.
//!
//! [`Failure`] holds genuine errors, each carried by its own struct.
//! [`Termination`] is the wider set of non-success stops: a failure, a user
//! cancellation, or the player's own non-zero exit. `run` returns a
//! `Termination`, and the `error: ` prefix is added once, on
//! [`Termination::Failed`].

use derive_more::Display;
use fuzzy_select::fuzzy::ResolveError;
use lyrics_core::video_descriptor::Language;
use play_with_lyrics::library::VideoLookupError;
use play_with_lyrics::player::SubtitleFormat;
use std::path::PathBuf;
use std::process::ExitCode;

/// A genuine error. Each variant wraps a struct whose `Display` carries the
/// message; the `error: ` prefix is added by [`Termination::Failed`].
#[derive(Debug, Display)]
pub(crate) enum Failure {
    NoVideos(NoVideos),
    NoSubtitles(NoSubtitles),
    UnresolvedTitle(UnresolvedTitle),
    LanguageUnavailable(LanguageUnavailable),
    FormatUnavailable(FormatUnavailable),
    NotInteractive(NotInteractive),
    VideoLookup(VideoLookupError),
}

/// No videos were found in the source directory.
#[derive(Debug, Display)]
#[display("No videos found in source directory {source:?}.")]
pub(crate) struct NoVideos {
    pub(crate) source: PathBuf,
}

/// No subtitle files exist for the chosen video in the media library.
#[derive(Debug, Display)]
#[display("No subtitles for {video_title:?} were found in {collection_dir:?}.")]
pub(crate) struct NoSubtitles {
    pub(crate) video_title: String,
    pub(crate) collection_dir: PathBuf,
}

/// The `--title` value did not resolve to exactly one video.
#[derive(Debug, Display)]
#[display("--title {query:?}: {error}.")]
pub(crate) struct UnresolvedTitle {
    pub(crate) query: String,
    pub(crate) error: ResolveError,
}

/// The requested subtitle language is not available for the chosen video.
#[derive(Debug, Display)]
#[display("no {requested} subtitle is available for this video (available: {available}).")]
pub(crate) struct LanguageUnavailable {
    pub(crate) requested: Language,
    pub(crate) available: String,
}

/// The requested subtitle format is not available for the chosen language.
#[derive(Debug, Display)]
#[display("no {requested} subtitle is available in {language} (available: {available}).")]
pub(crate) struct FormatUnavailable {
    pub(crate) language: Language,
    pub(crate) requested: SubtitleFormat,
    pub(crate) available: String,
}

/// A choice had to be made interactively, but standard input is not a
/// terminal.
#[derive(Debug, Display)]
#[display(
    "{what} must be selected interactively, but stdin is not a terminal. Provide the corresponding flag instead."
)]
pub(crate) struct NotInteractive {
    pub(crate) what: &'static str,
}

/// A non-success way the program can stop. A [`Failure`] is one of them; the
/// others are not errors and so do not print an `error: ` message.
#[derive(Debug, Display)]
pub(crate) enum Termination {
    /// The command failed.
    #[display("error: {_0}")]
    Failed(Failure),

    /// The user cancelled an interactive selection.
    #[display("Cancelled.")]
    Cancelled,

    /// The media player ran but exited with a non-zero status. This is the
    /// player's own status, not a failure of this command.
    #[display("the media player exited with status {_0}.")]
    PlayerExited(u8),
}

impl From<Failure> for Termination {
    fn from(failure: Failure) -> Self {
        Termination::Failed(failure)
    }
}

impl Termination {
    /// The process exit code this termination maps to.
    pub(crate) fn exit_code(&self) -> ExitCode {
        match self {
            Termination::Failed(_) => ExitCode::FAILURE,
            Termination::Cancelled => ExitCode::from(exit_codes::CANCELLED),
            Termination::PlayerExited(code) => ExitCode::from(*code),
        }
    }
}

#[cfg(test)]
mod tests;
