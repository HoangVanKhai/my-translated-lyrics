//! Driver for the `generate-subtitles` binary.
//!
//! The driver walks the source directory, loads each song's descriptor
//! files and per-language lyrics, renders them, and writes the output
//! into the destination directory. A dry-run mode prints the planned
//! writes and leaves the filesystem untouched; the equivalent of
//! `install-local-lyrics`'s `--execute` flag opts into the actual write.

use super::parse::{ParseLyricsError, SubtitleCue, parse_lyrics};
use super::render_srt::{RenderSrtError, render_srt};
use super::render_vtt::{RenderVttError, render_vtt};
use crate::credits_descriptor::{CREDITS_CONFIG_FILE_NAME, CreditsDesc};
use crate::line_markers_descriptor::{LINE_MARKERS_CONFIG_FILE_NAME, LineMarkersDesc};
use crate::video_descriptor::{Language, VIDEO_CONFIG_FILE_NAME, VideoDesc};
use clap::Parser;
use derive_more::{Display, Error};
use itertools::Itertools;
use std::collections::BTreeMap;
use std::fs::{DirEntry, create_dir_all, read_dir, read_to_string, write as write_file};
use std::io;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const LYRICS_TXT_SUFFIX: &str = ".txt";

#[derive(Debug, Clone, Parser)]
#[clap(about = "Build `.srt` and `.vtt` subtitle files from the structured lyrics sources.")]
struct Args {
    /// Source directory that contains one song subdirectory per video
    /// (typically the repository's `sources/` directory).
    sources: PathBuf,

    /// Destination directory into which subtitle files are written
    /// (typically the repository's `dist/` directory).
    dist: PathBuf,

    /// Print the planned writes without touching the filesystem.
    /// Mirrors the safety posture of `install-local-lyrics`, which
    /// defaults to a dry run until `--execute` is given.
    #[clap(long, short = 'x')]
    execute: bool,
}

/// Everything the renderers need for one song's one language.
pub struct LanguageBundle {
    pub language: Language,
    pub cues: Vec<SubtitleCue>,
}

/// Parsed representation of a song directory, ready for rendering.
pub struct Song {
    pub directory_name: String,
    pub markers: LineMarkersDesc,
    pub credits: CreditsDesc,
    pub languages: Vec<LanguageBundle>,
}

/// Builds the subtitles for a single song by rendering each language
/// to both `.srt` and `.vtt` and writing the result into `dist_dir`.
/// Returns the count of files that were (or, in dry-run mode, would
/// have been) written.
pub fn render_song_to_disk(
    song: &Song,
    dist_dir: &Path,
    execute: bool,
) -> Result<usize, GenerateError> {
    let destination_dir = dist_dir.join(&song.directory_name);
    if execute {
        create_dir_all(&destination_dir).map_err(|cause| {
            GenerateError::CreateDir(GenerateErrorCreateDirPayload {
                path: destination_dir.clone(),
                cause,
            })
        })?;
    }

    let mut written: usize = 0;
    for bundle in &song.languages {
        let vtt = render_vtt(&bundle.cues, &song.markers, &song.credits, &bundle.language)
            .map_err(|cause| {
                GenerateError::RenderVtt(GenerateErrorRenderVttPayload {
                    song: song.directory_name.clone(),
                    language: bundle.language.clone(),
                    cause,
                })
            })?;
        let vtt_path = destination_dir.join(format!("lyrics.{}.vtt", bundle.language));
        write_subtitle(&vtt_path, &vtt, execute)?;
        written += 1;

        let srt = render_srt(&bundle.cues, &song.markers, &song.credits, &bundle.language)
            .map_err(|cause| {
                GenerateError::RenderSrt(GenerateErrorRenderSrtPayload {
                    song: song.directory_name.clone(),
                    language: bundle.language.clone(),
                    cause,
                })
            })?;
        let srt_path = destination_dir.join(format!("lyrics.{}.srt", bundle.language));
        write_subtitle(&srt_path, &srt, execute)?;
        written += 1;
    }
    Ok(written)
}

fn write_subtitle(path: &Path, content: &str, execute: bool) -> Result<(), GenerateError> {
    eprintln!("write {path:?}");
    if !execute {
        return Ok(());
    }
    write_file(path, content).map_err(|cause| {
        GenerateError::WriteFile(GenerateErrorWriteFilePayload {
            path: path.to_path_buf(),
            cause,
        })
    })
}

