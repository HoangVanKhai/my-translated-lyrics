//! Parser for `lyrics.{lang}.txt` cue files.
//!
//! Each file is a sequence of timestamped events. A line that starts
//! with `MM:SS.mmm` opens an event. If the event's first non-whitespace
//! token is [`CLEAR_MARKER`], the currently open cue is closed at that
//! timestamp; if it is [`END_OF_VIDEO_MARKER`], the line is ignored.
//! Any other event opens a new cue; continuation lines that lack a
//! leading timestamp are appended to the most recently opened cue.
//!
//! [`CLEAR_MARKER`]: crate::line_markers_descriptor::CLEAR_MARKER
//! [`END_OF_VIDEO_MARKER`]: crate::line_markers_descriptor::END_OF_VIDEO_MARKER

use crate::line_markers_descriptor::{CLEAR_MARKER, END_OF_VIDEO_MARKER};
use crate::timestamp::{TIMESTAMP_STR_LEN, TakeTimestampError, Timestamp};
use core::fmt;
use derive_more::{Display, Error};

/// Indent width of a line that opens a new marker at the same start
/// time as the cue immediately above. Equals the byte length of an
/// `MM:SS.mmm` timestamp plus one ASCII space.
const TIMESTAMP_PREFIX_WIDTH: usize = TIMESTAMP_STR_LEN + 1;

/// A subtitle cue with a resolved end time, ready for rendering.
///
/// A cue groups one or more [`CuePart`]s that share a start time.
/// Each part carries its own marker and text; the renderer emits
/// the parts as a single subtitle block whose body has one line
/// per part.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubtitleCue {
    /// Timestamp at which the cue begins to display. Read directly
    /// from the `MM:SS.mmm` prefix on the cue-opening line.
    pub start: Timestamp,
    /// Timestamp at which the cue stops displaying. Taken from the
    /// timestamp of the next event in the source file, whether that
    /// is the next cue or a `clr` sentinel.
    pub end: Timestamp,
    /// One or more parts that share this cue's start and end times.
    /// Each part carries its own marker and text and renders to a
    /// separate line within the resulting SRT or VTT cue block.
    pub parts: Vec<CuePart>,
}

/// One marker-text pair within a [`SubtitleCue`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CuePart {
    /// The leading marker token that the cue-opening line declared, for
    /// example `ttl` in `ttl: 《Song》`.
    pub marker: String,
    /// Cue text, with line breaks preserved between the opening line
    /// and any continuation lines.
    pub text: String,
}

/// Payload of an [`Event::Cue`]. The start time is the one declared
/// in the source file; the end time is resolved later by looking at
/// the next event in the stream. The list of parts mirrors
/// [`SubtitleCue::parts`]: a fresh timestamped header line opens a
/// group with one part, a column-`TIMESTAMP_PREFIX_WIDTH` shorthand
/// line appends a new part to that group, and a continuation line
/// extends the most recent part's text.
#[derive(Debug, Clone, PartialEq, Eq)]
struct CueGroup {
    start: Timestamp,
    parts: Vec<CuePart>,
}

/// An intermediate event extracted from a source file before end times
/// are resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Event {
    Cue(CueGroup),
    Clear(Timestamp),
}

impl Event {
    fn start(&self) -> Timestamp {
        match self {
            Event::Cue(group) => group.start,
            Event::Clear(start) => *start,
        }
    }
}

/// Parses `content` into a list of cues ordered by start time.
pub fn parse_lyrics(content: &str) -> Result<Vec<SubtitleCue>, ParseLyricsError> {
    let events = collect_events(content)?;
    resolve_cues(events)
}

