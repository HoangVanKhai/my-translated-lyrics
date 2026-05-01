pub mod _utils;
pub use _utils::*;

use itertools::Itertools;
use pipe_trait::Pipe;
use pretty_assertions::assert_eq;
use std::collections::BTreeSet;
use std::ffi::OsString;
use std::fs::{DirEntry, read_dir, read_to_string};
use std::path::{Path, PathBuf};
use translated_lyrics::generate_subtitles::{load_song, render_song};
use walkdir::WalkDir;

/// Exhaustively re-renders each song directory in `sources/` and
/// compares the generated `.srt` and `.vtt` files against the checked-in
/// copies under `dist/`. This guards against silent drift between the
/// source lyrics text files and the rendered subtitle artifacts and
/// against stale artifacts left behind in `dist/`. When the test fails,
/// follow its instruction to regenerate `dist/`.
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

    let mut expected_dist_files: BTreeSet<PathBuf> = BTreeSet::new();
    let mut rendered_song_names: BTreeSet<String> = BTreeSet::new();
    for entry in entries {
        let song_dir = entry.path();
        if !has_lyrics_txt(&song_dir) {
            continue;
        }
        rendered_song_names.insert(
            song_dir
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap()
                .to_string(),
        );
        let song = load_song(&song_dir).unwrap();
        let rendered_count = render_song(&song, &scratch_dir, true).unwrap();
        let subtitle_files = collect_subtitle_files(&scratch_dir.join(&song.directory_name));
        dbg!(&subtitle_files);
        assert_eq!(subtitle_files.len(), rendered_count);
        for generated_path in subtitle_files {
            let relative = generated_path.strip_prefix(&scratch_dir).unwrap();
            let expected_path = dist_dir.join(relative);
            let generated = read_to_string(&generated_path).unwrap();
            let expected = read_to_string(&expected_path).unwrap_or_else(|error| {
                panic!(
                    "expected dist artifact {} is missing ({error}). Regenerate with `cargo run --bin generate-subtitles -- sources dist --execute`.",
                    expected_path.display(),
                )
            });
            assert_eq!(
                generated,
                expected,
                "{} drifted from dist. Regenerate with `cargo run --bin generate-subtitles -- sources dist --execute`.",
                relative.display(),
            );
            expected_dist_files.insert(expected_path);
        }
    }

    assert!(
        !expected_dist_files.is_empty(),
        "no songs were rendered; is sources/ empty?",
    );

    let actual_dist_files: BTreeSet<PathBuf> = rendered_song_names
        .iter()
        .flat_map(|song_name| collect_subtitle_files(&dist_dir.join(song_name)))
        .collect();
    let stale: Vec<&PathBuf> = actual_dist_files.difference(&expected_dist_files).collect();
    assert!(
        stale.is_empty(),
        "dist/ contains stale subtitle artifacts that the generator no longer produces: {stale:#?}. The generator does not remove files it no longer writes; delete them manually, then rerun `cargo run --bin generate-subtitles -- sources dist --execute` to verify.",
    );
    let missing: Vec<&PathBuf> = expected_dist_files.difference(&actual_dist_files).collect();
    assert!(
        missing.is_empty(),
        "dist/ is missing subtitle artifacts the generator just wrote: {missing:#?}. Regenerate with `cargo run --bin generate-subtitles -- sources dist --execute`.",
    );
}

fn has_lyrics_txt(song_dir: &Path) -> bool {
    song_dir
        .pipe(read_dir)
        .into_iter() // ignore Err
        .flatten()
        .map(Result::<DirEntry, _>::unwrap)
        .map(|entry| entry.file_name())
        .flat_map(OsString::into_string)
        .any(|name| name.starts_with("lyrics.") && name.ends_with(".txt"))
}

fn collect_subtitle_files(root: &Path) -> BTreeSet<PathBuf> {
    root.pipe(WalkDir::new)
        .into_iter()
        .map(Result::<walkdir::DirEntry, _>::unwrap)
        .filter(|entry| entry.file_type().is_file())
        .map(walkdir::DirEntry::into_path)
        .filter(|path| {
            path.extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| ext == "srt" || ext == "vtt")
        })
        .collect()
}
