//! The ways the command can stop without success.
//!
//! [`Failure`] holds genuine errors, whose messages all begin with
//! `error: `. [`Termination`] is the wider set of non-success stops: a
//! failure, a user cancellation, or the player's own non-zero exit. `run`
//! returns a `Termination`.

use derive_more::Display;
use fuzzy_select::fuzzy::ResolveError;
use lyrics_core::video_descriptor::Language;
use play_with_lyrics::library::VideoLookupError;
use play_with_lyrics::player::SubtitleFormat;
use std::path::PathBuf;
use std::process::ExitCode;

/// A failure of the command. Every variant's message begins with `error: `
/// and maps to [`ExitCode::FAILURE`].
#[derive(Debug, Display)]
pub(crate) enum Failure {
    #[display("error: No videos found in source directory {_0:?}.")]
    NoVideos(PathBuf),

    #[display("{_0}")]
    NoSubtitles(NoSubtitles),

    #[display("{_0}")]
    UnresolvedTitle(UnresolvedTitle),

    #[display("{_0}")]
    LanguageUnavailable(LanguageUnavailable),

    #[display("{_0}")]
    FormatUnavailable(FormatUnavailable),

    #[display(
        "error: {_0} must be selected interactively, but stdin is not a terminal. Provide the corresponding flag instead."
    )]
    NotInteractive(&'static str),

    #[display("error: {_0}")]
    VideoLookup(VideoLookupError),
}

/// No subtitle files exist for the chosen video in the media library.
#[derive(Debug, Display)]
#[display("error: No subtitles for {video_title:?} were found in {collection_dir:?}.")]
pub(crate) struct NoSubtitles {
    pub(crate) video_title: String,
    pub(crate) collection_dir: PathBuf,
}

/// The `--title` value did not resolve to exactly one video.
#[derive(Debug, Display)]
#[display("error: --title {query:?}: {error}.")]
pub(crate) struct UnresolvedTitle {
    pub(crate) query: String,
    pub(crate) error: ResolveError,
}

/// The requested subtitle language is not available for the chosen video.
#[derive(Debug, Display)]
#[display("error: no {requested} subtitle is available for this video (available: {available}).")]
pub(crate) struct LanguageUnavailable {
    pub(crate) requested: Language,
    pub(crate) available: String,
}

/// The requested subtitle format is not available for the chosen language.
#[derive(Debug, Display)]
#[display("error: no {requested} subtitle is available in {language} (available: {available}).")]
pub(crate) struct FormatUnavailable {
    pub(crate) language: Language,
    pub(crate) requested: SubtitleFormat,
    pub(crate) available: String,
}

/// A non-success way the program can stop. A [`Failure`] is one of them; the
/// others are not errors and so do not print an `error: ` message.
#[derive(Debug, Display)]
pub(crate) enum Termination {
    /// The command failed.
    #[display("{_0}")]
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
