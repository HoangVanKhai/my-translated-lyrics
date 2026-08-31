//! Tests for the rules an `<additive>` region imposes. A region does not
//! nest, does not stay open at the end of the file, does not close without
//! having opened, does not enclose zero cues, and admits no control
//! marker.

use crate::parse::error::{
    AdditiveRegionError, ControlMarkerInRegion, EmptyRegion, NestedRegion, ParseLyricsError,
    ParseLyricsErrorKind, UnclosedRegion, UnopenedRegion,
};
use crate::parse::parse_lyrics;
use lyrics_core::line_markers_descriptor::ReservedMarker;
use pipe_trait::Pipe;
use pretty_assertions::assert_eq;
use text_block_macros::text_block_fnl;

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
        ParseLyricsError {
            line_number: 2,
            kind: NestedRegion(1)
                .pipe(AdditiveRegionError::Nested)
                .pipe(ParseLyricsErrorKind::AdditiveRegion),
        },
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
        ParseLyricsError {
            line_number: 3,
            kind: NestedRegion(1)
                .pipe(AdditiveRegionError::Nested)
                .pipe(ParseLyricsErrorKind::AdditiveRegion),
        },
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
        ParseLyricsError {
            line_number: 3,
            kind: ControlMarkerInRegion {
                marker: ReservedMarker::Clear,
                opened_at: 1,
            }
            .pipe(AdditiveRegionError::ControlMarker)
            .pipe(ParseLyricsErrorKind::AdditiveRegion),
        },
    );

    let end_of_video_input = text_block_fnl! {
        "<additive>"
        "07:11.111 LRC: first line"
        "07:22.222 eov"
        "</additive>"
    };
    assert_eq!(
        parse_lyrics(end_of_video_input).unwrap_err(),
        ParseLyricsError {
            line_number: 3,
            kind: ControlMarkerInRegion {
                marker: ReservedMarker::EndOfVideo,
                opened_at: 1,
            }
            .pipe(AdditiveRegionError::ControlMarker)
            .pipe(ParseLyricsErrorKind::AdditiveRegion),
        },
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
        ParseLyricsError {
            line_number: 2,
            kind: UnclosedRegion
                .pipe(AdditiveRegionError::Unclosed)
                .pipe(ParseLyricsErrorKind::AdditiveRegion),
        },
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
        ParseLyricsError {
            line_number: 2,
            kind: UnopenedRegion
                .pipe(AdditiveRegionError::Unopened)
                .pipe(ParseLyricsErrorKind::AdditiveRegion),
        },
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
        ParseLyricsError {
            line_number: 4,
            kind: EmptyRegion(1)
                .pipe(AdditiveRegionError::Empty)
                .pipe(ParseLyricsErrorKind::AdditiveRegion),
        },
    );
}
