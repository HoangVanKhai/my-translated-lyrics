//! Operations on the media library: the `target` directory that holds the
//! video files and the installed subtitles.
//!
//! Within the library, each collection is a subdirectory. A video file is
//! named `{video_title}.{ext}` and an installed subtitle is named
//! `{video_title}.{language}.{format}`, the same names
//! `install-local-lyrics` writes.

use crate::player::SubtitleFormat;
use derive_more::Display;
use into_deduped::IntoDeduped;
use into_sorted::IntoSorted;
use itertools::Itertools;
use lyrics_core::video_descriptor::Language;
use std::fs::read_dir;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

/// File extensions treated as playable video files. The lookup compares
/// extensions case-insensitively.
pub const VIDEO_EXTENSIONS: &[&str] = &[
    "mkv", "mp4", "webm", "avi", "mov", "m4v", "flv", "wmv", "ts",
];

/// The subtitle files available for `video_title` inside `collection_dir`,
/// as `(language, format)` pairs, sorted and deduplicated.
///
/// A collection directory that does not exist is treated as having no
/// subtitles rather than as an error, because the media library may simply
/// not contain that collection yet.
pub fn available_subtitles(
    collection_dir: &Path,
    video_title: &str,
) -> Vec<(Language, SubtitleFormat)> {
    let entries = match read_dir(collection_dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == ErrorKind::NotFound => return Vec::new(),
        Err(error) => panic!("error: Cannot read directory {collection_dir:?}: {error}"),
    };
    entries
        .map(|entry| {
            entry.unwrap_or_else(|error| {
                panic!("error: Cannot read an entry of directory {collection_dir:?}: {error}")
            })
        })
        .filter(|entry| {
            entry
                .file_type()
                .unwrap_or_else(|error| {
                    panic!(
                        "error: Cannot read file type of {:?}: {error}",
                        entry.path(),
                    )
                })
                .is_file()
        })
        .filter_map(|entry| {
            let file_name = entry.file_name();
            parse_subtitle_name(file_name.to_str()?, video_title)
        })
        .collect::<Vec<_>>()
        .into_sorted()
        .into_deduped()
}

/// Parses a subtitle file name of the form `{video_title}.{language}.{format}`
/// into its language and format, returning `None` when it does not match.
fn parse_subtitle_name(file_name: &str, video_title: &str) -> Option<(Language, SubtitleFormat)> {
    let rest = file_name.strip_prefix(video_title)?.strip_prefix('.')?;
    let (language, format) = rest.rsplit_once('.')?;
    Some((language.parse().ok()?, format.parse().ok()?))
}

/// The path an installed subtitle file would have in the library.
pub fn subtitle_path(
    collection_dir: &Path,
    video_title: &str,
    language: Language,
    format: SubtitleFormat,
) -> PathBuf {
    collection_dir.join(format!("{video_title}.{language}.{format}"))
}

/// Reason a video file could not be uniquely located in the library.
#[derive(Debug, Display)]
pub enum VideoLookupError {
    /// No file named `{video_title}.{ext}` with a known video extension
    /// exists in the collection directory.
    #[display("no video file for {video_title:?} was found in {collection_dir:?}")]
    NotFound {
        collection_dir: PathBuf,
        video_title: String,
    },
    /// More than one matching video file exists, so the choice would be
    /// ambiguous.
    #[display("multiple video files match {video_title:?} in {collection_dir:?}: {}", matches.iter().map(|path| format!("{path:?}")).join(", "))]
    Multiple {
        collection_dir: PathBuf,
        video_title: String,
        matches: Vec<PathBuf>,
    },
}

/// Finds the single playable video file for `video_title` inside
/// `collection_dir`.
///
/// A candidate is a regular file whose name is `{video_title}.{ext}` with
/// `ext` a known video extension. Directories are ignored, so a directory
/// that happens to be named like a video file cannot be matched. The
/// lookup is an error unless exactly one candidate exists.
pub fn find_video_file(
    collection_dir: &Path,
    video_title: &str,
) -> Result<PathBuf, VideoLookupError> {
    let entries = match read_dir(collection_dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == ErrorKind::NotFound => {
            return Err(VideoLookupError::NotFound {
                collection_dir: collection_dir.to_path_buf(),
                video_title: video_title.to_string(),
            });
        }
        Err(error) => panic!("error: Cannot read directory {collection_dir:?}: {error}"),
    };
    let mut matches: Vec<PathBuf> = entries
        .map(|entry| {
            entry.unwrap_or_else(|error| {
                panic!("error: Cannot read an entry of directory {collection_dir:?}: {error}")
            })
        })
        .filter(|entry| {
            entry
                .file_type()
                .unwrap_or_else(|error| {
                    panic!(
                        "error: Cannot read file type of {:?}: {error}",
                        entry.path(),
                    )
                })
                .is_file()
        })
        .filter(|entry| {
            entry
                .file_name()
                .to_str()
                .and_then(|file_name| is_video_file(file_name, video_title))
                .unwrap_or(false)
        })
        .map(|entry| entry.path())
        .collect();
    // `read_dir` yields entries in an unspecified order; sort so the reported
    // ambiguity lists the matching files the same way on every run.
    matches.sort();

    match matches.len() {
        0 => Err(VideoLookupError::NotFound {
            collection_dir: collection_dir.to_path_buf(),
            video_title: video_title.to_string(),
        }),
        1 => Ok(matches.remove(0)),
        _ => Err(VideoLookupError::Multiple {
            collection_dir: collection_dir.to_path_buf(),
            video_title: video_title.to_string(),
            matches,
        }),
    }
}

/// Returns `Some(true)` when `file_name` is `{video_title}.{ext}` with a
/// known video extension, `Some(false)` when the stem matches but the
/// extension does not, and `None` when the stem does not match at all.
fn is_video_file(file_name: &str, video_title: &str) -> Option<bool> {
    let extension = file_name.strip_prefix(video_title)?.strip_prefix('.')?;
    let is_video = VIDEO_EXTENSIONS
        .iter()
        .any(|known| known.eq_ignore_ascii_case(extension));
    Some(is_video)
}

#[cfg(test)]
mod tests;
