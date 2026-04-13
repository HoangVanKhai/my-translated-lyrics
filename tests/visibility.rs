pub mod _utils;
pub use _utils::*;

use my_translated_lyrics::video_descriptor::{UNIFIED_COLLECTION, Visibility};
use pipe_trait::Pipe;
use pretty_assertions::assert_eq;
use std::fs::{read_to_string, write as write_file};

#[test]
fn hidden_visibility_causes_removal() {
    let workspace = Workspace::new();
    let collection = "Feng Ling Yu Xiu";
    let video_title = "Song Whose Subtitles Are Hidden";
    let desc = video_desc(collection, video_title, Visibility::Hidden);

    let separated = workspace.target_path(collection, &format!("{video_title}.vi.srt"));
    let unified = workspace.target_path(UNIFIED_COLLECTION, &format!("{video_title}.vi.srt"));
    write_file(&separated, "old content").unwrap();
    write_file(&unified, "old content").unwrap();

    workspace.add_video(
        "SongWhoseSubtitlesAreHidden",
        &desc,
        &[("lyrics.vi.srt", "new content that should not be installed")],
    );

    workspace.run(["--execute"]);

    assert!(
        !separated.exists(),
        "hidden song's separated file should be removed",
    );
    assert!(
        !unified.exists(),
        "hidden song's unified file should be removed",
    );
}

#[test]
fn manual_visibility_preserves_existing_files() {
    let workspace = Workspace::new();
    let collection = "Feng Ling Yu Xiu";
    let video_title = "Song Whose Subtitles Are Manually Managed";
    let desc = video_desc(collection, video_title, Visibility::Manual);
    let manual_content = "manually edited content";

    let separated = workspace.target_path(collection, &format!("{video_title}.vi.srt"));
    let unified = workspace.target_path(UNIFIED_COLLECTION, &format!("{video_title}.vi.srt"));
    write_file(&separated, manual_content).unwrap();
    write_file(&unified, manual_content).unwrap();

    workspace.add_video(
        "SongWhoseSubtitlesAreManuallyManaged",
        &desc,
        &[("lyrics.vi.srt", "source content that should not overwrite")],
    );

    workspace.run(["--execute"]);

    assert_eq!(separated.pipe_ref(read_to_string).unwrap(), manual_content);
    assert_eq!(unified.pipe_ref(read_to_string).unwrap(), manual_content);
}
