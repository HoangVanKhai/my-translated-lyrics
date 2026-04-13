pub mod _utils;
pub use _utils::*;

use my_translated_lyrics::video_descriptor::{UNIFIED_COLLECTION, Visibility};
use pipe_trait::Pipe;
use pretty_assertions::assert_eq;
use std::fs::{read_to_string, write as write_file};

#[test]
fn hidden_visibility_causes_removal() {
    let env = InstallLocalLyricsEnv::prepare();
    let collection_name = "Feng Ling Yu Xiu";
    let video_title = "【示例表演者 | 日本語タグ】《示例歌曲名》 [ExampleID]";
    let desc = video_desc(collection_name, video_title, Visibility::Hidden);

    let separated = env.target_path(collection_name, &format!("{video_title}.vi.srt"));
    let unified = env.target_path(UNIFIED_COLLECTION, &format!("{video_title}.vi.srt"));
    write_file(&separated, "old content").unwrap();
    write_file(&unified, "old content").unwrap();

    env.add_source_entry(
        "SongWhoseSubtitlesAreHidden",
        &desc,
        &[("lyrics.vi.srt", "new content that should not be installed")],
    );

    env.run(["--execute"]);

    assert!(!separated.exists());
    assert!(!unified.exists());
}

#[test]
fn manual_visibility_preserves_existing_files() {
    let env = InstallLocalLyricsEnv::prepare();
    let collection_name = "Feng Ling Yu Xiu";
    let video_title =
        "【FULL ver.】Example Performer 示例表演者 - Example Song 示例歌曲【示例标签】";
    let desc = video_desc(collection_name, video_title, Visibility::Manual);
    let manual_content = "manually edited content";

    let separated = env.target_path(collection_name, &format!("{video_title}.vi.srt"));
    let unified = env.target_path(UNIFIED_COLLECTION, &format!("{video_title}.vi.srt"));
    write_file(&separated, manual_content).unwrap();
    write_file(&unified, manual_content).unwrap();

    env.add_source_entry(
        "SongWhoseSubtitlesAreManuallyManaged",
        &desc,
        &[("lyrics.vi.srt", "source content that should not overwrite")],
    );

    env.run(["--execute"]);

    assert_eq!(separated.pipe_ref(read_to_string).unwrap(), manual_content);
    assert_eq!(unified.pipe_ref(read_to_string).unwrap(), manual_content);
}
