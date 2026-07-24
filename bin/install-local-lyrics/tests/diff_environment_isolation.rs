use pretty_assertions::assert_eq;
use std::fs::{
    create_dir_all, metadata, read_to_string, remove_file, set_permissions, write as write_file,
};
use std::os::unix::fs::PermissionsExt;
use test_utils::{InstallLocalLyricsEnv, Temp, prepare_outdated, run_git};
use text_block_macros::text_block_fnl;

const INSTALL_LOCAL_LYRICS: &str = env!("CARGO_BIN_EXE_install-local-lyrics");

#[test]
fn honors_diff_despite_global_gitignore_and_gitattributes() {
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

    // A global gitignore for *.srt and a `-diff` attribute for *.srt
    // would drop these files from the patch or render them as binary
    // unless the tool isolated git from the environment. The tool clears
    // HOME and XDG_CONFIG_HOME, the only paths by which git locates them.
    let home = Temp::new_dir();
    let xdg_config = home.join(".config");
    let git_config = xdg_config.join("git");
    create_dir_all(&git_config).unwrap();
    write_file(git_config.join("ignore"), "*.srt\n").unwrap();
    write_file(git_config.join("attributes"), "*.srt -diff\n").unwrap();

    let stdout = env.run_diff_with_env(&[
        ("HOME", home.to_str().unwrap()),
        ("XDG_CONFIG_HOME", xdg_config.to_str().unwrap()),
    ]);
    let patch_text = str::from_utf8(&stdout).unwrap();

    let separated_rel = format!("{collection_name}/{video_title}.vi.srt");
    assert!(
        patch_text.contains(&format!("diff --git a/{separated_rel} b/{separated_rel}")),
        "the .srt was dropped from the patch:\n{patch_text}",
    );
    assert!(
        patch_text.contains("-line two\n+line two changed\n"),
        "the .srt was rendered as binary rather than a text diff:\n{patch_text}",
    );
    assert_eq!(read_to_string(&separated).unwrap(), target_content);
    assert_eq!(read_to_string(&unified).unwrap(), target_content);
}

#[test]
fn honors_diff_despite_git_template_dir() {
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

    // A GIT_TEMPLATE_DIR whose info/exclude ignores *.srt would, unless the
    // tool initializes the repository from an empty template, seed that
    // exclude into the throwaway repository and drop these files from the
    // patch.
    let template = Temp::new_dir();
    let template_info = template.join("info");
    create_dir_all(&template_info).unwrap();
    write_file(template_info.join("exclude"), "*.srt\n").unwrap();

    let stdout = env.run_diff_with_env(&[("GIT_TEMPLATE_DIR", template.to_str().unwrap())]);
    let patch_text = str::from_utf8(&stdout).unwrap();

    let separated_rel = format!("{collection_name}/{video_title}.vi.srt");
    assert!(
        patch_text.contains(&format!("diff --git a/{separated_rel} b/{separated_rel}")),
        "the .srt was dropped from the patch:\n{patch_text}",
    );
    assert_eq!(read_to_string(&separated).unwrap(), target_content);
    assert_eq!(read_to_string(&unified).unwrap(), target_content);
}

#[test]
fn honors_diff_despite_git_external_diff() {
    let env = InstallLocalLyricsEnv::prepare(INSTALL_LOCAL_LYRICS);
    let collection_name = "Feng Ling Yu Xiu";
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

/// Asserts that configuration injected through the given environment
/// variables does not perturb the patch: the `a/`/`b/` prefixes survive
/// and the patch still applies cleanly. Each caller injects
/// `diff.noprefix=true`, which would otherwise strip the prefixes and
/// leave a patch `git apply` cannot place.
fn assert_config_injection_neutralized(env: &InstallLocalLyricsEnv, vars: &[(&str, &str)]) {
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
        env,
        collection_name,
        video_title,
        source_content,
        target_content,
    );

    let stdout = env.run_diff_with_env(vars);
    let patch_text = str::from_utf8(&stdout).unwrap();

    let separated_rel = format!("{collection_name}/{video_title}.vi.srt");
    assert!(
        patch_text.contains(&format!("diff --git a/{separated_rel} b/{separated_rel}")),
        "injected config stripped the a/ b/ prefixes:\n{patch_text}",
    );

    // The patch still applies cleanly against the target directory.
    run_git(&env.target, &["init", "-q", "."]);
    let patch_file = env.target.join("outdated.patch");
    write_file(&patch_file, &stdout).unwrap();
    run_git(&env.target, &["apply", "outdated.patch"]);
    remove_file(&patch_file).unwrap();
    assert_eq!(read_to_string(&separated).unwrap(), source_content);
    assert_eq!(read_to_string(&unified).unwrap(), source_content);
}

#[test]
fn honors_diff_despite_config_injected_via_count() {
    let env = InstallLocalLyricsEnv::prepare(INSTALL_LOCAL_LYRICS);
    assert_config_injection_neutralized(
        &env,
        &[
            ("GIT_CONFIG_COUNT", "1"),
            ("GIT_CONFIG_KEY_0", "diff.noprefix"),
            ("GIT_CONFIG_VALUE_0", "true"),
        ],
    );
}

#[test]
fn honors_diff_despite_config_injected_via_parameters() {
    let env = InstallLocalLyricsEnv::prepare(INSTALL_LOCAL_LYRICS);
    assert_config_injection_neutralized(&env, &[("GIT_CONFIG_PARAMETERS", "'diff.noprefix=true'")]);
}

#[test]
fn honors_diff_despite_git_dir_and_work_tree() {
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

    // GIT_DIFF_OPTS=--unified=0 would strip the surrounding context that
    // git apply needs, and no configuration or command-line flag can
    // override it, so the tool must clear it. The resulting patch must
    // still apply cleanly.
    let patch = env.run_diff_with_env(&[("GIT_DIFF_OPTS", "--unified=0")]);

    run_git(&env.target, &["init", "-q", "."]);
    let patch_file = env.target.join("outdated.patch");
    write_file(&patch_file, &patch).unwrap();
    run_git(&env.target, &["apply", "outdated.patch"]);
    remove_file(&patch_file).unwrap();
    assert_eq!(read_to_string(&separated).unwrap(), source_content);
    assert_eq!(read_to_string(&unified).unwrap(), source_content);
}

#[test]
fn honors_diff_despite_git_attr_source() {
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

    // GIT_ATTR_SOURCE points attribute lookup at a tree the throwaway
    // repository does not have, which aborts `git add` unless the tool
    // clears it.
    let stdout = env.run_diff_with_env(&[("GIT_ATTR_SOURCE", "HEAD")]);
    let patch_text = str::from_utf8(&stdout).unwrap();

    let separated_rel = format!("{collection_name}/{video_title}.vi.srt");
    assert!(
        patch_text.contains(&format!("diff --git a/{separated_rel} b/{separated_rel}")),
        "GIT_ATTR_SOURCE disrupted the diff:\n{patch_text}",
    );
    assert!(
        patch_text.contains("-line two\n+line two changed\n"),
        "unexpected diff body:\n{patch_text}",
    );
    assert_eq!(read_to_string(&separated).unwrap(), target_content);
    assert_eq!(read_to_string(&unified).unwrap(), target_content);
}
