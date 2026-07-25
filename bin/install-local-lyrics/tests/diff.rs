use lyrics_core::video_descriptor::UNIFIED_COLLECTION;
use pretty_assertions::assert_eq;
use std::fs::{
    metadata, read, read_dir, read_to_string, remove_file, set_permissions, write as write_file,
};
use std::os::unix::fs::PermissionsExt;
use test_utils::{InstallLocalLyricsEnv, Temp, prepare_outdated, run_git};
use text_block_macros::text_block_fnl;

const INSTALL_LOCAL_LYRICS: &str = env!("CARGO_BIN_EXE_install-local-lyrics");

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
fn dry_run_without_diff_flag_emits_no_stdout() {
    let env = InstallLocalLyricsEnv::prepare(INSTALL_LOCAL_LYRICS);
    let (separated, unified) = prepare_outdated(
        &env,
        "Feng Ling Yu Xiu",
        "【示例表演者】《示例歌曲》Example Song [ExampleID]",
        "new content\n",
        "old content\n",
    );

    let output = env.run([]);

    // The outdated files are reported on stderr, but a dry run without
    // --diff must not write anything to stdout.
    assert!(
        output.stdout.is_empty(),
        "a dry run without --diff must not write to stdout, got:\n{}",
        String::from_utf8_lossy(&output.stdout),
    );
    assert_eq!(read_to_string(&separated).unwrap(), "old content\n");
    assert_eq!(read_to_string(&unified).unwrap(), "old content\n");
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

    let (separated, unified) = prepare_outdated(
        &env,
        collection_name,
        video_title,
        source_content,
        target_content,
    );

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

#[test]
fn renders_git_apply_compatible_diff_for_binary_content() {
    let env = InstallLocalLyricsEnv::prepare(INSTALL_LOCAL_LYRICS);
    let collection_name = "Feng Ling Yu Xiu";
    let video_title = "【示例表演者】《示例歌曲》Example Song [ExampleID]";
    // A leading NUL byte makes git classify the content as binary. The
    // two lengths differ so the outdated check never reads the bytes as
    // UTF-8, which lets the binary target reach the diff.
    let source_content = "\0binary source content";
    let target_content = "\0binary target";
    let (separated, unified) = prepare_outdated(
        &env,
        collection_name,
        video_title,
        source_content,
        target_content,
    );

    let output = env.run(["--diff"]);
    let patch = output.stdout;

    // `--binary` yields an applicable binary patch rather than a lossy
    // "Binary files differ" line.
    assert!(
        String::from_utf8_lossy(&patch).contains("GIT binary patch"),
        "expected a binary patch:\n{}",
        String::from_utf8_lossy(&patch),
    );

    run_git(&env.target, &["init", "-q", "."]);
    let patch_file = env.target.join("outdated.patch");
    write_file(&patch_file, &patch).unwrap();
    run_git(&env.target, &["apply", "outdated.patch"]);
    remove_file(&patch_file).unwrap();
    let expected = source_content.as_bytes().to_vec();
    assert_eq!(read(&separated).unwrap(), expected);
    assert_eq!(read(&unified).unwrap(), expected);
}

#[test]
fn removes_the_temporary_repository_after_diff() {
    let env = InstallLocalLyricsEnv::prepare(INSTALL_LOCAL_LYRICS);
    let _targets = prepare_outdated(
        &env,
        "Feng Ling Yu Xiu",
        "【示例表演者】《示例歌曲》Example Song [ExampleID]",
        "new\n",
        "old\n",
    );

    // Point the binary's temporary directory (`std::env::temp_dir()` reads
    // `TMPDIR`) at a private, initially empty directory, so the leftover
    // check is not disturbed by other processes running in parallel.
    let temp = Temp::new_dir();
    let stdout = env.run_diff_with_env(&[("TMPDIR", temp.to_str().unwrap())]);

    // A diff was produced, so the throwaway repository was created.
    assert!(!stdout.is_empty(), "expected a diff to be produced");

    // Once the diff is done, no throwaway repository is left behind.
    let leftovers: Vec<_> = read_dir(&*temp)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .filter(|name| {
            name.to_string_lossy()
                .starts_with("install-local-lyrics-diff.")
        })
        .collect();
    assert!(
        leftovers.is_empty(),
        "the temporary diff repository was not cleaned up: {leftovers:?}",
    );
}

#[test]
fn diff_reports_content_changes_without_mode_changes() {
    let env = InstallLocalLyricsEnv::prepare(INSTALL_LOCAL_LYRICS);
    let collection_name = "Feng Ling Yu Xiu";
    let video_title = "【示例表演者】《示例歌曲》Example Song [ExampleID]";
    let (separated, unified) = prepare_outdated(
        &env,
        collection_name,
        video_title,
        "new content\n",
        "old content\n",
    );

    // Give the target files a non-default mode. The staged file keeps the
    // target's mode, so overwriting it by copying the source (which carries
    // the source's mode) or by removing and recreating it (which resets the
    // mode to the umask default) would introduce a mode change.
    for target in [&separated, &unified] {
        let mut permissions = metadata(target).unwrap().permissions();
        permissions.set_mode(0o755);
        set_permissions(target, permissions).unwrap();
    }

    let output = env.run(["--diff"]);
    let patch_text = str::from_utf8(&output.stdout).unwrap();

    // The patch reports the content change alone, never a mode change.
    assert!(
        !patch_text.contains("old mode") && !patch_text.contains("new mode"),
        "the patch contains a mode change:\n{patch_text}",
    );
    assert!(
        patch_text.contains("-old content\n+new content\n"),
        "the content change is missing:\n{patch_text}",
    );
}

/// A CRLF subtitle keeps its carriage returns in the emitted patch, so
/// `--diff` never silently rewrites line endings. A system-wide attributes
/// file could force such a normalization, but planting one needs privileges
/// a portable test lacks; `render_diff` neutralizes that channel.
#[test]
fn diff_preserves_crlf_line_endings() {
    let env = InstallLocalLyricsEnv::prepare(INSTALL_LOCAL_LYRICS);
    let collection_name = "Feng Ling Yu Xiu";
    let video_title = "【示例表演者】《示例歌曲》Example Song [ExampleID]";
    let source_content = "line one\r\nline two changed\r\nline three\r\n";
    let target_content = "line one\r\nline two\r\n";
    let (separated, unified) = prepare_outdated(
        &env,
        collection_name,
        video_title,
        source_content,
        target_content,
    );

    let patch = env.run(["--diff"]).stdout;

    let contains = |needle: &[u8]| patch.windows(needle.len()).any(|window| window == needle);
    assert!(
        contains(b"+line two changed\r\n"),
        "the added line lost its CRLF ending:\n{}",
        String::from_utf8_lossy(&patch),
    );
    assert!(
        contains(b"-line two\r\n"),
        "the removed line lost its CRLF ending:\n{}",
        String::from_utf8_lossy(&patch),
    );

    // A dry run leaves the target files untouched and still CRLF.
    assert_eq!(read_to_string(&separated).unwrap(), target_content);
    assert_eq!(read_to_string(&unified).unwrap(), target_content);
}
