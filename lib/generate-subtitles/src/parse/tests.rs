use super::error::{
    AdditiveRegionError, ControlMarkerInRegion, CueTextReservedCharacter, EmptyAnnotation,
    EmptyCueBody, EmptyRegion, ExtraTextAfterControlMarker, InvalidTimestamp, MalformedHeader,
    MalformedIndentation, MalformedTagLine, MissingMarker, MissingSeparatorAfterTimestamp,
    NestedRegion, OrphanedAnnotation, OrphanedShorthandMarker, OutOfOrder, ParseLyricsError,
    RepeatedTimestamp, ReservedControlMarker, TabIndentation, UnclosedCue, UnclosedRegion,
    UnopenedRegion,
};
use super::parse_lyrics;
use lyrics_core::line_markers_descriptor::ReservedMarker;
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
            marker: ReservedMarker::Clear,
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
            marker: ReservedMarker::EndOfVideo,
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
            marker: ReservedMarker::Clear,
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
            marker: ReservedMarker::EndOfVideo,
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
            marker: ReservedMarker::Annotation,
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

/// The defining behavior of an additive region: a cue does not
/// replace the one above it but renders below it, so the region
/// builds up a line at a time. The final cue ends at the event that
/// follows the closing tag, which is the `clr` here.
#[test]
fn additive_region_accumulates_each_cue_onto_the_ones_above_it() {
    let input = text_block_fnl! {
        "<additive>"
        "07:11.111 LRC: first line"
        "07:22.222 LRC: second line"
        "07:33.333 LRC: third line"
        "07:44.444 LRC: fourth line"
        "</additive>"
        ""
        "07:55.555 clr"
    };
    let cues = parse_lyrics(input).unwrap();
    let bodies: Vec<Vec<&str>> = cues
        .iter()
        .map(|cue| cue.parts.iter().map(|part| part.text.as_str()).collect())
        .collect();
    assert_eq!(
        bodies,
        vec![
            vec!["first line"],
            vec!["first line", "second line"],
            vec!["first line", "second line", "third line"],
            vec!["first line", "second line", "third line", "fourth line"],
        ],
    );
    assert_eq!(cues[0].start, Timestamp::new(7, 11, 111).unwrap());
    assert_eq!(cues[0].end, Timestamp::new(7, 22, 222).unwrap());
    assert_eq!(cues[3].start, Timestamp::new(7, 44, 444).unwrap());
    assert_eq!(cues[3].end, Timestamp::new(7, 55, 555).unwrap());
}

/// Every carried part keeps the marker of the line that wrote it,
/// so a region may mix markers and each accumulated line still
/// renders under its own presentation rules.
#[test]
fn carried_parts_keep_their_own_markers() {
    let input = text_block_fnl! {
        "<additive>"
        "07:11.111 ttl: title body"
        "07:22.222 LRC: lyric body"
        "</additive>"
        "07:33.333 clr"
    };
    let cues = parse_lyrics(input).unwrap();
    let markers: Vec<&str> = cues[1]
        .parts
        .iter()
        .map(|part| part.marker.as_str())
        .collect();
    assert_eq!(markers, ["ttl", "LRC"]);
}

/// A cue group inside a region may carry several parts of its own
/// through the shorthand column, and all of them carry forward
/// together.
#[test]
fn a_multi_part_cue_group_carries_all_of_its_parts_forward() {
    let input = text_block_fnl! {
        "<additive>"
        "07:11.111 ttl: title body"
        "          cre: credit body"
        "07:22.222 LRC: lyric body"
        "</additive>"
        "07:33.333 clr"
    };
    let cues = parse_lyrics(input).unwrap();
    let bodies: Vec<&str> = cues[1]
        .parts
        .iter()
        .map(|part| part.text.as_str())
        .collect();
    assert_eq!(bodies, ["title body", "credit body", "lyric body"]);
}

