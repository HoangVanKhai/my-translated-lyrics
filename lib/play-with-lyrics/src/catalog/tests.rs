use crate::catalog::load;
use lyrics_core::video_descriptor::{VIDEO_CONFIG_FILE_NAME, Visibility};
use pretty_assertions::assert_eq;
use std::fs::{create_dir_all, write as write_file};
use std::path::Path;
use test_utils::{SEPARATED_COLLECTION, Temp, collection_name, video_desc, video_title};

/// Writes a `video.toml` for `video_title` in its own subdirectory of `source`.
fn add_video(source: &Path, dir_name: &str, title: String) {
    let video_dir = source.join(dir_name);
    create_dir_all(&video_dir).unwrap();
    let descriptor = video_desc(
        collection_name(SEPARATED_COLLECTION),
        video_title(title),
        Visibility::Visible,
    );
    let contents = toml::to_string(&descriptor).unwrap();
    write_file(video_dir.join(VIDEO_CONFIG_FILE_NAME), contents).unwrap();
}

/// [`load`] reads every descriptor and returns the videos ordered by title. The
/// descriptors here carry no English title, so the order falls back to the raw
/// video title.
#[test]
fn load_returns_videos_sorted_by_title() {
    let source = Temp::new_dir();
    add_video(&source, "bravo", "Bravo [id]".to_owned());
    add_video(&source, "alpha", "Alpha [id]".to_owned());

    let videos = load(&source);

    let titles: Vec<&str> = videos
        .iter()
        .map(|video| video.desc.video_title.as_ref())
        .collect();
    assert_eq!(titles, vec!["Alpha [id]", "Bravo [id]"]);
}
