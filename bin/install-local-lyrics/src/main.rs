#![cfg_attr(dylint_lib = "perfectionist", feature(register_tool))]
#![cfg_attr(dylint_lib = "perfectionist", register_tool(perfectionist))]

use clap::Parser;
use command_extra::CommandExtra;
use itertools::Itertools;
use lyrics_core::file_snapshot::FileSnapshot;
use lyrics_core::video_descriptor::{
    LyricsFileName, ParseLyricsFileNameError, SEPARATED_COLLECTIONS, UNIFIED_COLLECTION,
    VIDEO_CONFIG_FILE_NAME, VideoDesc, Visibility,
};
use pipe_trait::Pipe;
use rand::distr::Alphanumeric;
use rand::{RngExt, rng};
use reflink::reflink_or_copy;
use std::collections::{HashMap, HashSet};
use std::env::temp_dir;
use std::fs::{
    DirEntry, copy, create_dir, create_dir_all, hard_link, read, read_dir, read_to_string,
    remove_dir_all, remove_file, write,
};
use std::io::{self, ErrorKind, Write};
use std::iter::once;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Clone, Debug, Parser)]
#[clap(about = "Synchronize the lyrics")]
struct Args {
    /// For safety reasons, this programs list actions by default, this flag makes the program take those actions.
    #[clap(long, short = 'x')]
    execute: bool,

    /// Overwrite target files that are newer than their source instead of keeping them.
    #[clap(long, short = 'f')]
    force: bool,

    /// Render a diff of the outdated subtitles that a dry run would update.
    /// Diffing only inspects changes without applying them, so this flag conflicts with --execute.
    #[clap(long, short = 'd', conflicts_with = "execute")]
    diff: bool,

    /// Source directory of the subtitles.
    source: PathBuf,

    /// Container of the target directories of the subtitles.
    target: PathBuf,
}

/// Try hardlink, then try reflink, and finally copy.
fn link_or_copy(source: &Path, target: &Path) -> io::Result<()> {
    if hard_link(source, target).is_ok() {
        return Ok(());
    }

    reflink_or_copy(source, target)?;

    Ok(())
}

fn uninstall(execute: bool, target: &Path) {
    eprintln!("remove {target:?}");
    if execute {
        remove_file(target).unwrap();
    }
}

/// Warn that a target file is kept because it is newer than its source.
/// No filesystem change is made regardless of the `--execute` flag.
fn keep(target: &Path, source: &Path) {
    eprintln!("warning: Keeping {target:?} because it is newer than {source:?}");
}

/// Build a `git` command that runs inside `repo`, isolated from the
/// developer's environment so that a personal setting cannot alter the
/// patch. Isolation happens on two fronts because git reads configuration
/// and ignore rules from more than one place:
///
/// - The global and system configuration files are redirected to
///   `/dev/null`, and the configuration injected through the environment
///   (`GIT_CONFIG_COUNT` with `GIT_CONFIG_KEY_<n>`, and the older
///   `GIT_CONFIG_PARAMETERS`) is discarded. Without this, an injected
///   `diff.noprefix` would strip the `a/`/`b/` prefixes the patch needs,
///   and an injected `core.autocrlf` would rewrite line endings.
/// - The default excludes and attributes files are overridden. Git reads
///   its default global gitignore and attributes file even when no
///   configuration points at them, so a `*.srt` ignore rule would
///   silently drop files from the patch and a `*.srt -diff` attribute
///   would turn a text change into a non-applicable binary patch.
fn git_command(repo: &Path) -> Command {
    Command::new("git")
        .with_env("GIT_CONFIG_GLOBAL", "/dev/null")
        .with_env("GIT_CONFIG_SYSTEM", "/dev/null")
        .with_env("GIT_CONFIG_COUNT", "0")
        .with_env("GIT_CONFIG_PARAMETERS", "")
        .with_arg("-C")
        .with_arg(repo)
        .with_arg("-c")
        .with_arg("core.excludesFile=/dev/null")
        .with_arg("-c")
        .with_arg("core.attributesFile=/dev/null")
}

/// Run a `git` subcommand inside `repo` and require it to succeed.
fn run_git(repo: &Path, args: &[&str]) {
    let status = git_command(repo)
        .with_args(args)
        .status()
        .unwrap_or_else(|error| panic!("error: Cannot run git {args:?}: {error}"));
    if !status.success() {
        panic!("error: git {args:?} failed with {status}");
    }
}

