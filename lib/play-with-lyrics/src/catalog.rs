//! Loading the catalogue of videos from the source directory.
//!
//! The source directory has the same layout as the one
//! `install-local-lyrics` reads: one subdirectory per video, each holding
//! a `video.toml` descriptor. The descriptors supply the English,
//! Vietnamese, and Chinese titles shown in the interactive table, together
//! with the collection and video title used to locate the playable file in
//! the media library.

use fuzzy_select::selection::Searchable;
use lyrics_core::video_descriptor::{Language, VIDEO_CONFIG_FILE_NAME, VideoDesc};
use pipe_trait::Pipe;
use std::fs::{read_dir, read_to_string};
use std::path::Path;

/// A single video loaded from the source directory.
pub struct Video {
    /// The parsed `video.toml` descriptor.
    pub desc: VideoDesc,
}

impl Video {
    /// The song title in the given language, when the descriptor provides
    /// one. Not every video has a title in every language.
    pub fn title(&self, language: Language) -> Option<&str> {
        self.desc.song_titles.get(&language).map(String::as_str)
    }
}

impl Searchable for Video {
    fn search_keys(&self) -> Vec<&str> {
        // The raw video title is searchable too, so a query can match the
        // original upload title as well as any translation.
        let mut keys = vec![self.desc.video_title.as_ref()];
        for language in [Language::English, Language::Vietnamese, Language::Chinese] {
            if let Some(title) = self.title(language) {
                keys.push(title);
            }
        }
        keys
    }
}

/// Reads every `video.toml` under `source` and returns the videos sorted
/// by English title, falling back to the raw video title when a video has
/// no English title. The order is deterministic so the table looks the
/// same on every run.
pub fn load(source: &Path) -> Vec<Video> {
    let mut videos: Vec<Video> = source
        .pipe(read_dir)
        .unwrap_or_else(|error| panic!("error: Cannot read source directory {source:?}: {error}"))
        .map(|entry| {
            entry.unwrap_or_else(|error| {
                panic!("error: Cannot read an entry of directory {source:?}: {error}")
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
                .is_dir()
        })
        .map(|entry| {
            let desc_path = entry.path().join(VIDEO_CONFIG_FILE_NAME);
            let content = desc_path
                .pipe_ref(read_to_string)
                .unwrap_or_else(|error| panic!("error: Cannot read {desc_path:?}: {error}"));
            let desc: VideoDesc = content
                .pipe_as_ref(toml::from_str)
                .unwrap_or_else(|error| panic!("error: Cannot parse {desc_path:?}: {error}"));
            Video { desc }
        })
        .collect();
    videos.sort_by_key(sort_key);
    videos
}

/// The case-insensitive key a video is ordered by within the table.
fn sort_key(video: &Video) -> String {
    video
        .title(Language::English)
        .unwrap_or_else(|| video.desc.video_title.as_ref())
        .to_lowercase()
}

/// The human-readable name of a language, shown in the language selector.
pub fn language_label(language: Language) -> &'static str {
    match language {
        Language::English => "English",
        Language::Vietnamese => "Vietnamese",
        Language::Chinese => "Chinese",
    }
}
