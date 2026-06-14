//! End-to-end tests that drive the compiled binary. The failure-path tests
//! exercise paths that return before any player launches. The happy-path
//! tests run the binary against fake player programs on `PATH` (Unix-only),
//! so no real media player is spawned, and assert on the arguments the fake
//! player was launched with.

// cspell:ignore bài hát ví dụ

use command_extra::CommandExtra;
use std::ffi::OsStr;
use std::fs::{create_dir, create_dir_all, write as write_file};
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use test_utils::Temp;

const PLAY_WITH_LYRICS: &str = env!("CARGO_BIN_EXE_play-with-lyrics");
const COLLECTION: &str = "Feng Ling Yu Xiu";
const VIDEO_TITLE: &str = "Example Song [id]";

/// A temporary source directory and media library for one test.
struct Env {
    _temp: Temp,
    source: PathBuf,
    target: PathBuf,
}

impl Env {
    fn new() -> Self {
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

    /// Writes a `video.toml` describing the sample video, with all three
    /// titles present.
    fn add_video(&self) {
        let video_dir = self.source.join("ExampleSong");
        create_dir_all(&video_dir).unwrap();
        let descriptor = format!(
            r#"collection = "{COLLECTION}"
video-title = "{VIDEO_TITLE}"

[song-titles]
en = "Example Song"
vi = "Bài Hát Ví Dụ"
zh = "示例歌曲"
"#,
        );
        write_file(video_dir.join("video.toml"), descriptor).unwrap();
    }

    /// The collection directory inside the media library, created on first
    /// use.
    fn collection_dir(&self) -> PathBuf {
        let dir = self.target.join(COLLECTION);
        create_dir_all(&dir).unwrap();
        dir
    }

    /// Creates an empty file named `file_name` inside the collection
    /// directory.
    fn add_library_file(&self, file_name: &str) {
        write_file(self.collection_dir().join(file_name), "").unwrap();
    }

    fn run<Args>(&self, args: Args) -> Output
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

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
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
    fn install_fake_players(&self) {
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
    fn run_played<Args>(&self, args: Args) -> (Output, Vec<String>)
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
    fn library_path(&self, file_name: &str) -> PathBuf {
        self.collection_dir().join(file_name)
    }
}

#[test]
fn an_unmatched_title_is_an_error() {
    let env = Env::new();
    env.add_video();
    env.add_library_file(&format!("{VIDEO_TITLE}.mkv"));
    env.add_library_file(&format!("{VIDEO_TITLE}.vi.srt"));

    let output = env.run(["--title", "nonexistent"]);

    assert!(!output.status.success());
    assert!(
        stderr(&output).contains("no candidate matched"),
        "{output:?}",
    );
}

#[test]
fn a_video_without_subtitles_is_an_error() {
    let env = Env::new();
    env.add_video();
    env.add_library_file(&format!("{VIDEO_TITLE}.mkv"));

    let output = env.run(["--title", "example"]);

    assert!(!output.status.success());
    assert!(stderr(&output).contains("No subtitles"), "{output:?}");
}

#[test]
fn an_unavailable_language_is_an_error() {
    let env = Env::new();
    env.add_video();
    env.add_library_file(&format!("{VIDEO_TITLE}.mkv"));
    env.add_library_file(&format!("{VIDEO_TITLE}.vi.srt"));

    // Only Vietnamese is available, so English is rejected. The full
    // language name is given here, exercising one of the aliases.
    let output = env.run(["--title", "example", "--language", "english"]);

    assert!(!output.status.success());
    assert!(stderr(&output).contains("available: vi"), "{output:?}");
}

#[test]
fn an_unavailable_format_is_an_error() {
    let env = Env::new();
    env.add_video();
    env.add_library_file(&format!("{VIDEO_TITLE}.mkv"));
    env.add_library_file(&format!("{VIDEO_TITLE}.vi.srt"));

    // Only SubRip is available for Vietnamese, so WebVTT is rejected. The
    // canonical "vi" code and the "web-vtt" alias are exercised here.
    let output = env.run([
        "--title",
        "example",
        "--language",
        "vi",
        "--format",
        "web-vtt",
    ]);

    assert!(!output.status.success());
    assert!(stderr(&output).contains("available: srt"), "{output:?}");
}

#[test]
fn an_ambiguous_language_without_a_flag_is_an_error_when_not_interactive() {
    let env = Env::new();
    env.add_video();
    env.add_library_file(&format!("{VIDEO_TITLE}.mkv"));
    env.add_library_file(&format!("{VIDEO_TITLE}.vi.srt"));
    env.add_library_file(&format!("{VIDEO_TITLE}.zh.srt"));

    // Two languages are available and stdin is not a terminal, so the
    // program cannot prompt and must fail.
    let output = env.run(["--title", "example"]);

    assert!(!output.status.success());
    assert!(
        stderr(&output).contains("stdin is not a terminal"),
        "{output:?}",
    );
}

#[test]
fn a_missing_video_file_is_an_error() {
    let env = Env::new();
    env.add_video();
    // Only the subtitle exists; there is no playable video file.
    env.add_library_file(&format!("{VIDEO_TITLE}.vi.srt"));

    let output = env.run([
        "--title",
        "example",
        "--language",
        "vi",
        "--format",
        "srt",
        "--player",
        "mpv",
    ]);

    assert!(!output.status.success());
    assert!(stderr(&output).contains("no video file"), "{output:?}");
}

#[test]
fn an_invalid_player_value_is_rejected_by_clap() {
    let env = Env::new();
    env.add_video();
    env.add_library_file(&format!("{VIDEO_TITLE}.mkv"));
    env.add_library_file(&format!("{VIDEO_TITLE}.vi.srt"));

    // "vlc" is not one of the accepted players, so clap rejects the value.
    let output = env.run(["--title", "example", "--player", "vlc"]);

    assert!(!output.status.success());
    assert!(stderr(&output).contains("vlc"), "{output:?}");
}

#[cfg(unix)]
#[test]
fn launches_mpv_with_the_resolved_files() {
    let env = Env::new();
    env.add_video();
    env.add_library_file(&format!("{VIDEO_TITLE}.mkv"));
    env.add_library_file(&format!("{VIDEO_TITLE}.vi.srt"));
    env.install_fake_players();

    let (output, recorded) = env.run_played([
        "--title",
        "example",
        "--language",
        "vi",
        "--format",
        "srt",
        "--player",
        "mpv",
    ]);

    assert!(output.status.success(), "{output:?}");
    let video = env.library_path(&format!("{VIDEO_TITLE}.mkv"));
    let subtitle = env.library_path(&format!("{VIDEO_TITLE}.vi.srt"));
    assert_eq!(
        recorded,
        vec![
            format!("--sub-file={}", subtitle.display()),
            video.display().to_string(),
        ],
    );
}

#[cfg(unix)]
#[test]
fn launches_celluloid_with_the_mpv_prefixed_flag() {
    let env = Env::new();
    env.add_video();
    env.add_library_file(&format!("{VIDEO_TITLE}.mkv"));
    env.add_library_file(&format!("{VIDEO_TITLE}.zh.vtt"));
    env.install_fake_players();

    let (output, recorded) = env.run_played([
        "--title",
        "example",
        "--language",
        "zh",
        "--format",
        "vtt",
        "--player",
        "celluloid",
    ]);

    assert!(output.status.success(), "{output:?}");
    let video = env.library_path(&format!("{VIDEO_TITLE}.mkv"));
    let subtitle = env.library_path(&format!("{VIDEO_TITLE}.zh.vtt"));
    assert_eq!(
        recorded,
        vec![
            format!("--mpv-sub-file={}", subtitle.display()),
            video.display().to_string(),
        ],
    );
}

#[cfg(unix)]
#[test]
fn a_single_language_and_format_are_selected_automatically() {
    let env = Env::new();
    env.add_video();
    env.add_library_file(&format!("{VIDEO_TITLE}.mkv"));
    env.add_library_file(&format!("{VIDEO_TITLE}.vi.srt"));
    env.install_fake_players();

    // Only one language and format are available, so neither flag is needed.
    let (output, recorded) = env.run_played(["--title", "example", "--player", "mpv"]);

    assert!(output.status.success(), "{output:?}");
    let subtitle = env.library_path(&format!("{VIDEO_TITLE}.vi.srt"));
    assert_eq!(recorded[0], format!("--sub-file={}", subtitle.display()));
}
