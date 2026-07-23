use lyrics_core::video_descriptor::{UNIFIED_COLLECTION, Visibility};
use pipe_trait::Pipe;
use pretty_assertions::assert_eq;
use std::fs::{read_to_string, write as write_file};
use test_utils::{InstallLocalLyricsEnv, expected_stderr, video_desc};

const INSTALL_LOCAL_LYRICS: &str = env!("CARGO_BIN_EXE_install-local-lyrics");

#[test]
fn hidden_visibility_causes_removal() {
    let env = InstallLocalLyricsEnv::prepare(INSTALL_LOCAL_LYRICS);
    let collection_name = "Feng Ling Yu Xiu";
    let video_title = "【示例表演者 | 日本語タグ】《示例歌曲名》 [ExampleID]";
    let desc = video_desc(
        collection_name.to_owned(),
        video_title.to_owned(),
        Visibility::Hidden,
    );

    let separated = env.target_path(collection_name, &format!("{video_title}.vi.srt"));
    let unified = env.target_path(UNIFIED_COLLECTION, &format!("{video_title}.vi.srt"));
    write_file(&separated, "old content").unwrap();
    write_file(&unified, "old content").unwrap();

    env.add_source_entry(
        "SongWhoseSubtitlesAreHidden",
        &desc,
        &[("lyrics.vi.srt", "new content that should not be installed")],
    );

    let output = env.run(["--execute"]);
    assert!(!separated.exists());
    assert!(!unified.exists());
    assert_eq!(
        output.stderr.pipe_as_ref(str::from_utf8).unwrap(),
        expected_stderr(2, &[separated, unified], &[], &[], &[], false),
    );
}

#[test]
fn dry_run_does_not_remove_hidden_files() {
    let env = InstallLocalLyricsEnv::prepare(INSTALL_LOCAL_LYRICS);
    let collection_name = "Feng Ling Yu Xiu";
    let video_title = "【示例表演者 | 日本語タグ】《示例歌曲名》 [ExampleID]";
    let desc = video_desc(
        collection_name.to_owned(),
        video_title.to_owned(),
        Visibility::Hidden,
    );

    let separated = env.target_path(collection_name, &format!("{video_title}.vi.srt"));
    let unified = env.target_path(UNIFIED_COLLECTION, &format!("{video_title}.vi.srt"));
    write_file(&separated, "old content").unwrap();
    write_file(&unified, "old content").unwrap();

    env.add_source_entry(
        "SongWhoseSubtitlesAreHidden",
        &desc,
        &[("lyrics.vi.srt", "new content that should not be installed")],
    );

    let output = env.run([]);
    assert!(separated.exists());
    assert!(unified.exists());
    assert_eq!(
        output.stderr.pipe_as_ref(str::from_utf8).unwrap(),
        expected_stderr(2, &[separated, unified], &[], &[], &[], true),
    );
}

#[test]
fn manual_visibility_preserves_existing_files() {
    let env = InstallLocalLyricsEnv::prepare(INSTALL_LOCAL_LYRICS);
    let collection_name = "Feng Ling Yu Xiu";
    let video_title =
        "【FULL ver.】Example Performer 示例表演者 - Example Song 示例歌曲【示例标签】";
    let desc = video_desc(
        collection_name.to_owned(),
        video_title.to_owned(),
        Visibility::Manual,
    );
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

    let output = env.run(["--execute"]);
    assert_eq!(separated.pipe_ref(read_to_string).unwrap(), manual_content);
    assert_eq!(unified.pipe_ref(read_to_string).unwrap(), manual_content);
    assert_eq!(
        output.stderr.pipe_as_ref(str::from_utf8).unwrap(),
        expected_stderr(2, &[], &[], &[], &[], false),
    );
}

#[test]
fn dry_run_manual_visibility_preserves_existing_files() {
    let env = InstallLocalLyricsEnv::prepare(INSTALL_LOCAL_LYRICS);
    let collection_name = "Feng Ling Yu Xiu";
    let video_title =
        "【FULL ver.】Example Performer 示例表演者 - Example Song 示例歌曲【示例标签】";
    let desc = video_desc(
        collection_name.to_owned(),
        video_title.to_owned(),
        Visibility::Manual,
    );
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

    let output = env.run([]);
    assert_eq!(separated.pipe_ref(read_to_string).unwrap(), manual_content);
    assert_eq!(unified.pipe_ref(read_to_string).unwrap(), manual_content);
    assert_eq!(
        output.stderr.pipe_as_ref(str::from_utf8).unwrap(),
        expected_stderr(2, &[], &[], &[], &[], true),
    );
}
