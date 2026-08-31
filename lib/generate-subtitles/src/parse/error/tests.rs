//! Tests for the way a [`ParseLyricsError`] renders. The location
//! prefix is written once, on the error itself, rather than by each
//! payload, so what is checked here is that every kind reads as
//! `line {n}: ` followed by its own message and nothing else.

use super::{
    AdditiveRegionError, EmptyRegion, MalformedIndentation, ParseLyricsError, ParseLyricsErrorKind,
    TabIndentation,
};
use pipe_trait::Pipe;
use pretty_assertions::assert_eq;

/// A payload with no fields of its own still renders with the
/// location, because the location is no longer the payload's to
/// carry.
#[test]
fn a_field_less_kind_renders_with_the_location_prefix() {
    let error = ParseLyricsError {
        line_number: 7,
        kind: ParseLyricsErrorKind::TabIndentation(TabIndentation),
    };
    assert_eq!(
        error.to_string(),
        "line 7: indentation contains a tab; only ASCII spaces are allowed in leading whitespace",
    );
}

/// A payload that names another line keeps that number in its own
/// message. The two read as the position at which the parser gave up
/// and the line that message points back at.
#[test]
fn a_kind_naming_another_line_keeps_both_numbers() {
    let error = ParseLyricsError {
        line_number: 9,
        kind: EmptyRegion(4)
            .pipe(AdditiveRegionError::Empty)
            .pipe(ParseLyricsErrorKind::AdditiveRegion),
    };
    assert_eq!(
        error.to_string(),
        "line 9: the additive region opened on line 4 encloses no cue",
    );
}

/// [`MalformedIndentation`] writes its own `Display` rather than
/// deriving one, so the prefix has to reach it the same way it
/// reaches the derived kinds.
#[test]
fn a_hand_written_kind_takes_the_same_prefix() {
    let error = ParseLyricsError {
        line_number: 3,
        kind: ParseLyricsErrorKind::MalformedIndentation(MalformedIndentation {
            actual: 12,
            shorthand_indent: 10,
            continuation_indent: Some(15),
        }),
    };
    assert_eq!(
        error.to_string(),
        "line 3: indent of 12 space(s) matches no expected width; \
        expected 10 for a shorthand marker line \
        or 15 for a continuation of the current marker",
    );
}
