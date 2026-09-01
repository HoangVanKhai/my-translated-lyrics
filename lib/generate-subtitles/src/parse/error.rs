//! Error types produced by [`parse_lyrics`].
//!
//! [`ParseLyricsError`] is the single error returned by the parser;
//! each of its variants wraps a dedicated payload struct that carries
//! the source line number (or timestamp) and whatever context the
//! diagnostic needs. The payloads are split out from the parsing
//! engine in [`super`] so the engine reads as one algorithm and the
//! vocabulary of failures sits on its own.
//!
//! [`parse_lyrics`]: super::parse_lyrics

use super::{LineNumber, TIMESTAMP_PREFIX_WIDTH, TagName};
use core::fmt;
use derive_more::Display;
use lyrics_core::line_markers_descriptor::{MarkerName, ReservedMarker};
use lyrics_core::timestamp::{TakeTimestampError, Timestamp};

/// Payload for [`ParseLyricsError::InvalidTimestamp`]. Wraps the
/// underlying [`TakeTimestampError`] and pairs it with the source
/// line number.
#[derive(Clone, Debug, Display, Eq, PartialEq)]
#[display("line {line_number}: {cause}")]
pub struct InvalidTimestamp {
    pub line_number: LineNumber,
    pub cause: TakeTimestampError,
}

/// Payload for [`ParseLyricsError::MissingMarker`]. Raised when a
/// cue body has no `:` separator at all, and also when it has a `:`
/// but the marker half before it is empty.
#[derive(Clone, Debug, Display, Eq, PartialEq)]
#[display("line {line_number}: cue body {content:?} carries no marker before the `:` separator")]
pub struct MissingMarker {
    pub line_number: LineNumber,
    pub content: String,
}

/// Payload for [`ParseLyricsError::MissingSeparatorAfterTimestamp`].
#[derive(Clone, Debug, Display, Eq, PartialEq)]
#[display("line {line_number}: timestamp in {content:?} is not followed by whitespace")]
pub struct MissingSeparatorAfterTimestamp {
    pub line_number: LineNumber,
    pub content: String,
}

/// Payload for [`ParseLyricsError::ExtraTextAfterControlMarker`].
#[derive(Clone, Debug, Display, Eq, PartialEq)]
#[display(
    "line {line_number}: control marker `{marker}` must stand alone but is followed by {trailing:?}"
)]
pub struct ExtraTextAfterControlMarker {
    pub line_number: LineNumber,
    pub marker: ReservedMarker,
    pub trailing: String,
}

/// Payload for [`ParseLyricsError::OutOfOrder`].
#[derive(Clone, Debug, Display, Eq, PartialEq)]
#[display("events out of order: event at {previous} is followed by an earlier event at {next}")]
pub struct OutOfOrder {
    pub previous: Timestamp,
    pub next: Timestamp,
}

/// Payload for [`ParseLyricsError::ReservedControlMarker`].
#[derive(Clone, Debug, Display, Eq, PartialEq)]
#[display("line {line_number}: marker `{marker}` is reserved by the parser and cannot name a cue")]
pub struct ReservedControlMarker {
    pub line_number: LineNumber,
    pub marker: ReservedMarker,
}

/// Payload for [`ParseLyricsError::EmptyAnnotation`]. Raised when an
/// annotation line carries no text after its `:` separator.
#[derive(Clone, Debug, Display, Eq, PartialEq)]
#[display(
    "line {_0}: annotation marker `{}` has an empty body",
    ReservedMarker::Annotation
)]
pub struct EmptyAnnotation(pub LineNumber);

/// Payload for [`ParseLyricsError::MalformedTagLine`].
#[derive(Clone, Debug, Display, Eq, PartialEq)]
#[display(
    "line {line_number}: {content:?} is not a tag line; a tag line reads exactly \
    `<{tag}>` or `</{tag}>`",
    tag = TagName::Additive
)]
pub struct MalformedTagLine {
    pub line_number: LineNumber,
    pub content: String,
}

/// Payload for [`AdditiveRegionError::Nested`].
#[derive(Clone, Debug, Display, Eq, PartialEq)]
#[display(
    "line {line_number}: `<{tag}>` opens an additive region inside the one opened on line \
    {opened_at}",
    tag = TagName::Additive
)]
pub struct NestedRegion {
    pub line_number: LineNumber,
    pub opened_at: LineNumber,
}

