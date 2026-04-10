use pipe_trait::Pipe;
use pretty_assertions::assert_eq;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

fn collect_subtitle_files(files: &mut Vec<PathBuf>, dir: &Path) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect_subtitle_files(files, &path);
            continue;
        }

        let path_str = path.to_str().expect("path isn't valid UTF-8");
        if path_str.ends_with(".srt") || path_str.ends_with(".vtt") {
            files.push(path);
        }
    }
}

/// Returns a grouping key that is shared by all language variants of the same
/// subtitle file (e.g. `.../{video_title}.{lang}.srt`). The language component
/// is stripped so that different translations map to the same key.
fn subtitle_group_key(path: &Path) -> Option<String> {
    let path = path.to_str().expect("path isn't valid UTF-8");
    for format in ["srt", "vtt"] {
        let suffix = format!(".{format}");
        let without_format = path.strip_suffix(&suffix)?;
        if let Some(dot_pos) = without_format.rfind('.') {
            let lang = &without_format[dot_pos + 1..];
            if !lang.is_empty() && lang.len() <= 5 && lang.chars().all(|c| c.is_ascii_lowercase()) {
                let stem = &without_format[..dot_pos];
                return Some(format!("{stem}::{format}"));
            }
        }
    }
    None
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
        if let Some(key) = subtitle_group_key(&path) {
            groups.entry(key).or_default().push(path);
        }
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
                    .pipe(fs::read_to_string)
                    .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
                (name, content)
            })
            .collect();

        let (first_content, remaining_contents) = contents.split_first().unwrap();
        let (first_name, first_timestamps) = first_content;
        let first_timestamps = extract_timestamps(first_timestamps);

        for (name, content) in remaining_contents {
            let timestamps = extract_timestamps(content);
            assert_eq!(
                first_timestamps, timestamps,
                "timestamp mismatch: {first_name} vs {name}",
            );
        }
    }
}
