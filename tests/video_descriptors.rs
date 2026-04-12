use itertools::Itertools;
use my_translated_lyrics::video_descriptor::{VIDEO_CONFIG_FILE_NAME, VideoDesc};
use pipe_trait::Pipe;
use std::fs;
use std::fs::DirEntry;
use std::path::Path;

/// Every `data/*/video.toml` must parse as a valid [`VideoDesc`].
#[test]
fn data_video_descriptors_are_valid() {
    let data_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("data");

    let entries = data_dir
        .pipe(fs::read_dir)
        .unwrap()
        .map(Result::unwrap)
        .sorted_by_key(DirEntry::file_name);

    for entry in entries {
        let video_dir = entry.path();
        if !video_dir.is_dir() {
            continue;
        }

        let toml_path = video_dir.join(VIDEO_CONFIG_FILE_NAME);
        let video_name = entry.file_name();
        let video_name = video_name.to_str().expect("path isn't valid UTF-8");

        eprintln!("CASE: {video_name}");
        let content = fs::read_to_string(&toml_path)
            .unwrap_or_else(|error| panic!("cannot read {toml_path:?}: {error}"));
        let _desc: VideoDesc = toml::from_str(&content)
            .unwrap_or_else(|error| panic!("cannot parse {toml_path:?}: {error}"));
    }
}
