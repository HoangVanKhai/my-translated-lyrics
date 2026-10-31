use super::{NoSubtitles, Termination};
use std::path::PathBuf;
use std::process::ExitCode;
use test_utils::video_title;

/// Cancellation and a player's non-zero exit map to their own process codes.
/// `ExitCode` is not comparable, so the mapping is checked through its `Debug`
/// form, which is identical for codes built the same way.
#[test]
fn exit_code_maps_cancellation_and_player_failure() {
    let debug = |code: ExitCode| format!("{code:?}");
    assert_eq!(
        debug(Termination::Cancelled.exit_code()),
        debug(ExitCode::from(exit_codes::CANCELLED)),
    );
    assert_eq!(
        debug(Termination::PlayerExited(7).exit_code()),
        debug(ExitCode::from(7)),
    );
}

/// The video title is quoted as the descriptor spells it, rather than as the
/// `Debug` form of the type that carries it.
#[test]
fn no_subtitles_quotes_the_video_title() {
    let failure = NoSubtitles {
        video_title: video_title("Example Song [id]"),
        collection_dir: PathBuf::from("/library/Example Collection"),
    };
    assert_eq!(
        failure.to_string(),
        r#"No subtitles for "Example Song [id]" were found in "/library/Example Collection"."#,
    );
}
