use into_sorted::IntoSorted;
use itertools::Itertools;
use pipe_trait::Pipe;
use pretty_assertions::assert_eq;
use std::fs::{DirEntry, read_dir, read_to_string};
use std::path::Path;
use translated_lyrics::subtitle_descriptor::{SUBTITLE_CONFIG_FILE_NAME, SubtitleDesc};

/// Each `sources/*/subtitle.yaml` must be in canonical form.
#[test]
fn source_subtitle_descriptors() {
    let sources_dir = env!("CARGO_MANIFEST_DIR").pipe(Path::new).join("sources");
    assert!(
        sources_dir.is_dir(),
        "expected sources directory to exist: {}",
        sources_dir.display()
    );

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

        let subtitle_path = song_dir.join(SUBTITLE_CONFIG_FILE_NAME);
        if !subtitle_path.exists() {
            continue;
        }

        let song_name = entry.file_name();
        eprintln!("CASE: {}", song_name.display());

        let original = subtitle_path.pipe(read_to_string).unwrap();
        let mut desc: SubtitleDesc = original.pipe_as_ref(serde_norway::from_str).unwrap();
        desc.credit_roles = desc.credit_roles.into_sorted();
        desc.credit_names = desc.credit_names.into_sorted();

        let canonical = serde_norway::to_string(&desc).unwrap();
        assert_eq!(
            original, canonical,
            "{song_name:?} is not in canonical form",
        );
    }
}
