//! Cases that fail before any media player is launched.

use crate::env::{Env, VIDEO_TITLE, stderr};

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
