//! Tests for the rules an `<additive>` region imposes. A region does not
//! nest, does not stay open at the end of the file, does not close without
//! having opened, does not enclose zero cues, and admits no control
//! marker.

use crate::parse::error::{
    AdditiveRegionError, ControlMarkerInRegion, EmptyRegion, NestedRegion, ParseLyricsError,
    UnclosedRegion, UnopenedRegion,
};
use crate::parse::{LineNumber, parse_lyrics};
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
        NestedRegion {
            line_number: LineNumber::new(2),
            opened_at: LineNumber::new(1),
        }
        .pipe(AdditiveRegionError::Nested)
        .pipe(ParseLyricsError::AdditiveRegion),
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
        NestedRegion {
            line_number: LineNumber::new(3),
            opened_at: LineNumber::new(1),
        }
        .pipe(AdditiveRegionError::Nested)
        .pipe(ParseLyricsError::AdditiveRegion),
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
        ControlMarkerInRegion {
            line_number: LineNumber::new(3),
            marker: ReservedMarker::Clear,
            opened_at: LineNumber::new(1),
        }
        .pipe(AdditiveRegionError::ControlMarker)
        .pipe(ParseLyricsError::AdditiveRegion),
    );

    let end_of_video_input = text_block_fnl! {
        "<additive>"
        "07:11.111 LRC: first line"
        "07:22.222 eov"
        "</additive>"
    };
    assert_eq!(
        parse_lyrics(end_of_video_input).unwrap_err(),
        ControlMarkerInRegion {
            line_number: LineNumber::new(3),
            marker: ReservedMarker::EndOfVideo,
            opened_at: LineNumber::new(1),
        }
        .pipe(AdditiveRegionError::ControlMarker)
        .pipe(ParseLyricsError::AdditiveRegion),
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
        UnclosedRegion {
            line_number: LineNumber::new(2)
        }
        .pipe(AdditiveRegionError::Unclosed)
        .pipe(ParseLyricsError::AdditiveRegion),
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
        UnopenedRegion {
            line_number: LineNumber::new(2)
        }
        .pipe(AdditiveRegionError::Unopened)
        .pipe(ParseLyricsError::AdditiveRegion),
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
        EmptyRegion {
            line_number: LineNumber::new(4),
            opened_at: LineNumber::new(1),
        }
        .pipe(AdditiveRegionError::Empty)
        .pipe(ParseLyricsError::AdditiveRegion),
    );
}