/// Run `git diff` inside `repo` and return its standard output verbatim.
/// `core.quotePath` is disabled so that non-ASCII file names appear as
/// their literal glyphs rather than octal escapes, which `git apply`
/// still accepts. `--binary` makes the patch applicable even when a file's
/// content is classified as binary, rather than emitting a lossy
/// `Binary files differ` line. `--no-ext-diff` ignores any external diff
/// program, whether set through `GIT_EXTERNAL_DIFF` or a `diff.external`
/// configuration, which would otherwise replace the patch with the
/// program's own output.
fn git_diff(repo: &Path) -> Vec<u8> {
    let output = git_command(repo)
        .with_args([
            "-c",
            "core.quotePath=false",
            "diff",
            "--no-color",
            "--binary",
            "--no-ext-diff",
        ])
        .output()
        .unwrap_or_else(|error| panic!("error: Cannot run git diff: {error}"));
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("error: git diff failed with {}: {stderr}", output.status);
    }
    output.stdout
}

/// A directory that is removed when the value is dropped, so that a panic
/// while building the diff does not leave the throwaway git repository
/// behind.
struct TempRepoDir(PathBuf);

impl Drop for TempRepoDir {
    fn drop(&mut self) {
        if let Err(error) = remove_dir_all(&self.0)
            && error.kind() != ErrorKind::NotFound
        {
            eprintln!(
                "warning: Failed to delete temporary diff directory {:?}: {error}",
                self.0,
            );
        }
    }
}

/// Create a freshly named, empty directory under the system temporary
/// directory and return a guard that removes it on drop.
///
/// The name is randomized and the directory is created exclusively, so a
/// pre-existing entry left by another process, or a symlink planted by
/// another user in a shared temporary directory, cannot be followed or
/// force a collision. On the astronomically unlikely chance that the name
/// already exists, another name is drawn.
fn create_temp_repo_dir() -> TempRepoDir {
    loop {
        let name: String = rng()
            .sample_iter(&Alphanumeric)
            .take(15)
            .map(char::from)
            .collect();
        let path = temp_dir().join(format!("install-local-lyrics-diff.{name}"));
        match create_dir(&path) {
            Ok(()) => return TempRepoDir(path),
            Err(error) if error.kind() == ErrorKind::AlreadyExists => continue,
            Err(error) => {
                panic!("error: Cannot create temporary diff directory {path:?}: {error}")
            }
        }
    }
}

/// Render a single unified diff of all outdated subtitles to standard
/// output. The patch is produced by `git diff`, so it carries the usual
/// `a/`/`b/` prefixes and limited context and can be replayed against the
/// target directory with `git apply`.
///
/// No diff between the target files and their sources exists yet, so the
/// current target files are staged in a throwaway git repository under
/// their target-relative paths, overwritten with the source content, and
/// handed to `git diff`. This runs only on a dry run, so the real target
/// files are never modified.
fn render_diff(target_root: &Path, updates: &[(&Path, &Path)]) {
    // The guard removes the directory on return, including when a later
    // step panics.
    let repo_dir = create_temp_repo_dir();
    let repo = repo_dir.0.as_path();
    // `--template=` starts from an empty template, so a `GIT_TEMPLATE_DIR`
    // in the environment cannot seed `.git/info/exclude` or
    // `.git/info/attributes` into the repository and perturb the patch.
    run_git(repo, &["init", "-q", "--template="]);

    let staged: Vec<PathBuf> = updates
        .iter()
        .map(|(_, target)| {
            let relative = target.strip_prefix(target_root).unwrap_or_else(|error| {
                panic!("error: {target:?} is not inside {target_root:?}: {error}")
            });
            repo.join(relative)
        })
        .collect();

    // Stage a copy of each current target file under its target-relative
    // path. The copy carries the target's permissions.
    for ((_, target), staged) in updates.iter().zip(&staged) {
        if let Some(parent) = staged.parent() {
            create_dir_all(parent)
                .unwrap_or_else(|error| panic!("error: Cannot create {parent:?}: {error}"));
        }
        copy(target, staged)
            .unwrap_or_else(|error| panic!("error: Cannot copy {target:?} to {staged:?}: {error}"));
    }
    run_git(repo, &["add", "-A"]);

    // Overwrite each staged file's content with the source while leaving
    // its permissions untouched, so that `git diff` reports the content
    // change alone and never a mode change.
    for ((source, _), staged) in updates.iter().zip(&staged) {
        let content =
            read(source).unwrap_or_else(|error| panic!("error: Cannot read {source:?}: {error}"));
        write(staged, &content)
            .unwrap_or_else(|error| panic!("error: Cannot write {staged:?}: {error}"));
    }

    let patch = git_diff(repo);
    // A reader such as a pager may close the pipe before the whole patch
    // is written. That is a clean end of output rather than a failure.
    if let Err(error) = io::stdout().write_all(&patch)
        && error.kind() != ErrorKind::BrokenPipe
    {
        panic!("error: Cannot write diff to standard output: {error}");
    }
}

