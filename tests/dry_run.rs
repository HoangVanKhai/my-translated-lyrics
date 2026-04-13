pub mod _utils;
pub use _utils::*;

use my_translated_lyrics::video_descriptor::Visibility;
use text_block_macros::text_block_fnl;

#[test]
fn dry_run_does_not_create_files() {
    let env = InstallLocalLyricsEnv::prepare();
    let desc = video_desc(
        "Feng Ling Yu Xiu",
        "【示例表演者】《示例歌曲》Example Song [ExampleID]",
        Visibility::default(),
    );
    env.add_source_entry(
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

    let output = env.run(None::<&str>);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(stderr.contains("No changes were actually made"));
    assert!(
        env.target_subtitle_files().is_empty(),
        "dry run should not create any files",
    );
}
