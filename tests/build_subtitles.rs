pub mod _utils;
pub use _utils::*;

use itertools::Itertools;
use pretty_assertions::assert_eq;
use std::fs::{read_dir, read_to_string};
use std::path::{Path, PathBuf};
use translated_lyrics::build_subtitles::{load_song, render_song_to_disk};

/// Exhaustively re-renders each song directory in `sources/` and
/// compares the generated `.srt` and `.vtt` files against the checked-in
/// copies under `dist/`. This guards against silent drift between the
/// source lyrics text files and the rendered subtitle artifacts. When
/// the test fails, follow its instruction to regenerate `dist/`.
#[test]
fn dist_is_up_to_date_with_sources() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let sources_dir = manifest_dir.join("sources");
    let dist_dir = manifest_dir.join("dist");
    let scratch_dir = Temp::new_dir();

    let entries = read_dir(&sources_dir)
        .unwrap()
        .map(Result::unwrap)
        .filter(|entry| entry.file_type().unwrap().is_dir())
        .sorted_by_key(|entry| entry.file_name());

    let mut compared_paths: Vec<PathBuf> = Vec::new();
    for entry in entries {
        let song_dir = entry.path();
        if !has_lyrics_txt(&song_dir) {
            continue;
        }
        let song = load_song(&song_dir).expect("load song");
        let written =
            render_song_to_disk(&song, &scratch_dir, true).expect("render song to scratch");
        for generated_path in &written {
            let relative = generated_path
                .strip_prefix(&scratch_dir)
                .expect("generated path must be inside scratch dir");
            let expected_path = dist_dir.join(relative);
            let generated = read_to_string(generated_path).unwrap();
            let expected = read_to_string(&expected_path).unwrap_or_else(|error| {
                panic!(
                    "expected dist artifact {} is missing ({error}). Regenerate with `cargo run --bin build-subtitles -- sources dist --execute`.",
                    expected_path.display(),
                )
            });
            assert_eq!(
                generated,
                expected,
                "{} drifted from dist. Regenerate with `cargo run --bin build-subtitles -- sources dist --execute`.",
                relative.display(),
            );
            compared_paths.push(expected_path);
        }
    }

    assert!(
        !compared_paths.is_empty(),
        "no songs were rendered; is sources/ empty?",
    );
}

fn has_lyrics_txt(song_dir: &Path) -> bool {
    let Ok(entries) = read_dir(song_dir) else {
        return false;
    };
    entries.filter_map(Result::ok).any(|entry| {
        entry
            .file_name()
            .to_str()
            .map(|name| name.starts_with("lyrics.") && name.ends_with(".txt"))
            .unwrap_or(false)
    })
}
