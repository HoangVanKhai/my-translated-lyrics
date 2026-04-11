use crate::args::Args;
use crate::file_descriptor::FileDescriptor;
use clap::Parser;
use derive_more::{AsRef, Deref, Display, Error, Into};
use itertools::Itertools;
use pipe_trait::Pipe;
use reflink::reflink_or_copy;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::fs::{DirEntry, hard_link, read_dir, read_to_string, remove_file};
use std::io::{self, ErrorKind};
use std::iter::once;
use std::os::unix::ffi::OsStrExt;
use std::path::{Component, Path, PathBuf};

const SEPARATED_COLLECTIONS: &[&str] = &[
    "Feng Ling Yu Xiu",
    "Luo Tianyi, Yuezheng Ling/洛天依_乐正绫",
    "Touhou Hero of Ice Fairy",
];

const UNIFIED_COLLECTION: &str = "Short Relaxing Playlist 2025";

const VIDEO_CONFIG_FILENAME: &str = "video.toml";

#[derive(Deserialize)]
struct VideoDesc {
    collection: Collection,
    /// Title of the YouTube video this subtitle set translates. Used as
    /// the stem of target subtitle filenames.
    video_title: VideoTitle,
    /// Titles of the song in each supported language.
    #[serde(rename = "song-titles")]
    #[expect(dead_code, reason = "not used for now, may be used in the future")]
    song_titles: HashMap<Language, String>,
    /// Controls how the tool treats this video's target subtitle files.
    /// See [`Visibility`] for details.
    #[serde(default)]
    visibility: Visibility,
}

/// Target collection path. Only values listed in [`SEPARATED_COLLECTIONS`]
/// can construct this type.
///
/// The inner value is an owned `String` rather than `&'static str` even
/// though every valid value is statically known today. This leaves room
/// to replace the hard-coded [`SEPARATED_COLLECTIONS`] list with a runtime
/// source later without changing the type's shape.
#[derive(Deserialize, AsRef, Deref, Display, Into)]
#[as_ref(forward)]
#[deref(forward)]
#[serde(try_from = "String")]
struct Collection(String);

impl TryFrom<String> for Collection {
    type Error = UnknownCollection;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if SEPARATED_COLLECTIONS.contains(&value.as_str()) {
            Ok(Self(value))
        } else {
            Err(UnknownCollection(value))
        }
    }
}

#[derive(Debug, Display, Error)]
#[display("unknown collection: {_0:?}")]
struct UnknownCollection(#[error(not(source))] String);

/// Title of a YouTube video. The constructor enforces that the value is a
/// single normal path component with no backslashes, so it can be used
/// directly as the stem of an output filename.
#[derive(Deserialize, AsRef, Deref, Display, Into)]
#[as_ref(forward)]
#[deref(forward)]
#[serde(try_from = "String")]
struct VideoTitle(String);

impl TryFrom<String> for VideoTitle {
    type Error = VideoTitleError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        // Backslashes are rejected explicitly so configs behave consistently
        // regardless of platform (on Unix, `Path::components` treats `\` as
        // a normal character).
        if value.contains('\\') {
            return Err(VideoTitleError::ContainsBackslash);
        }
        let mut components = value.pipe_ref(Path::new).components();
        match (components.next(), components.next()) {
            (Some(Component::Normal(_)), None) => Ok(Self(value)),
            _ => Err(VideoTitleError::NotSingleComponent),
        }
    }
}

#[derive(Debug, Display, Error)]
enum VideoTitleError {
    #[display("video_title must not contain backslashes")]
    ContainsBackslash,
    #[display("video_title must be a single normal path component")]
    NotSingleComponent,
}

#[derive(Deserialize, Eq, PartialEq, Hash)]
enum Language {
    #[serde(rename = "en")]
    English,
    #[serde(rename = "vi")]
    Vietnamese,
    #[serde(rename = "zh")]
    Chinese,
}

