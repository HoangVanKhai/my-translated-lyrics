use super::error::{
    CueTextReservedCharacter, EmptyAnnotation, EmptyCueBody, ExtraTextAfterControlMarker,
    InvalidTimestamp, MalformedHeader, MalformedIndentation, MissingMarker,
    MissingSeparatorAfterTimestamp, OrphanedAnnotation, OrphanedShorthandMarker, OutOfOrder,
    ParseLyricsError, RepeatedTimestamp, ReservedControlMarker, TabIndentation, UnclosedCue,
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
/// `split_marker` the error would surface as [`MissingMarker`]
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
/// parser raises [`MalformedIndentation`] with both expected
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
/// [`MalformedHeader`] diagnostic.
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
/// `MissingMarker { content: "" }`. The dedicated [`EmptyCueBody`]
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

/// An `ann` line at the column-10 shorthand indent attaches its text
/// to the cue part written above it. It opens no cue of its own, so
/// the cue count and every timestamp are the same as they would be
/// with the annotation deleted.
#[test]
fn annotation_attaches_to_the_part_above_it() {
    let input = text_block_fnl! {
        "00:00.000 ttl: title body"
        "00:02.000 LRC: lyric body"
        "          ann: a note about the lyric"
        "00:06.000 clr"
    };
    let cues = parse_lyrics(input).unwrap();
    assert_eq!(cues.len(), 2);
    assert_eq!(cues[0].parts[0].annotations, Vec::<String>::new());
    assert_eq!(cues[1].start, Timestamp::new(0, 2, 0).unwrap());
    assert_eq!(cues[1].end, Timestamp::new(0, 6, 0).unwrap());
    assert_eq!(cues[1].parts.len(), 1);
    assert_eq!(cues[1].parts[0].marker, "LRC");
    assert_eq!(cues[1].parts[0].text, "lyric body");
    assert_eq!(cues[1].parts[0].annotations, ["a note about the lyric"]);
}

/// An annotation is exactly one line, so consecutive `ann` lines
/// each append a separate annotation to the same part rather than
/// the later ones replacing or extending the earlier ones. A note
/// spanning several lines is written this way.
#[test]
fn consecutive_annotations_stack_on_one_part() {
    let input = text_block_fnl! {
        "00:00.000 LRC: lyric body"
        "          ann: first note"
        "          ann: second note"
        "00:04.000 clr"
    };
    let cues = parse_lyrics(input).unwrap();
    assert_eq!(cues[0].parts[0].annotations, ["first note", "second note"]);
    assert_eq!(cues[0].parts[0].text, "lyric body");
}

/// A part's continuation lines and its annotations sit in one cue
/// part without interfering, each line extending whatever its indent
/// names.
#[test]
fn a_part_continues_above_its_annotations() {
    let input = text_block_fnl! {
        "00:00.000 cre: role-a name-a"
        "               role-a name-b"
        "          ann: a note about the credits"
        "00:04.000 clr"
    };
    let cues = parse_lyrics(input).unwrap();
    assert_eq!(cues[0].parts[0].text, "role-a name-a\nrole-a name-b");
    assert_eq!(cues[0].parts[0].annotations, ["a note about the credits"]);
}

/// The part's continuation width is unaffected by an intervening
/// annotation. `chorus: ` is 8 bytes, so its continuation indent is
/// 18 and differs from the 15 an `ann: ` prefix would imply; the
/// part's own width is the one that stays in force.
#[test]
fn annotations_do_not_disturb_the_part_continuation_width() {
    let input = text_block_fnl! {
        "00:00.000 chorus: part opener"
        "                  part continuation"
        "          ann: a note"
        "00:04.000 clr"
    };
    let cues = parse_lyrics(input).unwrap();
    assert_eq!(cues[0].parts.len(), 1);
    assert_eq!(cues[0].parts[0].text, "part opener\npart continuation");
    assert_eq!(cues[0].parts[0].annotations, ["a note"]);
}

/// A shorthand marker line after an annotation opens a fresh part
/// and moves the continuation scope back to that part's text, so an
/// annotation does not strand the rest of the cue group.
#[test]
fn shorthand_marker_after_an_annotation_opens_a_new_part() {
    let input = text_block_fnl! {
        "00:00.000 ttl: title body"
        "          ann: a note about the title"
        "          cre: credit body"
        "               credit continuation"
        "00:04.000 clr"
    };
    let cues = parse_lyrics(input).unwrap();
    assert_eq!(cues[0].parts.len(), 2);
    assert_eq!(cues[0].parts[0].annotations, ["a note about the title"]);
    assert_eq!(cues[0].parts[1].marker, "cre");
    assert_eq!(cues[0].parts[1].text, "credit body\ncredit continuation");
    assert_eq!(cues[0].parts[1].annotations, Vec::<String>::new());
}

/// `<` and `>` are rejected in cue text because the renderers hand
/// that text to the WebVTT cue-tag grammar. Annotation text reaches
/// no renderer, so the same characters are ordinary punctuation
/// there and must survive both the opening line and a continuation.
#[test]
fn annotation_text_accepts_angle_brackets() {
    let input = text_block_fnl! {
        "00:00.000 LRC: lyric body"
        "          ann: compare <source a>"
        "               with <source b>"
        "00:04.000 clr"
    };
    let cues = parse_lyrics(input).unwrap();
    assert_eq!(
        cues[0].parts[0].annotations,
        ["compare <source a>\nwith <source b>"],
    );
}

/// Unlike `clr` and `eov`, an annotation exists only to carry prose,
/// so an empty one is rejected.
#[test]
fn rejects_annotation_without_a_body() {
    let empty_body = text_block_fnl! {
        "00:00.000 LRC: lyric body"
        "          ann:"
        "00:04.000 clr"
    };
    assert_eq!(
        parse_lyrics(empty_body).unwrap_err(),
        ParseLyricsError::EmptyAnnotation(EmptyAnnotation { line_number: 2 }),
    );

    // Without a `:` the line names no marker at all, so it draws the
    // ordinary diagnostic for a line that carries none rather than an
    // annotation one. No line is searched for a marker it does not
    // declare.
    let no_separator = text_block_fnl! {
        "00:00.000 LRC: lyric body"
        "          ann"
        "00:04.000 clr"
    };
    assert_eq!(
        parse_lyrics(no_separator).unwrap_err(),
        ParseLyricsError::MissingMarker(MissingMarker {
            line_number: 2,
            content: "ann".to_string(),
        }),
    );
}

/// An annotation needs a part to attach to, so one that appears
/// before any cue is open reports its own diagnostic rather than the
/// shorthand-marker one.
#[test]
fn rejects_annotation_before_any_cue_is_open() {
    let input = text_block_fnl! {
        "          ann: orphan note"
        "00:01.000 clr"
    };
    assert_eq!(
        parse_lyrics(input).unwrap_err(),
        ParseLyricsError::OrphanedAnnotation(OrphanedAnnotation {
            line_number: 1,
            content: "ann: orphan note".to_string(),
        }),
    );
}

/// A `clr` closes the open cue, so an annotation after one has no
/// part to attach to even though a timestamp opened a cue earlier in
/// the file.
#[test]
fn rejects_annotation_after_a_clear() {
    let input = text_block_fnl! {
        "00:00.000 LRC: lyric body"
        "00:02.000 clr"
        "          ann: stray note"
        "00:06.000 clr"
    };
    assert_eq!(
        parse_lyrics(input).unwrap_err(),
        ParseLyricsError::OrphanedAnnotation(OrphanedAnnotation {
            line_number: 3,
            content: "ann: stray note".to_string(),
        }),
    );
}

/// An annotation attaches to the most recently opened part, not to
/// the first part of the cue group, so a note under a shorthand
/// marker line belongs to that marker's part.
#[test]
fn annotation_attaches_to_a_shorthand_opened_part() {
    let input = text_block_fnl! {
        "00:00.000 ttl: title body"
        "          cre: credit body"
        "          ann: a note about the credit"
        "00:04.000 clr"
    };
    let cues = parse_lyrics(input).unwrap();
    assert_eq!(cues[0].parts.len(), 2);
    assert_eq!(cues[0].parts[0].annotations, Vec::<String>::new());
    assert_eq!(cues[0].parts[1].marker, "cre");
    assert_eq!(cues[0].parts[1].annotations, ["a note about the credit"]);
}

/// The annotation marker is reserved wherever a marker is parsed, so
/// writing it with a timestamp is rejected the same way `clr` and
/// `eov` are rather than opening a cue that would render the note.
#[test]
fn annotation_marker_cannot_name_a_cue() {
    let input = text_block_fnl! {
        "00:00.000 LRC: lyric body"
        "00:02.000 ann: a note"
        "00:04.000 clr"
    };
    assert_eq!(
        parse_lyrics(input).unwrap_err(),
        ParseLyricsError::ReservedControlMarker(ReservedControlMarker {
            line_number: 2,
            marker: "ann".to_string(),
        }),
    );
}

/// Continuation text is arbitrary prose and is never inspected for a
/// marker. A continuation that happens to read like an annotation is
/// cue text, exactly as any other continuation line would be, and the
/// indent alone decides where it goes.
#[test]
fn a_continuation_is_never_read_as_an_annotation() {
    let input = text_block_fnl! {
        "00:00.000 cre: role-a name-a"
        "               ann, my dear"
        "          ann: a note about the credits"
        "00:04.000 clr"
    };
    let cues = parse_lyrics(input).unwrap();
    assert_eq!(cues[0].parts[0].text, "role-a name-a\nann, my dear");
    assert_eq!(cues[0].parts[0].annotations, ["a note about the credits"]);
}

/// An annotation takes continuation lines of its own, indented under
/// its text just as a cue part's are. Each `ann` line opens a fresh
/// annotation, so a part carrying two notes of two lines each yields
/// two entries with one line break apiece, and none of that text
/// reaches the cue.
#[test]
fn an_annotation_takes_continuation_lines() {
    let input = text_block_fnl! {
        "00:16.000 LRC: lyric content"
        "          ann: first line of first annotation"
        "               second line of first annotation"
        "          ann: first line of second annotation"
        "               second line of second annotation"
        "00:20.000 clr"
    };
    let cues = parse_lyrics(input).unwrap();
    assert_eq!(cues[0].parts[0].text, "lyric content");
    assert_eq!(
        cues[0].parts[0].annotations,
        [
            "first line of first annotation\nsecond line of first annotation",
            "first line of second annotation\nsecond line of second annotation",
        ],
    );
}

/// Notes on one part may mix lengths freely: the entry count follows
/// the `ann` lines and the line breaks follow the continuations, so
/// each of the two carries one job.
#[test]
fn annotations_on_one_part_may_differ_in_length() {
    let input = text_block_fnl! {
        "00:16.000 LRC: lyric content"
        "          ann: first annotation"
        "          ann: first line of second annotation"
        "               second line of second annotation"
        "00:20.000 clr"
    };
    let cues = parse_lyrics(input).unwrap();
    assert_eq!(cues[0].parts[0].text, "lyric content");
    assert_eq!(
        cues[0].parts[0].annotations,
        [
            "first annotation",
            "first line of second annotation\nsecond line of second annotation",
        ],
    );
}
