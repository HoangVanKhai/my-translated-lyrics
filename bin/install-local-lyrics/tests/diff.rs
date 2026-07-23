use lyrics_core::video_descriptor::{UNIFIED_COLLECTION, Visibility};
use pipe_trait::Pipe;
use pretty_assertions::assert_eq;
use std::fs::{OpenOptions, read_to_string, write as write_file};
use std::path::Path;
use std::time::{Duration, SystemTime};
use test_utils::{InstallLocalLyricsEnv, video_desc};
use text_block_macros::text_block_fnl;

const INSTALL_LOCAL_LYRICS: &str = env!("CARGO_BIN_EXE_install-local-lyrics");

/// Sets the modification time of a file to a fixed point relative to the
/// Unix epoch. Explicit times keep the source newer than the target so
/// the target counts as outdated regardless of the wall-clock order in
/// which the test wrote the files.
fn set_mtime(path: &Path, seconds_since_epoch: u64) {
    let time = SystemTime::UNIX_EPOCH + Duration::from_secs(seconds_since_epoch);
    OpenOptions::new()
        .write(true)
        .open(path)
        .unwrap()
        .set_modified(time)
        .unwrap();
}

#[test]
fn diff_conflicts_with_execute() {
    let env = InstallLocalLyricsEnv::prepare(INSTALL_LOCAL_LYRICS);

    let output = env.run_allow_failure(["--diff", "--execute"]);

    assert!(
        !output.status.success(),
        "expected install-local-lyrics to reject --diff together with --execute",
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("the argument '--diff' cannot be used with '--execute'"),
        "expected a conflict message naming both flags, got:\n{stderr}",
    );
}

#[test]
fn renders_diff_of_outdated_subtitles() {
    let env = InstallLocalLyricsEnv::prepare(INSTALL_LOCAL_LYRICS);
    let collection_name = "Feng Ling Yu Xiu";
    let video_title = "【示例表演者】《示例歌曲》Example Song [ExampleID]";
    let source_content = text_block_fnl! {
        "line one"
        "line two changed"
        "line three"
    };
    let target_content = text_block_fnl! {
        "line one"
        "line two"
        "line three"
    };

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
    set_mtime(&separated, 1_000_000);
    set_mtime(&unified, 1_000_000);
    set_mtime(&source_file, 2_000_000);

    let output = env.run(["--diff"]);

    // The diff is rendered on standard output while the dry-run report
    // stays on standard error.
    let stdout = output.stdout.pipe_as_ref(str::from_utf8).unwrap();
    for target in [&separated, &unified] {
        assert!(
            stdout.contains(&format!("--- {target:?} (current)")),
            "missing current-file header for {target:?} in:\n{stdout}",
        );
    }
    assert!(
        stdout.contains(&format!("+++ {source_file:?} (new)")),
        "missing new-file header for {source_file:?} in:\n{stdout}",
    );
    assert!(
        stdout.contains(" line one\n-line two\n+line two changed\n line three\n"),
        "unexpected diff body in:\n{stdout}",
    );

    // A dry run leaves the outdated files on disk untouched.
    assert_eq!(read_to_string(&separated).unwrap(), target_content);
    assert_eq!(read_to_string(&unified).unwrap(), target_content);
}
