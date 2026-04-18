use crate::video_descriptor::{VIDEO_CONFIG_FILE_NAME, VideoDesc};
use clap::Parser;
use command_extra::CommandExtra;
use pipe_trait::Pipe;
use std::fs::{read_dir, read_to_string};
use std::path::{Path, PathBuf};
use std::process::Command;

/// The video player to use when launching a video with subtitles.
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum Player {
    /// MPV media player.
    Mpv,
    /// Celluloid (GNOME MPV) media player.
    Celluloid,
}

#[derive(Debug, Clone, Parser)]
#[clap(about = "Play a video with its translated subtitle files")]
struct Args {
    /// Source directory containing video.toml and subtitle files.
    source: PathBuf,

    /// Media player to launch.
    #[clap(long, default_value = "mpv")]
    player: Player,

    /// Local video file to play. When omitted, the video is streamed
    /// from YouTube using the ID embedded in the video title.
    #[clap(long)]
    video: Option<PathBuf>,
}

/// Extracts the YouTube video ID from a video title.
///
/// Video titles in this repository end with `[<id>]`, for example:
/// `【洛天依&乐正绫】云边梦话 [i-hmLz5bslY]`.
pub fn extract_youtube_id(video_title: &str) -> Option<&str> {
    let close = video_title.rfind(']')?;
    let open = video_title[..close].rfind('[')?;
    let id = &video_title[open + 1..close];
    if id.is_empty() { None } else { Some(id) }
}

fn collect_srt_files(source: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = source
        .pipe_ref(read_dir)
        .unwrap_or_else(|error| panic!("error: Cannot read directory {source:?}: {error}"))
        .map(|entry| {
            entry.unwrap_or_else(|error| {
                panic!("error: Cannot read directory entry in {source:?}: {error}")
            })
        })
        .filter(|entry| {
            entry
                .path()
                .extension()
                .map(|ext| ext == "srt")
                .unwrap_or(false)
        })
        .map(|entry| entry.path())
        .collect();
    files.sort();
    files
}

pub fn main() {
    let Args {
        source,
        player,
        video,
    } = Args::parse();

    let source = source
        .canonicalize()
        .unwrap_or_else(|error| panic!("error: Cannot resolve path {source:?}: {error}"));

    let desc_path = source.join(VIDEO_CONFIG_FILE_NAME);
    let desc_content = desc_path
        .pipe_ref(read_to_string)
        .unwrap_or_else(|error| panic!("error: Cannot read {desc_path:?}: {error}"));
    let desc: VideoDesc = desc_content
        .pipe_as_ref(toml::from_str)
        .unwrap_or_else(|error| panic!("error: Cannot parse {desc_path:?}: {error}"));

    let video_uri: String = match video {
        Some(video_file) => video_file
            .canonicalize()
            .unwrap_or_else(|error| {
                panic!("error: Cannot resolve video file path {video_file:?}: {error}")
            })
            .to_string_lossy()
            .into_owned(),
        None => extract_youtube_id(&desc.video_title)
            .map(|id| format!("https://youtu.be/{id}"))
            .unwrap_or_else(|| {
                panic!(
                    "error: Cannot find YouTube ID in video title: {:?}",
                    &*desc.video_title
                )
            }),
    };

    let srt_files = collect_srt_files(&source);
    eprintln!("info: Using {} subtitle file(s)", srt_files.len());
    for srt_file in &srt_files {
        eprintln!("info:   {srt_file:?}");
    }

    let status = match player {
        Player::Mpv => {
            eprintln!("info: Launching mpv with {video_uri:?}");
            srt_files
                .iter()
                .fold(Command::new("mpv"), |acc, srt_file| {
                    acc.with_arg(format!("--sub-file={}", srt_file.display()))
                })
                .with_arg(&video_uri)
                .status()
        }
        Player::Celluloid => {
            eprintln!("info: Launching celluloid with {video_uri:?}");
            // Celluloid forwards options prefixed with --mpv- to the underlying
            // mpv instance after stripping the prefix.
            srt_files
                .iter()
                .fold(Command::new("celluloid"), |acc, srt_file| {
                    acc.with_arg(format!("--mpv-sub-file={}", srt_file.display()))
                })
                .with_arg(&video_uri)
                .status()
        }
    }
    .unwrap_or_else(|error| panic!("error: Failed to launch player: {error}"));

    if !status.success() {
        eprintln!("warning: Player exited with non-zero status: {status}");
    }
}

#[cfg(test)]
mod tests {
    use super::extract_youtube_id;

    #[test]
    fn extracts_id_from_typical_title() {
        let title =
            "【洛天依&乐正绫】云边梦话(Cloudside Dreams)【原创PV付】【南北组原创】 [i-hmLz5bslY]";
        assert_eq!(extract_youtube_id(title), Some("i-hmLz5bslY"));
    }

    #[test]
    fn extracts_id_from_simple_title() {
        assert_eq!(
            extract_youtube_id("Song Title [AbCdEfGhIjK]"),
            Some("AbCdEfGhIjK")
        );
    }

    #[test]
    fn returns_none_for_empty_brackets() {
        assert_eq!(extract_youtube_id("Song Title []"), None);
    }

    #[test]
    fn returns_none_for_no_brackets() {
        assert_eq!(extract_youtube_id("Song Title without ID"), None);
    }

    #[test]
    fn uses_last_bracket_pair() {
        assert_eq!(
            extract_youtube_id("【Tag】Song Title [AbCdEfGhIjK]"),
            Some("AbCdEfGhIjK"),
        );
    }
}
