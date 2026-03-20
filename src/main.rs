mod args;
mod file_descriptor;

use args::Args;
use clap::Parser;
use file_descriptor::FileDescriptor;
use itertools::Itertools;
use pipe_trait::Pipe;
use reflink::reflink_or_copy;
use std::collections::{HashMap, HashSet};
use std::fs::{hard_link, read_dir, remove_file, DirEntry};
use std::io::{self, ErrorKind};
use std::iter::once;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};

const SEPARATED_COLLECTIONS: &[&str] = &[
    "Feng Ling Yu Xiu",
    "Luo Tianyi, Yuezheng Ling/洛天依_乐正绫",
    "Touhou Hero of Ice Fairy",
];

const UNIFIED_COLLECTION: &str = "Short Relaxing Playlist 2025";

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
        if let Err(error) = remove_file(target) {
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

fn main() {
    let Args {
        execute,
        source,
        target,
    } = Args::parse();

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

    for suffix in SEPARATED_COLLECTIONS {
        let source_dir = source.join(suffix);
        let separated_target_dir = target.join(suffix);
        let unified_target_dir = target.join(UNIFIED_COLLECTION);

        let source_entries = match read_dir(&source_dir) {
            Ok(source_entries) => source_entries,
            Err(error) if error.kind() == ErrorKind::NotFound => continue,
            Err(error) => panic!("error: Cannot read directory {source_dir:?}: {error}"),
        };

        for source_entry in source_entries {
            let Ok(source_entry) = source_entry else {
                continue;
            };
            if !is_subtitle_file(&source_entry) {
                continue;
            }
            let file_name = source_entry.file_name();
            let source_file = source_dir.join(&file_name);
            let separated_target_file = separated_target_dir.join(&file_name);
            let unified_target_file = unified_target_dir.join(&file_name);

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
