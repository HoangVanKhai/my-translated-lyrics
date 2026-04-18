use crate::video_descriptor::{Language, SubtitleFormat, VIDEO_CONFIG_FILE_NAME, VideoDesc};
use clap::{Parser, ValueEnum};
use command_extra::CommandExtra;
use pipe_trait::Pipe;
use std::fs::{read_dir, read_to_string};
use std::path::{Path, PathBuf};
use std::process::Command;

/// The video player to use when launching a video with subtitles.
#[derive(Debug, Clone, Copy, ValueEnum, strum::Display)]
pub enum Player {
    /// MPV media player.
    #[strum(serialize = "mpv")]
    Mpv,
    /// Celluloid (GNOME MPV) media player.
    #[strum(serialize = "celluloid")]
    Celluloid,
}

/// Video file extensions that the player can open.
const VIDEO_EXTENSIONS: &[&str] = &["mp4", "mkv", "webm", "avi", "m4v", "mov"];

#[derive(Debug, Clone, Parser)]
#[clap(about = "Play a video with its translated subtitle files")]
struct Args {
    /// Media player to launch.
    #[clap(long, default_value_t = Player::Mpv)]
    player: Player,

    /// Language of the subtitle to load.
    #[clap(long, short)]
    language: Language,

    /// Format of the subtitle file to load.
    #[clap(long, short)]
    format: SubtitleFormat,

    /// Source directory containing video.toml.
    source: PathBuf,

    /// Container of the target directories of the subtitles and video files.
    target: PathBuf,
}

/// Finds a video file in `collection_dir` whose stem exactly matches
/// `video_title` and whose extension is one of [`VIDEO_EXTENSIONS`].
fn find_video_file(collection_dir: &Path, video_title: &str) -> PathBuf {
    read_dir(collection_dir)
        .unwrap_or_else(|error| panic!("error: Cannot read directory {collection_dir:?}: {error}"))
        .map(|entry| {
            entry.unwrap_or_else(|error| {
                panic!("error: Cannot read an entry of directory {collection_dir:?}: {error}")
            })
        })
        .map(|entry| entry.path())
        .find(|path| {
            let Some(stem) = path.file_stem() else {
                return false;
            };
            let Some(ext) = path.extension() else {
                return false;
            };
            let stem = stem
                .to_str()
                .unwrap_or_else(|| panic!("error: Non-UTF-8 filename in {collection_dir:?}"));
            let ext = ext
                .to_str()
                .unwrap_or_else(|| panic!("error: Non-UTF-8 filename in {collection_dir:?}"));
            stem == video_title && VIDEO_EXTENSIONS.contains(&ext)
        })
        .unwrap_or_else(|| {
            panic!(
                "error: No video file found for {video_title:?} in {collection_dir:?} (tried extensions: {})",
                VIDEO_EXTENSIONS.join(", "),
            )
        })
}

pub fn main() {
    let Args {
        source,
        target,
        player,
        language,
        format,
    } = Args::parse();

    let source = source
        .canonicalize()
        .unwrap_or_else(|error| panic!("error: Cannot resolve path {source:?}: {error}"));
    let target = target
        .canonicalize()
        .unwrap_or_else(|error| panic!("error: Cannot resolve path {target:?}: {error}"));

    let desc_path = source.join(VIDEO_CONFIG_FILE_NAME);
    let desc_content = desc_path
        .pipe_ref(read_to_string)
        .unwrap_or_else(|error| panic!("error: Cannot read {desc_path:?}: {error}"));
    let desc: VideoDesc = desc_content
        .pipe_as_ref(toml::from_str)
        .unwrap_or_else(|error| panic!("error: Cannot parse {desc_path:?}: {error}"));

    let collection_dir = target.join(&*desc.collection);

    let video_file = find_video_file(&collection_dir, &desc.video_title);

    let subtitle_name = format!("{}.{language}.{format}", desc.video_title);
    let subtitle_file = collection_dir.join(&subtitle_name);
    if !subtitle_file.exists() {
        panic!("error: Subtitle file not found: {subtitle_file:?}");
    }
    let subtitle_path_str = subtitle_file
        .to_str()
        .unwrap_or_else(|| panic!("error: Non-UTF-8 path: {subtitle_file:?}"));

    eprintln!("info: Video file: {video_file:?}");
    eprintln!("info: Subtitle file: {subtitle_file:?}");

    let status = match player {
        Player::Mpv => {
            eprintln!("info: Launching mpv");
            // mpv accepts --sub-file=<path> to load an external subtitle file.
            Command::new("mpv")
                .with_arg(format!("--sub-file={subtitle_path_str}"))
                .with_arg(&video_file)
                .status()
        }
        Player::Celluloid => {
            eprintln!("info: Launching celluloid");
            // Celluloid accepts --mpv-<option>=<value> and forwards the option
            // to the underlying mpv instance after stripping the --mpv- prefix.
            Command::new("celluloid")
                .with_arg(format!("--mpv-sub-file={subtitle_path_str}"))
                .with_arg(&video_file)
                .status()
        }
    }
    .unwrap_or_else(|error| panic!("error: Failed to launch player: {error}"));

    if !status.success() {
        eprintln!("warning: Player exited with non-zero status: {status}");
    }
}
