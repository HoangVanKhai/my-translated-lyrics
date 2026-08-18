use pretty_assertions::assert_eq;
use std::fs::{metadata, read_to_string, remove_file, set_permissions, write as write_file};
use std::os::unix::fs::PermissionsExt;
use test_utils::{InstallLocalLyricsEnv, SEPARATED_COLLECTION, Temp, prepare_outdated, run_git};
use text_block_macros::text_block_fnl;

const INSTALL_LOCAL_LYRICS: &str = env!("CARGO_BIN_EXE_install-local-lyrics");

#[test]
fn honors_diff_despite_git_external_diff() {
    let env = InstallLocalLyricsEnv::prepare(INSTALL_LOCAL_LYRICS);
    let collection_name = SEPARATED_COLLECTION;
    let video_title = "【示例表演者】《示例歌曲》Example Song [ExampleID]";
    let (separated, unified) =
        prepare_outdated(&env, collection_name, video_title, "new\n", "old\n");

    // GIT_EXTERNAL_DIFF names a program git would run in place of its own
    // diff. Without `--no-ext-diff`, the patch would be the program's
    // output instead of a real diff.
    let script_dir = Temp::new_dir();
    let script = script_dir.join("external-diff");
    write_file(&script, "#!/bin/sh\necho HIJACKED\n").unwrap();
    let mut permissions = metadata(&script).unwrap().permissions();
    permissions.set_mode(0o755);
    set_permissions(&script, permissions).unwrap();

    let stdout = env.run_diff_with_env(&[("GIT_EXTERNAL_DIFF", script.to_str().unwrap())]);
    let patch_text = str::from_utf8(&stdout).unwrap();

    let separated_rel = format!("{collection_name}/{video_title}.vi.srt");
    assert!(
        patch_text.contains(&format!("diff --git a/{separated_rel} b/{separated_rel}")),
        "the external diff program replaced the patch:\n{patch_text}",
    );
    assert!(
        !patch_text.contains("HIJACKED"),
        "the external diff program ran:\n{patch_text}",
    );
    assert_eq!(read_to_string(&separated).unwrap(), "old\n");
    assert_eq!(read_to_string(&unified).unwrap(), "old\n");
}

#[test]
fn honors_diff_despite_git_dir_and_work_tree() {
    let env = InstallLocalLyricsEnv::prepare(INSTALL_LOCAL_LYRICS);
    let collection_name = SEPARATED_COLLECTION;
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

    // GIT_DIR together with GIT_WORK_TREE, as exported for a bare dotfiles
    // repository, would send every git invocation to a foreign repository
    // and silently empty the patch unless the tool clears them.
    let git_dir = Temp::new_dir();
    let work_tree = Temp::new_dir();
    let stdout = env.run_diff_with_env(&[
        ("GIT_DIR", git_dir.to_str().unwrap()),
        ("GIT_WORK_TREE", work_tree.to_str().unwrap()),
    ]);
    let patch_text = str::from_utf8(&stdout).unwrap();

    let separated_rel = format!("{collection_name}/{video_title}.vi.srt");
    assert!(
        patch_text.contains(&format!("diff --git a/{separated_rel} b/{separated_rel}")),
        "the patch was diverted to a foreign repository:\n{patch_text}",
    );
    assert!(
        patch_text.contains("-line two\n+line two changed\n"),
        "unexpected diff body:\n{patch_text}",
    );
    assert_eq!(read_to_string(&separated).unwrap(), target_content);
    assert_eq!(read_to_string(&unified).unwrap(), target_content);
}

#[test]
fn honors_diff_despite_git_diff_opts() {
    let env = InstallLocalLyricsEnv::prepare(INSTALL_LOCAL_LYRICS);
    let collection_name = SEPARATED_COLLECTION;
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

    // GIT_DIFF_OPTS=--unified=0 would strip the surrounding context that
    // git apply needs, and no configuration or command-line flag can
    // override it, so the tool must clear it. The resulting patch must
    // still apply cleanly.
    let stdout = env.run_diff_with_env(&[("GIT_DIFF_OPTS", "--unified=0")]);

    run_git(&env.target, &["init", "-q", "."]);
    let patch_file = env.target.join("outdated.patch");
    write_file(&patch_file, &stdout).unwrap();
    run_git(&env.target, &["apply", "outdated.patch"]);
    remove_file(&patch_file).unwrap();
    assert_eq!(read_to_string(&separated).unwrap(), source_content);
    assert_eq!(read_to_string(&unified).unwrap(), source_content);
}