/// Loads all source artifacts for a single song into memory and parses
/// each cue list.
pub fn load_song(song_dir: &Path) -> Result<Song, GenerateError> {
    let directory_name = song_dir
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_else(|| panic!("song directory {song_dir:?} has a non-UTF-8 name"))
        .to_string();

    // `video.toml` is parsed purely to validate that the file exists
    // and is well-formed. None of its fields flow into the rendered
    // output today; the parse catches corrupted descriptors at load
    // time instead of deferring the failure to a downstream consumer
    // that does not exist yet.
    let video_path = song_dir.join(VIDEO_CONFIG_FILE_NAME);
    let video_content = read_to_string(&video_path).map_err(|cause| {
        GenerateError::ReadFile(GenerateErrorReadFilePayload {
            path: video_path.clone(),
            cause,
        })
    })?;
    let _: VideoDesc = toml::from_str(&video_content).map_err(|cause| {
        GenerateError::ParseVideoDesc(GenerateErrorParseVideoDescPayload {
            path: video_path.clone(),
            cause,
        })
    })?;

    let markers_path = song_dir.join(LINE_MARKERS_CONFIG_FILE_NAME);
    let markers: LineMarkersDesc = if markers_path.exists() {
        let markers_content = read_to_string(&markers_path).map_err(|cause| {
            GenerateError::ReadFile(GenerateErrorReadFilePayload {
                path: markers_path.clone(),
                cause,
            })
        })?;
        toml::from_str(&markers_content).map_err(|cause| {
            GenerateError::ParseLineMarkers(GenerateErrorParseLineMarkersPayload {
                path: markers_path.clone(),
                cause,
            })
        })?
    } else {
        LineMarkersDesc::default()
    };

    let credits_path = song_dir.join(CREDITS_CONFIG_FILE_NAME);
    let credits: CreditsDesc = if credits_path.exists() {
        let credits_content = read_to_string(&credits_path).map_err(|cause| {
            GenerateError::ReadFile(GenerateErrorReadFilePayload {
                path: credits_path.clone(),
                cause,
            })
        })?;
        serde_saphyr::from_str(&credits_content).map_err(|cause| {
            GenerateError::ParseCredits(GenerateErrorParseCreditsPayload {
                path: credits_path.clone(),
                cause: cause.to_string(),
            })
        })?
    } else {
        CreditsDesc::default()
    };

    let mut languages: BTreeMap<Language, LanguageBundle> = BTreeMap::new();
    let entries = read_dir(song_dir).map_err(|cause| {
        GenerateError::ReadDir(GenerateErrorReadDirPayload {
            path: song_dir.to_path_buf(),
            cause,
        })
    })?;
    for entry in entries {
        let entry = entry.map_err(|cause| {
            GenerateError::ReadDir(GenerateErrorReadDirPayload {
                path: song_dir.to_path_buf(),
                cause,
            })
        })?;
        let file_name = entry.file_name();
        let Some(file_name) = file_name.to_str() else {
            panic!(
                "lyrics directory entry {:?} has a non-UTF-8 filename",
                entry.path(),
            );
        };
        let middle = file_name
            .strip_prefix("lyrics.")
            .and_then(|rest| rest.strip_suffix(LYRICS_TXT_SUFFIX));
        let Some(middle) = middle else {
            continue;
        };
        let lyrics_path = entry.path();
        let language = middle.parse::<Language>().map_err(|_| {
            GenerateError::UnrecognizedLanguage(GenerateErrorUnrecognizedLanguagePayload {
                path: lyrics_path.clone(),
                code: middle.to_string(),
            })
        })?;
        let content = read_to_string(&lyrics_path).map_err(|cause| {
            GenerateError::ReadFile(GenerateErrorReadFilePayload {
                path: lyrics_path.clone(),
                cause,
            })
        })?;
        let cues = parse_lyrics(&content).map_err(|cause| {
            GenerateError::ParseLyrics(GenerateErrorParseLyricsPayload {
                path: lyrics_path.clone(),
                cause,
            })
        })?;
        languages.insert(language.clone(), LanguageBundle { language, cues });
    }

    Ok(Song {
        directory_name,
        markers,
        credits,
        languages: languages.into_values().collect(),
    })
}

pub fn main() -> ExitCode {
    let args = Args::parse();

    let entries = match read_dir(&args.sources) {
        Ok(iter) => iter,
        Err(error) => {
            eprintln!(
                "error: Cannot read sources directory {path:?}: {error}",
                path = args.sources,
            );
            return ExitCode::FAILURE;
        }
    };
    let song_dirs = entries
        .map(Result::<DirEntry, _>::unwrap)
        .filter(|entry| {
            entry
                .file_type()
                .unwrap_or_else(|error| {
                    panic!(
                        "error: Cannot read file type of {path:?}: {error}",
                        path = entry.path(),
                    )
                })
                .is_dir()
        })
        .map(|entry| entry.path())
        .sorted();

    let mut total_written = 0usize;
    for song_dir in song_dirs {
        let has_txt = read_dir(&song_dir)
            .unwrap_or_else(|error| panic!("error: Cannot read directory {song_dir:?}: {error}"))
            .map(Result::<DirEntry, _>::unwrap)
            .any(|entry| {
                entry
                    .file_name()
                    .to_str()
                    .map(|name| name.starts_with("lyrics.") && name.ends_with(".txt"))
                    .unwrap_or(false)
            });
        if !has_txt {
            continue;
        }
        let song = match load_song(&song_dir) {
            Ok(song) => song,
            Err(error) => {
                eprintln!("error: {error}");
                return ExitCode::FAILURE;
            }
        };
        eprintln!("stage: Rendering {:?}", song.directory_name);
        let written = match render_song_to_disk(&song, &args.dist, args.execute) {
            Ok(count) => count,
            Err(error) => {
                eprintln!("error: {error}");
                return ExitCode::FAILURE;
            }
        };
        total_written += written;
    }

    if !args.execute {
        eprintln!();
        eprintln!("info: No files were written. Rerun with --execute to apply changes.");
        eprintln!("info: {total_written} files would be written.");
    } else {
        eprintln!();
        eprintln!("info: Wrote {total_written} files.");
    }
    ExitCode::SUCCESS
}

