//! End-to-end cases that run all the way through to launching a player,
//! verified against fake player programs on `PATH`. The fake programs are
//! shell scripts, so the whole suite is Unix-only.

#![cfg(unix)]

pub mod _utils;
pub use _utils::*;

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
