#![cfg_attr(dylint_lib = "perfectionist", feature(register_tool))]
#![cfg_attr(dylint_lib = "perfectionist", register_tool(perfectionist))]

use command_extra::CommandExtra;
use derive_more::{AsRef, Deref};
use itertools::Itertools;
use lyrics_core::collections_descriptor::{
    COLLECTIONS_CONFIG_FILE_NAME, CollectionName, CollectionsDesc,
};
use lyrics_core::video_descriptor::{Language, VideoDesc, VideoTitle, Visibility};
use maplit::hashmap;
use pipe_trait::Pipe;
use rand::distr::Alphanumeric;
use rand::{RngExt, rng};
use std::env::temp_dir;
use std::ffi::OsString;
use std::fs::{
    DirEntry, OpenOptions, create_dir, create_dir_all, read_dir, read_to_string, remove_dir_all,
    write as write_file,
};
use std::iter::once;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime};

/// The separated collection a test names in its video descriptors when
/// it does not care which one it uses.
pub const SEPARATED_COLLECTION: &str = "Example Collection";

/// The second declared separated collection, for a test that needs two
/// of them. Declaring a collection that most tests never write into
/// keeps the fixture honest, because a program that assumed a single
/// separated collection instead of reading the declared list would still
/// pass every test that uses only [`SEPARATED_COLLECTION`].
pub const OTHER_SEPARATED_COLLECTION: &str = "Another Example Collection";

/// The separated collections that [`InstallLocalLyricsEnv`] declares in
/// the `collections.toml` of its source directory.
pub const SEPARATED_COLLECTIONS: &[&str] = &[SEPARATED_COLLECTION, OTHER_SEPARATED_COLLECTION];

/// The collection that receives a copy of every subtitle file.
pub const UNIFIED_COLLECTION: &str = "Example Unified Collection";

/// Absolute path to the workspace root directory.
///
/// This crate always lives at `lib/test-utils`, two levels below the
/// workspace root, so the root is the grandparent of this crate's
/// manifest directory. Tests in other crates call this helper to reach
/// the repository data directories such as `dist/` and `sources/`,
/// which moved away from each crate's own manifest directory when the
/// project became a workspace.
pub fn workspace_dir() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("test-utils lives two levels below the workspace root")
}

/// Temporary directory that will be cleaned up on drop.
#[derive(AsRef, Debug, Deref)]
#[as_ref(forward)]
#[deref(forward)]
pub struct Temp(PathBuf);

impl Temp {
    /// Create a temporary directory.
    pub fn new_dir() -> Self {
        let path = rng()
            .sample_iter(&Alphanumeric)
            .take(15)
            .map(char::from)
            .collect::<String>()
            .pipe(|name| temp_dir().join(name));
        if path.exists() {
            return Self::new_dir();
        }
        create_dir(&path).unwrap_or_else(|error| panic!("failed to create {path:?}: {error}"));
        Temp(path)
    }
}

impl Drop for Temp {
    fn drop(&mut self) {
        let path = &self.0;
        if let Err(error) = remove_dir_all(path) {
            eprintln!("warning: Failed to delete {path:?}: {error}");
        }
    }
}

/// Test environment for `install-local-lyrics` with temporary
/// source and target directories.
pub struct InstallLocalLyricsEnv {
    _temp: Temp,
    bin: &'static str,
    pub source: PathBuf,
    pub target: PathBuf,
}

impl InstallLocalLyricsEnv {
    /// Prepares a new environment with empty source and target
    /// directories. The source directory receives a `collections.toml`
    /// declaring [`SEPARATED_COLLECTIONS`] and [`UNIFIED_COLLECTION`],
    /// and the target directory is pre-populated with a subdirectory for
    /// each of them. The `bin` argument is the path to the
    /// `install-local-lyrics` executable, which the caller obtains
    /// through `env!("CARGO_BIN_EXE_install-local-lyrics")`.
    pub fn prepare(bin: &'static str) -> Self {
        let temp = Temp::new_dir();
        let source = temp.join("source");
        let target = temp.join("target");
        create_dir(&source).unwrap();
        create_dir(&target).unwrap();
        let collections = CollectionsDesc {
            unified: vec![collection_name(UNIFIED_COLLECTION)],
            separated: SEPARATED_COLLECTIONS
                .iter()
                .copied()
                .map(collection_name)
                .collect(),
        };
        // Serialized through the real descriptor type, so the fixture
        // manifest cannot drift from the shape the program reads.
        write_file(
            source.join(COLLECTIONS_CONFIG_FILE_NAME),
            toml::to_string(&collections).unwrap(),
        )
        .unwrap();
        collections
            .names()
            .map(|name| target.join(name))
            .try_for_each(create_dir_all)
            .unwrap();
        InstallLocalLyricsEnv {
            _temp: temp,
            bin,
            source,
            target,
        }
    }

