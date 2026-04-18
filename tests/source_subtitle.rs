use into_sorted::IntoSorted;
use itertools::Itertools;
use pipe_trait::Pipe;
use pretty_assertions::assert_eq;
use pretty_yaml::config::FormatOptions;
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

    let format_options = FormatOptions::default();

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
        let desc: SubtitleDesc = original.pipe_as_ref(serde_saphyr::from_str).unwrap();

        assert_eq!(
            desc.credit_roles,
            desc.credit_roles.clone().into_sorted(),
            "credit-roles in {song_name:?} are not in sorted order",
        );

        assert_eq!(
            desc.credit_names,
            desc.credit_names.clone().into_sorted(),
            "credit-names in {song_name:?} are not in sorted order",
        );

        let formatted = pretty_yaml::format_text(&original, &format_options).unwrap();
        assert_eq!(
            original, formatted,
            "{song_name:?} is not in canonical form",
        );
    }
}
