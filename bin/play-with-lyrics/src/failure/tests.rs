use super::Termination;
use std::process::ExitCode;

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
