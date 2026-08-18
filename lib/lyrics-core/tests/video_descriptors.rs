use itertools::Itertools;
use lyrics_core::collections_descriptor::{COLLECTIONS_CONFIG_FILE_NAME, CollectionsDesc};
use lyrics_core::video_descriptor::{VIDEO_CONFIG_FILE_NAME, VideoDesc};
use pipe_trait::Pipe;
use std::fs::{DirEntry, read_dir, read_to_string};
use std::path::Path;

/// Reads and parses the collections manifest of `dist/`, the one
/// declaration of the collections the whole repository targets.
fn load_collections(dist_dir: &Path) -> CollectionsDesc {
    dist_dir
        .join(COLLECTIONS_CONFIG_FILE_NAME)
        .pipe(read_to_string)
        .unwrap()
        .pipe_as_ref(toml::from_str::<CollectionsDesc>)
        .unwrap()
}

/// The collections manifest must parse and must declare at least one
/// separated collection, since every video descriptor names one.
#[test]
fn dist_collections_manifest_is_valid() {
    let dist_dir = test_utils::workspace_dir().join("dist");
    let collections = load_collections(&dist_dir);
    assert!(
        !collections.separated.is_empty(),
        "`dist/{COLLECTIONS_CONFIG_FILE_NAME}` declares no separated collection",
    );
}

/// Every `video.toml` must parse as a valid [`VideoDesc`] and must name
/// a separated collection that the manifest declares, so a misspelled
/// collection is caught here rather than at installation time. A
/// descriptor is mandatory in `dist/` and optional in the other two
/// directories, where a song may still be in preparation.
#[test]
fn dist_drafts_and_sources_video_descriptors_are_valid() {
    let workspace_dir = test_utils::workspace_dir();
    let collections = load_collections(&workspace_dir.join("dist"));

    for top_dir_name in ["dist", "drafts", "sources"] {
        let top_dir = workspace_dir.join(top_dir_name);
        if !top_dir.exists() {
            continue;
        }

        let entries = top_dir
            .pipe(read_dir)
            .unwrap()
            .map(Result::<DirEntry, _>::unwrap)
            .sorted_by_key(DirEntry::file_name);

        for entry in entries {
            let video_dir = entry.path();
            if !video_dir.is_dir() {
                continue;
            }
            let desc_path = video_dir.join(VIDEO_CONFIG_FILE_NAME);
            if top_dir_name != "dist" && !desc_path.is_file() {
                continue;
            }

            let song_name = entry.file_name();
            let song_name = song_name.to_str().expect("path isn't valid UTF-8");
            eprintln!("CASE: {top_dir_name}/{song_name}");
            let desc: VideoDesc = desc_path
                .pipe(read_to_string)
                .unwrap()
                .pipe_as_ref(toml::from_str)
                .unwrap();
            collections
                .check_separated(&desc.collection)
                .unwrap_or_else(|error| {
                    panic!("{top_dir_name}/{song_name}/{VIDEO_CONFIG_FILE_NAME}: {error}")
                });
        }
    }
}
