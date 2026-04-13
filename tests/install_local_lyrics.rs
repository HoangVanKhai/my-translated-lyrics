use pretty_assertions::assert_eq;
use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

const INSTALL_LOCAL_LYRICS: &str = env!("CARGO_BIN_EXE_install-local-lyrics");

const SEPARATED_COLLECTIONS: &[&str] = &[
    "Feng Ling Yu Xiu",
    "Luo Tianyi, Yuezheng Ling/洛天依_乐正绫",
    "Touhou Hero of Ice Fairy",
];
const UNIFIED_COLLECTION: &str = "Short Relaxing Playlist 2025";

/// Test environment with temporary source and target directories.
struct TestEnv {
    _temp: TempDir,
    source: PathBuf,
    target: PathBuf,
}

impl TestEnv {
    /// Creates a new test environment with empty source and target
    /// directories. The target directory is pre-populated with the
    /// required collection subdirectories.
    fn new() -> Self {
        let temp = TempDir::new().unwrap();
        let source = temp.path().join("source");
        let target = temp.path().join("target");
        fs::create_dir(&source).unwrap();
        fs::create_dir(&target).unwrap();
        for name in SEPARATED_COLLECTIONS
            .iter()
            .copied()
            .chain(std::iter::once(UNIFIED_COLLECTION))
        {
            fs::create_dir_all(target.join(name)).unwrap();
        }
        TestEnv {
            _temp: temp,
            source,
            target,
        }
    }

    /// Creates a video source directory with the given subtitle files.
    fn create_video(
        &self,
        dir_name: &str,
        collection: &str,
        video_title: &str,
        visibility: Option<&str>,
        lyrics: &[(&str, &str)],
    ) {
        let video_dir = self.source.join(dir_name);
        fs::create_dir_all(&video_dir).unwrap();

        let visibility_line = visibility
            .map(|vis| format!("visibility = \"{vis}\"\n"))
            .unwrap_or_default();
        let toml_content = format!(
            "collection = \"{collection}\"\n\
             video-title = \"{video_title}\"\n\
             {visibility_line}\n\
             [song-titles]\n\
             vi = \"test\"\n\
             zh = \"test\"\n"
        );
        fs::write(video_dir.join("video.toml"), toml_content).unwrap();

        for (file_name, content) in lyrics {
            fs::write(video_dir.join(file_name), content).unwrap();
        }
    }

    /// Runs `install-local-lyrics` and asserts it exits successfully.
    fn run(&self, execute: bool) -> std::process::Output {
        let mut cmd = Command::new(INSTALL_LOCAL_LYRICS);
        if execute {
            cmd.arg("--execute");
        }
        cmd.arg(&self.source).arg(&self.target);
        let output = cmd.output().expect("failed to spawn install-local-lyrics");
        assert!(
            output.status.success(),
            "install-local-lyrics failed:\n{}",
            String::from_utf8_lossy(&output.stderr),
        );
        output
    }

    /// Collects all subtitle file paths relative to the target
    /// directory, sorted for deterministic comparison.
    fn target_subtitle_files(&self) -> BTreeSet<String> {
        let mut files = BTreeSet::new();
        for name in SEPARATED_COLLECTIONS
            .iter()
            .copied()
            .chain(std::iter::once(UNIFIED_COLLECTION))
        {
            let dir = self.target.join(name);
            for entry in fs::read_dir(&dir).unwrap() {
                let entry = entry.unwrap();
                if entry.file_type().unwrap().is_file() {
                    let file_name = entry.file_name();
                    let file_name = file_name.to_str().unwrap();
                    files.insert(format!("{name}/{file_name}"));
                }
            }
        }
        files
    }

    /// Reads a target file's content.
    fn read_target(&self, collection: &str, file_name: &str) -> String {
        fs::read_to_string(self.target.join(collection).join(file_name)).unwrap()
    }

    /// Returns the path to a target file.
    fn target_path(&self, collection: &str, file_name: &str) -> PathBuf {
        self.target.join(collection).join(file_name)
    }
}

#[test]
fn dry_run_does_not_create_files() {
    let env = TestEnv::new();
    env.create_video(
        "TestSong",
        "Feng Ling Yu Xiu",
        "TestVideo",
        None,
        &[("lyrics.vi.srt", "1\n00:00:01,000 --> 00:00:02,000\nHello\n")],
    );

    let output = env.run(false);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        stderr.contains("No changes were actually made"),
        "expected dry-run message in stderr:\n{stderr}",
    );
    assert!(
        env.target_subtitle_files().is_empty(),
        "dry run should not create any files",
    );
}

