use super::{
    CueTextReservedCharacter, EmptyCueBody, ExtraTextAfterControlMarker, InvalidTimestamp,
    MissingMarker, MissingSeparatorAfterTimestamp, OutOfOrder, ParseLyricsError,
    ReservedControlMarker, TabIndentation, parse_lyrics,
};
use crate::timestamp::{SecondsOutOfRange, TakeTimestampError, Timestamp};
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
    assert_eq!(cues[0].marker, "ttl");
    assert_eq!(cues[0].text, "Hello");
    assert_eq!(cues[1].start, Timestamp::new(0, 2, 0).unwrap());
    assert_eq!(cues[1].end, Timestamp::new(0, 4, 0).unwrap());
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
        ParseLyricsError::ExtraTextAfterControlMarker(ExtraTextAfterControlMarker {
            line_number: 2,
            marker: "clr".to_string(),
            trailing: "some trailing text".to_string(),
        }),
    );

    let eov_input = text_block_fnl! {
        "00:00.000 ttl: Hello"
        "00:02.000 clr"
        "00:05.000 eov\tend of video"
    };
    assert_eq!(
        parse_lyrics(eov_input).unwrap_err(),
        ParseLyricsError::ExtraTextAfterControlMarker(ExtraTextAfterControlMarker {
            line_number: 3,
            marker: "eov".to_string(),
            trailing: "end of video".to_string(),
        }),
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
        "            second line"
        "00:05.000 clr"
    };
    // `eov` is documented as "ignored entirely": it must not reset
    // the continuation scope, so the indented `second line` after it
    // still appends to the `cre` cue opened on line 1, and the cue
    // does not close until the `clr` on line 4.
    let cues = parse_lyrics(input).unwrap();
    assert_eq!(cues.len(), 1);
    assert_eq!(cues[0].text, "first line\nsecond line");
    assert_eq!(cues[0].end, Timestamp::new(0, 5, 0).unwrap());
}

#[test]
fn rejects_cue_line_without_marker() {
    let input = text_block_fnl! {
        "00:00.000 Plain text without marker"
        "00:02.000 clr"
    };
    assert_eq!(
        parse_lyrics(input).unwrap_err(),
        ParseLyricsError::MissingMarker(MissingMarker {
            line_number: 1,
            content: "Plain text without marker".to_string(),
        }),
    );
}

#[test]
fn rejects_timestamp_without_separator_after_prefix() {
    let input = text_block_fnl! {
        "00:00.000 ttl: Hello"
        "00:02.000ttl: no space after timestamp"
        "00:05.000 clr"
    };
    assert_eq!(
        parse_lyrics(input).unwrap_err(),
        ParseLyricsError::MissingSeparatorAfterTimestamp(MissingSeparatorAfterTimestamp {
            line_number: 2,
            content: "00:02.000ttl: no space after timestamp".to_string(),
        }),
    );
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
        ParseLyricsError::UnclosedCue(Timestamp::new(0, 0, 0).unwrap()),
    );
}

#[test]
fn rejects_out_of_order_events() {
    let input = text_block_fnl! {
        "00:02.000 ttl: A"
        "00:01.000 ttl: B"
        "00:03.000 clr"
    };
    assert_eq!(
        parse_lyrics(input).unwrap_err(),
        ParseLyricsError::OutOfOrder(OutOfOrder {
            previous: Timestamp::new(0, 2, 0).unwrap(),
            next: Timestamp::new(0, 1, 0).unwrap(),
        }),
    );
}

#[test]
fn rejects_cue_marker_that_collides_with_control_token() {
    let clr_input = text_block_fnl! {
        "00:00.000 clr: Hello"
        "00:02.000 clr"
    };
    assert_eq!(
        parse_lyrics(clr_input).unwrap_err(),
        ParseLyricsError::ReservedControlMarker(ReservedControlMarker {
            line_number: 1,
            marker: "clr".to_string(),
        }),
    );

    let eov_input = text_block_fnl! {
        "00:00.000 eov: whatever"
        "00:02.000 clr"
    };
    assert_eq!(
        parse_lyrics(eov_input).unwrap_err(),
        ParseLyricsError::ReservedControlMarker(ReservedControlMarker {
            line_number: 1,
            marker: "eov".to_string(),
        }),
    );
}

#[test]
fn rejects_cue_with_empty_body() {
    let input = text_block_fnl! {
        "00:00.000 ttl:"
        "00:02.000 clr"
    };
    assert_eq!(
        parse_lyrics(input).unwrap_err(),
        ParseLyricsError::EmptyCueBody(EmptyCueBody {
            line_number: 1,
            marker: "ttl".to_string(),
        }),
    );
}

