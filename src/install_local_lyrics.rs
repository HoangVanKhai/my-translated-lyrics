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
use std::path::{Path, PathBuf};

const UNIFIED_COLLECTION: &str = "Short Relaxing Playlist 2025";
const SONG_CONFIG_FILENAME: &str = "song.toml";

#[derive(Deserialize)]
struct SongConfig {
    collection: String,
    filename: String,
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

    // Read all song configurations from source directories
    let songs: Vec<(PathBuf, SongConfig)> = read_dir(&source)
        .unwrap_or_else(|error| panic!("error: Cannot read source directory {source:?}: {error}"))
        .flatten()
        .filter(|entry| entry.file_type().is_ok_and(|file_type| file_type.is_dir()))
        .filter_map(|entry| {
            let song_dir = entry.path();
            let config_path = song_dir.join(SONG_CONFIG_FILENAME);
            let content = read_to_string(&config_path).ok()?;
            let config: SongConfig = toml::from_str(&content)
                .unwrap_or_else(|error| panic!("error: Cannot parse {config_path:?}: {error}"));
            Some((song_dir, config))
        })
        .collect();

    // Derive target collection directories from song configurations
    let target_collections: HashSet<&str> = songs
        .iter()
        .map(|(_, config)| config.collection.as_str())
        .collect();

    let existing_target_files: HashMap<PathBuf, FileDescriptor> = target_collections
        .iter()
        .map(|collection| target.join(collection))
        .chain(once(target.join(UNIFIED_COLLECTION)))
        .flat_map(|ref path| match read_dir(path) {
            Ok(entries) => entries
                .flatten()
                .filter(is_subtitle_file)
                .map(|entry| entry.path())
                .collect::<Vec<_>>(),
            Err(error) if error.kind() == ErrorKind::NotFound => Vec::new(),
            Err(error) => panic!("error: Cannot read directory {path:?}: {error}"),
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

    for (song_dir, config) in &songs {
        let separated_target_dir = target.join(&config.collection);
        let unified_target_dir = target.join(UNIFIED_COLLECTION);

        let source_entries = read_dir(song_dir)
            .unwrap_or_else(|error| panic!("error: Cannot read directory {song_dir:?}: {error}"));

        for source_entry in source_entries {
            let Ok(source_entry) = source_entry else {
                continue;
            };
            if !is_subtitle_file(&source_entry) {
                continue;
            }

            let local_name = source_entry.file_name();
            let local_name_str = local_name
                .to_str()
                .unwrap_or_else(|| panic!("error: Non-UTF-8 filename in {song_dir:?}"));

            // Map lyrics.{lang}.{ext} → {config.filename}.{lang}.{ext}
            let target_name = local_name_str
                .strip_prefix("lyrics")
                .map(|suffix| format!("{}{suffix}", config.filename))
                .unwrap_or_else(|| local_name_str.to_owned());

            let source_file = song_dir.join(&local_name);
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
