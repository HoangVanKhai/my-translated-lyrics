use clap::Parser;
use pipe_trait::Pipe;
use reflink::reflink_or_copy;
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

    entry.file_name().as_bytes().ends_with(b".srt")
}

fn main() {
    let Args {
        execute,
        source,
        target,
    } = Args::parse();

    eprintln!();
    eprintln!("stage: Removing old subtitle files");
    SEPARATED_COLLECTIONS
        .iter()
        .copied()
        .chain(once(UNIFIED_COLLECTION))
        .map(|suffix| target.join(suffix))
        .flat_map(|ref target| {
            target
                .pipe(read_dir)
                .unwrap_or_else(|error| panic!("error: Cannot read directory {target:?}: {error}"))
                .flatten()
                .filter(is_subtitle_file)
                .map(|entry| entry.path())
        })
        .collect::<Vec<_>>() // Force early errors, preventing incomplete operations
        .into_iter()
        .for_each(|ref target| uninstall(execute, target));

    eprintln!();
    eprintln!("stage: Installing subtitles");
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
            install(execute, &source_file, &separated_target_file);
            install(execute, &source_file, &unified_target_file);
        }
    }

    if !execute {
        eprintln!();
        eprintln!("info: No changes were actually made.");
        eprintln!("info: Run the command again with --execute to make actual changes.");
    }
}
