use itertools::Itertools;
use pipe_trait::Pipe;
use std::fs;
use std::fs::DirEntry;
use std::path::Path;

/// Verify that `data/` and `drafts/` have a flat two-level structure:
///
/// ```text
/// data/
/// ├── SongName/
/// │   ├── song.toml
/// │   ├── lyrics.vi.srt
/// │   └── lyrics.zh.srt
/// └── AnotherSong/
///     └── ...
/// ```
///
/// Rejects files placed directly under the top-level directory (too shallow)
/// and directories nested inside a song directory (too deep).
#[test]
fn data_and_drafts_have_flat_structure() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));

    for top_dir_name in ["data", "drafts"] {
        let top_dir = manifest_dir.join(top_dir_name);
        if !top_dir.exists() {
            continue;
        }

        let entries: Vec<_> = top_dir
            .pipe_ref(fs::read_dir)
            .unwrap()
            .map(Result::unwrap)
            .sorted_by_key(DirEntry::file_name)
            .collect();

        for entry in &entries {
            let path = entry.path();
            let name = entry.file_name();
            let name = name.to_str().expect("path isn't valid UTF-8");

            assert!(
                path.is_dir(),
                "{top_dir_name}/{name} should be a directory, not a file",
            );

            let inner_entries: Vec<_> = path
                .pipe(fs::read_dir)
                .unwrap()
                .map(Result::unwrap)
                .sorted_by_key(DirEntry::file_name)
                .collect();

            for inner_entry in &inner_entries {
                let inner_path = inner_entry.path();
                let inner_name = inner_entry.file_name();
                let inner_name = inner_name.to_str().expect("path isn't valid UTF-8");

                assert!(
                    inner_path.is_file(),
                    "{top_dir_name}/{name}/{inner_name} should be a file, not a directory",
                );
            }
        }
    }
}
