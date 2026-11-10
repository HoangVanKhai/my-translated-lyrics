use itertools::Itertools;
use lyrics_core::line_markers_descriptor::{LINE_MARKERS_CONFIG_FILE_NAME, LineMarkersDesc};
use pipe_trait::Pipe;
use std::fs::{DirEntry, read_dir, read_to_string};

/// Every `sources/*/line-markers.toml` must parse as a valid [`LineMarkersDesc`].
#[test]
fn source_line_markers_descriptors() {
    let sources_dir = test_utils::workspace_dir().join("sources");
    assert!(
        sources_dir.is_dir(),
        "expected sources directory to exist: {}",
        sources_dir.display(),
    );

    let entries = sources_dir
        .as_path()
        .pipe(read_dir)
        .unwrap()
        .map(Result::<DirEntry, _>::unwrap)
        .sorted_by_key(DirEntry::file_name);

    // Each song may omit the file, so the walk skips what it does not
    // find. Count what it did parse: without this the test reports
    // success over an empty `sources/`, or over a
    // `LINE_MARKERS_CONFIG_FILE_NAME` that matches nothing on disk.
    let mut parsed = 0_usize;

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
        parsed += 1;
    }

    assert!(
        parsed > 0,
        "no {LINE_MARKERS_CONFIG_FILE_NAME} found under {}",
        sources_dir.display(),
    );
}
