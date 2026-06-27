//! Parser for `lyrics.{lang}.txt` cue files.
//!
//! Each file is a sequence of timestamped events. A line that starts
//! with `MM:SS.mmm` opens an event. If the event's first non-whitespace
//! token is [`CLEAR_MARKER`], the currently open cue is closed at that
//! timestamp; if it is [`END_OF_VIDEO_MARKER`], the line is ignored.
//! Any other event opens a new cue; continuation lines that lack a
//! leading timestamp are appended to the most recently opened cue.
//!
//! [`CLEAR_MARKER`]: lyrics_core::line_markers_descriptor::CLEAR_MARKER
//! [`END_OF_VIDEO_MARKER`]: lyrics_core::line_markers_descriptor::END_OF_VIDEO_MARKER

pub mod error;

use error::{
    CueTextReservedCharacter, EmptyCueBody, ExtraTextAfterControlMarker, InvalidTimestamp,
    MalformedHeader, MalformedIndentation, MissingMarker, MissingSeparatorAfterTimestamp,
    OrphanedShorthandMarker, OutOfOrder, ParseLyricsError, RepeatedTimestamp,
    ReservedControlMarker, TabIndentation, UnclosedCue,
};
use lyrics_core::line_markers_descriptor::{CLEAR_MARKER, END_OF_VIDEO_MARKER};
use lyrics_core::timestamp::{TIMESTAMP_STR_LEN, TakeTimestampError, Timestamp};

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
/// group with one part, a column-[`TIMESTAMP_PREFIX_WIDTH`] shorthand
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
    let mut events = Vec::<Event>::new();
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
            .is_some_and(|width| indent == TIMESTAMP_PREFIX_WIDTH + width)
        {
            handle_continuation_line(body, line_number, &mut events, last_cue_index)?;
        } else {
            return Err(ParseLyricsError::MalformedIndentation(
                MalformedIndentation {
                    line_number,
                    actual: indent,
                    shorthand_indent: TIMESTAMP_PREFIX_WIDTH,
                    continuation_indent: last_part_marker_prefix_width
                        .map(|width| TIMESTAMP_PREFIX_WIDTH + width),
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
    let mut cues = Vec::<SubtitleCue>::new();

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
/// report per line is the convention every other [`ParseLyricsError`]
/// variant follows, and the author almost always types the two
/// angle brackets together; seeing the `<` once, fixing the whole
/// tag, and rerunning is the same workflow as for [`MissingMarker`]
/// or [`ReservedControlMarker`].
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
