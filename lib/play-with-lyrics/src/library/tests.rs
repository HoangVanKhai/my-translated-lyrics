use crate::library::{VideoLookupError, available_subtitles, find_video_file, subtitle_path};
use crate::player::SubtitleFormat;
use lyrics_core::video_descriptor::Language;
use pretty_assertions::assert_eq;
use std::fs::write as write_file;
use std::path::Path;
use test_utils::Temp;

fn touch(dir: &Path, file_name: &str) {
    write_file(dir.join(file_name), "").unwrap();
}

#[test]
fn lists_available_subtitles_sorted_and_deduplicated() {
    let title = "Some Title [id]";
    let dir = Temp::new_dir();
    touch(&dir, &format!("{title}.vi.srt"));
    touch(&dir, &format!("{title}.zh.srt"));
    touch(&dir, &format!("{title}.vi.vtt"));
    // A video file and an unrelated file must be ignored.
    touch(&dir, &format!("{title}.mkv"));
    touch(&dir, "unrelated.txt");

    let available = available_subtitles(&dir, title);
    assert_eq!(
        available,
        vec![
            (Language::Vietnamese, SubtitleFormat::SubRip),
            (Language::Vietnamese, SubtitleFormat::WebVtt),
            (Language::Chinese, SubtitleFormat::SubRip),
        ],
    );
}

#[test]
fn missing_collection_directory_has_no_subtitles() {
    let dir = Temp::new_dir();
    let missing = dir.join("does-not-exist");
    assert_eq!(available_subtitles(&missing, "Some Title [id]"), Vec::new());
}

#[test]
fn finds_a_single_video_file() {
    let title = "Some Title [id]";
    let dir = Temp::new_dir();
    touch(&dir, &format!("{title}.mkv"));
    touch(&dir, &format!("{title}.vi.srt"));

    let found = find_video_file(&dir, title).unwrap();
    assert_eq!(found, dir.join(format!("{title}.mkv")));
}

#[test]
fn reports_a_missing_video_file() {
    let title = "Some Title [id]";
    let dir = Temp::new_dir();
    touch(&dir, &format!("{title}.vi.srt"));

    let error = find_video_file(&dir, title).unwrap_err();
    assert!(matches!(error, VideoLookupError::NotFound { .. }));
}

#[test]
fn a_missing_collection_directory_reports_no_video_file() {
    let dir = Temp::new_dir();
    let missing = dir.join("does-not-exist");

    let error = find_video_file(&missing, "Some Title [id]").unwrap_err();
    assert!(matches!(error, VideoLookupError::NotFound { .. }));
}

#[test]
fn reports_multiple_matching_video_files() {
    let title = "Some Title [id]";
    let dir = Temp::new_dir();
    touch(&dir, &format!("{title}.mkv"));
    touch(&dir, &format!("{title}.mp4"));

    let error = find_video_file(&dir, title).unwrap_err();
    assert!(matches!(error, VideoLookupError::Multiple { .. }));
}

#[test]
fn a_title_that_is_a_prefix_of_another_is_not_matched() {
    let title = "Some Title [id]";
    let dir = Temp::new_dir();
    touch(&dir, &format!("{title} Extended.mkv"));

    let error = find_video_file(&dir, title).unwrap_err();
    assert!(matches!(error, VideoLookupError::NotFound { .. }));
}

#[test]
fn builds_the_subtitle_path() {
    let path = subtitle_path(
        Path::new("/library/Coll"),
        "Some Title [id]",
        Language::Vietnamese,
        SubtitleFormat::SubRip,
    );
    assert_eq!(path, Path::new("/library/Coll/Some Title [id].vi.srt"));
}