/// Payload for [`AdditiveRegionError::Unopened`].
#[derive(Clone, Debug, Display, Eq, PartialEq)]
#[display("line {_0}: stray `</{tag}>`", tag = TagName::Additive)]
pub struct UnopenedRegion(pub LineNumber);

/// Payload for [`AdditiveRegionError::Unclosed`].
#[derive(Clone, Debug, Display, Eq, PartialEq)]
#[display("line {_0}: unclosed `<{tag}>`", tag = TagName::Additive)]
pub struct UnclosedRegion(pub LineNumber);

/// Payload for [`AdditiveRegionError::Empty`].
#[derive(Clone, Debug, Display, Eq, PartialEq)]
#[display("line {line_number}: the additive region opened on line {opened_at} encloses no cue")]
pub struct EmptyRegion {
    pub line_number: LineNumber,
    pub opened_at: LineNumber,
}

/// Payload for [`AdditiveRegionError::ControlMarker`].
#[derive(Clone, Debug, Display, Eq, PartialEq)]
#[display(
    "line {line_number}: control marker `{marker}` appears inside the additive region opened \
    on line {opened_at}; close the region before the marker"
)]
pub struct ControlMarkerInRegion {
    pub line_number: LineNumber,
    pub marker: ReservedMarker,
    pub opened_at: LineNumber,
}

/// Payload for [`ParseLyricsError::EmptyCueBody`].
#[derive(Clone, Debug, Display, Eq, PartialEq)]
#[display(
    "line {line_number}: cue with marker {:?} has an empty body",
    marker.as_str(),
)]
pub struct EmptyCueBody {
    pub line_number: LineNumber,
    pub marker: MarkerName,
}

/// Payload for [`ParseLyricsError::MalformedHeader`]. Raised when
/// a column-zero line does not begin with an `MM:SS.mmm` timestamp;
/// every column-zero line in the source format is expected to open
/// either a fresh cue or a `clr` / `eov` control event.
#[derive(Clone, Debug, Display, Eq, PartialEq)]
#[display(
    "line {line_number}: header line {content:?} does not begin with an `MM:SS.mmm` timestamp"
)]
pub struct MalformedHeader {
    pub line_number: LineNumber,
    pub content: String,
}

/// Payload for [`ParseLyricsError::OrphanedShorthandMarker`]. Raised
/// when a column-`TIMESTAMP_PREFIX_WIDTH` line carries a marker but
/// no cue is open above it for the new marker to share a start
/// time with.
#[derive(Clone, Debug, Display, Eq, PartialEq)]
#[display(
    "line {line_number}: shorthand marker line {content:?} appears before any timestamp opens a cue"
)]
pub struct OrphanedShorthandMarker {
    pub line_number: LineNumber,
    pub content: String,
}

/// Payload for [`ParseLyricsError::OrphanedAnnotation`]. Raised when
/// an annotation line appears where no cue is open, whether before
/// the first cue or after a `clr` has closed one.
#[derive(Clone, Debug, Display, Eq, PartialEq)]
#[display("line {line_number}: annotation line {content:?} appears where no cue is open")]
pub struct OrphanedAnnotation {
    pub line_number: LineNumber,
    pub content: String,
}

/// Payload for [`ParseLyricsError::MalformedIndentation`]. Lists the
/// observed indent and the two values the parser would have
/// accepted at this point in the input. `continuation_indent` is
/// `None` when no part is currently open (so a continuation could
/// not be valid here regardless of indent).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MalformedIndentation {
    pub line_number: LineNumber,
    pub actual: usize,
    pub shorthand_indent: usize,
    pub continuation_indent: Option<usize>,
}

impl fmt::Display for MalformedIndentation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "line {line_number}: indent of {actual} space(s) matches no expected width; expected {shorthand} for a shorthand marker line",
            line_number = self.line_number,
            actual = self.actual,
            shorthand = self.shorthand_indent,
        )?;
        match self.continuation_indent {
            Some(width) => write!(f, " or {width} for a continuation of the current marker"),
            None => Ok(()),
        }
    }
}

