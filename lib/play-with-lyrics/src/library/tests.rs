use crate::library::{VideoLookupError, available_subtitles, find_video_file, subtitle_path};
use crate::player::SubtitleFormat;
use lyrics_core::video_descriptor::Language;
use pretty_assertions::assert_eq;
use std::fs::write as write_file;
use std::path::Path;
use test_utils::Temp;

const TITLE: &str = "Some Title [id]";

fn touch(dir: &Path, file_name: &str) {
    write_file(dir.join(file_name), "").unwrap();
}

#[test]
fn lists_available_subtitles_sorted_and_deduplicated() {
    let dir = Temp::new_dir();
    touch(&dir, &format!("{TITLE}.vi.srt"));
    touch(&dir, &format!("{TITLE}.zh.srt"));
    touch(&dir, &format!("{TITLE}.vi.vtt"));
    // A video file and an unrelated file must be ignored.
    touch(&dir, &format!("{TITLE}.mkv"));
    touch(&dir, "unrelated.txt");

    let available = available_subtitles(&dir, TITLE);
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
    assert_eq!(available_subtitles(&missing, TITLE), Vec::new());
}

#[test]
fn finds_a_single_video_file() {
    let dir = Temp::new_dir();
    touch(&dir, &format!("{TITLE}.mkv"));
    touch(&dir, &format!("{TITLE}.vi.srt"));

    let found = find_video_file(&dir, TITLE).unwrap();
    assert_eq!(found, dir.join(format!("{TITLE}.mkv")));
}

#[test]
fn reports_a_missing_video_file() {
    let dir = Temp::new_dir();
    touch(&dir, &format!("{TITLE}.vi.srt"));

    let error = find_video_file(&dir, TITLE).unwrap_err();
    assert!(matches!(error, VideoLookupError::NotFound { .. }));
}

#[test]
fn a_missing_collection_directory_reports_no_video_file() {
    let dir = Temp::new_dir();
    let missing = dir.join("does-not-exist");

    let error = find_video_file(&missing, TITLE).unwrap_err();
    assert!(matches!(error, VideoLookupError::NotFound { .. }));
}

#[test]
fn reports_multiple_matching_video_files() {
    let dir = Temp::new_dir();
    touch(&dir, &format!("{TITLE}.mkv"));
    touch(&dir, &format!("{TITLE}.mp4"));

    let error = find_video_file(&dir, TITLE).unwrap_err();
    assert!(matches!(error, VideoLookupError::Multiple { .. }));
}

#[test]
fn a_title_that_is_a_prefix_of_another_is_not_matched() {
    let dir = Temp::new_dir();
    touch(&dir, &format!("{TITLE} Extended.mkv"));

    let error = find_video_file(&dir, TITLE).unwrap_err();
    assert!(matches!(error, VideoLookupError::NotFound { .. }));
}

#[test]
fn builds_the_subtitle_path() {
    let path = subtitle_path(
        Path::new("/library/Coll"),
        TITLE,
        Language::Vietnamese,
        SubtitleFormat::SubRip,
    );
    assert_eq!(path, Path::new("/library/Coll/Some Title [id].vi.srt"));
}
