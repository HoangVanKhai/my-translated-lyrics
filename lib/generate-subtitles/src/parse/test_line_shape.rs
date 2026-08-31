//! Tests for the diagnostics a malformed line draws. They cover a
//! column-zero line that carries no timestamp, a header that omits the
//! separator after its timestamp, a body that declares no marker or no
//! text, and an indent that matches neither of the two recognized widths.

use crate::_test_utils::marker_name;
use crate::parse::error::{
    EmptyCueBody, MalformedHeader, MalformedIndentation, MissingMarker,
    MissingSeparatorAfterTimestamp, ParseLyricsError, TabIndentation,
};
use crate::parse::parse_lyrics;
use pretty_assertions::assert_eq;
use text_block_macros::text_block_fnl;

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
fn rejects_cue_with_empty_body() {
    let input = text_block_fnl! {
        "00:00.000 ttl:"
        "00:02.000 clr"
    };
    assert_eq!(
        parse_lyrics(input).unwrap_err(),
        ParseLyricsError::EmptyCueBody(EmptyCueBody {
            line_number: 1,
            marker: marker_name("ttl"),
        }),
    );
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
