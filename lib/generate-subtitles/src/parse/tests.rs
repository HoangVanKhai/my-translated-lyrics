use super::error::{
    CueTextReservedCharacter, EmptyCueBody, ExtraTextAfterControlMarker, InvalidTimestamp,
    MalformedHeader, MalformedIndentation, MissingMarker, MissingSeparatorAfterTimestamp,
    OrphanedShorthandMarker, OutOfOrder, ParseLyricsError, RepeatedTimestamp,
    ReservedControlMarker, TabIndentation, UnclosedCue,
};
use super::parse_lyrics;
use lyrics_core::timestamp::{SecondsOutOfRange, TakeTimestampError, Timestamp};
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
    assert_eq!(cues[0].parts[0].marker, "ttl");
    assert_eq!(cues[0].parts[0].text, "Hello");
    assert_eq!(cues[1].start, Timestamp::new(0, 2, 0).unwrap());
    assert_eq!(cues[1].end, Timestamp::new(0, 4, 0).unwrap());
    assert_eq!(cues[1].parts[0].marker, "LRC");
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
        ParseLyricsError::UnclosedCue(UnclosedCue {
            start: Timestamp::new(0, 0, 0).unwrap()
        }),
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

/// `<` and `>` belong to the WebVTT cue-tag grammar, not to the
/// `lyrics.{lang}.txt` source format. The renderer later
/// HTML-entity-escapes the cue text, so a literal `<` in the
/// source would only survive to the output as `&lt;`; rejecting
/// it at parse time surfaces the author's intent early.
#[test]
fn rejects_angle_bracket_in_cue_opening_body() {
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

/// The validator fires on every cue-text line, not only on the
/// opening body, so a reserved character that only appears on a
/// continuation line is still caught at the line that contains
/// it. Cover both `<` and `>` so the continuation path is
/// locked symmetrically with the opening-line test above.
#[test]
fn rejects_angle_bracket_in_continuation_line() {
    let lt_input = text_block_fnl! {
        "00:00.000 cre: first line"
        "               second <tag line"
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
        "               end of tag>"
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

/// A line such as `00:00.000 <v>foo</v>` has no `:` separator,
/// so without the reserved-character check running before
/// `split_marker` the error would surface as `MissingMarker`
/// even though the real problem is the angle brackets. Verify
/// that the more specific diagnostic wins here.
#[test]
fn marker_less_body_with_reserved_character_reports_reserved_character() {
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

/// Indentation must be ASCII spaces only. A tab in the leading
/// whitespace produces a focused diagnostic at the line that
/// contains it. Tabs that appear after the first non-whitespace
/// character are not rejected by this rule, since the
/// restriction only governs the indentation column.
#[test]
fn rejects_tab_in_leading_whitespace() {
    let input = text_block_fnl! {
        "00:00.000 ttl: Hello"
        "\t            cre: tabbed indent"
        "00:05.000 clr"
    };
    assert_eq!(
        parse_lyrics(input).unwrap_err(),
        ParseLyricsError::TabIndentation(TabIndentation { line_number: 2 }),
    );
}

/// `&` is not VTT-specific on its own; it only forms markup when
/// it introduces an entity reference, and even then the renderer
/// HTML-entity-escapes the cue text before emission, so a lone
/// `&` in the source round-trips correctly.
#[test]
fn accepts_ampersand_in_cue_text() {
    let input = text_block_fnl! {
        "00:00.000 ttl: R&B classics"
        "00:02.000 clr"
    };
    let cues = parse_lyrics(input).unwrap();
    assert_eq!(cues.len(), 1);
    assert_eq!(cues[0].parts[0].text, "R&B classics");
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
    assert_eq!(cues[0].parts[0].marker, "ttl");
    assert_eq!(cues[0].parts[0].text, "title body");
    assert_eq!(cues[0].parts[1].marker, "cre");
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
    assert_eq!(cues[0].parts[1].marker, "chorus");
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
        ParseLyricsError::OrphanedShorthandMarker(OrphanedShorthandMarker {
            line_number: 1,
            content: "ttl: orphan".to_string(),
        }),
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
        ParseLyricsError::RepeatedTimestamp(RepeatedTimestamp {
            line_number: 2,
            start: Timestamp::new(0, 10, 80).unwrap(),
        }),
    );
}

/// 12 spaces is neither the column-10 shorthand indent nor the
/// 15-space continuation indent that `cre: ` expects, so the
/// parser raises `MalformedIndentation` with both expected
/// widths in the diagnostic.
#[test]
fn rejects_malformed_indentation_between_recognized_widths() {
    let input = text_block_fnl! {
        "00:10.080 cre: First"
        "            wrong indent"
        "00:18.000 clr"
    };
    assert_eq!(
        parse_lyrics(input).unwrap_err(),
        ParseLyricsError::MalformedIndentation(MalformedIndentation {
            line_number: 2,
            actual: 12,
            shorthand_indent: 10,
            continuation_indent: Some(15),
        }),
    );
}

/// The first non-blank, non-comment line at column zero is
/// expected to open a cue or fire a control event; without a
/// leading `MM:SS.mmm` shape the parser surfaces the dedicated
/// `MalformedHeader` diagnostic.
#[test]
fn rejects_malformed_header_when_column_zero_line_has_no_timestamp() {
    let input = "no timestamp here\n";
    assert_eq!(
        parse_lyrics(input).unwrap_err(),
        ParseLyricsError::MalformedHeader(MalformedHeader {
            line_number: 1,
            content: "no timestamp here".to_string(),
        }),
    );
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

/// A header line like `00:00.000   ` (timestamp, run of spaces,
/// no body) parses as `Timestamp::take` succeeding with three
/// trailing spaces, then `cue_body = after_prefix.trim_start()`
/// yields the empty string. The empty body has no `:` and no
/// marker, so `parse_marker_part` falls into the
/// `split_marker(body) -> None` branch and raises
/// `MissingMarker { content: "" }`. The dedicated `EmptyCueBody`
/// variant cannot apply here because it carries the marker
/// name, and a whitespace-only body has none. Lock the current
/// outcome so a future reader does not assume the diagnostic
/// is something else.
#[test]
fn whitespace_only_cue_body_falls_through_to_missing_marker() {
    let input = text_block_fnl! {
        "00:00.000   "
        "00:02.000 clr"
    };
    assert_eq!(
        parse_lyrics(input).unwrap_err(),
        ParseLyricsError::MissingMarker(MissingMarker {
            line_number: 1,
            content: String::new(),
        }),
    );
}