/// Continuation lines belong to the part that opened them, so a
/// multi-line part carries forward as one part with its line breaks
/// intact rather than as several.
#[test]
fn a_carried_part_keeps_its_continuation_lines() {
    let input = text_block_fnl! {
        "<additive>"
        "07:11.111 cre: first body line"
        "               second body line"
        "07:22.222 LRC: lyric body"
        "</additive>"
        "07:33.333 clr"
    };
    let cues = parse_lyrics(input).unwrap();
    assert_eq!(cues[1].parts.len(), 2);
    assert_eq!(cues[1].parts[0].text, "first body line\nsecond body line");
    assert_eq!(cues[1].parts[1].text, "lyric body");
}

/// A cue after the closing tag is an ordinary cue again: it replaces
/// what the region left on screen instead of extending it.
#[test]
fn a_cue_after_the_region_does_not_accumulate() {
    let input = text_block_fnl! {
        "<additive>"
        "07:11.111 LRC: first line"
        "07:22.222 LRC: second line"
        "</additive>"
        "07:33.333 LRC: after the region"
        "07:44.444 clr"
    };
    let cues = parse_lyrics(input).unwrap();
    assert_eq!(cues.len(), 3);
    assert_eq!(cues[2].parts.len(), 1);
    assert_eq!(cues[2].parts[0].text, "after the region");
}

/// A cue before the opening tag is likewise untouched, and the
/// region starts its accumulation from nothing rather than from
/// whatever preceded it.
#[test]
fn a_cue_before_the_region_is_not_carried_into_it() {
    let input = text_block_fnl! {
        "07:11.111 LRC: before the region"
        "<additive>"
        "07:22.222 LRC: first line"
        "</additive>"
        "07:33.333 clr"
    };
    let cues = parse_lyrics(input).unwrap();
    assert_eq!(cues.len(), 2);
    assert_eq!(cues[0].parts[0].text, "before the region");
    assert_eq!(cues[1].parts.len(), 1);
    assert_eq!(cues[1].parts[0].text, "first line");
}

/// Two regions may sit back to back with no event between them. Each
/// accumulates on its own, so the second starts empty rather than
/// continuing the first.
#[test]
fn adjacent_regions_accumulate_independently() {
    let input = text_block_fnl! {
        "<additive>"
        "07:11.111 LRC: first line"
        "07:22.222 LRC: second line"
        "</additive>"
        "<additive>"
        "07:33.333 LRC: third line"
        "07:44.444 LRC: fourth line"
        "</additive>"
        "07:55.555 clr"
    };
    let cues = parse_lyrics(input).unwrap();
    let bodies: Vec<Vec<&str>> = cues
        .iter()
        .map(|cue| cue.parts.iter().map(|part| part.text.as_str()).collect())
        .collect();
    assert_eq!(
        bodies,
        vec![
            vec!["first line"],
            vec!["first line", "second line"],
            vec!["third line"],
            vec!["third line", "fourth line"],
        ],
    );
}

/// A region of one cue is legal and renders exactly as the same cue
/// would without the tags, because there is nothing above it to
/// accumulate.
#[test]
fn a_region_of_one_cue_renders_that_cue_alone() {
    let input = text_block_fnl! {
        "<additive>"
        "07:11.111 LRC: only line"
        "</additive>"
        "07:22.222 clr"
    };
    let cues = parse_lyrics(input).unwrap();
    assert_eq!(cues.len(), 1);
    assert_eq!(cues[0].parts.len(), 1);
    assert_eq!(cues[0].parts[0].text, "only line");
    assert_eq!(cues[0].end, Timestamp::new(7, 22, 222).unwrap());
}

/// An annotation documents the line its author wrote it under, so it
/// stays on that cue rather than repeating on every cue below it in
/// the region.
#[test]
fn annotations_do_not_carry_forward_within_a_region() {
    let input = text_block_fnl! {
        "<additive>"
        "07:11.111 LRC: first line"
        "          ann: a note about the first line"
        "07:22.222 LRC: second line"
        "</additive>"
        "07:33.333 clr"
    };
    let cues = parse_lyrics(input).unwrap();
    assert_eq!(
        cues[0].parts[0].annotations,
        ["a note about the first line"],
    );
    assert_eq!(cues[1].parts.len(), 2);
    assert_eq!(cues[1].parts[0].annotations, Vec::<String>::new());
    assert_eq!(cues[1].parts[1].annotations, Vec::<String>::new());
}

