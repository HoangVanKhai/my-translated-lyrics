//! Tests for the timestamps that order the event stream. They cover the
//! rejection of a start time that repeats or precedes the one before it,
//! the diagnostic a timestamp with an out-of-range field carries, and the
//! application of the same rules to the cues inside an `<additive>`
//! region.

use crate::parse::error::{
    InvalidTimestamp, OutOfOrder, ParseLyricsError, ParseLyricsErrorKind, RepeatedTimestamp,
};
use crate::parse::parse_lyrics;
use lyrics_core::timestamp::{SecondsOutOfRange, TakeTimestampError, Timestamp};
use pipe_trait::Pipe;
use pretty_assertions::assert_eq;
use text_block_macros::text_block_fnl;

#[test]
fn rejects_out_of_order_events() {
    let input = text_block_fnl! {
        "00:02.000 ttl: A"
        "00:01.000 ttl: B"
        "00:03.000 clr"
    };
    assert_eq!(
        parse_lyrics(input).unwrap_err(),
        ParseLyricsError {
            line_number: 2,
            kind: ParseLyricsErrorKind::OutOfOrder(OutOfOrder {
                previous: Timestamp::new(0, 2, 0).unwrap(),
                next: Timestamp::new(0, 1, 0).unwrap(),
            }),
        },
    );
}

#[test]
fn rejects_repeated_timestamp_for_consecutive_cue_lines() {
    let input = text_block_fnl! {
        "00:10.080 ttl: Title"
        "00:10.080 cre: Credit"
        "00:18.000 clr"
    };
    assert_eq!(
        parse_lyrics(input).unwrap_err(),
        ParseLyricsError {
            line_number: 2,
            kind: Timestamp::new(0, 10, 80)
                .unwrap()
                .pipe(RepeatedTimestamp)
                .pipe(ParseLyricsErrorKind::RepeatedTimestamp),
        },
    );
}

#[test]
fn invalid_timestamp_preserves_line_and_cause() {
    let input = text_block_fnl! {
        "00:60.000 ttl: seconds too high"
    };
    assert_eq!(
        parse_lyrics(input).unwrap_err(),
        ParseLyricsError {
            line_number: 1,
            kind: TakeTimestampError::SecondsOutOfRange(SecondsOutOfRange {
                raw: "00:60.000".to_string(),
                value: 60,
            })
            .pipe(InvalidTimestamp)
            .pipe(ParseLyricsErrorKind::InvalidTimestamp),
        },
    );
}

/// Cues inside a region are ordinary events for the ordering rules,
/// so the checks that guard the rest of the file still apply.
#[test]
fn ordering_rules_still_apply_inside_a_region() {
    let out_of_order = text_block_fnl! {
        "<additive>"
        "07:22.222 LRC: first line"
        "07:11.111 LRC: second line"
        "</additive>"
        "07:33.333 clr"
    };
    assert_eq!(
        parse_lyrics(out_of_order).unwrap_err(),
        ParseLyricsError {
            line_number: 3,
            kind: ParseLyricsErrorKind::OutOfOrder(OutOfOrder {
                previous: Timestamp::new(7, 22, 222).unwrap(),
                next: Timestamp::new(7, 11, 111).unwrap(),
            }),
        },
    );

    let repeated = text_block_fnl! {
        "<additive>"
        "07:11.111 LRC: first line"
        "07:11.111 LRC: second line"
        "</additive>"
        "07:22.222 clr"
    };
    assert_eq!(
        parse_lyrics(repeated).unwrap_err(),
        ParseLyricsError {
            line_number: 3,
            kind: Timestamp::new(7, 11, 111)
                .unwrap()
                .pipe(RepeatedTimestamp)
                .pipe(ParseLyricsErrorKind::RepeatedTimestamp),
        },
    );
}
