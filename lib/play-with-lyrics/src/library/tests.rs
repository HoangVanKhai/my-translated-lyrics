use crate::library::{
    VideoLookupError, VideoLookupErrorKind, available_subtitles, find_video_file, subtitle_path,
};
use crate::player::SubtitleFormat;
use lyrics_core::video_descriptor::Language;
use pretty_assertions::assert_eq;
use std::fs::write as write_file;
use std::path::{Path, PathBuf};
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
    assert!(matches!(error.kind, VideoLookupErrorKind::NotFound));
}

#[test]
fn a_missing_collection_directory_reports_no_video_file() {
    let dir = Temp::new_dir();
    let missing = dir.join("does-not-exist");

    let error = find_video_file(&missing, "Some Title [id]").unwrap_err();
    assert!(matches!(error.kind, VideoLookupErrorKind::NotFound));
}

#[test]
fn reports_multiple_matching_video_files() {
    let title = "Some Title [id]";
    let dir = Temp::new_dir();
    touch(&dir, &format!("{title}.mkv"));
    touch(&dir, &format!("{title}.mp4"));

    let error = find_video_file(&dir, title).unwrap_err();
    assert!(matches!(error.kind, VideoLookupErrorKind::Multiple(_)));
}

#[test]
fn a_title_that_is_a_prefix_of_another_is_not_matched() {
    let title = "Some Title [id]";
    let dir = Temp::new_dir();
    touch(&dir, &format!("{title} Extended.mkv"));

    let error = find_video_file(&dir, title).unwrap_err();
    assert!(matches!(error.kind, VideoLookupErrorKind::NotFound));
}

#[test]
fn a_not_found_error_renders_the_lookup_context_before_its_kind() {
    let error = VideoLookupError {
        collection_dir: PathBuf::from("/library/Coll"),
        video_title: "Some Title [id]".to_string(),
        kind: VideoLookupErrorKind::NotFound,
    };
    assert_eq!(
        error.to_string(),
        r#"video lookup for "Some Title [id]" in "/library/Coll": no video file was found"#,
    );
}

#[test]
fn a_multiple_error_lists_its_matches_after_the_lookup_context() {
    let error = VideoLookupError {
        collection_dir: PathBuf::from("/library/Coll"),
        video_title: "Some Title [id]".to_string(),
        kind: VideoLookupErrorKind::Multiple(vec![
            PathBuf::from("/library/Coll/Some Title [id].mkv"),
            PathBuf::from("/library/Coll/Some Title [id].mp4"),
        ]),
    };
    assert_eq!(
        error.to_string(),
        r#"video lookup for "Some Title [id]" in "/library/Coll": multiple video files match: "/library/Coll/Some Title [id].mkv", "/library/Coll/Some Title [id].mp4""#,
    );
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