    /// Creates a source entry directory with a video descriptor and
    /// the given subtitle files.
    pub fn add_source_entry(&self, dir_name: &str, desc: &VideoDesc, lyrics: &[(&str, &str)]) {
        let video_dir = self.source.join(dir_name);
        create_dir_all(&video_dir).unwrap();

        let toml_content = toml::to_string(desc).unwrap();
        write_file(video_dir.join("video.toml"), toml_content).unwrap();

        for (file_name, content) in lyrics {
            write_file(video_dir.join(file_name), content).unwrap();
        }
    }

    /// Runs `install-local-lyrics` with the given arguments and returns
    /// the raw process output without asserting on the exit status.
    /// Callers that expect success should use [`run`](Self::run); callers
    /// that assert on a failure, such as an argument conflict, use this
    /// instead.
    pub fn run_unchecked<Args: IntoIterator<Item = &'static str>>(
        &self,
        args: Args,
    ) -> std::process::Output {
        self.bin
            .pipe(Command::new)
            .with_args(args)
            .with_arg(&self.source)
            .with_arg(&self.target)
            .output()
            .expect("failed to spawn install-local-lyrics")
    }

    /// Runs `install-local-lyrics` and asserts it exits successfully.
    pub fn run<Args: IntoIterator<Item = &'static str>>(&self, args: Args) -> std::process::Output {
        let output = self.run_unchecked(args);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stdout = stdout.trim();
        if !stdout.is_empty() {
            eprintln!("STDOUT:\n{stdout}\n");
        }
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stderr = stderr.trim();
        if !stderr.is_empty() {
            eprintln!("STDERR:\n{stderr}\n");
        }
        assert!(output.status.success(), "install-local-lyrics failed");
        output
    }

    /// Runs `install-local-lyrics --diff` with extra environment variables
    /// set, asserts it succeeds, and returns its standard output. Used to
    /// check that a hostile git environment does not perturb the patch.
    pub fn run_diff_with_env(&self, vars: &[(&str, &str)]) -> Vec<u8> {
        let output = self
            .bin
            .pipe(Command::new)
            .with_envs(vars.iter().copied())
            .with_arg("--diff")
            .with_arg(&self.source)
            .with_arg(&self.target)
            .output()
            .expect("failed to spawn install-local-lyrics");
        assert!(
            output.status.success(),
            "install-local-lyrics failed:\n{}",
            String::from_utf8_lossy(&output.stderr),
        );
        output.stdout
    }

    /// Collects all subtitle file paths relative to the target
    /// directory, sorted for deterministic comparison.
    pub fn target_subtitle_files(&self) -> Vec<String> {
        SEPARATED_COLLECTIONS
            .iter()
            .copied()
            .chain(once(UNIFIED_COLLECTION))
            .flat_map(|name| {
                self.target
                    .join(name)
                    .pipe(read_dir)
                    .unwrap()
                    .map(Result::<DirEntry, _>::unwrap)
                    .filter(|entry| entry.file_type().unwrap().is_file())
                    .map(|entry| entry.file_name())
                    .map(OsString::into_string)
                    .map(Result::<String, _>::unwrap)
                    .map(move |file_name| format!("{name}/{file_name}"))
            })
            .sorted()
            .collect()
    }

    /// Reads a target file's content.
    pub fn read_target(&self, collection_name: &str, file_name: &str) -> String {
        self.target
            .join(collection_name)
            .join(file_name)
            .pipe(read_to_string)
            .unwrap()
    }

    /// Returns the path to a target file.
    pub fn target_path(&self, collection_name: &str, file_name: &str) -> PathBuf {
        self.target.join(collection_name).join(file_name)
    }
}

/// Wraps a fixture collection name, which is always of a valid shape.
pub fn collection_name(name: impl Into<String>) -> CollectionName {
    name.into().pipe(CollectionName::try_from).unwrap()
}

/// Wraps a fixture video title, which is always of a valid shape.
pub fn video_title(title: impl Into<String>) -> VideoTitle {
    title.into().pipe(VideoTitle::try_from).unwrap()
}

