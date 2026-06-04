use std::fs::{DirEntry, read_dir, read_to_string};
use std::path::Path;

use itertools::Itertools;
use pipe_trait::Pipe;
use translated_lyrics::video_descriptor::{VIDEO_CONFIG_FILE_NAME, VideoDesc};

/// Every `dist/*/video.toml` must parse as a valid [`VideoDesc`].
#[test]
fn dist_video_descriptors_are_valid() {
    let entries = env!("CARGO_MANIFEST_DIR")
        .pipe(Path::new)
        .join("dist")
        .pipe(read_dir)
        .unwrap()
        .map(Result::<DirEntry, _>::unwrap)
        .sorted_by_key(DirEntry::file_name);

    for entry in entries {
        let video_dir = entry.path();
        if !video_dir.is_dir() {
            continue;
        }

        eprintln!("CASE: {}", entry.file_name().display());
        video_dir
            .join(VIDEO_CONFIG_FILE_NAME)
            .pipe(read_to_string)
            .unwrap()
            .pipe_as_ref(toml::from_str::<VideoDesc>)
            .unwrap()
            .pipe(drop::<VideoDesc>);
    }
}
