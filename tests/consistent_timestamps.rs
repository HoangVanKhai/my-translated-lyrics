use my_translated_lyrics::video_descriptor::LyricsFileName;
use pipe_trait::Pipe;
use pretty_assertions::assert_eq;
use std::collections::BTreeMap;
use std::fs::{read_dir, read_to_string};
use std::path::{Path, PathBuf};

fn collect_subtitle_files(files: &mut Vec<PathBuf>, data_dir: &Path) {
    for entry in read_dir(data_dir).unwrap() {
        let song_dir = entry.unwrap().path();
        if !song_dir.is_dir() {
            continue;
        }
        for entry in read_dir(&song_dir).unwrap() {
            let path = entry.unwrap().path();
            let path_str = path.to_str().expect("path isn't valid UTF-8");
            if path_str.ends_with(".srt") || path_str.ends_with(".vtt") {
                files.push(path);
            }
        }
    }
}

/// Returns a grouping key that is shared by all language variants of the same
/// subtitle file (e.g. `.../{song_dir}/lyrics.{lang}.srt`). The language component
/// is stripped so that different translations map to the same key.
fn subtitle_group_key(path: &Path) -> String {
    let file_name = path
        .file_name()
        .unwrap()
        .to_str()
        .expect("path isn't valid UTF-8");
    file_name.parse::<LyricsFileName>().unwrap();
    let format = path
        .extension()
        .unwrap()
        .to_str()
        .expect("extension isn't valid UTF-8");
    let path_str = path.to_str().expect("path isn't valid UTF-8");
    let stem = path_str
        .strip_suffix(&format!(".{format}"))
        .unwrap()
        .rsplit_once('.')
        .unwrap()
        .0;
    format!("{stem}::{format}")
}

fn extract_timestamps(content: &str) -> Vec<&str> {
    content
        .lines()
        .filter(|line| line.contains(" --> "))
        .collect()
}

#[test]
fn file_timestamps_match() {
    let data_dir = env!("CARGO_MANIFEST_DIR").pipe(Path::new).join("data");

    let mut files = Vec::new();
    collect_subtitle_files(&mut files, &data_dir);
    files.sort();

    // Group files by (stem, format) so that language variants share a key.
    let mut groups: BTreeMap<String, Vec<PathBuf>> = BTreeMap::new();
    for path in files {
        let key = subtitle_group_key(&path);
        groups.entry(key).or_default().push(path);
    }

    assert!(
        groups.values().any(|paths| paths.len() >= 2),
        "no subtitle file pairs found to compare in {}",
        data_dir.display(),
    );

    for paths in groups.values() {
        if paths.len() < 2 {
            continue;
        }

        let contents: Vec<_> = paths
            .iter()
            .map(|path| {
                let name = path
                    .file_name()
                    .unwrap()
                    .to_str()
                    .expect("path isn't valid UTF-8");
                let content = path
                    .pipe(read_to_string)
                    .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
                (name, content)
            })
            .collect();

        let (first_content, remaining_contents) = contents.split_first().unwrap();
        let (first_name, first_content) = first_content;
        let first_timestamps = extract_timestamps(first_content);
        assert!(
            !first_timestamps.is_empty(),
            "no timestamps found in {first_name}",
        );

        for (name, content) in remaining_contents {
            let timestamps = extract_timestamps(content);
            assert_eq!(
                first_timestamps, timestamps,
                "timestamp mismatch: {first_name} vs {name}",
            );
        }
    }
}
