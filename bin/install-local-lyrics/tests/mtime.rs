use lyrics_core::video_descriptor::{UNIFIED_COLLECTION, Visibility};
use pretty_assertions::assert_eq;
use std::fs::{OpenOptions, read_to_string, write as write_file};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use test_utils::{InstallLocalLyricsEnv, expected_stderr, video_desc};

const INSTALL_LOCAL_LYRICS: &str = env!("CARGO_BIN_EXE_install-local-lyrics");

/// Sets the modification time of a file to a fixed point relative to the
/// Unix epoch. Explicit times keep the comparison between source and
/// target deterministic instead of relying on wall-clock ordering, whose
/// resolution varies between filesystems.
fn set_mtime(path: &Path, seconds_since_epoch: u64) {
    let time = SystemTime::UNIX_EPOCH + Duration::from_secs(seconds_since_epoch);
    OpenOptions::new()
        .write(true)
        .open(path)
        .unwrap()
        .set_modified(time)
        .unwrap();
}

/// Prepares an environment where the source and both target files exist
/// with differing content, and returns the paths to the source file, the
/// separated target file, and the unified target file. The caller then
/// sets modification times to exercise the newer-than comparison.
fn prepare_conflicting_files(
    env: &InstallLocalLyricsEnv,
    collection_name: &str,
    video_title: &str,
    source_content: &str,
    target_content: &str,
) -> (PathBuf, PathBuf, PathBuf) {
    let desc = video_desc(
        collection_name.to_owned(),
        video_title.to_owned(),
        Visibility::default(),
    );
    env.add_source_entry("ExampleSong", &desc, &[("lyrics.vi.srt", source_content)]);

    let separated = env.target_path(collection_name, &format!("{video_title}.vi.srt"));
    let unified = env.target_path(UNIFIED_COLLECTION, &format!("{video_title}.vi.srt"));
    write_file(&separated, target_content).unwrap();
    write_file(&unified, target_content).unwrap();

    let source_file = env.source.join("ExampleSong").join("lyrics.vi.srt");
    (source_file, separated, unified)
}

#[test]
fn keeps_target_files_newer_than_source() {
    let env = InstallLocalLyricsEnv::prepare(INSTALL_LOCAL_LYRICS);
    let collection_name = "Feng Ling Yu Xiu";
    let video_title = "【示例表演者】《示例歌曲》Example Song [ExampleID]";
    let source_content = "source content";
    let target_content = "newer target content";

    let (source_file, separated, unified) = prepare_conflicting_files(
        &env,
        collection_name,
        video_title,
        source_content,
        target_content,
    );
    set_mtime(&source_file, 1_000_000);
    set_mtime(&separated, 2_000_000);
    set_mtime(&unified, 2_000_000);

    let output = env.run(["--execute"]);

    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        expected_stderr(
            2,
            &[],
            &[],
            &[],
            &[
                (source_file.clone(), separated.clone()),
                (source_file, unified.clone()),
            ],
            false,
        ),
    );
    // The newer target files are left untouched.
    assert_eq!(read_to_string(&separated).unwrap(), target_content);
    assert_eq!(read_to_string(&unified).unwrap(), target_content);
}

#[test]
fn dry_run_keeps_target_files_newer_than_source() {
    let env = InstallLocalLyricsEnv::prepare(INSTALL_LOCAL_LYRICS);
    let collection_name = "Feng Ling Yu Xiu";
    let video_title = "【示例表演者】《示例歌曲》Example Song [ExampleID]";
    let source_content = "source content";
    let target_content = "newer target content";

    let (source_file, separated, unified) = prepare_conflicting_files(
        &env,
        collection_name,
        video_title,
        source_content,
        target_content,
    );
    set_mtime(&source_file, 1_000_000);
    set_mtime(&separated, 2_000_000);
    set_mtime(&unified, 2_000_000);

    let output = env.run([]);

    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        expected_stderr(
            2,
            &[],
            &[],
            &[],
            &[
                (source_file.clone(), separated.clone()),
                (source_file, unified.clone()),
            ],
            true,
        ),
    );
    assert_eq!(read_to_string(&separated).unwrap(), target_content);
    assert_eq!(read_to_string(&unified).unwrap(), target_content);
}

#[test]
fn force_overwrites_target_files_newer_than_source() {
    let env = InstallLocalLyricsEnv::prepare(INSTALL_LOCAL_LYRICS);
    let collection_name = "Feng Ling Yu Xiu";
    let video_title = "【示例表演者】《示例歌曲》Example Song [ExampleID]";
    let source_content = "source content";
    let target_content = "newer target content";

    let (source_file, separated, unified) = prepare_conflicting_files(
        &env,
        collection_name,
        video_title,
        source_content,
        target_content,
    );
    set_mtime(&source_file, 1_000_000);
    set_mtime(&separated, 2_000_000);
    set_mtime(&unified, 2_000_000);

    let output = env.run(["--execute", "--force"]);

    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        expected_stderr(
            2,
            &[],
            &[],
            &[
                (source_file.clone(), separated.clone()),
                (source_file, unified.clone()),
            ],
            &[],
            false,
        ),
    );
    // With --force, the newer target files are overwritten by the source.
    assert_eq!(read_to_string(&separated).unwrap(), source_content);
    assert_eq!(read_to_string(&unified).unwrap(), source_content);
}

#[test]
fn updates_target_files_older_than_source() {
    let env = InstallLocalLyricsEnv::prepare(INSTALL_LOCAL_LYRICS);
    let collection_name = "Feng Ling Yu Xiu";
    let video_title = "【示例表演者】《示例歌曲》Example Song [ExampleID]";
    let source_content = "newer source content";
    let target_content = "older target content";

    let (source_file, separated, unified) = prepare_conflicting_files(
        &env,
        collection_name,
        video_title,
        source_content,
        target_content,
    );
    set_mtime(&separated, 1_000_000);
    set_mtime(&unified, 1_000_000);
    set_mtime(&source_file, 2_000_000);

    let output = env.run(["--execute"]);

    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        expected_stderr(
            2,
            &[],
            &[],
            &[
                (source_file.clone(), separated.clone()),
                (source_file, unified.clone()),
            ],
            &[],
            false,
        ),
    );
    // A target older than its source is updated as before.
    assert_eq!(read_to_string(&separated).unwrap(), source_content);
    assert_eq!(read_to_string(&unified).unwrap(), source_content);
}
