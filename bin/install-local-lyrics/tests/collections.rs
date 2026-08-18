use lyrics_core::collections_descriptor::COLLECTIONS_CONFIG_FILE_NAME;
use lyrics_core::video_descriptor::Visibility;
use pipe_trait::Pipe;
use pretty_assertions::assert_eq;
use std::fs::{create_dir, remove_file, write as write_file};
use test_utils::{
    InstallLocalLyricsEnv, OTHER_SEPARATED_COLLECTION, SEPARATED_COLLECTION, UNIFIED_COLLECTION,
    video_desc,
};
use text_block_macros::text_block_fnl;

const INSTALL_LOCAL_LYRICS: &str = env!("CARGO_BIN_EXE_install-local-lyrics");

/// Each video is installed into the separated collection its descriptor
/// names, whichever of the declared ones that is, and into the unified
/// collection the manifest declares.
#[test]
fn installs_into_every_declared_collection() {
    let env = InstallLocalLyricsEnv::prepare(INSTALL_LOCAL_LYRICS);
    let video_title = "【示例表演者】《示例歌曲》Example Song [ExampleID]";
    let other_title = "【示例表演者】《示例歌曲》Other Example Song [OtherID]";
    env.add_source_entry(
        "ExampleSong",
        &video_desc(
            SEPARATED_COLLECTION.to_owned(),
            video_title.to_owned(),
            Visibility::default(),
        ),
        &[("lyrics.vi.srt", "line one\n")],
    );
    env.add_source_entry(
        "OtherExampleSong",
        &video_desc(
            OTHER_SEPARATED_COLLECTION.to_owned(),
            other_title.to_owned(),
            Visibility::default(),
        ),
        &[("lyrics.vi.srt", "line two\n")],
    );

    env.run(["--execute"]);

    assert_eq!(
        env.target_subtitle_files(),
        [
            format!("{OTHER_SEPARATED_COLLECTION}/{other_title}.vi.srt"),
            format!("{SEPARATED_COLLECTION}/{video_title}.vi.srt"),
            format!("{UNIFIED_COLLECTION}/{video_title}.vi.srt"),
            format!("{UNIFIED_COLLECTION}/{other_title}.vi.srt"),
        ],
    );
}

/// A manifest may declare more than one unified collection, and every
/// video is then installed into each of them alongside its separated
/// collection.
#[test]
fn installs_into_every_unified_collection() {
    let env = InstallLocalLyricsEnv::prepare(INSTALL_LOCAL_LYRICS);
    let video_title = "【示例表演者】《示例歌曲》Example Song [ExampleID]";
    let other_unified = "Another Example Unified Collection";
    let manifest = format!(
        "unified = [{UNIFIED_COLLECTION:?}, {other_unified:?}]\nseparated = [{SEPARATED_COLLECTION:?}]\n",
    );
    write_file(env.source.join(COLLECTIONS_CONFIG_FILE_NAME), manifest).unwrap();
    create_dir(env.target.join(other_unified)).unwrap();
    env.add_source_entry(
        "ExampleSong",
        &video_desc(
            SEPARATED_COLLECTION.to_owned(),
            video_title.to_owned(),
            Visibility::default(),
        ),
        &[("lyrics.vi.srt", "line one\n")],
    );

    env.run(["--execute"]);

    for collection in [SEPARATED_COLLECTION, UNIFIED_COLLECTION, other_unified] {
        let installed = env.target_path(collection, &format!("{video_title}.vi.srt"));
        assert!(installed.is_file(), "{installed:?} was not installed");
    }
}

/// A descriptor that names a collection the manifest does not declare is
/// a typo, and the run fails before any file is touched.
#[test]
fn rejects_a_descriptor_naming_an_undeclared_collection() {
    let env = InstallLocalLyricsEnv::prepare(INSTALL_LOCAL_LYRICS);
    let video_title = "【示例表演者】《示例歌曲》Example Song [ExampleID]";
    let desc = video_desc(
        "Undeclared Example Collection".to_owned(),
        video_title.to_owned(),
        Visibility::default(),
    );
    env.add_source_entry("ExampleSong", &desc, &[("lyrics.vi.srt", "line one\n")]);

    let output = env.run_unchecked(["--execute"]);

    assert!(
        !output.status.success(),
        "expected an undeclared collection to fail the run",
    );
    let stderr = output.stderr.pipe_as_ref(str::from_utf8).unwrap();
    assert!(
        stderr.contains(r#"unknown collection: "Undeclared Example Collection""#),
        "expected the undeclared collection to be named, got:\n{stderr}",
    );
    assert_eq!(env.target_subtitle_files(), [] as [String; 0]);
}

/// The manifest is required: without it there is no declared set of
/// collections to install into.
#[test]
fn rejects_a_missing_manifest() {
    let env = InstallLocalLyricsEnv::prepare(INSTALL_LOCAL_LYRICS);
    env.source
        .join(COLLECTIONS_CONFIG_FILE_NAME)
        .pipe(remove_file)
        .unwrap();

    let output = env.run_unchecked(["--execute"]);

    assert!(
        !output.status.success(),
        "expected a missing manifest to fail the run",
    );
    let stderr = output.stderr.pipe_as_ref(str::from_utf8).unwrap();
    assert!(
        stderr.contains(COLLECTIONS_CONFIG_FILE_NAME),
        "expected the missing manifest to be named, got:\n{stderr}",
    );
}

/// A collection name is a path relative to the library root, so a name
/// that would reach outside that root is rejected as the manifest is
/// parsed.
#[test]
fn rejects_a_manifest_naming_a_collection_outside_the_library() {
    let env = InstallLocalLyricsEnv::prepare(INSTALL_LOCAL_LYRICS);
    let manifest = text_block_fnl! {
        r#"unified = ["Example Unified Collection"]"#
        r#"separated = ["../Escaping Example Collection"]"#
    };
    write_file(env.source.join(COLLECTIONS_CONFIG_FILE_NAME), manifest).unwrap();

    let output = env.run_unchecked(["--execute"]);

    assert!(
        !output.status.success(),
        "expected an escaping collection name to fail the run",
    );
    let stderr = output.stderr.pipe_as_ref(str::from_utf8).unwrap();
    assert!(
        stderr.contains(r#"collection name must not contain a "." or ".." path component"#),
        "expected the shape of the collection name to be reported, got:\n{stderr}",
    );
}
