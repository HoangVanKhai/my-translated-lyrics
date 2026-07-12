use super::parse::{SubtitleCue, parse_lyrics};
use super::render_srt::render_srt;
use super::render_vtt::render_vtt;
use super::styles::StylePalette;
use derive_more::AddAssign;
use lyrics_core::credits_descriptor::{CREDITS_CONFIG_FILE_NAME, CreditsDesc};
use lyrics_core::file_snapshot::FileSnapshot;
use lyrics_core::line_markers_descriptor::{LINE_MARKERS_CONFIG_FILE_NAME, LineMarkersDesc};
use lyrics_core::video_descriptor::{Language, VIDEO_CONFIG_FILE_NAME, VideoDesc};
use pipe_trait::Pipe;
use std::collections::BTreeMap;
use std::fs::{create_dir_all, read_dir, read_to_string, write as write_file};
use std::path::Path;

const LYRICS_TXT_SUFFIX: &str = ".txt";

pub struct LanguageBundle {
    pub language: Language,
    pub cues: Vec<SubtitleCue>,
}

pub struct Song<'a> {
    pub directory_name: &'a str,
    pub markers: LineMarkersDesc,
    pub credits: CreditsDesc,
    pub languages: Vec<LanguageBundle>,
}

#[derive(AddAssign, Clone, Copy, Debug, Default)]
pub struct RenderCounts {
    pub added: usize,
    pub updated: usize,
}

impl RenderCounts {
    pub fn total(self) -> usize {
        self.added + self.updated
    }

    fn record(&mut self, outcome: WriteOutcome) {
        match outcome {
            WriteOutcome::Added => self.added += 1,
            WriteOutcome::Updated => self.updated += 1,
            WriteOutcome::Unchanged => {}
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum WriteOutcome {
    Added,
    Updated,
    Unchanged,
}

pub fn render_song(
    song: &Song,
    palette: &StylePalette,
    dist_dir: &Path,
    execute: bool,
) -> RenderCounts {
    let destination_dir = dist_dir.join(song.directory_name);
    if execute {
        create_dir_all(&destination_dir).unwrap_or_else(|error| {
            panic!("error: Cannot create directory {destination_dir:?}: {error}")
        });
    }

    let mut counts = RenderCounts::default();
    for bundle in &song.languages {
        let vtt = render_vtt(
            &bundle.cues,
            &song.markers,
            &song.credits,
            palette,
            &bundle.language,
        )
        .unwrap_or_else(|error| {
            panic!(
                "error: Failed to render {song}/lyrics.{language}.vtt: {error}",
                song = song.directory_name,
                language = bundle.language,
            )
        });
        let vtt_path = destination_dir.join(format!("lyrics.{}.vtt", bundle.language));
        counts.record(write_subtitle(&vtt_path, &vtt, execute));

        let srt = render_srt(
            &bundle.cues,
            &song.markers,
            &song.credits,
            palette,
            &bundle.language,
        )
        .unwrap_or_else(|error| {
            panic!(
                "error: Failed to render {song}/lyrics.{language}.srt: {error}",
                song = song.directory_name,
                language = bundle.language,
            )
        });
        let srt_path = destination_dir.join(format!("lyrics.{}.srt", bundle.language));
        counts.record(write_subtitle(&srt_path, &srt, execute));
    }
    counts
}

fn write_subtitle(path: &Path, content: &str, execute: bool) -> WriteOutcome {
    let (verb, outcome) = if path.exists() {
        let snapshot = path
            .to_path_buf()
            .pipe(FileSnapshot::new)
            .unwrap_or_else(|error| panic!("error: Cannot read {path:?}: {error}"));
        if snapshot.content_eq_str(content) {
            return WriteOutcome::Unchanged;
        }
        ("update", WriteOutcome::Updated)
    } else {
        ("add", WriteOutcome::Added)
    };
    eprintln!("{verb} {path:?}");
    if execute {
        write_file(path, content)
            .unwrap_or_else(|error| panic!("error: Cannot write {path:?}: {error}"));
    }
    outcome
}

/// Reads and parses the shared presentation palette from `path`. The
/// palette maps each voice marker and named class to its color and text
/// decoration; see [`StylePalette`]. Any read or parse failure aborts
/// the program with a message naming the offending file.
pub fn load_palette(path: &Path) -> StylePalette {
    path.pipe(read_to_string)
        .unwrap_or_else(|error| panic!("error: Cannot read {path:?}: {error}"))
        .pipe_as_ref(toml::from_str::<StylePalette>)
        .unwrap_or_else(|error| panic!("error: Cannot parse {path:?}: {error}"))
}

pub fn load_song(song_dir: &Path) -> Song<'_> {
    let directory_name = song_dir
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_else(|| panic!("error: song directory {song_dir:?} has a non-UTF-8 name"));

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
                    "error: lyrics directory entry {:?} has a non-UTF-8 filename",
                    entry.path(),
                )
            })
            .strip_prefix("lyrics.")
            .and_then(|rest| rest.strip_suffix(LYRICS_TXT_SUFFIX));
        let Some(middle) = middle else {
            continue;
        };
        let lyrics_path = entry.path();
        let language = middle.parse::<Language>().unwrap_or_else(
            |strum::ParseError::VariantNotFound| {
                panic!(
                    "error: lyrics file {lyrics_path:?} has unrecognized language code {middle:?}",
                )
            },
        );
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