/// Payload for [`ParseLyricsError::RepeatedTimestamp`]. Raised when
/// two consecutive timestamped header lines share a start time;
/// the column-`TIMESTAMP_PREFIX_WIDTH` shorthand is the canonical
/// way to attach multiple markers to a single timestamp, and a
/// repeated timestamp form would create two separate cues that
/// the renderer would emit as overlapping subtitle blocks.
#[derive(Clone, Debug, Display, Eq, PartialEq)]
#[display(
    "line {line_number}: timestamp {start} repeats the start time of the immediately previous event; \
    use the column-{TIMESTAMP_PREFIX_WIDTH} shorthand to attach a second marker to the same timestamp"
)]
pub struct RepeatedTimestamp {
    pub line_number: LineNumber,
    pub start: Timestamp,
}

/// Payload for [`ParseLyricsError::TabIndentation`].
///
/// The parser requires every line's leading whitespace to consist
/// of ASCII spaces only. Tabs would render at different visual
/// widths under different terminal settings, which interacts
/// poorly with the column-exact indentation rules the format
/// uses to distinguish a continuation of the prior marker from a
/// new marker at the same timestamp.
#[derive(Clone, Debug, Display, Eq, PartialEq)]
#[display(
    "line {_0}: indentation contains a tab; only ASCII spaces are allowed in leading whitespace"
)]
pub struct TabIndentation(pub LineNumber);

/// Payload for [`ParseLyricsError::CueTextReservedCharacter`].
///
/// The `lyrics.{lang}.txt` source format is plain prose; the cue
/// text reaches the WebVTT and SubRip renderers after HTML-entity
/// escape, so there is no author-level way to embed a literal
/// `<` or `>` into the rendered cue. Any such character in the
/// source is almost certainly an attempt to hand-author WebVTT
/// markup, which belongs in the renderer's vocabulary (class and
/// voice markers in `line-markers.toml`), not in the prose.
#[derive(Clone, Debug, Display, Eq, PartialEq)]
#[display(
    "line {line_number}: cue text contains {character:?}, which the WebVTT cue-tag grammar reserves for tag delimiters"
)]
pub struct CueTextReservedCharacter {
    pub line_number: LineNumber,
    pub character: char,
}

/// Payload for [`ParseLyricsError::UnclosedCue`]. Carries the
/// start timestamp of the cue that has no following event to
/// close it.
#[derive(Clone, Debug, Display, Eq, PartialEq)]
#[display("cue at {start} has no following cue or `clr`")]
pub struct UnclosedCue {
    pub start: Timestamp,
}

/// The ways an `<additive>` region can be malformed.
#[derive(Clone, Debug, Display, Eq, PartialEq)]
#[non_exhaustive]
pub enum AdditiveRegionError {
    Nested(NestedRegion),
    Unopened(UnopenedRegion),
    Unclosed(UnclosedRegion),
    Empty(EmptyRegion),
    ControlMarker(ControlMarkerInRegion),
}

#[derive(Clone, Debug, Display, Eq, PartialEq)]
#[non_exhaustive]
pub enum ParseLyricsError {
    TabIndentation(TabIndentation),
    MalformedIndentation(MalformedIndentation),
    MalformedHeader(MalformedHeader),
    MalformedTagLine(MalformedTagLine),
    AdditiveRegion(AdditiveRegionError),
    InvalidTimestamp(InvalidTimestamp),
    MissingSeparatorAfterTimestamp(MissingSeparatorAfterTimestamp),
    ExtraTextAfterControlMarker(ExtraTextAfterControlMarker),
    RepeatedTimestamp(RepeatedTimestamp),
    OutOfOrder(OutOfOrder),
    CueTextReservedCharacter(CueTextReservedCharacter),
    MissingMarker(MissingMarker),
    ReservedControlMarker(ReservedControlMarker),
    EmptyCueBody(EmptyCueBody),
    EmptyAnnotation(EmptyAnnotation),
    OrphanedShorthandMarker(OrphanedShorthandMarker),
    OrphanedAnnotation(OrphanedAnnotation),
    UnclosedCue(UnclosedCue),
}