#[test]
fn installs_subtitles_to_separated_and_unified_collections() {
    let env = TestEnv::new();
    let collection = "Feng Ling Yu Xiu";
    let video_title = "TestVideo";
    let srt_content = "1\n00:00:01,000 --> 00:00:02,000\nHello\n";
    let vtt_content = "WEBVTT\n\n00:00:01.000 --> 00:00:02.000\nHello\n";

    env.create_video(
        "TestSong",
        collection,
        video_title,
        None,
        &[
            ("lyrics.vi.srt", srt_content),
            ("lyrics.zh.vtt", vtt_content),
        ],
    );

    env.run(true);

    let expected: BTreeSet<String> = [
        format!("{collection}/{video_title}.vi.srt"),
        format!("{collection}/{video_title}.zh.vtt"),
        format!("{UNIFIED_COLLECTION}/{video_title}.vi.srt"),
        format!("{UNIFIED_COLLECTION}/{video_title}.zh.vtt"),
    ]
    .into_iter()
    .collect();
    assert_eq!(env.target_subtitle_files(), expected);

    assert_eq!(
        env.read_target(collection, &format!("{video_title}.vi.srt")),
        srt_content,
    );
    assert_eq!(
        env.read_target(collection, &format!("{video_title}.zh.vtt")),
        vtt_content,
    );
    assert_eq!(
        env.read_target(UNIFIED_COLLECTION, &format!("{video_title}.vi.srt")),
        srt_content,
    );
    assert_eq!(
        env.read_target(UNIFIED_COLLECTION, &format!("{video_title}.zh.vtt")),
        vtt_content,
    );
}

#[test]
fn skips_up_to_date_files() {
    let env = TestEnv::new();
    env.create_video(
        "TestSong",
        "Feng Ling Yu Xiu",
        "TestVideo",
        None,
        &[("lyrics.vi.srt", "1\n00:00:01,000 --> 00:00:02,000\nHello\n")],
    );

    env.run(true);

    let output = env.run(true);
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
    let env = TestEnv::new();
    let collection = "Feng Ling Yu Xiu";
    let video_title = "UpdateVideo";
    let original = "1\n00:00:01,000 --> 00:00:02,000\nOriginal\n";
    let updated = "1\n00:00:01,000 --> 00:00:02,000\nUpdated\n";

    env.create_video(
        "UpdateSong",
        collection,
        video_title,
        None,
        &[("lyrics.vi.srt", original)],
    );
    env.run(true);

    // Break the hardlink by removing and recreating the source file
    let source_file = env.source.join("UpdateSong").join("lyrics.vi.srt");
    fs::remove_file(&source_file).unwrap();
    fs::write(&source_file, updated).unwrap();

    env.run(true);

    assert_eq!(
        env.read_target(collection, &format!("{video_title}.vi.srt")),
        updated,
    );
    assert_eq!(
        env.read_target(UNIFIED_COLLECTION, &format!("{video_title}.vi.srt")),
        updated,
    );
}

#[test]
fn removes_orphaned_target_files() {
    let env = TestEnv::new();
    let collection = "Feng Ling Yu Xiu";

    let orphaned = env.target_path(collection, "Orphaned.vi.srt");
    fs::write(&orphaned, "orphaned content").unwrap();

    env.run(true);

    assert!(
        !orphaned.exists(),
        "orphaned file should be removed from target",
    );
}

#[test]
fn hidden_visibility_causes_removal() {
    let env = TestEnv::new();
    let collection = "Feng Ling Yu Xiu";
    let video_title = "HiddenVideo";

    let separated = env.target_path(collection, &format!("{video_title}.vi.srt"));
    let unified = env.target_path(UNIFIED_COLLECTION, &format!("{video_title}.vi.srt"));
    fs::write(&separated, "old content").unwrap();
    fs::write(&unified, "old content").unwrap();

    env.create_video(
        "HiddenSong",
        collection,
        video_title,
        Some("hidden"),
        &[("lyrics.vi.srt", "new content that should not be installed")],
    );

    env.run(true);

    assert!(
        !separated.exists(),
        "hidden video's separated file should be removed",
    );
    assert!(
        !unified.exists(),
        "hidden video's unified file should be removed",
    );
}

#[test]
fn manual_visibility_preserves_existing_files() {
    let env = TestEnv::new();
    let collection = "Feng Ling Yu Xiu";
    let video_title = "ManualVideo";
    let manual_content = "manually edited content";

    let separated = env.target_path(collection, &format!("{video_title}.vi.srt"));
    let unified = env.target_path(UNIFIED_COLLECTION, &format!("{video_title}.vi.srt"));
    fs::write(&separated, manual_content).unwrap();
    fs::write(&unified, manual_content).unwrap();

    env.create_video(
        "ManualSong",
        collection,
        video_title,
        Some("manual"),
        &[("lyrics.vi.srt", "source content that should not overwrite")],
    );

    env.run(true);

    assert_eq!(fs::read_to_string(&separated).unwrap(), manual_content);
    assert_eq!(fs::read_to_string(&unified).unwrap(), manual_content);
}
