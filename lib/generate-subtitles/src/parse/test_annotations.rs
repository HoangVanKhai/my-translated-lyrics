//! Tests for `ann` lines. They cover the part an annotation attaches to,
//! the way several annotations stack on one part, the continuation lines
//! an annotation takes of its own, and the positions in which an
//! annotation is rejected.

use crate::parse::error::{
    EmptyAnnotation, MissingMarker, OrphanedAnnotation, ParseLyricsError, ParseLyricsErrorKind,
    ReservedControlMarker,
};
use crate::parse::parse_lyrics;
use lyrics_core::line_markers_descriptor::ReservedMarker;
use lyrics_core::timestamp::Timestamp;
use pretty_assertions::assert_eq;
use text_block_macros::text_block_fnl;

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
        ParseLyricsError {
            line_number: 2,
            kind: ParseLyricsErrorKind::EmptyAnnotation(EmptyAnnotation),
        },
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
        ParseLyricsError {
            line_number: 2,
            kind: ParseLyricsErrorKind::MissingMarker(MissingMarker {
                content: "ann".to_string(),
            }),
        },
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
        ParseLyricsError {
            line_number: 1,
            kind: ParseLyricsErrorKind::OrphanedAnnotation(OrphanedAnnotation {
                content: "ann: orphan note".to_string(),
            }),
        },
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
        ParseLyricsError {
            line_number: 3,
            kind: ParseLyricsErrorKind::OrphanedAnnotation(OrphanedAnnotation {
                content: "ann: stray note".to_string(),
            }),
        },
    );
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
        ParseLyricsError {
            line_number: 2,
            kind: ParseLyricsErrorKind::ReservedControlMarker(ReservedControlMarker {
                marker: ReservedMarker::Annotation,
            }),
        },
    );
}
