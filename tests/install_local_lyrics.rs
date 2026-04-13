pub mod _utils;
pub use _utils::*;

use command_extra::CommandExtra;
use itertools::Itertools;
use maplit::hashmap;
use my_translated_lyrics::video_descriptor::{
    Language, SEPARATED_COLLECTIONS, UNIFIED_COLLECTION, VideoDesc, Visibility,
};
use pipe_trait::Pipe;
use pretty_assertions::assert_eq;
use std::fs::{
    create_dir, create_dir_all, read_dir, read_to_string, remove_file, write as write_file,
};
use std::iter::once;
use std::path::PathBuf;
use std::process::Command;
use text_block_macros::text_block_fnl;

const INSTALL_LOCAL_LYRICS: &str = env!("CARGO_BIN_EXE_install-local-lyrics");

/// Test workspace with temporary source and target directories.
struct Workspace {
    _temp: Temp,
    source: PathBuf,
    target: PathBuf,
}

impl Workspace {
    /// Creates a new workspace with empty source and target
    /// directories. The target directory is pre-populated with the
    /// required collection subdirectories.
    fn new() -> Self {
        let temp = Temp::new_dir();
        let source = temp.join("source");
        let target = temp.join("target");
        create_dir(&source).unwrap();
        create_dir(&target).unwrap();
        SEPARATED_COLLECTIONS
            .iter()
            .copied()
            .chain(once(UNIFIED_COLLECTION))
            .map(|name| target.join(name))
            .try_for_each(create_dir_all)
            .unwrap();
        Workspace {
            _temp: temp,
            source,
            target,
        }
    }

    /// Creates a video source directory with the given subtitle files.
    fn add_video(&self, dir_name: &str, desc: &VideoDesc, lyrics: &[(&str, &str)]) {
        let video_dir = self.source.join(dir_name);
        create_dir_all(&video_dir).unwrap();

        let toml_content = toml::to_string(desc).unwrap();
        write_file(video_dir.join("video.toml"), toml_content).unwrap();

        for (file_name, content) in lyrics {
            write_file(video_dir.join(file_name), content).unwrap();
        }
    }

    /// Runs `install-local-lyrics` and asserts it exits successfully.
    fn run<Args: IntoIterator<Item = &'static str>>(&self, args: Args) -> std::process::Output {
        let output = Command::new(INSTALL_LOCAL_LYRICS)
            .with_args(args)
            .with_arg(&self.source)
            .with_arg(&self.target)
            .output()
            .expect("failed to spawn install-local-lyrics");
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stderr = stderr.trim();
        if !stderr.is_empty() {
            eprintln!("STDERR:\n{stderr}\n");
        }
        assert!(output.status.success(), "install-local-lyrics failed");
        output
    }

    /// Collects all subtitle file paths relative to the target
    /// directory, sorted for deterministic comparison.
    fn target_subtitle_files(&self) -> Vec<String> {
        SEPARATED_COLLECTIONS
            .iter()
            .copied()
            .chain(once(UNIFIED_COLLECTION))
            .flat_map(|name| {
                self.target
                    .join(name)
                    .pipe(read_dir)
                    .unwrap()
                    .map(Result::unwrap)
                    .filter(|entry| entry.file_type().unwrap().is_file())
                    .map(move |entry| format!("{name}/{}", entry.file_name().to_str().unwrap()))
            })
            .sorted()
            .collect()
    }

    /// Reads a target file's content.
    fn read_target(&self, collection: &str, file_name: &str) -> String {
        self.target
            .join(collection)
            .join(file_name)
            .pipe(read_to_string)
            .unwrap()
    }

    /// Returns the path to a target file.
    fn target_path(&self, collection: &str, file_name: &str) -> PathBuf {
        self.target.join(collection).join(file_name)
    }
}

fn video_desc(collection: &str, video_title: &str, visibility: Visibility) -> VideoDesc {
    VideoDesc {
        collection: collection.to_string().try_into().unwrap(),
        video_title: video_title.to_string().try_into().unwrap(),
        song_titles: hashmap! {
            Language::Vietnamese => "test".to_string(),
            Language::Chinese => "test".to_string(),
        },
        visibility,
    }
}

#[test]
fn dry_run_does_not_create_files() {
    let workspace = Workspace::new();
    let desc = video_desc("Feng Ling Yu Xiu", "Example Song", Visibility::default());
    workspace.add_video(
        "ExampleSong",
        &desc,
        &[(
            "lyrics.vi.srt",
            text_block_fnl! {
                "1"
                "00:00:01,000 --> 00:00:02,000"
                "Hello"
            },
        )],
    );

    let output = workspace.run(None::<&str>);
    let stderr = String::from_utf8_lossy(&output.stderr);
    eprintln!("STDERR:\n{stderr}");

    assert!(
        stderr.contains("No changes were actually made"),
        "expected dry-run message in stderr:\n{stderr}",
    );
    assert!(
        workspace.target_subtitle_files().is_empty(),
        "dry run should not create any files",
    );
}