fn collect_events(content: &str) -> Result<Vec<Event>, ParseLyricsError> {
    let mut events: Vec<Event> = Vec::new();
    let mut last_cue_index: Option<usize> = None;
    // Byte length of `marker: ` for the most recently added part of
    // the most recently opened cue group. A continuation line is
    // valid only when its indent equals
    // `TIMESTAMP_PREFIX_WIDTH + last_part_marker_prefix_width`.
    let mut last_part_marker_prefix_width: Option<usize> = None;

    for (line_index, raw_line) in content.lines().enumerate() {
        let line_number = line_index + 1;
        if raw_line.trim().is_empty() || raw_line.trim_start().starts_with('#') {
            continue;
        }

        if raw_line.trim_start_matches(' ').starts_with('\t') {
            return Err(ParseLyricsError::TabIndentation(TabIndentation {
                line_number,
            }));
        }

        let indent = raw_line.bytes().take_while(|&b| b == b' ').count();
        let body = &raw_line[indent..];

        if indent == 0 {
            handle_header_line(
                body,
                line_number,
                &mut events,
                &mut last_cue_index,
                &mut last_part_marker_prefix_width,
            )?;
        } else if indent == TIMESTAMP_PREFIX_WIDTH {
            handle_shorthand_marker_line(
                body,
                line_number,
                &mut events,
                last_cue_index,
                &mut last_part_marker_prefix_width,
            )?;
        } else if last_part_marker_prefix_width
            .is_some_and(|w| indent == TIMESTAMP_PREFIX_WIDTH + w)
        {
            handle_continuation_line(body, line_number, &mut events, last_cue_index)?;
        } else {
            return Err(ParseLyricsError::MalformedIndentation(
                MalformedIndentation {
                    line_number,
                    actual: indent,
                    shorthand_indent: TIMESTAMP_PREFIX_WIDTH,
                    continuation_indent: last_part_marker_prefix_width
                        .map(|w| TIMESTAMP_PREFIX_WIDTH + w),
                },
            ));
        }
    }

    Ok(events)
}

fn handle_header_line(
    body: &str,
    line_number: usize,
    events: &mut Vec<Event>,
    last_cue_index: &mut Option<usize>,
    last_part_marker_prefix_width: &mut Option<usize>,
) -> Result<(), ParseLyricsError> {
    let (start, after_prefix) = match Timestamp::take(body) {
        Ok(parsed) => parsed,
        Err(TakeTimestampError::ShapeMismatch) => {
            return Err(ParseLyricsError::MalformedHeader(MalformedHeader {
                line_number,
                content: body.to_string(),
            }));
        }
        Err(cause) => {
            return Err(ParseLyricsError::InvalidTimestamp(InvalidTimestamp {
                line_number,
                cause,
            }));
        }
    };

    let cue_body = after_prefix.trim_start();
    if cue_body.len() == after_prefix.len() {
        return Err(ParseLyricsError::MissingSeparatorAfterTimestamp(
            MissingSeparatorAfterTimestamp {
                line_number,
                content: body.to_string(),
            },
        ));
    }

    let first_token = cue_body.split_whitespace().next().unwrap_or("");
    if first_token == END_OF_VIDEO_MARKER || first_token == CLEAR_MARKER {
        let trailing = cue_body[first_token.len()..].trim();
        if !trailing.is_empty() {
            return Err(ParseLyricsError::ExtraTextAfterControlMarker(
                ExtraTextAfterControlMarker {
                    line_number,
                    marker: first_token.to_string(),
                    trailing: trailing.to_string(),
                },
            ));
        }
        if first_token == CLEAR_MARKER {
            check_event_order(start, line_number, events)?;
            events.push(Event::Clear(start));
            *last_cue_index = None;
            *last_part_marker_prefix_width = None;
        }
        // `eov` is documented as "ignored entirely"; it pushes no
        // event and so does not participate in the repeated- or
        // out-of-order checks. This lets a source file note both
        // `clr` and `eov` at the moment the video ends, since the
        // `eov` is a documentation sentinel rather than a competing
        // cue boundary.
        return Ok(());
    }

    check_event_order(start, line_number, events)?;
    let (marker, text) = parse_marker_part(cue_body, line_number)?;
    events.push(Event::Cue(CueGroup {
        start,
        parts: vec![CuePart {
            marker: marker.to_string(),
            text: text.to_string(),
        }],
    }));
    *last_cue_index = Some(events.len() - 1);
    *last_part_marker_prefix_width = Some(marker_prefix_width(marker));
    Ok(())
}

