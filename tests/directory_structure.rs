use itertools::Itertools;
use pipe_trait::Pipe;
use std::fs::{DirEntry, read_dir};
use std::path::Path;
use translated_lyrics::video_descriptor::{LyricsFileName, ParseLyricsFileNameError};

/// Verify that `data/` and `drafts/` have a flat two-level structure:
///
/// ```text
/// data/
/// ├── SongName/
/// │   ├── video.toml
/// │   ├── lyrics.vi.srt
/// │   └── lyrics.zh.srt
/// └── AnotherSong/
///     └── ...
/// ```
///
/// Rejects files placed directly under the top-level directory (too shallow)
/// and directories nested inside a song directory (too deep).
#[test]
fn data_and_drafts_have_flat_structure() {
    for top_dir_name in ["data", "drafts"] {
        let top_dir = env!("CARGO_MANIFEST_DIR")
            .pipe(Path::new)
            .join(top_dir_name);
        if !top_dir.exists() {
            assert_ne!(top_dir_name, "data", "data/ directory must exist",);
            continue;
        }

        let entries = top_dir
            .pipe(read_dir)
            .unwrap()
            .map(Result::unwrap)
            .sorted_by_key(DirEntry::file_name);

        for entry in entries {
            let path = entry.path();
            let name = entry.file_name();
            let name = name.to_str().expect("path isn't valid UTF-8");

            assert!(
                path.is_dir(),
                "{top_dir_name}/{name} should be a directory, not a file",
            );

            let inner_entries = path
                .pipe(read_dir)
                .unwrap()
                .map(Result::unwrap)
                .sorted_by_key(DirEntry::file_name);

            for inner_entry in inner_entries {
                let inner_path = inner_entry.path();
                let inner_name = inner_entry.file_name();
                let inner_name = inner_name.to_str().expect("path isn't valid UTF-8");

                assert!(
                    inner_path.is_file(),
                    "{top_dir_name}/{name}/{inner_name} should be a file, not a directory",
                );
            }
        }
    }
}

/// Subtitle files in `data/` must be named `lyrics.{lang}.{ext}`.
#[test]
fn data_subtitle_file_names_are_canonical() {
    let data_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("data");

    let entries = data_dir
        .pipe(read_dir)
        .unwrap()
        .map(Result::unwrap)
        .sorted_by_key(DirEntry::file_name);

    for entry in entries {
        let song_dir = entry.path();
        if !song_dir.is_dir() {
            continue;
        }

        let song_name = entry.file_name();
        let song_name = song_name.to_str().expect("path isn't valid UTF-8");

        let inner_entries = song_dir
            .pipe(read_dir)
            .unwrap()
            .map(Result::unwrap)
            .sorted_by_key(DirEntry::file_name);

        for inner_entry in inner_entries {
            let name = inner_entry.file_name();
            let name = name.to_str().expect("path isn't valid UTF-8");

            match name.parse::<LyricsFileName>() {
                Ok(_) => {}
                Err(ParseLyricsFileNameError::NotLyricsFile) => continue,
                Err(error) => panic!("data/{song_name}/{name}: {error}"),
            }
        }
    }
}
