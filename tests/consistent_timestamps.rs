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

        let name = path.to_string_lossy();
        if name.ends_with(".srt") || name.ends_with(".vtt") {
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
        !groups.is_empty(),
        "no subtitle file groups found in {}",
        data_dir.display(),
    );

    let mut pairs_checked = 0u32;

    for paths in groups.values() {
        if paths.len() < 2 {
            continue;
        }

        let contents: Vec<_> = paths
            .iter()
            .map(|path| {
                let name = path.file_name().unwrap().to_string_lossy().into_owned();
                let content = fs::read_to_string(path)
                    .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
                (name, content)
            })
            .collect();

        let first_name = &contents[0].0;
        let first_timestamps = extract_timestamps(&contents[0].1);

        for (name, content) in &contents[1..] {
            let timestamps = extract_timestamps(content);
            assert_eq!(
                first_timestamps, timestamps,
                "timestamp mismatch: {first_name} vs {name}",
            );
            pairs_checked += 1;
        }
    }

    assert!(
        pairs_checked > 0,
        "no subtitle file pairs found to compare in {}",
        data_dir.display(),
    );
}
