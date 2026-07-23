use command_extra::CommandExtra;
use lyrics_core::video_descriptor::{UNIFIED_COLLECTION, Visibility};
use pretty_assertions::assert_eq;
use std::fs::{OpenOptions, read_to_string, remove_file, write as write_file};
use std::path::Path;
use std::process::Command;
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

/// Runs a `git` subcommand inside `dir` and asserts it succeeds. The
/// global and system configuration files are redirected to `/dev/null`
/// so the test does not depend on the developer's git settings.
fn run_git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .with_env("GIT_CONFIG_GLOBAL", "/dev/null")
        .with_env("GIT_CONFIG_SYSTEM", "/dev/null")
        .with_arg("-C")
        .with_arg(dir)
        .with_args(args)
        .status()
        .unwrap();
    assert!(status.success(), "git {args:?} failed");
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
fn renders_git_apply_compatible_diff_of_outdated_subtitles() {
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
    let patch = output.stdout;
    let patch_text = str::from_utf8(&patch).unwrap();

    // A single patch on standard output covers every outdated target
    // file, one `git diff` section per target-relative path.
    let separated_rel = format!("{collection_name}/{video_title}.vi.srt");
    let unified_rel = format!("{UNIFIED_COLLECTION}/{video_title}.vi.srt");
    for rel in [&separated_rel, &unified_rel] {
        assert!(
            patch_text.contains(&format!("diff --git a/{rel} b/{rel}")),
            "patch is missing a section for {rel}:\n{patch_text}",
        );
    }
    // Hunk headers, rather than the whole file, so context stays bounded.
    assert!(
        patch_text.contains("@@"),
        "patch has no hunk header:\n{patch_text}",
    );
    assert!(
        patch_text.contains("-line two\n+line two changed\n"),
        "patch does not show the changed line:\n{patch_text}",
    );

    // A dry run leaves the target files on disk untouched.
    assert_eq!(read_to_string(&separated).unwrap(), target_content);
    assert_eq!(read_to_string(&unified).unwrap(), target_content);

    // Treating the target directory as a git repository, the emitted
    // patch applies cleanly and turns each outdated file into its source.
    run_git(&env.target, &["init", "-q", "."]);
    let patch_file = env.target.join("outdated.patch");
    write_file(&patch_file, &patch).unwrap();
    run_git(&env.target, &["apply", "outdated.patch"]);
    remove_file(&patch_file).unwrap();
    assert_eq!(read_to_string(&separated).unwrap(), source_content);
    assert_eq!(read_to_string(&unified).unwrap(), source_content);
}
