use into_sorted::IntoSorted;
use itertools::Itertools;
use pipe_trait::Pipe;
use pretty_assertions::assert_eq;
use std::fs::{DirEntry, read_dir, read_to_string};
use std::path::Path;
use translated_lyrics::credits_descriptor::{CREDITS_CONFIG_FILE_NAME, CreditsDesc};

/// Each `sources/*/credits.yaml` must parse as a valid [`CreditsDesc`] whose
/// `credit-roles` and `credit-names` are in natural sorted order.
#[test]
fn source_credits_descriptors() {
    let sources_dir = env!("CARGO_MANIFEST_DIR").pipe(Path::new).join("sources");
    assert!(
        sources_dir.is_dir(),
        "expected sources directory to exist: {}",
        sources_dir.display(),
    );

    let entries = sources_dir
        .pipe(read_dir)
        .unwrap()
        .map(Result::<DirEntry, _>::unwrap)
        .sorted_by_key(DirEntry::file_name);

    for entry in entries {
        let song_dir = entry.path();
        if !song_dir.is_dir() {
            continue;
        }

        let credits_path = song_dir.join(CREDITS_CONFIG_FILE_NAME);
        if !credits_path.exists() {
            continue;
        }

        let song_name = entry.file_name();
        eprintln!("CASE: {}", song_name.display());

        let desc: CreditsDesc = credits_path
            .pipe(read_to_string)
            .unwrap()
            .pipe_as_ref(serde_saphyr::from_str)
            .unwrap();

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
    }
}
