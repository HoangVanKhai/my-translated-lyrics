use itertools::Itertools;
use pipe_trait::Pipe;
use std::fs::{DirEntry, read_dir, read_to_string};
use std::path::Path;
use translated_lyrics::words_descriptor::{WORDS_CONFIG_FILE_NAME, WordsDesc};

/// Every `sources/*/words.toml` must parse as a valid [`WordsDesc`].
#[test]
fn source_words_descriptors_are_valid() {
    let sources_dir = env!("CARGO_MANIFEST_DIR").pipe(Path::new).join("sources");
    if !sources_dir.exists() {
        return;
    }

    let entries = sources_dir
        .pipe(read_dir)
        .unwrap()
        .map(Result::unwrap)
        .sorted_by_key(DirEntry::file_name);

    for entry in entries {
        let song_dir = entry.path();
        if !song_dir.is_dir() {
            continue;
        }

        let words_path = song_dir.join(WORDS_CONFIG_FILE_NAME);
        if !words_path.exists() {
            continue;
        }

        eprintln!("CASE: {}", entry.file_name().display());
        words_path
            .pipe(read_to_string)
            .unwrap()
            .pipe_as_ref(toml::from_str::<WordsDesc>)
            .unwrap()
            .pipe(drop::<WordsDesc>);
    }
}