/// Payload for [`GenerateError::ReadFile`].
#[derive(Debug, Display)]
#[display("cannot read {path:?}: {cause}")]
pub struct GenerateErrorReadFilePayload {
    pub path: PathBuf,
    pub cause: io::Error,
}

/// Payload for [`GenerateError::ReadDir`].
#[derive(Debug, Display)]
#[display("cannot read directory {path:?}: {cause}")]
pub struct GenerateErrorReadDirPayload {
    pub path: PathBuf,
    pub cause: io::Error,
}

/// Payload for [`GenerateError::CreateDir`].
#[derive(Debug, Display)]
#[display("cannot create directory {path:?}: {cause}")]
pub struct GenerateErrorCreateDirPayload {
    pub path: PathBuf,
    pub cause: io::Error,
}

/// Payload for [`GenerateError::WriteFile`].
#[derive(Debug, Display)]
#[display("cannot write {path:?}: {cause}")]
pub struct GenerateErrorWriteFilePayload {
    pub path: PathBuf,
    pub cause: io::Error,
}

/// Payload for [`GenerateError::UnrecognizedLanguage`].
#[derive(Debug, Display, Clone, PartialEq, Eq)]
#[display("lyrics file {path:?} has unrecognized language code {code:?}")]
pub struct GenerateErrorUnrecognizedLanguagePayload {
    pub path: PathBuf,
    pub code: String,
}

/// Payload for [`GenerateError::ParseVideoDesc`].
#[derive(Debug, Display)]
#[display("failed to parse {path:?}: {cause}")]
pub struct GenerateErrorParseVideoDescPayload {
    pub path: PathBuf,
    pub cause: toml::de::Error,
}

/// Payload for [`GenerateError::ParseLineMarkers`].
#[derive(Debug, Display)]
#[display("failed to parse {path:?}: {cause}")]
pub struct GenerateErrorParseLineMarkersPayload {
    pub path: PathBuf,
    pub cause: toml::de::Error,
}

/// Payload for [`GenerateError::ParseCredits`].
#[derive(Debug, Display, Clone, PartialEq, Eq)]
#[display("failed to parse {path:?}: {cause}")]
pub struct GenerateErrorParseCreditsPayload {
    pub path: PathBuf,
    pub cause: String,
}

/// Payload for [`GenerateError::ParseLyrics`].
#[derive(Debug, Display, Clone, PartialEq, Eq)]
#[display("failed to parse {path:?}: {cause}")]
pub struct GenerateErrorParseLyricsPayload {
    pub path: PathBuf,
    pub cause: ParseLyricsError,
}

/// Payload for [`GenerateError::RenderVtt`].
#[derive(Debug, Display, Clone, PartialEq, Eq)]
#[display("failed to render {song}/lyrics.{language}.vtt: {cause}")]
pub struct GenerateErrorRenderVttPayload {
    pub song: String,
    pub language: Language,
    pub cause: RenderVttError,
}

/// Payload for [`GenerateError::RenderSrt`].
#[derive(Debug, Display, Clone, PartialEq, Eq)]
#[display("failed to render {song}/lyrics.{language}.srt: {cause}")]
pub struct GenerateErrorRenderSrtPayload {
    pub song: String,
    pub language: Language,
    pub cause: RenderSrtError,
}

#[derive(Debug, Display, Error)]
#[non_exhaustive]
pub enum GenerateError {
    ReadFile(#[error(not(source))] GenerateErrorReadFilePayload),
    ReadDir(#[error(not(source))] GenerateErrorReadDirPayload),
    CreateDir(#[error(not(source))] GenerateErrorCreateDirPayload),
    WriteFile(#[error(not(source))] GenerateErrorWriteFilePayload),
    UnrecognizedLanguage(#[error(not(source))] GenerateErrorUnrecognizedLanguagePayload),
    ParseVideoDesc(#[error(not(source))] GenerateErrorParseVideoDescPayload),
    ParseLineMarkers(#[error(not(source))] GenerateErrorParseLineMarkersPayload),
    ParseCredits(#[error(not(source))] GenerateErrorParseCreditsPayload),
    ParseLyrics(#[error(not(source))] GenerateErrorParseLyricsPayload),
    RenderVtt(#[error(not(source))] GenerateErrorRenderVttPayload),
    RenderSrt(#[error(not(source))] GenerateErrorRenderSrtPayload),
}
