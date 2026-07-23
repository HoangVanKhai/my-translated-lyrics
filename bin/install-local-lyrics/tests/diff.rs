use command_extra::CommandExtra;
use lyrics_core::video_descriptor::{UNIFIED_COLLECTION, Visibility};
use pretty_assertions::assert_eq;
use std::fs::{
    OpenOptions, create_dir_all, metadata, read, read_to_string, remove_file, set_permissions,
    write as write_file,
};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime};
use test_utils::{InstallLocalLyricsEnv, Temp, video_desc};
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

/// Installs one outdated subtitle: a source that is newer than an
/// already-present target whose content differs, in both the separated
/// and unified collections. Returns the separated and unified target
/// files.
fn prepare_outdated(
    env: &InstallLocalLyricsEnv,
    collection_name: &str,
    video_title: &str,
    source_content: &str,
    target_content: &str,
) -> (PathBuf, PathBuf) {
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

    (separated, unified)
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

/// Runs `install-local-lyrics --diff` with extra environment variables
/// set, asserts it succeeds, and returns its standard output. It checks
/// that a hostile git environment does not perturb the emitted patch.
fn run_diff_with_env(env: &InstallLocalLyricsEnv, vars: &[(&str, &str)]) -> Vec<u8> {
    let mut command = Command::new(INSTALL_LOCAL_LYRICS);
    for &(key, value) in vars {
        command = command.with_env(key, value);
    }
    let output = command
        .with_arg("--diff")
        .with_arg(&env.source)
        .with_arg(&env.target)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "install-local-lyrics failed:\n{}",
        String::from_utf8_lossy(&output.stderr),
    );
    output.stdout
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
    let source_content = "\u{0}binary source content";
    let target_content = "\u{0}binary target";
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
    // would, without the tool overriding git's default excludes and
    // attributes files, drop these files from the patch or render them as
    // binary.
    let home = Temp::new_dir();
    let xdg_config = home.join(".config");
    let git_config = xdg_config.join("git");
    create_dir_all(&git_config).unwrap();
    write_file(git_config.join("ignore"), "*.srt\n").unwrap();
    write_file(git_config.join("attributes"), "*.srt -diff\n").unwrap();

    let stdout = run_diff_with_env(
        &env,
        &[
            ("HOME", home.to_str().unwrap()),
            ("XDG_CONFIG_HOME", xdg_config.to_str().unwrap()),
        ],
    );
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

    let stdout = run_diff_with_env(&env, &[("GIT_TEMPLATE_DIR", template.to_str().unwrap())]);
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

    let stdout = run_diff_with_env(&env, &[("GIT_EXTERNAL_DIFF", script.to_str().unwrap())]);
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
fn honors_diff_despite_injected_config() {
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

    // Injecting diff.noprefix through GIT_CONFIG_COUNT would strip the
    // a/ b/ prefixes and leave a patch git apply cannot place. The tool
    // discards config injected through the environment.
    let stdout = run_diff_with_env(
        &env,
        &[
            ("GIT_CONFIG_COUNT", "1"),
            ("GIT_CONFIG_KEY_0", "diff.noprefix"),
            ("GIT_CONFIG_VALUE_0", "true"),
        ],
    );
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
