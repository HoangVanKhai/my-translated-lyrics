pub mod _utils;
pub use _utils::*;

use my_translated_lyrics::video_descriptor::Visibility;
use text_block_macros::text_block_fnl;

#[test]
fn dry_run_does_not_create_files() {
    let workspace = Workspace::create();
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

    assert!(
        stderr.contains("No changes were actually made"),
        "expected dry-run message in stderr",
    );
    assert!(
        workspace.target_subtitle_files().is_empty(),
        "dry run should not create any files",
    );
}
