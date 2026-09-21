//! Tests for the ordinary cue stream. They cover the timestamped lines
//! that open cues, the continuation lines and shorthand marker lines that
//! extend them, and the end time each cue takes from the event that
//! follows it.

use crate::parse::error::{
    OrphanedShorthandMarker, ParseLyricsError, ParseLyricsErrorKind, UnclosedCue,
};
use crate::parse::{LineNumber, parse_lyrics};
use lyrics_core::timestamp::Timestamp;
use pipe_trait::Pipe;
use pretty_assertions::assert_eq;
use text_block_macros::text_block_fnl;

#[test]
fn parses_simple_sequence() {
    let input = text_block_fnl! {
        "00:00.000 ttl: Hello"
        "00:02.000 LRC: world"
        "00:04.000 clr"
    };
    let cues = parse_lyrics(input).unwrap();
    assert_eq!(cues.len(), 2);
    assert_eq!(cues[0].start, Timestamp::new(0, 0, 0).unwrap());
    assert_eq!(cues[0].end, Timestamp::new(0, 2, 0).unwrap());
    assert_eq!(cues[0].parts[0].marker.as_str(), "ttl");
    assert_eq!(cues[0].parts[0].text, "Hello");
    assert_eq!(cues[1].start, Timestamp::new(0, 2, 0).unwrap());
    assert_eq!(cues[1].end, Timestamp::new(0, 4, 0).unwrap());
    assert_eq!(cues[1].parts[0].marker.as_str(), "LRC");
    assert_eq!(cues[1].parts[0].text, "world");
}

#[test]
fn comments_and_blank_lines_are_skipped() {
    let input = text_block_fnl! {
        "# this is ignored"
        ""
        "00:00.000 ttl: Hello"
        "# still ignored"
        "00:02.000 clr"
    };
    let cues = parse_lyrics(input).unwrap();
    assert_eq!(cues.len(), 1);
    assert_eq!(cues[0].parts[0].text, "Hello");
}

#[test]
fn continuation_lines_append_to_current_cue() {
    let input = text_block_fnl! {
        "00:00.000 cre: first line"
        "               second line"
        "               third line"
        "00:05.000 clr"
    };
    let cues = parse_lyrics(input).unwrap();
    assert_eq!(cues.len(), 1);
    assert_eq!(cues[0].parts[0].text, "first line\nsecond line\nthird line");
}

#[test]
fn cue_ends_at_next_cue_when_no_clr() {
    let input = text_block_fnl! {
        "00:00.000 ttl: A"
        "00:01.000 ttl: B"
        "00:02.000 clr"
    };
    let cues = parse_lyrics(input).unwrap();
    assert_eq!(cues[0].end, Timestamp::new(0, 1, 0).unwrap());
    assert_eq!(cues[1].end, Timestamp::new(0, 2, 0).unwrap());
}

#[test]
fn rejects_cue_without_following_event() {
    let input = "00:00.000 ttl: Hello\n";
    assert_eq!(
        parse_lyrics(input).unwrap_err(),
        ParseLyricsError {
            line_number: LineNumber::new(1),
            kind: Timestamp::new(0, 0, 0)
                .unwrap()
                .pipe(UnclosedCue)
                .pipe(ParseLyricsErrorKind::UnclosedCue),
        },
    );
}

/// The failure is detected once the whole file has been read, so the
/// line it reports is the header that opened the cue rather than the
/// last line of the file. Comments and blank lines sit between the
/// two so a mistaken count cannot land on the right answer.
#[test]
fn an_unclosed_cue_names_the_line_that_opened_it() {
    let input = text_block_fnl! {
        "# a leading comment"
        ""
        "00:00.000 ttl: title body"
        "00:02.000 LRC: lyric body"
        "               a continuation"
        ""
        "# a trailing comment"
    };
    assert_eq!(
        parse_lyrics(input).unwrap_err(),
        ParseLyricsError {
            line_number: LineNumber::new(4),
            kind: Timestamp::new(0, 2, 0)
                .unwrap()
                .pipe(UnclosedCue)
                .pipe(ParseLyricsErrorKind::UnclosedCue),
        },
    );
}

/// The column-10 indent (`MM:SS.mmm `) opens a new marker that
/// shares the start time of the cue immediately above it. The
/// resulting `SubtitleCue` carries both markers as separate
/// parts; the renderer joins them into one subtitle block.
#[test]
fn shorthand_marker_attaches_a_second_part_to_the_same_cue() {
    let input = text_block_fnl! {
        "00:10.080 ttl: title body"
        "          cre: credit body"
        "00:18.000 clr"
    };
    let cues = parse_lyrics(input).unwrap();
    assert_eq!(cues.len(), 1);
    assert_eq!(cues[0].start, Timestamp::new(0, 10, 80).unwrap());
    assert_eq!(cues[0].end, Timestamp::new(0, 18, 0).unwrap());
    assert_eq!(cues[0].parts.len(), 2);
    assert_eq!(cues[0].parts[0].marker.as_str(), "ttl");
    assert_eq!(cues[0].parts[0].text, "title body");
    assert_eq!(cues[0].parts[1].marker.as_str(), "cre");
    assert_eq!(cues[0].parts[1].text, "credit body");
}

/// Once a shorthand marker line opens a new part, subsequent
/// continuation lines indent against that new marker's prefix
/// width, not the original first part's prefix width. Use a
/// marker whose prefix width differs from the first part's so
/// the rule cannot accidentally pass via shared indent.
/// `chorus: ` is 8 bytes, so the expected continuation indent
/// is `TIMESTAMP_PREFIX_WIDTH + 8 = 18`.
#[test]
fn shorthand_marker_part_can_carry_its_own_continuation_lines() {
    let input = text_block_fnl! {
        "00:10.080 ttl: first"
        "          chorus: opener"
        "                  continuation"
        "00:18.000 clr"
    };
    let cues = parse_lyrics(input).unwrap();
    assert_eq!(cues[0].parts.len(), 2);
    assert_eq!(cues[0].parts[1].marker.as_str(), "chorus");
    assert_eq!(cues[0].parts[1].text, "opener\ncontinuation");
}

/// A column-10 line cannot appear before a header has opened a
/// cue group; there is no start time to attach the new marker
/// to.
#[test]
fn rejects_shorthand_marker_before_any_cue_is_open() {
    let input = text_block_fnl! {
        "          ttl: orphan"
        "00:01.000 clr"
    };
    assert_eq!(
        parse_lyrics(input).unwrap_err(),
        ParseLyricsError {
            line_number: LineNumber::new(1),
            kind: "ttl: orphan"
                .to_string()
                .pipe(OrphanedShorthandMarker)
                .pipe(ParseLyricsErrorKind::OrphanedShorthandMarker),
        },
    );
}
