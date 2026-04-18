use itertools::Itertools;
use pipe_trait::Pipe;
use pretty_assertions::assert_eq;
use std::collections::BTreeMap;
use std::fs::{DirEntry, read_dir, read_to_string};
use std::path::Path;
use translated_lyrics::subtitle_descriptor::{SUBTITLE_CONFIG_FILE_NAME, SubtitleDesc};
use translated_lyrics::video_descriptor::Language::{self, Chinese as Zh};

fn sorted_by_zh(entries: &[BTreeMap<Language, String>]) -> Vec<BTreeMap<Language, String>> {
    let mut sorted = entries.to_vec();
    sorted.sort_by(|a, b| {
        a.get(&Zh)
            .map(String::as_str)
            .unwrap_or("")
            .cmp(b.get(&Zh).map(String::as_str).unwrap_or(""))
    });
    sorted
}

/// Every `sources/*/subtitle.yaml` must parse as a valid [`SubtitleDesc`] whose
/// `credit-roles` and `credit-names` are sorted by their `zh` value.
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

        assert_eq!(
            desc.credit_roles,
            sorted_by_zh(&desc.credit_roles),
            "credit-roles in {song_name:?} are not sorted by zh value",
        );

        assert_eq!(
            desc.credit_names,
            sorted_by_zh(&desc.credit_names),
            "credit-names in {song_name:?} are not sorted by zh value",
        );
    }
}
