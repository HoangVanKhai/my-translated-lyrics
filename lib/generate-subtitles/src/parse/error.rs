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

use super::TIMESTAMP_PREFIX_WIDTH;
use core::fmt;
use derive_more::Display;
use lyrics_core::timestamp::{TakeTimestampError, Timestamp};

/// Payload for [`ParseLyricsError::InvalidTimestamp`]. Wraps the
/// underlying [`TakeTimestampError`] and pairs it with the source
/// line number.
#[derive(Debug, Display, Clone, PartialEq, Eq)]
#[display("line {line_number}: {cause}")]
pub struct InvalidTimestamp {
    pub line_number: usize,
    pub cause: TakeTimestampError,
}

/// Payload for [`ParseLyricsError::MissingMarker`]. Raised when a
/// cue body has no `:` separator at all, and also when it has a `:`
/// but the marker half before it is empty.
#[derive(Debug, Display, Clone, PartialEq, Eq)]
#[display("line {line_number}: cue body {content:?} carries no marker before the `:` separator")]
pub struct MissingMarker {
    pub line_number: usize,
    pub content: String,
}

/// Payload for [`ParseLyricsError::MissingSeparatorAfterTimestamp`].
#[derive(Debug, Display, Clone, PartialEq, Eq)]
#[display("line {line_number}: timestamp in {content:?} is not followed by whitespace")]
pub struct MissingSeparatorAfterTimestamp {
    pub line_number: usize,
    pub content: String,
}

/// Payload for [`ParseLyricsError::ExtraTextAfterControlMarker`].
#[derive(Debug, Display, Clone, PartialEq, Eq)]
#[display(
    "line {line_number}: control marker {marker:?} must stand alone but is followed by {trailing:?}"
)]
pub struct ExtraTextAfterControlMarker {
    pub line_number: usize,
    pub marker: String,
    pub trailing: String,
}

/// Payload for [`ParseLyricsError::OutOfOrder`].
#[derive(Debug, Display, Clone, PartialEq, Eq)]
#[display("events out of order: event at {previous} is followed by an earlier event at {next}")]
pub struct OutOfOrder {
    pub previous: Timestamp,
    pub next: Timestamp,
}

/// Payload for [`ParseLyricsError::ReservedControlMarker`].
#[derive(Debug, Display, Clone, PartialEq, Eq)]
#[display(
    "line {line_number}: marker {marker:?} is reserved for the `clr`/`eov` control tokens and cannot name a cue"
)]
pub struct ReservedControlMarker {
    pub line_number: usize,
    pub marker: String,
}

/// Payload for [`ParseLyricsError::EmptyCueBody`].
#[derive(Debug, Display, Clone, PartialEq, Eq)]
#[display("line {line_number}: cue with marker {marker:?} has an empty body")]
pub struct EmptyCueBody {
    pub line_number: usize,
    pub marker: String,
}

/// Payload for [`ParseLyricsError::MalformedHeader`]. Raised when
/// a column-zero line does not begin with an `MM:SS.mmm` timestamp;
/// every column-zero line in the source format is expected to open
/// either a fresh cue or a `clr` / `eov` control event.
#[derive(Debug, Display, Clone, PartialEq, Eq)]
#[display(
    "line {line_number}: header line {content:?} does not begin with an `MM:SS.mmm` timestamp"
)]
pub struct MalformedHeader {
    pub line_number: usize,
    pub content: String,
}

/// Payload for [`ParseLyricsError::OrphanedShorthandMarker`]. Raised
/// when a column-`TIMESTAMP_PREFIX_WIDTH` line carries a marker but
/// no cue is open above it for the new marker to share a start
/// time with.
#[derive(Debug, Display, Clone, PartialEq, Eq)]
#[display(
    "line {line_number}: shorthand marker line {content:?} appears before any timestamp opens a cue"
)]
pub struct OrphanedShorthandMarker {
    pub line_number: usize,
    pub content: String,
}

/// Payload for [`ParseLyricsError::MalformedIndentation`]. Lists the
/// observed indent and the two values the parser would have
/// accepted at this point in the input. `continuation_indent` is
/// `None` when no part is currently open (so a continuation could
/// not be valid here regardless of indent).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MalformedIndentation {
    pub line_number: usize,
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
#[derive(Debug, Display, Clone, PartialEq, Eq)]
#[display(
    "line {line_number}: timestamp {start} repeats the start time of the immediately previous event; \
    use the column-{TIMESTAMP_PREFIX_WIDTH} shorthand to attach a second marker to the same timestamp"
)]
pub struct RepeatedTimestamp {
    pub line_number: usize,
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
#[derive(Debug, Display, Clone, PartialEq, Eq)]
#[display(
    "line {line_number}: indentation contains a tab; only ASCII spaces are allowed in leading whitespace"
)]
pub struct TabIndentation {
    pub line_number: usize,
}

/// Payload for [`ParseLyricsError::CueTextReservedCharacter`].
///
/// The `lyrics.{lang}.txt` source format is plain prose; the cue
/// text reaches the WebVTT and SubRip renderers after HTML-entity
/// escape, so there is no author-level way to embed a literal
/// `<` or `>` into the rendered cue. Any such character in the
/// source is almost certainly an attempt to hand-author WebVTT
/// markup, which belongs in the renderer's vocabulary (class and
/// voice markers in `line-markers.toml`), not in the prose.
#[derive(Debug, Display, Clone, PartialEq, Eq)]
#[display(
    "line {line_number}: cue text contains {character:?}, which the WebVTT cue-tag grammar reserves for tag delimiters"
)]
pub struct CueTextReservedCharacter {
    pub line_number: usize,
    pub character: char,
}

/// Payload for [`ParseLyricsError::UnclosedCue`]. Carries the
/// start timestamp of the cue that has no following event to
/// close it.
#[derive(Debug, Display, Clone, PartialEq, Eq)]
#[display("cue at {start} has no following cue or `clr`")]
pub struct UnclosedCue {
    pub start: Timestamp,
}

#[derive(Debug, Display, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ParseLyricsError {
    TabIndentation(TabIndentation),
    MalformedIndentation(MalformedIndentation),
    MalformedHeader(MalformedHeader),
    InvalidTimestamp(InvalidTimestamp),
    MissingSeparatorAfterTimestamp(MissingSeparatorAfterTimestamp),
    ExtraTextAfterControlMarker(ExtraTextAfterControlMarker),
    RepeatedTimestamp(RepeatedTimestamp),
    OutOfOrder(OutOfOrder),
    CueTextReservedCharacter(CueTextReservedCharacter),
    MissingMarker(MissingMarker),
    ReservedControlMarker(ReservedControlMarker),
    EmptyCueBody(EmptyCueBody),
    OrphanedShorthandMarker(OrphanedShorthandMarker),
    UnclosedCue(UnclosedCue),
}