#[test]
fn rejects_angle_bracket_in_cue_opening_body() {
    // `<` and `>` belong to the WebVTT cue-tag grammar, not to the
    // `lyrics.{lang}.txt` source format. The renderer later
    // HTML-entity-escapes the cue text, so a literal `<` in the
    // source would only survive to the output as `&lt;`; rejecting
    // it at parse time surfaces the author's intent early.
    let lt_input = text_block_fnl! {
        "00:00.000 ttl: hello <world>"
        "00:02.000 clr"
    };
    assert_eq!(
        parse_lyrics(lt_input).unwrap_err(),
        ParseLyricsError::CueTextReservedCharacter(CueTextReservedCharacter {
            line_number: 1,
            character: '<',
        }),
    );

    let gt_input = text_block_fnl! {
        "00:00.000 ttl: end>"
        "00:02.000 clr"
    };
    assert_eq!(
        parse_lyrics(gt_input).unwrap_err(),
        ParseLyricsError::CueTextReservedCharacter(CueTextReservedCharacter {
            line_number: 1,
            character: '>',
        }),
    );
}

#[test]
fn rejects_angle_bracket_in_continuation_line() {
    // The validator fires on every cue-text line, not only on the
    // opening body, so a reserved character that only appears on a
    // continuation line is still caught at the line that contains
    // it. Cover both `<` and `>` so the continuation path is
    // locked symmetrically with the opening-line test above.
    let lt_input = text_block_fnl! {
        "00:00.000 cre: first line"
        "            second <tag line"
        "00:05.000 clr"
    };
    assert_eq!(
        parse_lyrics(lt_input).unwrap_err(),
        ParseLyricsError::CueTextReservedCharacter(CueTextReservedCharacter {
            line_number: 2,
            character: '<',
        }),
    );

    let gt_input = text_block_fnl! {
        "00:00.000 cre: first line"
        "            end of tag>"
        "00:05.000 clr"
    };
    assert_eq!(
        parse_lyrics(gt_input).unwrap_err(),
        ParseLyricsError::CueTextReservedCharacter(CueTextReservedCharacter {
            line_number: 2,
            character: '>',
        }),
    );
}

#[test]
fn marker_less_body_with_reserved_character_reports_reserved_character() {
    // A line such as `00:00.000 <v>foo</v>` has no `:` separator,
    // so without the reserved-character check running before
    // `split_marker` the error would surface as `MissingMarker`
    // even though the real problem is the angle brackets. Verify
    // that the more specific diagnostic wins here.
    let input = text_block_fnl! {
        "00:00.000 <v>foo</v>"
        "00:02.000 clr"
    };
    assert_eq!(
        parse_lyrics(input).unwrap_err(),
        ParseLyricsError::CueTextReservedCharacter(CueTextReservedCharacter {
            line_number: 1,
            character: '<',
        }),
    );
}

#[test]
fn rejects_tab_in_leading_whitespace() {
    // Indentation must be ASCII spaces only. A tab in the leading
    // whitespace produces a focused diagnostic at the line that
    // contains it. Tabs that appear after the first non-whitespace
    // character are not rejected by this rule, since the
    // restriction only governs the indentation column.
    let input = "00:00.000 ttl: Hello\n\t            cre: tabbed indent\n00:05.000 clr\n";
    assert_eq!(
        parse_lyrics(input).unwrap_err(),
        ParseLyricsError::TabIndentation(TabIndentation { line_number: 2 }),
    );
}

#[test]
fn accepts_ampersand_in_cue_text() {
    // `&` is not VTT-specific on its own; it only forms markup when
    // it introduces an entity reference, and even then the renderer
    // HTML-entity-escapes the cue text before emission, so a lone
    // `&` in the source round-trips correctly.
    let input = text_block_fnl! {
        "00:00.000 ttl: R&B classics"
        "00:02.000 clr"
    };
    let cues = parse_lyrics(input).unwrap();
    assert_eq!(cues.len(), 1);
    assert_eq!(cues[0].text, "R&B classics");
}

#[test]
fn invalid_timestamp_preserves_line_and_cause() {
    let input = text_block_fnl! {
        "00:60.000 ttl: seconds too high"
    };
    assert_eq!(
        parse_lyrics(input).unwrap_err(),
        ParseLyricsError::InvalidTimestamp(InvalidTimestamp {
            line_number: 1,
            cause: TakeTimestampError::SecondsOutOfRange(SecondsOutOfRange {
                raw: "00:60.000".to_string(),
                value: 60,
            }),
        }),
    );
}
