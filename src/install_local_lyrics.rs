use crate::file_snapshot::FileSnapshot;
use crate::video_descriptor::{
    LyricsFileName, ParseLyricsFileNameError, SEPARATED_COLLECTIONS, UNIFIED_COLLECTION,
    VIDEO_CONFIG_FILE_NAME, VideoDesc, Visibility,
};
use clap::Parser;
use itertools::Itertools;
use pipe_trait::Pipe;
use reflink::reflink_or_copy;
use std::collections::{HashMap, HashSet};
use std::fs::{DirEntry, hard_link, read_dir, read_to_string, remove_file};
use std::io::{self, ErrorKind};
use std::iter::once;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Parser)]
#[clap(about = "Synchronize the lyrics")]
struct Args {
    /// For safety reasons, this programs list actions by default, this flag makes the program take those actions.
    #[clap(long, short = 'x')]
    execute: bool,

    /// Source directory of the subtitles.
    source: PathBuf,

    /// Container of the target directories of the subtitles.
    target: PathBuf,
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
        Err(error) => panic!(
            "error: Cannot read file type of {:?}: {error}",
            entry.path()
        ),
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
            let desc_path = video_dir.join(VIDEO_CONFIG_FILE_NAME);
            let content = desc_path
                .pipe_ref(read_to_string)
                .unwrap_or_else(|error| panic!("error: Cannot read {desc_path:?}: {error}"));
            let desc: VideoDesc = content
                .pipe_as_ref(toml::from_str)
                .unwrap_or_else(|error| panic!("error: Cannot parse {desc_path:?}: {error}"));
            (video_dir, desc)
        })
        .collect(); // eagerly validate all video descriptors before touching any files

    let existing_target_files: HashMap<PathBuf, FileSnapshot> = SEPARATED_COLLECTIONS
        .iter()
        .copied()
        .chain(once(UNIFIED_COLLECTION))
        .map(|suffix| target.join(suffix))
        .flat_map(|path| {
            path.pipe_ref(read_dir)
                .unwrap_or_else(|error| panic!("error: Cannot read directory {path:?}: {error}"))
                .map(move |entry| {
                    entry.unwrap_or_else(|error| {
                        panic!("error: Cannot read an entry of directory {path:?}: {error}")
                    })
                })
                .filter(is_subtitle_file)
                .map(|entry| entry.path())
        })
        .map(|path| {
            let snapshot = path
                .to_path_buf()
                .pipe(FileSnapshot::new)
                .unwrap_or_else(|error| panic!("error: Cannot read file {path:?}: {error}"));
            (path, snapshot)
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
                let name = target_path
                    .file_name()
                    .expect("target path has no file name")
                    .to_str()
                    .unwrap_or_else(|| {
                        panic!("error: Non-UTF-8 filename in target: {target_path:?}")
                    });
                !name.starts_with(&prefix)
            });
            continue;
        }

        let source_entries = video_dir
            .pipe(read_dir)
            .unwrap_or_else(|error| panic!("error: Cannot read directory {video_dir:?}: {error}"));

        for source_entry in source_entries {
            let source_entry = source_entry.unwrap_or_else(|error| {
                panic!("error: Cannot read an entry of directory {video_dir:?}: {error}")
            });
            if !is_subtitle_file(&source_entry) {
                continue;
            }

            let local_name = source_entry.file_name();
            let local_name = local_name
                .to_str()
                .unwrap_or_else(|| panic!("error: Non-UTF-8 filename in {video_dir:?}"));

            let lyrics = match local_name.parse::<LyricsFileName>() {
                Ok(lyrics) => lyrics,
                Err(ParseLyricsFileNameError::NotLyricsFile) => continue,
                Err(error) => panic!("error: {}/{local_name}: {error}", video_dir.display()),
            };
            let target_name = lyrics.target_file_name(&desc.video_title).to_string();

            let source_file = video_dir.join(local_name);
            let separated_target_file = separated_target_dir.join(&target_name);
            let unified_target_file = unified_target_dir.join(&target_name);

            let source_file_snapshot = source_file.clone().pipe(FileSnapshot::new);
            for target_file in [separated_target_file, unified_target_file] {
                let Some(target_file_snapshot) = existing_target_files.get(&target_file) else {
                    files_need_install.push((source_file.clone(), target_file));
                    continue;
                };

                let source_file_snapshot = source_file_snapshot.as_ref().unwrap_or_else(|error| {
                    panic!("error: Cannot read file {:?}: {error}", source_file.clone())
                });

                debug_assert!(
                    files_need_uninstall.remove(&target_file),
                    "Expecting {target_file:?} to still exist but it doesn't"
                );

                if target_file_snapshot.content_eq(source_file_snapshot) {
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
