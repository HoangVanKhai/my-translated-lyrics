use command_extra::CommandExtra;
use derive_more::{AsRef, Deref};
use itertools::Itertools;
use maplit::hashmap;
use pipe_trait::Pipe;
use rand::{RngExt, distr::Alphanumeric, rng};
use std::env::temp_dir;
use std::ffi::OsString;
use std::fs::{create_dir, create_dir_all, read_dir, read_to_string, write as write_file};
use std::iter::once;
use std::path::PathBuf;
use std::process::Command;
use translated_lyrics::video_descriptor::{
    Language, SEPARATED_COLLECTIONS, UNIFIED_COLLECTION, VideoDesc, Visibility,
};

const INSTALL_LOCAL_LYRICS: &str = env!("CARGO_BIN_EXE_install-local-lyrics");

/// Temporary directory that will be cleaned up on drop.
#[derive(Debug, AsRef, Deref)]
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
        if let Err(error) = std::fs::remove_dir_all(path) {
            eprintln!("warning: Failed to delete {path:?}: {error}");
        }
    }
}

/// Test environment for `install-local-lyrics` with temporary
/// source and target directories.
pub struct InstallLocalLyricsEnv {
    _temp: Temp,
    pub source: PathBuf,
    pub target: PathBuf,
}

impl InstallLocalLyricsEnv {
    /// Prepares a new environment with empty source and target
    /// directories. The target directory is pre-populated with the
    /// required collection subdirectories.
    pub fn prepare() -> Self {
        let temp = Temp::new_dir();
        let source = temp.join("source");
        let target = temp.join("target");
        create_dir(&source).unwrap();
        create_dir(&target).unwrap();
        SEPARATED_COLLECTIONS
            .iter()
            .copied()
            .chain(once(UNIFIED_COLLECTION))
            .map(|name| target.join(name))
            .try_for_each(create_dir_all)
            .unwrap();
        InstallLocalLyricsEnv {
            _temp: temp,
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

    /// Runs `install-local-lyrics` and asserts it exits successfully.
    pub fn run<Args: IntoIterator<Item = &'static str>>(&self, args: Args) -> std::process::Output {
        let output = Command::new(INSTALL_LOCAL_LYRICS)
            .with_args(args)
            .with_arg(&self.source)
            .with_arg(&self.target)
            .output()
            .expect("failed to spawn install-local-lyrics");
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
                    .map(Result::unwrap)
                    .filter(|entry| entry.file_type().unwrap().is_file())
                    .map(|entry| entry.file_name())
                    .map(OsString::into_string)
                    .map(Result::unwrap)
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

pub fn video_desc(collection_name: &str, video_title: &str, visibility: Visibility) -> VideoDesc {
    VideoDesc {
        collection: collection_name.to_string().try_into().unwrap(),
        video_title: video_title.to_string().try_into().unwrap(),
        song_titles: hashmap! {
            Language::Vietnamese => "test".to_string(),
            Language::Chinese => "test".to_string(),
        },
        visibility,
    }
}

pub fn expected_stderr(
    existing_count: usize,
    removes: &[PathBuf],
    installs: &[(PathBuf, PathBuf)],
    updates: &[(PathBuf, PathBuf)],
    dry_run: bool,
) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    writeln!(
        out,
        "info: There are currently {existing_count} existing files at the target location"
    )
    .unwrap();
    writeln!(
        out,
        "info: {} files would be removed from the target location",
        removes.len()
    )
    .unwrap();
    writeln!(
        out,
        "info: {} files would be added to the target location",
        installs.len()
    )
    .unwrap();
    writeln!(
        out,
        "info: {} files in the target location would be updated",
        updates.len()
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
    if dry_run {
        writeln!(out).unwrap();
        writeln!(out, "info: No changes were actually made.").unwrap();
        writeln!(
            out,
            "info: Run the command again with --execute to make actual changes."
        )
        .unwrap();
    }
    out
}
