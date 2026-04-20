use itertools::Itertools;
use pipe_trait::Pipe;
use std::fs::{DirEntry, read_dir, read_to_string};
use std::path::Path;
use translated_lyrics::line_markers_descriptor::{LINE_MARKERS_CONFIG_FILE_NAME, LineMarkersDesc};

/// Every `sources/*/line-markers.toml` must parse as a valid
/// [`LineMarkersDesc`].
#[test]
fn source_line_markers_descriptors() {
    let sources_dir = env!("CARGO_MANIFEST_DIR").pipe(Path::new).join("sources");
    assert!(
        sources_dir.is_dir(),
        "expected sources directory to exist: {}",
        sources_dir.display()
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

        let markers_path = song_dir.join(LINE_MARKERS_CONFIG_FILE_NAME);
        if !markers_path.exists() {
            continue;
        }

        eprintln!("CASE: {}", entry.file_name().display());
        markers_path
            .pipe(read_to_string)
            .unwrap()
            .pipe_as_ref(toml::from_str::<LineMarkersDesc>)
            .unwrap()
            .pipe(drop::<LineMarkersDesc>);
    }
}
