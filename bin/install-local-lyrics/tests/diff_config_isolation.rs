use pretty_assertions::assert_eq;
use std::fs::{create_dir_all, read_to_string, remove_file, write as write_file};
use test_utils::{InstallLocalLyricsEnv, SEPARATED_COLLECTION, Temp, prepare_outdated, run_git};
use text_block_macros::text_block_fnl;

const INSTALL_LOCAL_LYRICS: &str = env!("CARGO_BIN_EXE_install-local-lyrics");

#[test]
fn honors_diff_despite_global_gitignore_and_gitattributes() {
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

/// Asserts that configuration injected through the given environment
/// variables does not perturb the patch: the `a/`/`b/` prefixes survive
/// and the patch still applies cleanly. Each caller injects
/// `diff.noprefix=true`, which would otherwise strip the prefixes and
/// leave a patch `git apply` cannot place.
fn assert_config_injection_neutralized(env: &InstallLocalLyricsEnv, vars: &[(&str, &str)]) {
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
fn honors_diff_despite_git_attr_source() {
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
