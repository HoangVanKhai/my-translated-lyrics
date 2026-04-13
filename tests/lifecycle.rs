pub mod _utils;
pub use _utils::*;

use my_translated_lyrics::video_descriptor::{UNIFIED_COLLECTION, Visibility};
use pretty_assertions::assert_eq;
use std::fs::{remove_file, write as write_file};
use text_block_macros::text_block_fnl;

#[test]
fn installs_subtitles_to_separated_and_unified_collections() {
    let workspace = Workspace::new();
    let collection = "Feng Ling Yu Xiu";
    let video_title = "Example Song";
    let desc = video_desc(collection, video_title, Visibility::default());
    let srt_content = text_block_fnl! {
        "1"
        "00:00:01,000 --> 00:00:02,000"
        "Hello"
    };
    let vtt_content = text_block_fnl! {
        "WEBVTT"
        ""
        "00:00:01.000 --> 00:00:02.000"
        "Hello"
    };

    workspace.add_video(
        "ExampleSong",
        &desc,
        &[
            ("lyrics.vi.srt", srt_content),
            ("lyrics.zh.vtt", vtt_content),
        ],
    );

    workspace.run(["--execute"]);

    let expected = vec![
        format!("{collection}/{video_title}.vi.srt"),
        format!("{collection}/{video_title}.zh.vtt"),
        format!("{UNIFIED_COLLECTION}/{video_title}.vi.srt"),
        format!("{UNIFIED_COLLECTION}/{video_title}.zh.vtt"),
    ];
    assert_eq!(workspace.target_subtitle_files(), expected);

    assert_eq!(
        workspace.read_target(collection, &format!("{video_title}.vi.srt")),
        srt_content,
    );
    assert_eq!(
        workspace.read_target(collection, &format!("{video_title}.zh.vtt")),
        vtt_content,
    );
    assert_eq!(
        workspace.read_target(UNIFIED_COLLECTION, &format!("{video_title}.vi.srt")),
        srt_content,
    );
    assert_eq!(
        workspace.read_target(UNIFIED_COLLECTION, &format!("{video_title}.zh.vtt")),
        vtt_content,
    );
}

#[test]
fn skips_up_to_date_files() {
    let workspace = Workspace::new();
    let desc = video_desc("Feng Ling Yu Xiu", "Example Song", Visibility::default());
    workspace.add_video(
        "ExampleSong",
        &desc,
        &[(
            "lyrics.vi.srt",
            text_block_fnl! {
                "1"
                "00:00:01,000 --> 00:00:02,000"
                "Hello"
            },
        )],
    );

    workspace.run(["--execute"]);

    let output = workspace.run(["--execute"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("0 files would be removed from the target location"),
        "expected 0 removals:\n{stderr}",
    );
    assert!(
        stderr.contains("0 files would be added to the target location"),
        "expected 0 additions:\n{stderr}",
    );
    assert!(
        stderr.contains("0 files in the target location would be updated"),
        "expected 0 updates:\n{stderr}",
    );
}

#[test]
fn updates_modified_source_files() {
    let workspace = Workspace::new();
    let collection = "Feng Ling Yu Xiu";
    let video_title = "Song Whose Subtitles Get Updated";
    let desc = video_desc(collection, video_title, Visibility::default());
    let original = text_block_fnl! {
        "1"
        "00:00:01,000 --> 00:00:02,000"
        "Original"
    };
    let updated = text_block_fnl! {
        "1"
        "00:00:01,000 --> 00:00:02,000"
        "Updated"
    };

    workspace.add_video(
        "SongWhoseSubtitlesGetUpdated",
        &desc,
        &[("lyrics.vi.srt", original)],
    );
    workspace.run(["--execute"]);

    // Break the hardlink by removing and recreating the source file
    let source_file = workspace
        .source
        .join("SongWhoseSubtitlesGetUpdated")
        .join("lyrics.vi.srt");
    remove_file(&source_file).unwrap();
    write_file(&source_file, updated).unwrap();

    workspace.run(["--execute"]);

    assert_eq!(
        workspace.read_target(collection, &format!("{video_title}.vi.srt")),
        updated,
    );
    assert_eq!(
        workspace.read_target(UNIFIED_COLLECTION, &format!("{video_title}.vi.srt")),
        updated,
    );
}

#[test]
fn removes_orphaned_target_files() {
    let workspace = Workspace::new();
    let collection = "Feng Ling Yu Xiu";

    let orphaned = workspace.target_path(collection, "Orphaned.vi.srt");
    write_file(&orphaned, "orphaned content").unwrap();

    workspace.run(["--execute"]);

    assert!(
        !orphaned.exists(),
        "orphaned file should be removed from target",
    );
}