/// A region encloses cues, not the boundary events that end them, so
/// the two nesting shapes and the two control markers are all
/// rejected.
#[test]
fn rejects_a_region_opened_inside_another_region() {
    let doubled_tags = text_block_fnl! {
        "<additive>"
        "<additive>"
        "07:11.111 LRC: first line"
        "</additive>"
        "</additive>"
        "07:22.222 clr"
    };
    assert_eq!(
        parse_lyrics(doubled_tags).unwrap_err(),
        ParseLyricsError::AdditiveRegion(AdditiveRegionError::Nested(NestedRegion {
            line_number: 2,
            opened_at: 1,
        })),
    );

    let inner_region = text_block_fnl! {
        "<additive>"
        "07:11.111 LRC: first line"
        "<additive>"
        "07:22.222 LRC: second line"
        "</additive>"
        "07:33.333 LRC: third line"
        "</additive>"
        "07:44.444 clr"
    };
    assert_eq!(
        parse_lyrics(inner_region).unwrap_err(),
        ParseLyricsError::AdditiveRegion(AdditiveRegionError::Nested(NestedRegion {
            line_number: 3,
            opened_at: 1,
        })),
    );
}

#[test]
fn rejects_a_control_marker_inside_a_region() {
    let clear_input = text_block_fnl! {
        "<additive>"
        "07:11.111 LRC: first line"
        "07:22.222 clr"
        "</additive>"
    };
    assert_eq!(
        parse_lyrics(clear_input).unwrap_err(),
        ParseLyricsError::AdditiveRegion(AdditiveRegionError::ControlMarker(
            ControlMarkerInRegion {
                line_number: 3,
                marker: ReservedMarker::Clear,
                opened_at: 1,
            }
        )),
    );

    let end_of_video_input = text_block_fnl! {
        "<additive>"
        "07:11.111 LRC: first line"
        "07:22.222 eov"
        "</additive>"
    };
    assert_eq!(
        parse_lyrics(end_of_video_input).unwrap_err(),
        ParseLyricsError::AdditiveRegion(AdditiveRegionError::ControlMarker(
            ControlMarkerInRegion {
                line_number: 3,
                marker: ReservedMarker::EndOfVideo,
                opened_at: 1,
            }
        )),
    );
}

/// The diagnostic for an unclosed region names the opening tag
/// rather than the end of the file, because that is the line the
/// author has to revisit.
#[test]
fn rejects_a_region_that_is_never_closed() {
    let input = text_block_fnl! {
        "07:11.111 LRC: before the region"
        "<additive>"
        "07:22.222 LRC: first line"
        "07:33.333 LRC: second line"
    };
    assert_eq!(
        parse_lyrics(input).unwrap_err(),
        ParseLyricsError::AdditiveRegion(AdditiveRegionError::Unclosed(UnclosedRegion {
            line_number: 2
        })),
    );
}

#[test]
fn rejects_a_closing_tag_without_an_opening_one() {
    let input = text_block_fnl! {
        "07:11.111 LRC: first line"
        "</additive>"
        "07:22.222 clr"
    };
    assert_eq!(
        parse_lyrics(input).unwrap_err(),
        ParseLyricsError::AdditiveRegion(AdditiveRegionError::Unopened(UnopenedRegion {
            line_number: 2
        })),
    );
}

/// A region exists to accumulate cues, so one that encloses none is
/// an authoring mistake rather than a silent no-op. Comments and
/// blank lines do not count as content.
#[test]
fn rejects_a_region_that_encloses_no_cue() {
    let input = text_block_fnl! {
        "<additive>"
        "# a comment is not a cue"
        ""
        "</additive>"
        "07:11.111 LRC: first line"
        "07:22.222 clr"
    };
    assert_eq!(
        parse_lyrics(input).unwrap_err(),
        ParseLyricsError::AdditiveRegion(AdditiveRegionError::Empty(EmptyRegion {
            line_number: 4,
            opened_at: 1,
        })),
    );
}

