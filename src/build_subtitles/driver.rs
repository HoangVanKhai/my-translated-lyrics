//! Driver for the `build-subtitles` binary.
//!
//! The driver walks the source directory, loads each song's descriptor
//! files and per-language lyrics, renders them, and writes the output
//! into the destination directory. A dry-run mode prints the planned
//! writes and leaves the filesystem untouched; the equivalent of
//! `install-local-lyrics`'s `--execute` flag opts into the actual write.

use super::parse::{ParseLyricsError, SubtitleCue, parse_lyrics};
use super::render_srt::{RenderSrtError, render_file as render_srt_file};
use super::render_vtt::{RenderVttError, render_file as render_vtt_file};
use crate::credits_descriptor::{CREDITS_CONFIG_FILE_NAME, CreditsDesc};
use crate::line_markers_descriptor::{LINE_MARKERS_CONFIG_FILE_NAME, LineMarkersDesc};
use crate::video_descriptor::{Language, VIDEO_CONFIG_FILE_NAME, VideoDesc};
use clap::Parser;
use derive_more::{Display, Error};
use itertools::Itertools;
use std::collections::BTreeMap;
use std::fs::{create_dir_all, read_dir, read_to_string, write as write_file};
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
    pub lyrics_path: PathBuf,
    pub cues: Vec<SubtitleCue>,
}

/// Parsed representation of a song directory, ready for rendering.
pub struct Song {
    pub directory_name: String,
    pub source_dir: PathBuf,
    pub video: VideoDesc,
    pub markers: LineMarkersDesc,
    pub credits: CreditsDesc,
    pub languages: Vec<LanguageBundle>,
}

/// Builds the subtitles for a single song by rendering each language
/// to both `.srt` and `.vtt` and writing the result into `dist_dir`.
/// Returns the set of files that were (or, in dry-run mode, would
/// have been) written.
pub fn render_song_to_disk(
    song: &Song,
    dist_dir: &Path,
    execute: bool,
) -> Result<Vec<PathBuf>, BuildError> {
    let destination_dir = dist_dir.join(&song.directory_name);
    if execute {
        create_dir_all(&destination_dir).map_err(|source| BuildError::CreateDir {
            path: destination_dir.clone(),
            source,
        })?;
    }

    let mut written: Vec<PathBuf> = Vec::with_capacity(song.languages.len() * 2);
    for bundle in &song.languages {
        let vtt = render_vtt_file(&bundle.cues, &song.markers, &song.credits, &bundle.language)
            .map_err(|source| BuildError::RenderVtt {
                song: song.directory_name.clone(),
                language: bundle.language.clone(),
                source,
            })?;
        let srt = render_srt_file(&bundle.cues, &song.markers, &song.credits, &bundle.language)
            .map_err(|source| BuildError::RenderSrt {
                song: song.directory_name.clone(),
                language: bundle.language.clone(),
                source,
            })?;
        let vtt_path = destination_dir.join(format!("lyrics.{lang}.vtt", lang = bundle.language));
        let srt_path = destination_dir.join(format!("lyrics.{lang}.srt", lang = bundle.language));
        write_subtitle(&vtt_path, &vtt, execute)?;
        write_subtitle(&srt_path, &srt, execute)?;
        written.push(vtt_path);
        written.push(srt_path);
    }
    Ok(written)
}

fn write_subtitle(path: &Path, content: &str, execute: bool) -> Result<(), BuildError> {
    eprintln!("write {path:?}");
    if !execute {
        return Ok(());
    }
    write_file(path, content).map_err(|source| BuildError::WriteFile {
        path: path.to_path_buf(),
        source,
    })
}

