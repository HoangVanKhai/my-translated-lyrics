//! Error types produced by [`parse_lyrics`].
//!
//! [`ParseLyricsError`] is the single error returned by the parser; it
//! pairs the source line with a [`ParseLyricsErrorKind`], whose
//! variants each wrap a dedicated payload struct that carries whatever
//! context the diagnostic needs beyond that line. The payloads are
//! split out from the parsing engine in [`super`] so the engine reads
//! as one algorithm and the vocabulary of failures sits on its own.
//!
//! [`parse_lyrics`]: super::parse_lyrics

use super::{TIMESTAMP_PREFIX_WIDTH, TagName};
use core::fmt;
use derive_more::Display;
use lyrics_core::line_markers_descriptor::ReservedMarker;
use lyrics_core::timestamp::{TakeTimestampError, Timestamp};

/// Payload for [`ParseLyricsErrorKind::InvalidTimestamp`]. Wraps the
/// underlying [`TakeTimestampError`].
#[derive(Clone, Debug, Display, Eq, PartialEq)]
#[display("{_0}")]
pub struct InvalidTimestamp(pub TakeTimestampError);

/// Payload for [`ParseLyricsErrorKind::MissingMarker`]. Raised when a
/// cue body has no `:` separator at all, and also when it has a `:`
/// but the marker half before it is empty.
#[derive(Clone, Debug, Display, Eq, PartialEq)]
#[display("cue body {_0:?} carries no marker before the `:` separator")]
pub struct MissingMarker(pub String);

/// Payload for [`ParseLyricsErrorKind::MissingSeparatorAfterTimestamp`].
#[derive(Clone, Debug, Display, Eq, PartialEq)]
#[display("timestamp in {_0:?} is not followed by whitespace")]
pub struct MissingSeparatorAfterTimestamp(pub String);

/// Payload for [`ParseLyricsErrorKind::ExtraTextAfterControlMarker`].
#[derive(Clone, Debug, Display, Eq, PartialEq)]
#[display("control marker `{marker}` must stand alone but is followed by {trailing:?}")]
pub struct ExtraTextAfterControlMarker {
    pub marker: ReservedMarker,
    pub trailing: String,
}

/// Payload for [`ParseLyricsErrorKind::OutOfOrder`].
#[derive(Clone, Debug, Display, Eq, PartialEq)]
#[display("events out of order: event at {previous} is followed by an earlier event at {next}")]
pub struct OutOfOrder {
    pub previous: Timestamp,
    pub next: Timestamp,
}

/// Payload for [`ParseLyricsErrorKind::ReservedControlMarker`].
#[derive(Clone, Debug, Display, Eq, PartialEq)]
#[display("marker `{_0}` is reserved by the parser and cannot name a cue")]
pub struct ReservedControlMarker(pub ReservedMarker);

/// Payload for [`ParseLyricsErrorKind::EmptyAnnotation`]. Raised when
/// an annotation line carries no text after its `:` separator.
#[derive(Clone, Debug, Display, Eq, PartialEq)]
#[display("annotation marker `{}` has an empty body", ReservedMarker::Annotation)]
pub struct EmptyAnnotation;

/// Payload for [`ParseLyricsErrorKind::MalformedTagLine`].
#[derive(Clone, Debug, Display, Eq, PartialEq)]
#[display(
    "{_0:?} is not a tag line; a tag line reads exactly `<{tag}>` or `</{tag}>`",
    tag = TagName::Additive
)]
pub struct MalformedTagLine(pub String);

/// Payload for [`AdditiveRegionError::Nested`].
#[derive(Clone, Debug, Display, Eq, PartialEq)]
#[display(
    "`<{tag}>` opens an additive region inside the one opened on line {_0}",
    tag = TagName::Additive
)]
pub struct NestedRegion(pub usize);

/// Payload for [`AdditiveRegionError::Unopened`].
#[derive(Clone, Debug, Display, Eq, PartialEq)]
#[display("stray `</{tag}>`", tag = TagName::Additive)]
pub struct UnopenedRegion;

/// Payload for [`AdditiveRegionError::Unclosed`]. The location the
/// parser reports for this failure is the `<additive>` line that
/// opened the region rather than the end of the file, because that is
/// where the author has to act.
#[derive(Clone, Debug, Display, Eq, PartialEq)]
#[display("unclosed `<{tag}>`", tag = TagName::Additive)]
pub struct UnclosedRegion;

/// Payload for [`AdditiveRegionError::Empty`].
#[derive(Clone, Debug, Display, Eq, PartialEq)]
#[display("the additive region opened on line {_0} encloses no cue")]
pub struct EmptyRegion(pub usize);

/// Payload for [`AdditiveRegionError::ControlMarker`].
#[derive(Clone, Debug, Display, Eq, PartialEq)]
#[display(
    "control marker `{marker}` appears inside the additive region opened on line \
    {opened_at}; close the region before the marker"
)]
pub struct ControlMarkerInRegion {
    pub marker: ReservedMarker,
    pub opened_at: usize,
}

/// Payload for [`ParseLyricsErrorKind::EmptyCueBody`].
#[derive(Clone, Debug, Display, Eq, PartialEq)]
#[display("cue with marker {_0:?} has an empty body")]
pub struct EmptyCueBody(pub String);

