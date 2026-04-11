use crate::args::Args;
use crate::file_descriptor::FileDescriptor;
use clap::Parser;
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
    collection: String,
    /// Title of the YouTube video this subtitle set translates. Used as
    /// the stem of target subtitle filenames: the final file is
    /// `{video_title}.{lang}.{ext}` (e.g. `{video_title}.vi.srt`).
    ///
    /// Not to be confused with the *song's title*. For example, the video
    /// `【洛天依&乐正绫】【中秋原创】《月轮回》(Lunar Cycle) 命运是为何物【PV付】 [MLG8OlppS9o]`
    /// has `月轮回` as the song's title and `Lunar Cycle` as the song's
    /// translated title.
    video_title: String,
    #[serde(rename = "song-titles")]
    #[expect(
        dead_code,
        reason = "parsed for documentation, not consumed by the tool"
    )]
    song_titles: HashMap<Language, String>,
    #[serde(default)]
    visibility: Visibility,
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

fn validate_video_desc(desc: &VideoDesc, desc_path: &Path) {
    // collection must be one of the known managed collections
    if !SEPARATED_COLLECTIONS.contains(&desc.collection.as_str()) {
        panic!(
            "error: unknown collection in {desc_path:?}: {:?}",
            desc.collection
        );
    }

    // video_title must be a single normal path component with no separators.
    // Backslashes are rejected explicitly so configs behave consistently
    // regardless of platform (on Unix, `Path::components` treats `\` as a
    // normal character).
    let mut title_components = desc.video_title.pipe_ref(Path::new).components();
    let has_valid_shape = matches!(
        (title_components.next(), title_components.next()),
        (Some(Component::Normal(_)), None)
    ) && !desc.video_title.contains('\\');
    if !has_valid_shape {
        panic!(
            "error: invalid video_title in {desc_path:?}: {:?}",
            desc.video_title
        );
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
    let video_descs: Vec<(PathBuf, VideoDesc)> = source
        .pipe_ref(read_dir)
        .unwrap_or_else(|error| panic!("error: Cannot read source directory {source:?}: {error}"))
        .map(|entry| {
            entry.unwrap_or_else(|error| {
                panic!("error: Cannot read an entry of directory {source:?}: {error}")
            })
        })
        .filter(|entry| entry.file_type().is_ok_and(|file_type| file_type.is_dir()))
        .map(|entry| {
            let video_dir = entry.path();
            let desc_path = video_dir.join(VIDEO_CONFIG_FILENAME);
            let content = desc_path
                .pipe_ref(read_to_string)
                .unwrap_or_else(|error| panic!("error: Cannot read {desc_path:?}: {error}"));
            let desc: VideoDesc = content
                .pipe_as_ref(toml::from_str)
                .unwrap_or_else(|error| panic!("error: Cannot parse {desc_path:?}: {error}"));
            validate_video_desc(&desc, &desc_path);
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

    for (video_dir, desc) in &video_descs {
        // Hidden: do nothing. Any existing target files stay in
        // `files_need_uninstall` and will be removed.
        if desc.visibility == Visibility::Hidden {
            continue;
        }

        let separated_target_dir = target.join(&desc.collection);
        let unified_target_dir = target.join(UNIFIED_COLLECTION);

        // Manual: target files are managed externally. Protect every target
        // file that matches this video's title prefix under either target
        // directory from being uninstalled, and do not install anything.
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

            // Map lyrics.{lang}.{ext} → {desc.video_title}.{lang}.{ext}
            let target_name = local_name
                .strip_prefix("lyrics.")
                .map(|suffix| format!("{}.{suffix}", desc.video_title))
                .unwrap_or_else(|| local_name.to_owned());

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