#[test]
fn installs_subtitles_to_separated_and_unified_collections() {
    let workspace = Workspace::new();
    let collection = "Feng Ling Yu Xiu";
    let video_title = "Example Song";
    let desc = video_desc(collection, video_title, Visibility::default());
    let srt_content = text_block_fnl! {
        "1"
        "00:00:01,000 --> 00:00:02,000"
        "Hello"
    };
    let vtt_content = text_block_fnl! {
        "WEBVTT"
        ""
        "00:00:01.000 --> 00:00:02.000"
        "Hello"
    };

    workspace.add_video(
        "ExampleSong",
        &desc,
        &[
            ("lyrics.vi.srt", srt_content),
            ("lyrics.zh.vtt", vtt_content),
        ],
    );

    workspace.run(["--execute"]);

    let expected = vec![
        format!("{collection}/{video_title}.vi.srt"),
        format!("{collection}/{video_title}.zh.vtt"),
        format!("{UNIFIED_COLLECTION}/{video_title}.vi.srt"),
        format!("{UNIFIED_COLLECTION}/{video_title}.zh.vtt"),
    ];
    assert_eq!(workspace.target_subtitle_files(), expected);

    assert_eq!(
        workspace.read_target(collection, &format!("{video_title}.vi.srt")),
        srt_content,
    );
    assert_eq!(
        workspace.read_target(collection, &format!("{video_title}.zh.vtt")),
        vtt_content,
    );
    assert_eq!(
        workspace.read_target(UNIFIED_COLLECTION, &format!("{video_title}.vi.srt")),
        srt_content,
    );
    assert_eq!(
        workspace.read_target(UNIFIED_COLLECTION, &format!("{video_title}.zh.vtt")),
        vtt_content,
    );
}

#[test]
fn skips_up_to_date_files() {
    let workspace = Workspace::new();
    let desc = video_desc("Feng Ling Yu Xiu", "Example Song", Visibility::default());
    workspace.add_video(
        "ExampleSong",
        &desc,
        &[(
            "lyrics.vi.srt",
            text_block_fnl! {
                "1"
                "00:00:01,000 --> 00:00:02,000"
                "Hello"
            },
        )],
    );

    workspace.run(["--execute"]);

    let output = workspace.run(["--execute"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("0 files would be removed from the target location"),
        "expected 0 removals:\n{stderr}",
    );
    assert!(
        stderr.contains("0 files would be added to the target location"),
        "expected 0 additions:\n{stderr}",
    );
    assert!(
        stderr.contains("0 files in the target location would be updated"),
        "expected 0 updates:\n{stderr}",
    );
}

#[test]
fn updates_modified_source_files() {
    let workspace = Workspace::new();
    let collection = "Feng Ling Yu Xiu";
    let video_title = "Song Whose Subtitles Get Updated";
    let desc = video_desc(collection, video_title, Visibility::default());
    let original = text_block_fnl! {
        "1"
        "00:00:01,000 --> 00:00:02,000"
        "Original"
    };
    let updated = text_block_fnl! {
        "1"
        "00:00:01,000 --> 00:00:02,000"
        "Updated"
    };

    workspace.add_video(
        "SongWhoseSubtitlesGetUpdated",
        &desc,
        &[("lyrics.vi.srt", original)],
    );
    workspace.run(["--execute"]);

    // Break the hardlink by removing and recreating the source file
    let source_file = workspace
        .source
        .join("SongWhoseSubtitlesGetUpdated")
        .join("lyrics.vi.srt");
    remove_file(&source_file).unwrap();
    write_file(&source_file, updated).unwrap();

    workspace.run(["--execute"]);

    assert_eq!(
        workspace.read_target(collection, &format!("{video_title}.vi.srt")),
        updated,
    );
    assert_eq!(
        workspace.read_target(UNIFIED_COLLECTION, &format!("{video_title}.vi.srt")),
        updated,
    );
}

#[test]
fn removes_orphaned_target_files() {
    let workspace = Workspace::new();
    let collection = "Feng Ling Yu Xiu";

    let orphaned = workspace.target_path(collection, "Orphaned.vi.srt");
    write_file(&orphaned, "orphaned content").unwrap();

    workspace.run(["--execute"]);

    assert!(
        !orphaned.exists(),
        "orphaned file should be removed from target",
    );
}

#[test]
fn hidden_visibility_causes_removal() {
    let workspace = Workspace::new();
    let collection = "Feng Ling Yu Xiu";
    let video_title = "Song Whose Subtitles Are Hidden";
    let desc = video_desc(collection, video_title, Visibility::Hidden);

    let separated = workspace.target_path(collection, &format!("{video_title}.vi.srt"));
    let unified = workspace.target_path(UNIFIED_COLLECTION, &format!("{video_title}.vi.srt"));
    write_file(&separated, "old content").unwrap();
    write_file(&unified, "old content").unwrap();

    workspace.add_video(
        "SongWhoseSubtitlesAreHidden",
        &desc,
        &[("lyrics.vi.srt", "new content that should not be installed")],
    );

    workspace.run(["--execute"]);

    assert!(
        !separated.exists(),
        "hidden song's separated file should be removed",
    );
    assert!(
        !unified.exists(),
        "hidden song's unified file should be removed",
    );
}

#[test]
fn manual_visibility_preserves_existing_files() {
    let workspace = Workspace::new();
    let collection = "Feng Ling Yu Xiu";
    let video_title = "Song Whose Subtitles Are Manually Managed";
    let desc = video_desc(collection, video_title, Visibility::Manual);
    let manual_content = "manually edited content";

    let separated = workspace.target_path(collection, &format!("{video_title}.vi.srt"));
    let unified = workspace.target_path(UNIFIED_COLLECTION, &format!("{video_title}.vi.srt"));
    write_file(&separated, manual_content).unwrap();
    write_file(&unified, manual_content).unwrap();

    workspace.add_video(
        "SongWhoseSubtitlesAreManuallyManaged",
        &desc,
        &[("lyrics.vi.srt", "source content that should not overwrite")],
    );

    workspace.run(["--execute"]);

    assert_eq!(separated.pipe_ref(read_to_string).unwrap(), manual_content);
    assert_eq!(unified.pipe_ref(read_to_string).unwrap(), manual_content);
}