fn handle_shorthand_marker_line(
    body: &str,
    line_number: usize,
    events: &mut [Event],
    last_cue_index: Option<usize>,
    last_part_marker_prefix_width: &mut Option<usize>,
) -> Result<(), ParseLyricsError> {
    let Some(cue_index) = last_cue_index else {
        return Err(ParseLyricsError::OrphanedShorthandMarker(
            OrphanedShorthandMarker {
                line_number,
                content: body.to_string(),
            },
        ));
    };
    let (marker, text) = parse_marker_part(body, line_number)?;
    let Event::Cue(group) = &mut events[cue_index] else {
        unreachable!("last_cue_index must point at a Cue event");
    };
    group.parts.push(CuePart {
        marker: marker.to_string(),
        text: text.to_string(),
    });
    *last_part_marker_prefix_width = Some(marker_prefix_width(marker));
    Ok(())
}

fn handle_continuation_line(
    body: &str,
    line_number: usize,
    events: &mut [Event],
    last_cue_index: Option<usize>,
) -> Result<(), ParseLyricsError> {
    let cue_index =
        last_cue_index.expect("indent matched continuation width, so a prior cue must exist");
    reject_reserved_cue_text_characters(body, line_number)?;
    let Event::Cue(group) = &mut events[cue_index] else {
        unreachable!("last_cue_index must point at a Cue event");
    };
    let part = group
        .parts
        .last_mut()
        .expect("a cue group always has at least one part once it is opened");
    part.text.push('\n');
    part.text.push_str(body);
    Ok(())
}

fn parse_marker_part(body: &str, line_number: usize) -> Result<(&str, &str), ParseLyricsError> {
    reject_reserved_cue_text_characters(body, line_number)?;
    let (marker, text) = split_marker(body).ok_or_else(|| {
        ParseLyricsError::MissingMarker(MissingMarker {
            line_number,
            content: body.to_string(),
        })
    })?;
    if marker == CLEAR_MARKER || marker == END_OF_VIDEO_MARKER {
        return Err(ParseLyricsError::ReservedControlMarker(
            ReservedControlMarker {
                line_number,
                marker: marker.to_string(),
            },
        ));
    }
    if text.is_empty() {
        return Err(ParseLyricsError::EmptyCueBody(EmptyCueBody {
            line_number,
            marker: marker.to_string(),
        }));
    }
    Ok((marker, text))
}

/// Byte width of `marker: ` (the marker name, a colon, and one
/// ASCII space). Used to compute the expected indent of a
/// continuation line under the part it continues.
fn marker_prefix_width(marker: &str) -> usize {
    marker.len() + 2
}

/// Rejects a new event whose start time matches or precedes the
/// most recent recorded event. Skipped for `eov` lines because
/// `eov` does not push an event and therefore should not compete
/// for the same start-time slot as a real cue or `clr`.
fn check_event_order(
    start: Timestamp,
    line_number: usize,
    events: &[Event],
) -> Result<(), ParseLyricsError> {
    let Some(previous_start) = events.last().map(Event::start) else {
        return Ok(());
    };
    if previous_start == start {
        return Err(ParseLyricsError::RepeatedTimestamp(RepeatedTimestamp {
            line_number,
            start,
        }));
    }
    if start < previous_start {
        return Err(ParseLyricsError::OutOfOrder(OutOfOrder {
            previous: previous_start,
            next: start,
        }));
    }
    Ok(())
}