#[derive(Default, Deserialize, PartialEq, Eq)]
enum Visibility {
    /// The target subtitle files should be created and
    /// synchronized with the source.
    #[default]
    #[serde(rename = "visible")]
    Visible,
    /// The target subtitle files should not be there despite
    /// the existence of the source.
    #[serde(rename = "hidden")]
    Hidden,
    /// The target subtitle files are managed externally; they shall
    /// neither be deleted nor created nor synchronized.
    #[serde(rename = "manual")]
    Manual,
}

/// Try hardlink, then try reflink, and finally copy.
fn link_or_copy(source: &Path, target: &Path) -> io::Result<()> {
    if hard_link(source, target).is_ok() {
        return Ok(());
    }

    reflink_or_copy(source, target)?;

    Ok(())
}

fn uninstall(execute: bool, target: &Path) {
    eprintln!("remove {target:?}");
    if execute {
        remove_file(target).unwrap();
    }
}

fn install(execute: bool, source: &Path, target: &Path) {
    eprintln!("copy {source:?} → {target:?}");
    if execute {
        if let Err(error) = remove_file(target)
            && error.kind() != ErrorKind::NotFound
        {
            eprintln!("warning: Cannot remove file {target:?}: {error}");
        }

        // Q: Why try hardlink before reflink?
        // A: It'd be convenient not having to re-run the script
        //    just to update the subtitles.
        link_or_copy(source, target).unwrap();
    }
}

fn is_subtitle_file(entry: &DirEntry) -> bool {
    match entry.file_type() {
        Err(_) => return false,
        Ok(file_type) if !file_type.is_file() => return false,
        Ok(file_type) => debug_assert!(file_type.is_file()),
    }

    let file_name = entry.file_name();
    let file_name = file_name.as_bytes();
    file_name.ends_with(b".srt") || file_name.ends_with(b".vtt")
}

