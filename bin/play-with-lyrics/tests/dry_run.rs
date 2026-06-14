//! End-to-end tests that drive the compiled binary with `--dry-run`, so
//! they verify the resolution of titles, languages, formats, and players
//! without launching a real media player.

// cspell:ignore bài hát ví dụ

use command_extra::CommandExtra;
use pretty_assertions::assert_eq;
use std::ffi::OsStr;
use std::fs::{create_dir, create_dir_all, write as write_file};
use std::path::{Path, PathBuf};
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
            "collection = \"{COLLECTION}\"\n\
             video-title = \"{VIDEO_TITLE}\"\n\
             \n\
             [song-titles]\n\
             en = \"Example Song\"\n\
             vi = \"Bài Hát Ví Dụ\"\n\
             zh = \"示例歌曲\"\n",
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
    fn add_library_file(&self, file_name: &str) -> PathBuf {
        let path = self.collection_dir().join(file_name);
        write_file(&path, "").unwrap();
        path
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
            // A null stdin guarantees the program treats the session as
            // non-interactive, so a missing selection becomes an error
            // rather than a blocked read.
            .with_stdin(Stdio::null())
            .output()
            .expect("failed to spawn play-with-lyrics")
    }
}

fn stdout_lines(output: &Output) -> Vec<String> {
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::to_string)
        .collect()
}

fn path_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

#[test]
fn fully_specified_invocation_builds_an_mpv_command() {
    let env = Env::new();
    env.add_video();
    let video = env.add_library_file(&format!("{VIDEO_TITLE}.mkv"));
    let subtitle = env.add_library_file(&format!("{VIDEO_TITLE}.vi.srt"));

    let output = env.run([
        "--title",
        "example",
        "--language",
        "vi",
        "--format",
        "srt",
        "--player",
        "mpv",
        "--dry-run",
    ]);

    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        stdout_lines(&output),
        vec![
            "mpv".to_string(),
            format!("--sub-file={}", path_string(&subtitle)),
            path_string(&video),
        ],
    );
}

#[test]
fn celluloid_receives_the_mpv_prefixed_flag() {
    let env = Env::new();
    env.add_video();
    env.add_library_file(&format!("{VIDEO_TITLE}.mkv"));
    let subtitle = env.add_library_file(&format!("{VIDEO_TITLE}.zh.vtt"));

    let output = env.run([
        "--title",
        "example",
        "--language",
        "zh",
        "--format",
        "vtt",
        "--player",
        "cell",
        "--dry-run",
    ]);

    assert!(output.status.success(), "{output:?}");
    let lines = stdout_lines(&output);
    assert_eq!(lines[0], "celluloid");
    assert_eq!(
        lines[1],
        format!("--mpv-sub-file={}", path_string(&subtitle)),
    );
}

#[test]
fn a_fuzzy_title_resolves_to_the_single_video() {
    let env = Env::new();
    env.add_video();
    env.add_library_file(&format!("{VIDEO_TITLE}.mkv"));
    env.add_library_file(&format!("{VIDEO_TITLE}.vi.srt"));

    // "ample" matches part of the "Example Song" title without being typed
    // in full.
    let output = env.run(["--title", "ample", "--player", "mpv", "--dry-run"]);

    assert!(output.status.success(), "{output:?}");
    assert_eq!(stdout_lines(&output)[0], "mpv");
}

#[test]
fn a_single_language_and_format_are_selected_automatically() {
    let env = Env::new();
    env.add_video();
    env.add_library_file(&format!("{VIDEO_TITLE}.mkv"));
    let subtitle = env.add_library_file(&format!("{VIDEO_TITLE}.vi.srt"));

    // Neither --language nor --format is given, but only one of each is
    // available, so the program picks them without prompting.
    let output = env.run(["--title", "example", "--player", "mpv", "--dry-run"]);

    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        stdout_lines(&output)[1],
        format!("--sub-file={}", path_string(&subtitle)),
    );
}

#[test]
fn an_unmatched_title_is_an_error() {
    let env = Env::new();
    env.add_video();
    env.add_library_file(&format!("{VIDEO_TITLE}.mkv"));
    env.add_library_file(&format!("{VIDEO_TITLE}.vi.srt"));

    let output = env.run(["--title", "nonexistent", "--player", "mpv", "--dry-run"]);

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("no candidate matched"), "{stderr}");
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
    let output = env.run(["--title", "example", "--player", "mpv", "--dry-run"]);

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("stdin is not a terminal"), "{stderr}");
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
        "--dry-run",
    ]);

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("no video file"), "{stderr}");
}
