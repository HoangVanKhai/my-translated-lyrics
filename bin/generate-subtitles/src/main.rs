#![cfg_attr(dylint_lib = "perfectionist", feature(register_tool))]
#![cfg_attr(dylint_lib = "perfectionist", register_tool(perfectionist))]

use clap::Parser;
use generate_subtitles::{RenderCounts, load_song, render_song};
use itertools::Itertools;
use pipe_trait::Pipe;
use std::fs::{DirEntry, read_dir};
use std::path::PathBuf;

#[derive(Debug, Clone, Parser)]
#[clap(about = "Generate the subtitles")]
struct Args {
    /// Source directory that contains one song subdirectory per video.
    sources: PathBuf,

    /// Destination directory into which subtitle files are written.
    dist: PathBuf,

    /// For safety reasons, this programs list actions by default, this flag makes the program take those actions.
    #[clap(long, short = 'x')]
    execute: bool,
}

fn main() {
    let args = Args::parse();

    let song_dirs = args
        .sources
        .pipe_ref(read_dir)
        .unwrap_or_else(|error| {
            panic!(
                "error: Cannot read sources directory {sources:?}: {error}",
                sources = args.sources,
            )
        })
        .map(Result::<DirEntry, _>::unwrap)
        .filter(|entry| {
            entry
                .file_type()
                .unwrap_or_else(|error| {
                    panic!(
                        "error: Cannot read file type of {path:?}: {error}",
                        path = entry.path(),
                    )
                })
                .is_dir()
        })
        .map(|entry| entry.path())
        .sorted();

    let mut totals = RenderCounts::default();
    let mut total_files: usize = 0;
    for song_dir in song_dirs {
        let has_txt = song_dir
            .pipe_ref(read_dir)
            .unwrap_or_else(|error| panic!("error: Cannot read directory {song_dir:?}: {error}"))
            .map(Result::<DirEntry, _>::unwrap)
            .any(|entry| {
                entry
                    .file_name()
                    .to_str()
                    .map(|name| name.starts_with("lyrics.") && name.ends_with(".txt"))
                    .unwrap_or(false)
            });
        if !has_txt {
            continue;
        }
        let song = load_song(&song_dir);
        eprintln!("info: Rendering {:?}", song.directory_name);
        total_files += song.languages.len() * 2;
        totals += render_song(&song, &args.dist, args.execute);
    }
    let total_unchanged = total_files - totals.total();

    eprintln!();
    if args.execute {
        eprintln!("info: Added {} files.", totals.added);
        eprintln!("info: Updated {} files.", totals.updated);
    } else {
        eprintln!("info: {} files would be added.", totals.added);
        eprintln!("info: {} files would be updated.", totals.updated);
    }
    eprintln!("info: {total_unchanged} files already up to date.");
    if !args.execute {
        eprintln!();
        eprintln!("info: No files were written. Rerun with --execute to apply changes.");
    }
}
