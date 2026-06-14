#![allow(
    dead_code,
    clippy::new_without_default,
    reason = "each test binary pulls this shared module in but uses only the part it needs, and `Env` is a fixture builder for which a `Default` impl would be meaningless"
)]

use command_extra::CommandExtra;
use lyrics_core::video_descriptor::Visibility;
use std::ffi::OsStr;
use std::fs::{create_dir, create_dir_all, write as write_file};
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use test_utils::{Temp, video_desc};

const PLAY_WITH_LYRICS: &str = env!("CARGO_BIN_EXE_play-with-lyrics");
const COLLECTION: &str = "Feng Ling Yu Xiu";
pub const VIDEO_TITLE: &str = "Example Song [id]";

/// A temporary source directory and media library for one test.
pub struct Env {
    _temp: Temp,
    source: PathBuf,
    target: PathBuf,
}

impl Env {
    pub fn new() -> Self {
        let temp = Temp::new_dir();
        let source = temp.join("source");
        let target = temp.join("target");
        create_dir(&source).unwrap();
        create_dir(&target).unwrap();
        Env {
            _temp: temp,
            source,
            target,
        }
    }

    /// Writes a `video.toml` serialized from a descriptor built with the
    /// test collection and title.
    pub fn add_video(&self) {
        let video_dir = self.source.join("ExampleSong");
        create_dir_all(&video_dir).unwrap();
        let descriptor = video_desc(COLLECTION, VIDEO_TITLE, Visibility::Visible);
        let contents = toml::to_string(&descriptor).unwrap();
        write_file(video_dir.join("video.toml"), contents).unwrap();
    }

    /// The collection directory inside the media library, created on first
    /// use.
    pub fn collection_dir(&self) -> PathBuf {
        let dir = self.target.join(COLLECTION);
        create_dir_all(&dir).unwrap();
        dir
    }

    /// Creates an empty file named `file_name` inside the collection
    /// directory.
    pub fn add_library_file(&self, file_name: &str) {
        write_file(self.collection_dir().join(file_name), "").unwrap();
    }

    pub fn run<Args>(&self, args: Args) -> Output
    where
        Args: IntoIterator,
        Args::Item: AsRef<OsStr>,
    {
        Command::new(PLAY_WITH_LYRICS)
            .with_arg(&self.source)
            .with_arg(&self.target)
            .with_args(args)
            .with_stdin(Stdio::null()) // null stdin keeps the session non-interactive
            .output()
            .expect("failed to spawn play-with-lyrics")
    }
}

/// Support for the happy-path tests, which run the binary against fake
/// player programs so no real media player is launched. The fake programs
/// are shell scripts and the support is therefore Unix-only.
#[cfg(unix)]
impl Env {
    /// The directory the fake player programs are installed into.
    fn bin_dir(&self) -> PathBuf {
        self._temp.join("bin")
    }

    /// The file the fake players record their arguments to.
    fn record_path(&self) -> PathBuf {
        self._temp.join("invocation")
    }

    /// Installs a fake `mpv` and `celluloid`, each a script that records the
    /// arguments it is launched with.
    pub fn install_fake_players(&self) {
        use std::os::unix::fs::PermissionsExt;
        let bin = self.bin_dir();
        create_dir_all(&bin).unwrap();
        let script = format!(
            "#!/bin/sh\nprintf '%s\\n' \"$@\" > '{}'\n",
            self.record_path().display(),
        );
        for name in ["mpv", "celluloid"] {
            let path = bin.join(name);
            write_file(&path, &script).unwrap();
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
    }

    /// Runs the binary with the fake-player directory at the front of `PATH`
    /// and returns the process output together with the arguments the fake
    /// player recorded.
    pub fn run_played<Args>(&self, args: Args) -> (Output, Vec<String>)
    where
        Args: IntoIterator,
        Args::Item: AsRef<OsStr>,
    {
        let path = std::env::var("PATH").unwrap_or_default();
        let output = Command::new(PLAY_WITH_LYRICS)
            .with_arg(&self.source)
            .with_arg(&self.target)
            .with_args(args)
            .with_env("PATH", format!("{}:{path}", self.bin_dir().display()))
            .with_stdin(Stdio::null())
            .output()
            .expect("failed to spawn play-with-lyrics");
        let recorded = std::fs::read_to_string(self.record_path())
            .unwrap_or_default()
            .lines()
            .map(str::to_string)
            .collect();
        (output, recorded)
    }

    /// The path a library file would have, for asserting on the launched
    /// command.
    pub fn library_path(&self, file_name: &str) -> PathBuf {
        self.collection_dir().join(file_name)
    }
}

pub fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