pub fn main() {
    let Args {
        execute,
        source,
        target,
    } = Args::parse();

    // Read all video descriptors from source directories
    let descriptors: Vec<(PathBuf, VideoDesc)> = source
        .pipe_ref(read_dir)
        .unwrap_or_else(|error| panic!("error: Cannot read source directory {source:?}: {error}"))
        .map(|entry| {
            entry.unwrap_or_else(|error| {
                panic!("error: Cannot read an entry of directory {source:?}: {error}")
            })
        })
        .filter(|entry| {
            entry
                .file_type()
                .unwrap_or_else(|error| {
                    panic!(
                        "error: Cannot read file type of {:?}: {error}",
                        entry.path()
                    )
                })
                .is_dir()
        })
        .map(|entry| {
            let video_dir = entry.path();
            let desc_path = video_dir.join(VIDEO_CONFIG_FILENAME);
            let content = desc_path
                .pipe_ref(read_to_string)
                .unwrap_or_else(|error| panic!("error: Cannot read {desc_path:?}: {error}"));
            let desc: VideoDesc = content
                .pipe_as_ref(toml::from_str)
                .unwrap_or_else(|error| panic!("error: Cannot parse {desc_path:?}: {error}"));
            (video_dir, desc)
        })
        .collect();

    let existing_target_files: HashMap<PathBuf, FileDescriptor> = SEPARATED_COLLECTIONS
        .iter()
        .copied()
        .chain(once(UNIFIED_COLLECTION))
        .map(|suffix| target.join(suffix))
        .flat_map(|ref path| {
            path.pipe(read_dir)
                .unwrap_or_else(|error| panic!("error: Cannot read directory {path:?}: {error}"))
                .flatten()
                .filter(is_subtitle_file)
                .map(|entry| entry.path())
        })
        .map(|path| {
            let desc = path
                .to_path_buf()
                .pipe(FileDescriptor::new)
                .unwrap_or_else(|error| panic!("error: Cannot read file {path:?}: {error}"));
            (path, desc)
        })
        .collect();
    eprintln!(
        "info: There are currently {} existing files at the target location",
        existing_target_files.len()
    );

    let mut files_need_update: Vec<(PathBuf, PathBuf)> =
        Vec::with_capacity(existing_target_files.len());
    let mut files_need_uninstall: HashSet<&PathBuf> = existing_target_files.keys().collect();
    let mut files_need_install: Vec<(PathBuf, PathBuf)> =
        Vec::with_capacity(existing_target_files.len());

    for (video_dir, desc) in &descriptors {
        // Hidden: do nothing. Any existing target files stay in
        // `files_need_uninstall` and will be removed.
        if desc.visibility == Visibility::Hidden {
            continue;
        }

        let separated_target_dir = target.join(&desc.collection);
        let unified_target_dir = target.join(UNIFIED_COLLECTION);

        if desc.visibility == Visibility::Manual {
            let prefix = format!("{}.", desc.video_title);
            let separated = separated_target_dir.as_path();
            let unified = unified_target_dir.as_path();
            files_need_uninstall.retain(|target_path| {
                let Some(parent) = target_path.parent() else {
                    return true;
                };
                if parent != separated && parent != unified {
                    return true;
                }
                let Some(name) = target_path.file_name().and_then(|n| n.to_str()) else {
                    return true;
                };
                !name.starts_with(&prefix)
            });
            continue;
        }

        let source_entries = video_dir
            .pipe(read_dir)
            .unwrap_or_else(|error| panic!("error: Cannot read directory {video_dir:?}: {error}"));

        for source_entry in source_entries {
            let Ok(source_entry) = source_entry else {
                continue;
            };
            if !is_subtitle_file(&source_entry) {
                continue;
            }

            let local_name = source_entry.file_name();
            let local_name = local_name
                .to_str()
                .unwrap_or_else(|| panic!("error: Non-UTF-8 filename in {video_dir:?}"));

            // Map lyrics.{lang}.{ext} → {video_title}.{lang}.{ext}
            let Some(suffix) = local_name.strip_prefix("lyrics.") else {
                continue;
            };
            let Some((lang, ext)) = suffix.rsplit_once('.') else {
                continue;
            };
            if !matches!(ext, "srt" | "vtt") {
                continue;
            }
            if lang.is_empty() || !lang.chars().all(|c| c.is_ascii_lowercase()) {
                continue;
            }
            let target_name = format!("{}.{suffix}", desc.video_title);

            let source_file = video_dir.join(local_name);
            let separated_target_file = separated_target_dir.join(&target_name);
            let unified_target_file = unified_target_dir.join(&target_name);

            let source_file_desc = source_file.clone().pipe(FileDescriptor::new);
            for target_file in [separated_target_file, unified_target_file] {
                let Some(target_file_desc) = existing_target_files.get(&target_file) else {
                    files_need_install.push((source_file.clone(), target_file));
                    continue;
                };

                let source_file_desc = source_file_desc.as_ref().unwrap_or_else(|error| {
                    panic!("error: Cannot read file {:?}: {error}", source_file.clone())
                });

                debug_assert!(
                    files_need_uninstall.remove(&target_file),
                    "Expecting {target_file:?} to still exist but it doesn't"
                );

                if target_file_desc.content_eq(source_file_desc) {
                    continue;
                }

                files_need_update.push((source_file.clone(), target_file));
            }
        }
    }

    eprintln!(
        "info: {} files would be removed from the target location",
        files_need_uninstall.len()
    );
    eprintln!(
        "info: {} files would be added to the target location",
        files_need_install.len()
    );
    eprintln!(
        "info: {} files in the target location would be updated",
        files_need_update.len()
    );

    eprintln!();
    eprintln!("stage: Removing old subtitles");
    for target in files_need_uninstall.iter().sorted() {
        uninstall(execute, target);
    }

    eprintln!();
    eprintln!("stage: Adding new subtitles");
    for (source, target) in files_need_install.iter().sorted() {
        install(execute, source, target);
    }

    eprintln!();
    eprintln!("stage: Updating outdated subtitles");
    for (source, target) in files_need_update.iter().sorted() {
        install(execute, source, target);
    }

    if !execute {
        eprintln!();
        eprintln!("info: No changes were actually made.");
        eprintln!("info: Run the command again with --execute to make actual changes.");
    }
}
