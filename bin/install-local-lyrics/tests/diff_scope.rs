use lyrics_core::video_descriptor::{UNIFIED_COLLECTION, Visibility};
use pretty_assertions::assert_eq;
use std::fs::{read_to_string, remove_file, write as write_file};
use test_utils::{InstallLocalLyricsEnv, prepare_outdated, run_git, set_mtime, video_desc};

const INSTALL_LOCAL_LYRICS: &str = env!("CARGO_BIN_EXE_install-local-lyrics");

#[test]
fn diff_includes_targets_newer_than_source_only_with_force() {
    let env = InstallLocalLyricsEnv::prepare(INSTALL_LOCAL_LYRICS);
    let collection_name = "Feng Ling Yu Xiu";
    let video_title = "【示例表演者】《示例歌曲》Example Song [ExampleID]";
    let desc = video_desc(
        collection_name.to_owned(),
        video_title.to_owned(),
        Visibility::default(),
    );
    env.add_source_entry(
        "ExampleSong",
        &desc,
        &[("lyrics.vi.srt", "source content\n")],
    );

    let separated = env.target_path(collection_name, &format!("{video_title}.vi.srt"));
    let unified = env.target_path(UNIFIED_COLLECTION, &format!("{video_title}.vi.srt"));
    write_file(&separated, "target content\n").unwrap();
    write_file(&unified, "target content\n").unwrap();

    // The targets differ from the source and are newer than it.
    let source_file = env.source.join("ExampleSong").join("lyrics.vi.srt");
    set_mtime(&source_file, 1_000_000);
    set_mtime(&separated, 2_000_000);
    set_mtime(&unified, 2_000_000);

    // A newer target is kept by default, so nothing is diffed.
    let plain = env.run(["--diff"]);
    assert!(
        plain.stdout.is_empty(),
        "a newer target must not be diffed without --force, got:\n{}",
        String::from_utf8_lossy(&plain.stdout),
    );

    // With --force the newer target becomes an update and is diffed.
    let forced = env.run(["--diff", "--force"]);
    let patch_text = str::from_utf8(&forced.stdout).unwrap();
    let separated_rel = format!("{collection_name}/{video_title}.vi.srt");
    assert!(
        patch_text.contains(&format!("diff --git a/{separated_rel} b/{separated_rel}")),
        "the newer target is missing from the --force patch:\n{patch_text}",
    );
    assert!(
        patch_text.contains("-target content\n+source content\n"),
        "unexpected diff body:\n{patch_text}",
    );
}

#[test]
fn diff_excludes_newly_installed_files() {
    let env = InstallLocalLyricsEnv::prepare(INSTALL_LOCAL_LYRICS);
    let collection_name = "Feng Ling Yu Xiu";
    let video_title = "【示例表演者】《示例歌曲》Example Song [ExampleID]";
    let desc = video_desc(
        collection_name.to_owned(),
        video_title.to_owned(),
        Visibility::default(),
    );
    // A source with no existing target files: these are new installs.
    env.add_source_entry(
        "ExampleSong",
        &desc,
        &[("lyrics.vi.srt", "brand new content\n")],
    );

    let output = env.run(["--diff"]);

    // The plan reports the files as additions on stderr...
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("2 files would be added"),
        "expected the files to be new installs:\n{stderr}",
    );
    // ...but a new install is not an outdated update, so it is not diffed.
    assert!(
        output.stdout.is_empty(),
        "a newly installed file must not appear in the diff, got:\n{}",
        String::from_utf8_lossy(&output.stdout),
    );
}

#[test]
fn diff_excludes_removals_by_default() {
    let env = InstallLocalLyricsEnv::prepare(INSTALL_LOCAL_LYRICS);
    let collection_name = "Feng Ling Yu Xiu";
    let (separated, unified) = prepare_outdated(
        &env,
        collection_name,
        "【示例表演者】《示例歌曲》Example Song [ExampleID]",
        "new content\n",
        "old content\n",
    );
    // A target file with no matching source would be removed by the sync.
    let orphan = env.target_path(
        collection_name,
        "【示例表演者】《示例歌曲》Orphan [RemovedID].vi.srt",
    );
    write_file(&orphan, "to be removed\n").unwrap();

    let patch = env.run(["--diff"]).stdout;
    let patch_text = str::from_utf8(&patch).unwrap();

    // The outdated update is shown, but the removal is not, by default.
    assert!(
        patch_text.contains("-old content\n+new content\n"),
        "the update is missing from the patch:\n{patch_text}",
    );
    assert!(
        !patch_text.contains("Orphan"),
        "a removed file must not appear in the diff by default:\n{patch_text}",
    );

    // The dry run changes nothing on disk.
    assert_eq!(read_to_string(&separated).unwrap(), "old content\n");
    assert_eq!(read_to_string(&unified).unwrap(), "old content\n");
    assert!(orphan.exists(), "the dry run must not delete the removal");
}

#[test]
fn include_removals_shows_removed_files_as_deletions() {
    let env = InstallLocalLyricsEnv::prepare(INSTALL_LOCAL_LYRICS);
    let collection_name = "Feng Ling Yu Xiu";
    // A target file with no matching source: the sync would remove it.
    let removed_rel =
        format!("{collection_name}/【示例表演者】《示例歌曲》Removed [RemovedID].vi.srt");
    let removed = env.target.join(&removed_rel);
    write_file(&removed, "line one\nline two\n").unwrap();

    let patch = env.run(["--diff", "--include-removals"]).stdout;
    let patch_text = str::from_utf8(&patch).unwrap();

    assert!(
        patch_text.contains(&format!("diff --git a/{removed_rel} b/{removed_rel}")),
        "the removed file is missing from the patch:\n{patch_text}",
    );
    assert!(
        patch_text.contains("deleted file mode"),
        "the removed file is not shown as a deletion:\n{patch_text}",
    );

    // The dry run leaves the file on disk; applying the patch deletes it.
    assert!(removed.exists(), "the dry run must not delete the file");
    run_git(&env.target, &["init", "-q", "."]);
    let patch_file = env.target.join("outdated.patch");
    write_file(&patch_file, &patch).unwrap();
    run_git(&env.target, &["apply", "outdated.patch"]);
    remove_file(&patch_file).unwrap();
    assert!(
        !removed.exists(),
        "applying the patch must delete the removed file",
    );
}

#[test]
fn include_removals_requires_diff() {
    let env = InstallLocalLyricsEnv::prepare(INSTALL_LOCAL_LYRICS);

    let output = env.run_allow_failure(["--include-removals"]);

    assert!(
        !output.status.success(),
        "expected --include-removals without --diff to be rejected",
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--diff"),
        "expected the error to mention the required --diff, got:\n{stderr}",
    );
}
