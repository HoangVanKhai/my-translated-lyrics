use crate::player::{Player, SubtitleFormat};
use pretty_assertions::assert_eq;
use std::path::Path;
use std::str::FromStr;

fn argv(player: Player) -> (String, Vec<String>) {
    let command = player.command(
        Path::new("/library/Coll/Some Title [id].mkv"),
        Path::new("/library/Coll/Some Title [id].vi.srt"),
    );
    let program = command
        .get_program()
        .to_str()
        .expect("the program is valid UTF-8")
        .to_string();
    let args = command
        .get_args()
        .map(|arg| {
            arg.to_str()
                .expect("the argument is valid UTF-8")
                .to_string()
        })
        .collect();
    (program, args)
}

#[test]
fn mpv_uses_sub_file_flag() {
    let (program, args) = argv(Player::Mpv);
    assert_eq!(program, "mpv");
    assert_eq!(
        args,
        vec![
            "--sub-file=/library/Coll/Some Title [id].vi.srt".to_string(),
            "/library/Coll/Some Title [id].mkv".to_string(),
        ],
    );
}

#[test]
fn celluloid_uses_mpv_sub_file_flag() {
    let (program, args) = argv(Player::Celluloid);
    assert_eq!(program, "celluloid");
    assert_eq!(
        args,
        vec![
            "--mpv-sub-file=/library/Coll/Some Title [id].vi.srt".to_string(),
            "/library/Coll/Some Title [id].mkv".to_string(),
        ],
    );
}

#[test]
fn players_round_trip_through_their_names() {
    assert_eq!(Player::from_str("mpv"), Ok(Player::Mpv));
    assert_eq!(Player::from_str("celluloid"), Ok(Player::Celluloid));
    assert_eq!(Player::Mpv.to_string(), "mpv");
}

#[test]
fn subtitle_formats_round_trip_through_their_extensions() {
    assert_eq!(SubtitleFormat::from_str("srt"), Ok(SubtitleFormat::SubRip));
    assert_eq!(SubtitleFormat::from_str("vtt"), Ok(SubtitleFormat::WebVtt));
    assert_eq!(SubtitleFormat::WebVtt.to_string(), "vtt");
}