/// Loads all source artifacts for a single song into memory and parses
/// each cue list.
pub fn load_song(song_dir: &Path) -> Result<Song, BuildError> {
    let directory_name = song_dir
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| BuildError::NonUtf8Path {
            path: song_dir.to_path_buf(),
        })?
        .to_string();

    let video_path = song_dir.join(VIDEO_CONFIG_FILE_NAME);
    let video_content = read_to_string(&video_path).map_err(|source| BuildError::ReadFile {
        path: video_path.clone(),
        source,
    })?;
    let video: VideoDesc =
        toml::from_str(&video_content).map_err(|source| BuildError::ParseVideoDesc {
            path: video_path.clone(),
            source,
        })?;

    let markers_path = song_dir.join(LINE_MARKERS_CONFIG_FILE_NAME);
    let markers: LineMarkersDesc = if markers_path.exists() {
        let markers_content =
            read_to_string(&markers_path).map_err(|source| BuildError::ReadFile {
                path: markers_path.clone(),
                source,
            })?;
        toml::from_str(&markers_content).map_err(|source| BuildError::ParseLineMarkers {
            path: markers_path.clone(),
            source,
        })?
    } else {
        LineMarkersDesc::default()
    };

    let credits_path = song_dir.join(CREDITS_CONFIG_FILE_NAME);
    let credits: CreditsDesc = if credits_path.exists() {
        let credits_content =
            read_to_string(&credits_path).map_err(|source| BuildError::ReadFile {
                path: credits_path.clone(),
                source,
            })?;
        serde_saphyr::from_str(&credits_content).map_err(|source| BuildError::ParseCredits {
            path: credits_path.clone(),
            source: source.to_string(),
        })?
    } else {
        CreditsDesc::default()
    };

    let mut languages: BTreeMap<Language, LanguageBundle> = BTreeMap::new();
    let entries = read_dir(song_dir).map_err(|source| BuildError::ReadDir {
        path: song_dir.to_path_buf(),
        source,
    })?;
    for entry in entries {
        let entry = entry.map_err(|source| BuildError::ReadDir {
            path: song_dir.to_path_buf(),
            source,
        })?;
        let file_name = entry.file_name();
        let Some(file_name) = file_name.to_str() else {
            continue;
        };
        let Some(middle) = file_name
            .strip_prefix("lyrics.")
            .and_then(|rest| rest.strip_suffix(LYRICS_TXT_SUFFIX))
        else {
            continue;
        };
        let Ok(language) = middle.parse::<Language>() else {
            continue;
        };
        let lyrics_path = entry.path();
        let content = read_to_string(&lyrics_path).map_err(|source| BuildError::ReadFile {
            path: lyrics_path.clone(),
            source,
        })?;
        let cues = parse_lyrics(&content).map_err(|source| BuildError::ParseLyrics {
            path: lyrics_path.clone(),
            source,
        })?;
        languages.insert(
            language.clone(),
            LanguageBundle {
                language,
                lyrics_path,
                cues,
            },
        );
    }

    Ok(Song {
        directory_name,
        source_dir: song_dir.to_path_buf(),
        video,
        markers,
        credits,
        languages: languages.into_values().collect(),
    })
}

pub fn main() -> ExitCode {
    let args = Args::parse();

    let song_dirs: Vec<PathBuf> = match read_dir(&args.sources) {
        Ok(iter) => iter
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false))
            .map(|entry| entry.path())
            .sorted()
            .collect(),
        Err(error) => {
            eprintln!(
                "error: Cannot read sources directory {path:?}: {error}",
                path = args.sources,
            );
            return ExitCode::FAILURE;
        }
    };

    let mut total_written = 0usize;
    for song_dir in song_dirs {
        let has_txt = read_dir(&song_dir)
            .map(|iter| {
                iter.filter_map(|entry| entry.ok()).any(|entry| {
                    entry
                        .file_name()
                        .to_str()
                        .map(|name| name.starts_with("lyrics.") && name.ends_with(".txt"))
                        .unwrap_or(false)
                })
            })
            .unwrap_or(false);
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
            Ok(paths) => paths,
            Err(error) => {
                eprintln!("error: {error}");
                return ExitCode::FAILURE;
            }
        };
        total_written += written.len();
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

#[derive(Debug, Display, Error)]
#[non_exhaustive]
pub enum BuildError {
    #[display("cannot read {path:?}: {source}")]
    ReadFile { path: PathBuf, source: io::Error },
    #[display("cannot read directory {path:?}: {source}")]
    ReadDir { path: PathBuf, source: io::Error },
    #[display("cannot create directory {path:?}: {source}")]
    CreateDir { path: PathBuf, source: io::Error },
    #[display("cannot write {path:?}: {source}")]
    WriteFile { path: PathBuf, source: io::Error },
    #[display("path is not valid UTF-8: {path:?}")]
    NonUtf8Path {
        #[error(not(source))]
        path: PathBuf,
    },
    #[display("failed to parse {path:?}: {source}")]
    ParseVideoDesc {
        #[error(not(source))]
        path: PathBuf,
        source: toml::de::Error,
    },
    #[display("failed to parse {path:?}: {source}")]
    ParseLineMarkers {
        #[error(not(source))]
        path: PathBuf,
        source: toml::de::Error,
    },
    #[display("failed to parse {path:?}: {source}")]
    ParseCredits {
        #[error(not(source))]
        path: PathBuf,
        #[error(not(source))]
        source: String,
    },
    #[display("failed to parse {path:?}: {source}")]
    ParseLyrics {
        #[error(not(source))]
        path: PathBuf,
        source: ParseLyricsError,
    },
    #[display("failed to render {song}.{language}.vtt: {source}")]
    RenderVtt {
        #[error(not(source))]
        song: String,
        #[error(not(source))]
        language: Language,
        source: RenderVttError,
    },
    #[display("failed to render {song}.{language}.srt: {source}")]
    RenderSrt {
        #[error(not(source))]
        song: String,
        #[error(not(source))]
        language: Language,
        source: RenderSrtError,
    },
}