/// Payload for [`ParseLyricsErrorKind::MalformedHeader`]. Raised when
/// a column-zero line does not begin with an `MM:SS.mmm` timestamp;
/// every column-zero line in the source format is expected to open
/// either a fresh cue or a `clr` / `eov` control event.
#[derive(Clone, Debug, Display, Eq, PartialEq)]
#[display("header line {_0:?} does not begin with an `MM:SS.mmm` timestamp")]
pub struct MalformedHeader(pub String);

/// Payload for [`ParseLyricsErrorKind::OrphanedShorthandMarker`].
/// Raised when a column-`TIMESTAMP_PREFIX_WIDTH` line carries a marker
/// but no cue is open above it for the new marker to share a start
/// time with.
#[derive(Clone, Debug, Display, Eq, PartialEq)]
#[display("shorthand marker line {_0:?} appears before any timestamp opens a cue")]
pub struct OrphanedShorthandMarker(pub String);

/// Payload for [`ParseLyricsErrorKind::OrphanedAnnotation`]. Raised
/// when an annotation line appears where no cue is open, whether
/// before the first cue or after a `clr` has closed one.
#[derive(Clone, Debug, Display, Eq, PartialEq)]
#[display("annotation line {_0:?} appears where no cue is open")]
pub struct OrphanedAnnotation(pub String);

/// Payload for [`ParseLyricsErrorKind::MalformedIndentation`]. Lists
/// the observed indent and the two values the parser would have
/// accepted at this point in the input. `continuation_indent` is
/// `None` when no part is currently open (so a continuation could
/// not be valid here regardless of indent).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MalformedIndentation {
    pub actual: usize,
    pub shorthand_indent: usize,
    pub continuation_indent: Option<usize>,
}

impl fmt::Display for MalformedIndentation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "indent of {actual} space(s) matches no expected width; expected {shorthand} for a shorthand marker line",
            actual = self.actual,
            shorthand = self.shorthand_indent,
        )?;
        match self.continuation_indent {
            Some(width) => write!(f, " or {width} for a continuation of the current marker"),
            None => Ok(()),
        }
    }
}

/// Payload for [`ParseLyricsErrorKind::RepeatedTimestamp`]. Raised
/// when two consecutive timestamped header lines share a start time;
/// the column-`TIMESTAMP_PREFIX_WIDTH` shorthand is the canonical
/// way to attach multiple markers to a single timestamp, and a
/// repeated timestamp form would create two separate cues that
/// the renderer would emit as overlapping subtitle blocks.
#[derive(Clone, Debug, Display, Eq, PartialEq)]
#[display(
    "timestamp {_0} repeats the start time of the immediately previous event; \
    use the column-{TIMESTAMP_PREFIX_WIDTH} shorthand to attach a second marker to the same timestamp"
)]
pub struct RepeatedTimestamp(pub Timestamp);

/// Payload for [`ParseLyricsErrorKind::TabIndentation`].
///
/// The parser requires every line's leading whitespace to consist
/// of ASCII spaces only. Tabs would render at different visual
/// widths under different terminal settings, which interacts
/// poorly with the column-exact indentation rules the format
/// uses to distinguish a continuation of the prior marker from a
/// new marker at the same timestamp.
#[derive(Clone, Debug, Display, Eq, PartialEq)]
#[display("indentation contains a tab; only ASCII spaces are allowed in leading whitespace")]
pub struct TabIndentation;

/// Payload for [`ParseLyricsErrorKind::CueTextReservedCharacter`].
///
/// The `lyrics.{lang}.txt` source format is plain prose; the cue
/// text reaches the WebVTT and SubRip renderers after HTML-entity
/// escape, so there is no author-level way to embed a literal
/// `<` or `>` into the rendered cue. Any such character in the
/// source is almost certainly an attempt to hand-author WebVTT
/// markup, which belongs in the renderer's vocabulary (class and
/// voice markers in `line-markers.toml`), not in the prose.
#[derive(Clone, Debug, Display, Eq, PartialEq)]
#[display("cue text contains {_0:?}, which the WebVTT cue-tag grammar reserves for tag delimiters")]
pub struct CueTextReservedCharacter(pub char);

/// Payload for [`ParseLyricsErrorKind::UnclosedCue`]. Carries the
/// start timestamp of the cue that has no following event to
/// close it. The location the parser reports for this failure is
/// the header line that opened the cue, since the file has already
/// ended by the time the failure is detected.
#[derive(Clone, Debug, Display, Eq, PartialEq)]
#[display("cue at {_0} has no following cue or `clr`")]
pub struct UnclosedCue(pub Timestamp);

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

/// What the parser rejected.
#[derive(Clone, Debug, Display, Eq, PartialEq)]
#[non_exhaustive]
pub enum ParseLyricsErrorKind {
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

/// The error [`parse_lyrics`](super::parse_lyrics) returns: what the
/// parser rejected, and the source line it points the author at.
#[derive(Clone, Debug, Display, Eq, PartialEq)]
#[display("line {line_number}: {kind}")]
pub struct ParseLyricsError {
    /// The line the author has to revisit, counting from 1.
    pub line_number: usize,
    /// What the parser rejected.
    pub kind: ParseLyricsErrorKind,
}

#[cfg(test)]
mod tests;
