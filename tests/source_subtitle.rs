use itertools::Itertools;
use pipe_trait::Pipe;
use pretty_assertions::assert_eq;
use std::collections::BTreeMap;
use std::fs::{DirEntry, read_dir, read_to_string};
use std::path::Path;
use translated_lyrics::subtitle_descriptor::{SUBTITLE_CONFIG_FILE_NAME, SubtitleDesc};
use translated_lyrics::video_descriptor::Language;

/// Every `sources/*/subtitle.yaml` must parse as a valid [`SubtitleDesc`] whose
/// `credit-roles` and `credit-names` are in natural sorted order.
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

        let desc: SubtitleDesc = subtitle_path
            .pipe(read_to_string)
            .unwrap()
            .pipe_as_ref(serde_saphyr::from_str)
            .unwrap();

        let mut sorted_credit_roles: Vec<BTreeMap<Language, String>> = desc.credit_roles.clone();
        sorted_credit_roles.sort();
        assert_eq!(
            desc.credit_roles, sorted_credit_roles,
            "credit-roles in {song_name:?} are not in sorted order",
        );

        let mut sorted_credit_names: Vec<BTreeMap<Language, String>> = desc.credit_names.clone();
        sorted_credit_names.sort();
        assert_eq!(
            desc.credit_names, sorted_credit_names,
            "credit-names in {song_name:?} are not in sorted order",
        );
    }
}
