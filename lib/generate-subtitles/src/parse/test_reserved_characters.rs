//! Tests for `<` and `>`, the two characters the WebVTT cue-tag grammar
//! reserves and cue text therefore may not carry, on an opening line as
//! much as on a continuation line. A character the grammar leaves alone,
//! such as the ampersand, reaches the cue unchanged.

use crate::parse::error::{CueTextReservedCharacter, ParseLyricsError, ParseLyricsErrorKind};
use crate::parse::parse_lyrics;
use pretty_assertions::assert_eq;
use text_block_macros::text_block_fnl;

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
        ParseLyricsError {
            line_number: 1,
            kind: ParseLyricsErrorKind::CueTextReservedCharacter(CueTextReservedCharacter('<')),
        },
    );

    let gt_input = text_block_fnl! {
        "00:00.000 ttl: end>"
        "00:02.000 clr"
    };
    assert_eq!(
        parse_lyrics(gt_input).unwrap_err(),
        ParseLyricsError {
            line_number: 1,
            kind: ParseLyricsErrorKind::CueTextReservedCharacter(CueTextReservedCharacter('>')),
        },
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
        ParseLyricsError {
            line_number: 2,
            kind: ParseLyricsErrorKind::CueTextReservedCharacter(CueTextReservedCharacter('<')),
        },
    );

    let gt_input = text_block_fnl! {
        "00:00.000 cre: first line"
        "               end of tag>"
        "00:05.000 clr"
    };
    assert_eq!(
        parse_lyrics(gt_input).unwrap_err(),
        ParseLyricsError {
            line_number: 2,
            kind: ParseLyricsErrorKind::CueTextReservedCharacter(CueTextReservedCharacter('>')),
        },
    );
}

/// A line such as `00:00.000 <v>foo</v>` has no `:` separator,
/// so without the reserved-character check running before
/// `split_marker` the error would surface as
/// [`MissingMarker`](crate::parse::error::MissingMarker) even
/// though the real problem is the angle brackets. Verify that
/// the more specific diagnostic wins here.
#[test]
fn marker_less_body_with_reserved_character_reports_reserved_character() {
    let input = text_block_fnl! {
        "00:00.000 <v>foo</v>"
        "00:02.000 clr"
    };
    assert_eq!(
        parse_lyrics(input).unwrap_err(),
        ParseLyricsError {
            line_number: 1,
            kind: ParseLyricsErrorKind::CueTextReservedCharacter(CueTextReservedCharacter('<')),
        },
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
        ParseLyricsError {
            line_number: 1,
            kind: ParseLyricsErrorKind::CueTextReservedCharacter(CueTextReservedCharacter('<')),
        },
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