fn install(execute: bool, source: &Path, target: &Path) {
    eprintln!("copy {source:?} → {target:?}");
    if execute {
        if let Err(error) = remove_file(target)
            && error.kind() != ErrorKind::NotFound
        {
            eprintln!("warning: Cannot remove file {target:?}: {error}");
        }

        // Q: Why try hardlink before reflink?
        // A: It'd be convenient not having to re-run the script
        //    just to update the subtitles.
        link_or_copy(source, target).unwrap();
    }
}

fn is_subtitle_file(entry: &DirEntry) -> bool {
    match entry.file_type() {
        Err(error) => panic!(
            "error: Cannot read file type of {:?}: {error}",
            entry.path(),
        ),
        Ok(file_type) if !file_type.is_file() => return false,
        Ok(file_type) => debug_assert!(file_type.is_file()),
    }

    let file_name = entry.file_name();
    let file_name = file_name.as_bytes();
    file_name.ends_with(b".srt") || file_name.ends_with(b".vtt")
}

fn main() {
    let Args {
        execute,
        force,
        diff,
        source,
        target,
    } = Args::parse();

    // Read all video descriptors from source directories
    let descriptors: Vec<(PathBuf, VideoDesc)> = source
        .pipe_ref(read_dir)
        .unwrap_or_else(|error| panic!("error: Cannot read source directory {source:?}: {error}"))
        .map(|entry| {
            entry.unwrap_or_else(|error| {
                panic!("error: Cannot read an entry of directory {source:?}: {error}")
            })
        })
        .filter(|entry| {
            entry
                .file_type()
                .unwrap_or_else(|error| {
                    panic!(
                        "error: Cannot read file type of {:?}: {error}",
                        entry.path(),
                    )
                })
                .is_dir()
        })
        .map(|entry| {
            let video_dir = entry.path();
            let desc_path = video_dir.join(VIDEO_CONFIG_FILE_NAME);
            let content = desc_path
                .pipe_ref(read_to_string)
                .unwrap_or_else(|error| panic!("error: Cannot read {desc_path:?}: {error}"));
            let desc: VideoDesc = content
                .pipe_as_ref(toml::from_str)
                .unwrap_or_else(|error| panic!("error: Cannot parse {desc_path:?}: {error}"));
            (video_dir, desc)
        })
        .collect(); // eagerly validate all video descriptors before touching any files

    let existing_target_files: HashMap<PathBuf, FileSnapshot> = SEPARATED_COLLECTIONS
        .iter()
        .copied()
        .chain(once(UNIFIED_COLLECTION))
        .map(|suffix| target.join(suffix))
        .flat_map(|path| {
            path.pipe_ref(read_dir)
                .unwrap_or_else(|error| panic!("error: Cannot read directory {path:?}: {error}"))
                .map(move |entry| {
                    entry.unwrap_or_else(|error| {
                        panic!("error: Cannot read an entry of directory {path:?}: {error}")
                    })
                })
                .filter(is_subtitle_file)
                .map(|entry| entry.path())
        })
        .map(|path| {
            let snapshot = path
                .to_path_buf()
                .pipe(FileSnapshot::new)
                .unwrap_or_else(|error| panic!("error: Cannot read file {path:?}: {error}"));
            (path, snapshot)
        })
        .collect();
    eprintln!(
        "info: There are currently {} existing files at the target location",
        existing_target_files.len(),
    );

    let mut files_need_update: Vec<(PathBuf, PathBuf)> =
        Vec::with_capacity(existing_target_files.len());
    let mut files_need_uninstall: HashSet<&PathBuf> = existing_target_files.keys().collect();
    let mut files_need_install: Vec<(PathBuf, PathBuf)> =
        Vec::with_capacity(existing_target_files.len());
    let mut files_kept_newer: Vec<(PathBuf, PathBuf)> = Vec::new();

    for (video_dir, desc) in &descriptors {
        // Hidden: do nothing. Any existing target files stay in
        // `files_need_uninstall` and will be removed.
        if desc.visibility == Visibility::Hidden {
            continue;
        }

        let separated_target_dir = target.join(&desc.collection);
        let unified_target_dir = target.join(UNIFIED_COLLECTION);

        if desc.visibility == Visibility::Manual {
            let prefix = format!("{}.", desc.video_title);
            let separated = separated_target_dir.as_path();
            let unified = unified_target_dir.as_path();
            files_need_uninstall.retain(|target_path| {
                let Some(parent) = target_path.parent() else {
                    return true;
                };
                if parent != separated && parent != unified {
                    return true;
                }
                let name = target_path
                    .file_name()
                    .expect("target path has no file name")
                    .to_str()
                    .unwrap_or_else(|| {
                        panic!("error: Non-UTF-8 filename in target: {target_path:?}")
                    });
                !name.starts_with(&prefix)
            });
            continue;
        }

        let source_entries = video_dir
            .pipe(read_dir)
            .unwrap_or_else(|error| panic!("error: Cannot read directory {video_dir:?}: {error}"));

        for source_entry in source_entries {
            let source_entry = source_entry.unwrap_or_else(|error| {
                panic!("error: Cannot read an entry of directory {video_dir:?}: {error}")
            });
            if !is_subtitle_file(&source_entry) {
                continue;
            }

            let local_name = source_entry.file_name();
            let local_name = local_name
                .to_str()
                .unwrap_or_else(|| panic!("error: Non-UTF-8 filename in {video_dir:?}"));

            let lyrics = match local_name.parse::<LyricsFileName>() {
                Ok(lyrics) => lyrics,
                Err(ParseLyricsFileNameError::NotLyricsFile) => continue,
                Err(error) => panic!(
                    "error: {dir}/{local_name}: {error}",
                    dir = video_dir.display(),
                ),
            };
            let target_name = lyrics.target_file_name(&desc.video_title).to_string();

            let source_file = video_dir.join(local_name);
            let separated_target_file = separated_target_dir.join(&target_name);
            let unified_target_file = unified_target_dir.join(&target_name);

            let source_file_snapshot = source_file.clone().pipe(FileSnapshot::new);
            for target_file in [separated_target_file, unified_target_file] {
                let Some(target_file_snapshot) = existing_target_files.get(&target_file) else {
                    files_need_install.push((source_file.clone(), target_file));
                    continue;
                };

                let source_file_snapshot = source_file_snapshot.as_ref().unwrap_or_else(|error| {
                    panic!("error: Cannot read file {:?}: {error}", source_file.clone())
                });

                let was_present = files_need_uninstall.remove(&target_file);
                debug_assert!(
                    was_present,
                    "Expecting {target_file:?} to still exist but it doesn't",
                );

                if target_file_snapshot.content_eq(source_file_snapshot) {
                    continue;
                }

                if !force && target_file_snapshot.is_newer_than(source_file_snapshot) {
                    files_kept_newer.push((source_file.clone(), target_file));
                    continue;
                }

                files_need_update.push((source_file.clone(), target_file));
            }
        }
    }

    eprintln!(
        "info: {} files would be removed from the target location",
        files_need_uninstall.len(),
    );
    eprintln!(
        "info: {} files would be added to the target location",
        files_need_install.len(),
    );
    eprintln!(
        "info: {} files in the target location would be updated",
        files_need_update.len(),
    );

    eprintln!();
    eprintln!("stage: Removing old subtitles");
    for target in files_need_uninstall.iter().sorted() {
        uninstall(execute, target);
    }

    eprintln!();
    eprintln!("stage: Adding new subtitles");
    for (source, target) in files_need_install.iter().sorted() {
        install(execute, source, target);
    }

    eprintln!();
    eprintln!("stage: Updating outdated subtitles");
    let updates: Vec<(&Path, &Path)> = files_need_update
        .iter()
        .sorted()
        .map(|(source, target)| (source.as_path(), target.as_path()))
        .collect();
    for &(source, target) in &updates {
        install(execute, source, target);
    }
    if diff && !updates.is_empty() {
        render_diff(&target, &updates);
    }

    if !files_kept_newer.is_empty() {
        eprintln!();
        for (source, target) in files_kept_newer.iter().sorted() {
            keep(target, source);
        }
        eprintln!("info: Pass --force to overwrite files that are newer than their source.");
    }

    if !execute {
        eprintln!();
        eprintln!("info: No changes were actually made.");
        eprintln!("info: Run the command again with --execute to make actual changes.");
    }
}