/// A descriptor for one fixture video. Both names are taken as the types
/// the descriptor declares, so a call that swapped the collection for the
/// title would not compile.
pub fn video_desc(
    collection: CollectionName,
    title: VideoTitle,
    visibility: Visibility,
) -> VideoDesc {
    VideoDesc {
        collection,
        video_title: title,
        song_titles: hashmap! {
            Language::Vietnamese => "test".to_string(),
            Language::Chinese => "test".to_string(),
        },
        visibility,
    }
}

/// Sets the modification time of a file to a fixed point relative to the
/// Unix epoch. Explicit times keep the comparison between source and
/// target deterministic instead of relying on wall-clock ordering, whose
/// resolution varies between filesystems.
pub fn set_mtime(path: &Path, seconds_since_epoch: u64) {
    let time = SystemTime::UNIX_EPOCH + Duration::from_secs(seconds_since_epoch);
    OpenOptions::new()
        .write(true)
        .open(path)
        .unwrap()
        .set_modified(time)
        .unwrap();
}

/// Installs one outdated subtitle: a source newer than an already-present
/// target whose content differs, in both the separated and unified
/// collections. Returns the separated and unified target files.
pub fn prepare_outdated(
    env: &InstallLocalLyricsEnv,
    collection: &str,
    title: &str,
    source_content: &str,
    target_content: &str,
) -> (PathBuf, PathBuf) {
    let desc = video_desc(
        collection_name(collection),
        video_title(title),
        Visibility::default(),
    );
    env.add_source_entry("ExampleSong", &desc, &[("lyrics.vi.srt", source_content)]);

    let separated = env.target_path(collection, &format!("{title}.vi.srt"));
    let unified = env.target_path(UNIFIED_COLLECTION, &format!("{title}.vi.srt"));
    write_file(&separated, target_content).unwrap();
    write_file(&unified, target_content).unwrap();

    let source_file = env.source.join("ExampleSong").join("lyrics.vi.srt");
    set_mtime(&separated, 1_000_000);
    set_mtime(&unified, 1_000_000);
    set_mtime(&source_file, 2_000_000);

    (separated, unified)
}

/// Runs a `git` subcommand inside `dir` and asserts it succeeds. The global
/// and system configuration files are redirected to `/dev/null` so the test
/// does not depend on the developer's git settings.
pub fn run_git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .with_env("GIT_CONFIG_GLOBAL", "/dev/null")
        .with_env("GIT_CONFIG_SYSTEM", "/dev/null")
        .with_current_dir(dir)
        .with_args(args)
        .status()
        .unwrap();
    assert!(status.success(), "git {args:?} failed");
}

pub fn expected_stderr(
    existing_count: usize,
    removes: &[PathBuf],
    installs: &[(PathBuf, PathBuf)],
    updates: &[(PathBuf, PathBuf)],
    kept: &[(PathBuf, PathBuf)],
    dry_run: bool,
) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    writeln!(
        out,
        "info: There are currently {existing_count} existing files at the target location",
    )
    .unwrap();
    writeln!(
        out,
        "info: {} files would be removed from the target location",
        removes.len(),
    )
    .unwrap();
    writeln!(
        out,
        "info: {} files would be added to the target location",
        installs.len(),
    )
    .unwrap();
    writeln!(
        out,
        "info: {} files in the target location would be updated",
        updates.len(),
    )
    .unwrap();
    writeln!(out).unwrap();
    writeln!(out, "stage: Removing old subtitles").unwrap();
    for target in removes {
        writeln!(out, "remove {target:?}").unwrap();
    }
    writeln!(out).unwrap();
    writeln!(out, "stage: Adding new subtitles").unwrap();
    for (source, target) in installs {
        writeln!(out, "copy {source:?} → {target:?}").unwrap();
    }
    writeln!(out).unwrap();
    writeln!(out, "stage: Updating outdated subtitles").unwrap();
    for (source, target) in updates {
        writeln!(out, "copy {source:?} → {target:?}").unwrap();
    }
    if !kept.is_empty() {
        writeln!(out).unwrap();
        for (source, target) in kept {
            writeln!(
                out,
                "warning: Keeping {target:?} because it is newer than {source:?}",
            )
            .unwrap();
        }
        writeln!(
            out,
            "info: Pass --force to overwrite files that are newer than their source.",
        )
        .unwrap();
    }
    if dry_run {
        writeln!(out).unwrap();
        writeln!(out, "info: No changes were actually made.").unwrap();
        writeln!(
            out,
            "info: Run the command again with --execute to make actual changes.",
        )
        .unwrap();
    }
    out
}
