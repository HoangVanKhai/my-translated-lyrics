pub mod _utils;
pub use _utils::*;

use my_translated_lyrics::video_descriptor::{UNIFIED_COLLECTION, Visibility};
use pretty_assertions::assert_eq;
use std::fs::{remove_file, write as write_file};
use text_block_macros::text_block_fnl;

#[test]
fn installs_subtitles_to_separated_and_unified_collections() {
    let env = InstallLocalLyricsEnv::prepare();
    let collection_name = "Feng Ling Yu Xiu";
    let video_title = "【示例表演者】《示例歌曲》Example Song [ExampleID]";
    let desc = video_desc(collection_name, video_title, Visibility::default());
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

    env.add_source_entry(
        "ExampleSong",
        &desc,
        &[
            ("lyrics.vi.srt", srt_content),
            ("lyrics.zh.vtt", vtt_content),
        ],
    );

    env.run(["--execute"]);

    let expected = vec![
        format!("{collection_name}/{video_title}.vi.srt"),
        format!("{collection_name}/{video_title}.zh.vtt"),
        format!("{UNIFIED_COLLECTION}/{video_title}.vi.srt"),
        format!("{UNIFIED_COLLECTION}/{video_title}.zh.vtt"),
    ];
    assert_eq!(env.target_subtitle_files(), expected);

    assert_eq!(
        env.read_target(collection_name, &format!("{video_title}.vi.srt")),
        srt_content,
    );
    assert_eq!(
        env.read_target(collection_name, &format!("{video_title}.zh.vtt")),
        vtt_content,
    );
    assert_eq!(
        env.read_target(UNIFIED_COLLECTION, &format!("{video_title}.vi.srt")),
        srt_content,
    );
    assert_eq!(
        env.read_target(UNIFIED_COLLECTION, &format!("{video_title}.zh.vtt")),
        vtt_content,
    );
}

#[test]
fn skips_up_to_date_files() {
    let env = InstallLocalLyricsEnv::prepare();
    let desc = video_desc(
        "Feng Ling Yu Xiu",
        "【示例表演者】《示例歌曲》Example Song [ExampleID]",
        Visibility::default(),
    );
    env.add_source_entry(
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

    env.run(["--execute"]);

    let output = env.run(["--execute"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("0 files would be removed from the target location"));
    assert!(stderr.contains("0 files would be added to the target location"));
    assert!(stderr.contains("0 files in the target location would be updated"));
}

#[test]
fn updates_modified_source_files() {
    let env = InstallLocalLyricsEnv::prepare();
    let collection_name = "Feng Ling Yu Xiu";
    let video_title = "【示例表演者】示例歌(Example Song)——“示例歌词”【示例标签】 [ExampleID]";
    let desc = video_desc(collection_name, video_title, Visibility::default());
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

    env.add_source_entry(
        "SongWhoseSubtitlesGetUpdated",
        &desc,
        &[("lyrics.vi.srt", original)],
    );
    env.run(["--execute"]);

    // Break the hardlink by removing and recreating the source file
    let source_file = env
        .source
        .join("SongWhoseSubtitlesGetUpdated")
        .join("lyrics.vi.srt");
    remove_file(&source_file).unwrap();
    write_file(&source_file, updated).unwrap();

    env.run(["--execute"]);

    assert_eq!(
        env.read_target(collection_name, &format!("{video_title}.vi.srt")),
        updated,
    );
    assert_eq!(
        env.read_target(UNIFIED_COLLECTION, &format!("{video_title}.vi.srt")),
        updated,
    );
}

#[test]
fn removes_orphaned_target_files() {
    let env = InstallLocalLyricsEnv::prepare();
    let collection_name = "Feng Ling Yu Xiu";

    let orphaned = env.target_path(collection_name, "Orphaned.vi.srt");
    write_file(&orphaned, "orphaned content").unwrap();

    env.run(["--execute"]);

    assert!(!orphaned.exists());
}
