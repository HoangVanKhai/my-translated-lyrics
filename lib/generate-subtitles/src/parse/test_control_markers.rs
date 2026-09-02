//! Tests for the control markers `clr` and `eov`. They cover the trailing
//! text each one accepts, the events each one does and does not push, and
//! the rejection of a cue whose marker names either of them.

use crate::parse::error::{
    ExtraTextAfterControlMarker, ParseLyricsError, ParseLyricsErrorKind, ReservedControlMarker,
};
use crate::parse::parse_lyrics;
use lyrics_core::line_markers_descriptor::ReservedMarker;
use lyrics_core::timestamp::Timestamp;
use pipe_trait::Pipe;
use pretty_assertions::assert_eq;
use text_block_macros::text_block_fnl;

#[test]
fn control_markers_accept_trailing_whitespace_only() {
    let input = text_block_fnl! {
        "00:00.000 ttl: Hello"
        "00:02.000 clr \t "
        "00:05.000 eov\t"
    };
    let cues = parse_lyrics(input).unwrap();
    assert_eq!(cues.len(), 1);
    assert_eq!(cues[0].end, Timestamp::new(0, 2, 0).unwrap());
}

#[test]
fn control_markers_reject_trailing_text() {
    let clr_input = text_block_fnl! {
        "00:00.000 ttl: Hello"
        "00:02.000 clr some trailing text"
    };
    assert_eq!(
        parse_lyrics(clr_input).unwrap_err(),
        ParseLyricsError {
            line_number: 2,
            kind: ParseLyricsErrorKind::ExtraTextAfterControlMarker(ExtraTextAfterControlMarker {
                marker: ReservedMarker::Clear,
                trailing: "some trailing text".to_string(),
            }),
        },
    );

    let eov_input = text_block_fnl! {
        "00:00.000 ttl: Hello"
        "00:02.000 clr"
        "00:05.000 eov\tend of video"
    };
    assert_eq!(
        parse_lyrics(eov_input).unwrap_err(),
        ParseLyricsError {
            line_number: 3,
            kind: ParseLyricsErrorKind::ExtraTextAfterControlMarker(ExtraTextAfterControlMarker {
                marker: ReservedMarker::EndOfVideo,
                trailing: "end of video".to_string(),
            }),
        },
    );
}

#[test]
fn eov_marker_does_not_produce_a_cue() {
    let input = text_block_fnl! {
        "00:00.000 ttl: Hello"
        "00:02.000 clr"
        ""
        "00:05.000 eov"
    };
    let cues = parse_lyrics(input).unwrap();
    assert_eq!(cues.len(), 1);
    assert_eq!(cues[0].end, Timestamp::new(0, 2, 0).unwrap());
}

#[test]
fn eov_between_a_cue_and_its_continuation_leaves_the_cue_open() {
    let input = text_block_fnl! {
        "00:00.000 cre: first line"
        "00:03.000 eov"
        "               second line"
        "00:05.000 clr"
    };
    // `eov` is documented as "ignored entirely": it must not reset
    // the continuation scope, so the indented `second line` after it
    // still appends to the `cre` cue opened on line 1, and the cue
    // does not close until the `clr` on line 4.
    let cues = parse_lyrics(input).unwrap();
    assert_eq!(cues.len(), 1);
    assert_eq!(cues[0].parts[0].text, "first line\nsecond line");
    assert_eq!(cues[0].end, Timestamp::new(0, 5, 0).unwrap());
}

/// `eov` is documented as "ignored entirely" and pushes no
/// event of its own; it therefore does not compete with the
/// preceding `clr` for the same timestamp slot. This is the
/// shape the real source files use to mark the end of the
/// video at the moment the final cue clears.
#[test]
fn allows_eov_to_share_a_timestamp_with_the_preceding_clr() {
    let input = text_block_fnl! {
        "00:10.000 ttl: Title"
        "00:18.000 clr"
        "00:18.000 eov"
    };
    let cues = parse_lyrics(input).unwrap();
    assert_eq!(cues.len(), 1);
    assert_eq!(cues[0].end, Timestamp::new(0, 18, 0).unwrap());
}

#[test]
fn rejects_cue_marker_that_collides_with_control_token() {
    let clr_input = text_block_fnl! {
        "00:00.000 clr: Hello"
        "00:02.000 clr"
    };
    assert_eq!(
        parse_lyrics(clr_input).unwrap_err(),
        ParseLyricsError {
            line_number: 1,
            kind: ReservedMarker::Clear
                .pipe(ReservedControlMarker)
                .pipe(ParseLyricsErrorKind::ReservedControlMarker),
        },
    );

    let eov_input = text_block_fnl! {
        "00:00.000 eov: whatever"
        "00:02.000 clr"
    };
    assert_eq!(
        parse_lyrics(eov_input).unwrap_err(),
        ParseLyricsError {
            line_number: 1,
            kind: ReservedMarker::EndOfVideo
                .pipe(ReservedControlMarker)
                .pipe(ParseLyricsErrorKind::ReservedControlMarker),
        },
    );
}