/// A tag carries no attributes, so the two spellings are matched
/// literally and every near miss draws the same diagnostic, which
/// names both spellings in full rather than describing a grammar.
/// Whitespace inside the delimiters is a near miss like any other:
/// `<additive >` would otherwise have to read as an empty attribute
/// list, and the format has no attributes to read.
#[test]
fn rejects_every_near_miss_of_a_tag_line() {
    let near_misses = [
        "<verse>",
        "<additive",
        "</additive",
        "<>",
        "</>",
        "< additive>",
        "<additive >",
        "</ additive>",
        "</additive >",
        "< /additive>",
        "<additive> and some trailing text",
        "<additive> ",
        "<additive></additive>",
    ];
    for content in near_misses {
        let input = format!(
            "{content}\n\
             07:11.111 LRC: first line\n\
             07:22.222 clr\n",
        );
        assert_eq!(
            parse_lyrics(&input).unwrap_err(),
            ParseLyricsError::MalformedTagLine(MalformedTagLine {
                line_number: 1,
                content: content.to_string(),
            }),
            "{content:?} must not be accepted as a tag line",
        );
    }
}

/// The other half of the literal match: both spellings are accepted
/// exactly as written.
#[test]
fn accepts_both_tag_spellings_exactly_as_written() {
    let input = text_block_fnl! {
        "<additive>"
        "07:11.111 LRC: first line"
        "07:22.222 LRC: second line"
        "</additive>"
        "07:33.333 clr"
    };
    let cues = parse_lyrics(input).unwrap();
    assert_eq!(cues.len(), 2);
    assert_eq!(cues[1].parts.len(), 2);
}

/// A tag ends the scope of the cue above it, exactly as `clr` does,
/// so a shorthand marker line below the tag has no cue to attach to.
#[test]
fn a_tag_ends_the_scope_of_the_cue_above_it() {
    let after_opening_tag = text_block_fnl! {
        "07:11.111 ttl: title body"
        "<additive>"
        "          cre: credit body"
        "</additive>"
        "07:22.222 clr"
    };
    assert_eq!(
        parse_lyrics(after_opening_tag).unwrap_err(),
        ParseLyricsError::OrphanedShorthandMarker(OrphanedShorthandMarker {
            line_number: 3,
            content: "cre: credit body".to_string(),
        }),
    );

    let after_closing_tag = text_block_fnl! {
        "<additive>"
        "07:11.111 ttl: title body"
        "</additive>"
        "          cre: credit body"
        "07:22.222 clr"
    };
    assert_eq!(
        parse_lyrics(after_closing_tag).unwrap_err(),
        ParseLyricsError::OrphanedShorthandMarker(OrphanedShorthandMarker {
            line_number: 4,
            content: "cre: credit body".to_string(),
        }),
    );
}

/// Angle brackets stay reserved inside cue text. A tag is recognized
/// only at column zero, so writing one in a cue body reports the
/// reserved character rather than opening a region.
#[test]
fn a_tag_written_inside_cue_text_is_still_a_reserved_character() {
    let input = text_block_fnl! {
        "07:11.111 LRC: <additive>"
        "07:22.222 clr"
    };
    assert_eq!(
        parse_lyrics(input).unwrap_err(),
        ParseLyricsError::CueTextReservedCharacter(CueTextReservedCharacter {
            line_number: 1,
            character: '<',
        }),
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
        ParseLyricsError::OutOfOrder(OutOfOrder {
            previous: Timestamp::new(7, 22, 222).unwrap(),
            next: Timestamp::new(7, 11, 111).unwrap(),
        }),
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
        ParseLyricsError::RepeatedTimestamp(RepeatedTimestamp {
            line_number: 3,
            start: Timestamp::new(7, 11, 111).unwrap(),
        }),
    );
}
