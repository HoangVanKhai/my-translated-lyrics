//! End-to-end tests that drive the compiled binary. Every case exercises a
//! path that fails before any media player is launched, so the tests never
//! spawn an external process. The successful path (which would launch a
//! player) is covered by the unit tests of the library crates.

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
            // A null stdin guarantees the program treats the session as
            // non-interactive, so a missing selection becomes an error
            // rather than a blocked read.
            .with_stdin(Stdio::null())
            .output()
            .expect("failed to spawn play-with-lyrics")
    }
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
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
    // language name is given here, exercising the canonical value name.
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
    // "vi" alias and the canonical "web-vtt" name are exercised here.
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
