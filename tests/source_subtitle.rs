use itertools::Itertools;
use pipe_trait::Pipe;
use std::fs::{DirEntry, read_dir, read_to_string};
use std::path::Path;
use translated_lyrics::subtitle_descriptor::{SUBTITLE_CONFIG_FILE_NAME, SubtitleDesc};

/// Every `sources/*/subtitle.yaml` must parse as a valid [`SubtitleDesc`].
#[test]
fn source_subtitle_descriptors_are_valid() {
    let sources_dir = env!("CARGO_MANIFEST_DIR").pipe(Path::new).join("sources");
    assert!(
        sources_dir.is_dir(),
        "expected sources directory to exist for subtitle descriptor validation: {}",
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

        eprintln!("CASE: {}", entry.file_name().display());
        subtitle_path
            .pipe(read_to_string)
            .unwrap()
            .pipe_as_ref(serde_saphyr::from_str::<SubtitleDesc>)
            .unwrap()
            .pipe(drop::<SubtitleDesc>);
    }
}