fn resolve_cues(events: Vec<Event>) -> Result<Vec<SubtitleCue>, ParseLyricsError> {
    let mut cues: Vec<SubtitleCue> = Vec::new();

    for (index, event) in events.iter().enumerate() {
        let Event::Cue(group) = event else {
            continue;
        };

        let end = events
            .get(index + 1)
            .map(Event::start)
            .ok_or(ParseLyricsError::UnclosedCue(UnclosedCue {
                start: group.start,
            }))?;

        cues.push(SubtitleCue {
            start: group.start,
            end,
            parts: group.parts.clone(),
        });
    }

    Ok(cues)
}

/// Splits a line body like `marker: text` into its two halves. Returns
/// `None` when the line has no `:` separator or when the marker half
/// is empty; the caller reports this as [`ParseLyricsError::MissingMarker`]
/// because every cue-opening line in the source format is expected to
/// carry a marker.
fn split_marker(body: &str) -> Option<(&str, &str)> {
    let (head, tail) = body.split_once(':')?;
    let marker = head.trim();
    if marker.is_empty() {
        return None;
    }
    Some((marker, tail.trim()))
}

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
    "line {line_number}: timestamp {start} repeats the start time of the immediately previous event; use the column-{prefix} shorthand to attach a second marker to the same timestamp",
    prefix = TIMESTAMP_PREFIX_WIDTH
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

#[derive(Debug, Display, Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ParseLyricsError {
    // Per-line failures, grouped by parse stage. The
    // indent-classifier guards (`TabIndentation`,
    // `MalformedIndentation`) fire first; the header path follows;
    // the shorthand path's diagnostic comes after.
    TabIndentation(#[error(not(source))] TabIndentation),
    MalformedIndentation(#[error(not(source))] MalformedIndentation),
    MalformedHeader(#[error(not(source))] MalformedHeader),
    InvalidTimestamp(#[error(not(source))] InvalidTimestamp),
    MissingSeparatorAfterTimestamp(#[error(not(source))] MissingSeparatorAfterTimestamp),
    ExtraTextAfterControlMarker(#[error(not(source))] ExtraTextAfterControlMarker),
    RepeatedTimestamp(#[error(not(source))] RepeatedTimestamp),
    OutOfOrder(#[error(not(source))] OutOfOrder),
    CueTextReservedCharacter(#[error(not(source))] CueTextReservedCharacter),
    MissingMarker(#[error(not(source))] MissingMarker),
    ReservedControlMarker(#[error(not(source))] ReservedControlMarker),
    EmptyCueBody(#[error(not(source))] EmptyCueBody),
    OrphanedShorthandMarker(#[error(not(source))] OrphanedShorthandMarker),
    // Post-pass failure, raised after `collect_events` returns
    // when `resolve_cues` finds a cue with no following event.
    UnclosedCue(#[error(not(source))] UnclosedCue),
}

/// Rejects any cue text (an opening line's body after the marker,
/// or a continuation line's contents) that contains a character
/// the WebVTT cue-tag grammar treats as a tag delimiter. The
/// renderer later HTML-entity-escapes the cue body, so literal
/// `<` and `>` in the source would not survive to the output as
/// themselves; they are rejected here to surface the author's
/// intent early rather than silently dropping it.
///
/// Reports the first offending character only. A line that
/// carries both `<` and `>` (the common `<tag>` shape) would in
/// principle benefit from a combined diagnostic, but a single
/// report per line is the convention every other `ParseLyricsError`
/// variant follows, and the author almost always types the two
/// angle brackets together; seeing the `<` once, fixing the whole
/// tag, and rerunning is the same workflow as for `MissingMarker`
/// or `ReservedControlMarker`.
fn reject_reserved_cue_text_characters(
    text: &str,
    line_number: usize,
) -> Result<(), ParseLyricsError> {
    if let Some(character) = text.chars().find(|&c| matches!(c, '<' | '>')) {
        return Err(ParseLyricsError::CueTextReservedCharacter(
            CueTextReservedCharacter {
                line_number,
                character,
            },
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests;
