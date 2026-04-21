use super::{ExtraTextAfterControlMarker, ParseLyricsError, parse_lyrics};
use crate::timestamp::Timestamp;
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
    assert_eq!(cues[0].start, Timestamp::new(0, 0, 0));
    assert_eq!(cues[0].end, Timestamp::new(0, 2, 0));
    assert_eq!(cues[0].marker, "ttl");
    assert_eq!(cues[0].text, "Hello");
    assert_eq!(cues[1].start, Timestamp::new(0, 2, 0));
    assert_eq!(cues[1].end, Timestamp::new(0, 4, 0));
    assert_eq!(cues[1].marker, "LRC");
    assert_eq!(cues[1].text, "world");
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
    assert_eq!(cues[0].text, "Hello");
}

#[test]
fn continuation_lines_append_to_current_cue() {
    let input = text_block_fnl! {
        "00:00.000 cre: first line"
        "            second line"
        "            third line"
        "00:05.000 clr"
    };
    let cues = parse_lyrics(input).unwrap();
    assert_eq!(cues.len(), 1);
    assert_eq!(cues[0].text, "first line\nsecond line\nthird line");
}

#[test]
fn control_markers_accept_trailing_whitespace_only() {
    let input = text_block_fnl! {
        "00:00.000 ttl: Hello"
        "00:02.000 clr \t "
        "00:05.000 eov\t"
    };
    let cues = parse_lyrics(input).unwrap();
    assert_eq!(cues.len(), 1);
    assert_eq!(cues[0].end, Timestamp::new(0, 2, 0));
}

#[test]
fn control_markers_reject_trailing_text() {
    let clr_input = text_block_fnl! {
        "00:00.000 ttl: Hello"
        "00:02.000 clr some trailing text"
    };
    assert!(matches!(
        parse_lyrics(clr_input),
        Err(ParseLyricsError::ExtraTextAfterControlMarker(
            ExtraTextAfterControlMarker { marker, .. },
        )) if marker == "clr",
    ));

    let eov_input = text_block_fnl! {
        "00:00.000 ttl: Hello"
        "00:02.000 clr"
        "00:05.000 eov\tend of video"
    };
    assert!(matches!(
        parse_lyrics(eov_input),
        Err(ParseLyricsError::ExtraTextAfterControlMarker(
            ExtraTextAfterControlMarker { marker, .. },
        )) if marker == "eov",
    ));
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
    assert_eq!(cues[0].end, Timestamp::new(0, 2, 0));
}

#[test]
fn rejects_cue_line_without_marker() {
    let input = text_block_fnl! {
        "00:00.000 Plain text without marker"
        "00:02.000 clr"
    };
    assert!(matches!(
        parse_lyrics(input),
        Err(ParseLyricsError::MissingMarker(_)),
    ));
}

#[test]
fn rejects_timestamp_without_separator_after_prefix() {
    let input = text_block_fnl! {
        "00:00.000 ttl: Hello"
        "00:02.000ttl: no space after timestamp"
        "00:05.000 clr"
    };
    assert!(matches!(
        parse_lyrics(input),
        Err(ParseLyricsError::MissingSeparatorAfterTimestamp(_)),
    ));
}

#[test]
fn cue_ends_at_next_cue_when_no_clr() {
    let input = text_block_fnl! {
        "00:00.000 ttl: A"
        "00:01.000 ttl: B"
        "00:02.000 clr"
    };
    let cues = parse_lyrics(input).unwrap();
    assert_eq!(cues[0].end, Timestamp::new(0, 1, 0));
    assert_eq!(cues[1].end, Timestamp::new(0, 2, 0));
}

#[test]
fn rejects_cue_without_following_event() {
    let input = "00:00.000 ttl: Hello\n";
    assert!(matches!(
        parse_lyrics(input),
        Err(ParseLyricsError::UnclosedCue(_)),
    ));
}

#[test]
fn rejects_out_of_order_events() {
    let input = text_block_fnl! {
        "00:02.000 ttl: A"
        "00:01.000 ttl: B"
        "00:03.000 clr"
    };
    assert!(matches!(
        parse_lyrics(input),
        Err(ParseLyricsError::OutOfOrder(_)),
    ));
}
