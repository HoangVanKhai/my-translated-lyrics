//! Driver for the `generate-subtitles` binary.
//!
//! The driver walks the source directory, loads each song's descriptor
//! files and per-language lyrics, renders them, and writes the output
//! into the destination directory. A dry-run mode prints the planned
//! writes and leaves the filesystem untouched; the equivalent of
//! `install-local-lyrics`'s `--execute` flag opts into the actual write.

use super::parse::{SubtitleCue, parse_lyrics};
use super::render_srt::render_srt;
use super::render_vtt::render_vtt;
use crate::credits_descriptor::{CREDITS_CONFIG_FILE_NAME, CreditsDesc};
use crate::line_markers_descriptor::{LINE_MARKERS_CONFIG_FILE_NAME, LineMarkersDesc};
use crate::video_descriptor::{Language, VIDEO_CONFIG_FILE_NAME, VideoDesc};
use clap::Parser;
use itertools::Itertools;
use pipe_trait::Pipe;
use std::collections::BTreeMap;
use std::fs::{DirEntry, create_dir_all, read_dir, read_to_string, write as write_file};
use std::path::{Path, PathBuf};

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
pub fn render_song(song: &Song, dist_dir: &Path, execute: bool) -> usize {
    let destination_dir = dist_dir.join(&song.directory_name);
    if execute {
        create_dir_all(&destination_dir).unwrap_or_else(|error| {
            panic!("error: Cannot create directory {destination_dir:?}: {error}")
        });
    }

    let mut written: usize = 0;
    for bundle in &song.languages {
        let vtt = render_vtt(&bundle.cues, &song.markers, &song.credits, &bundle.language)
            .unwrap_or_else(|error| {
                panic!(
                    "error: Failed to render {song}/lyrics.{language}.vtt: {error}",
                    song = song.directory_name,
                    language = bundle.language,
                )
            });
        let vtt_path = destination_dir.join(format!("lyrics.{}.vtt", bundle.language));
        write_subtitle(&vtt_path, &vtt, execute);
        written += 1;

        let srt = render_srt(&bundle.cues, &song.markers, &song.credits, &bundle.language)
            .unwrap_or_else(|error| {
                panic!(
                    "error: Failed to render {song}/lyrics.{language}.srt: {error}",
                    song = song.directory_name,
                    language = bundle.language,
                )
            });
        let srt_path = destination_dir.join(format!("lyrics.{}.srt", bundle.language));
        write_subtitle(&srt_path, &srt, execute);
        written += 1;
    }
    written
}

fn write_subtitle(path: &Path, content: &str, execute: bool) {
    eprintln!("write {path:?}");
    if !execute {
        return;
    }
    write_file(path, content)
        .unwrap_or_else(|error| panic!("error: Cannot write {path:?}: {error}"));
}

/// Loads all source artifacts for a single song into memory and parses
/// each cue list.
pub fn load_song(song_dir: &Path) -> Song {
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
    video_path
        .pipe_ref(read_to_string)
        .unwrap_or_else(|error| panic!("error: Cannot read {video_path:?}: {error}"))
        .pipe_as_ref(toml::from_str::<VideoDesc>)
        .unwrap_or_else(|error| panic!("error: Cannot parse {video_path:?}: {error}"))
        .pipe(drop::<VideoDesc>);

    let markers_path = song_dir.join(LINE_MARKERS_CONFIG_FILE_NAME);
    let markers: LineMarkersDesc = markers_path
        .exists()
        .then_some(&markers_path)
        .map(read_to_string)
        .map(|result| {
            result.unwrap_or_else(|error| panic!("error: Cannot read {markers_path:?}: {error}"))
        })
        .as_deref()
        .map(toml::from_str::<LineMarkersDesc>)
        .map(|result| {
            result.unwrap_or_else(|error| panic!("error: Cannot parse {markers_path:?}: {error}"))
        })
        .unwrap_or_default();

    let credits_path = song_dir.join(CREDITS_CONFIG_FILE_NAME);
    let credits: CreditsDesc = credits_path
        .exists()
        .then_some(&credits_path)
        .map(read_to_string)
        .map(|result| {
            result.unwrap_or_else(|error| panic!("error: Cannot read {credits_path:?}: {error}"))
        })
        .as_deref()
        .map(serde_saphyr::from_str::<CreditsDesc>)
        .map(|result| {
            result.unwrap_or_else(|error| panic!("error: Cannot parse {credits_path:?}: {error}"))
        })
        .unwrap_or_default();

    let mut languages = BTreeMap::<Language, LanguageBundle>::new();
    let entries = song_dir
        .pipe_ref(read_dir)
        .unwrap_or_else(|error| panic!("error: Cannot read directory {song_dir:?}: {error}"));
    for entry in entries {
        let entry = entry.unwrap_or_else(|error| {
            panic!("error: Cannot read an entry of directory {song_dir:?}: {error}")
        });
        let file_name = entry.file_name();
        let middle = file_name
            .to_str()
            .unwrap_or_else(|| {
                panic!(
                    "lyrics directory entry {:?} has a non-UTF-8 filename",
                    entry.path(),
                )
            })
            .strip_prefix("lyrics.")
            .and_then(|rest| rest.strip_suffix(LYRICS_TXT_SUFFIX));
        let Some(middle) = middle else {
            continue;
        };
        let lyrics_path = entry.path();
        let language = middle.parse::<Language>().unwrap_or_else(|_| {
            panic!("error: lyrics file {lyrics_path:?} has unrecognized language code {middle:?}")
        });
        let cues = lyrics_path
            .pipe_ref(read_to_string)
            .unwrap_or_else(|error| panic!("error: Cannot read {lyrics_path:?}: {error}"))
            .pipe_as_ref(parse_lyrics)
            .unwrap_or_else(|error| panic!("error: Failed to parse {lyrics_path:?}: {error}"));
        let ejected = languages.insert(language, LanguageBundle { language, cues });
        assert!(
            ejected.is_none(),
            "Unexpected language code duplication of {language}: {}",
            lyrics_path.display(),
        );
    }

    Song {
        directory_name,
        markers,
        credits,
        languages: languages.into_values().collect(),
    }
}

pub fn main() {
    let args = Args::parse();

    let song_dirs = args
        .sources
        .pipe_ref(read_dir)
        .unwrap_or_else(|error| {
            panic!(
                "error: Cannot read sources directory {sources:?}: {error}",
                sources = args.sources,
            )
        })
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

    let mut total_written: usize = 0;
    for song_dir in song_dirs {
        let has_txt = song_dir
            .pipe_ref(read_dir)
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
        let song = load_song(&song_dir);
        eprintln!("stage: Rendering {:?}", song.directory_name);
        total_written += render_song(&song, &args.dist, args.execute);
    }

    if !args.execute {
        eprintln!();
        eprintln!("info: No files were written. Rerun with --execute to apply changes.");
        eprintln!("info: {total_written} files would be written.");
    } else {
        eprintln!();
        eprintln!("info: Wrote {total_written} files.");
    }
}
