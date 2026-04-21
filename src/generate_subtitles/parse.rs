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
use crate::timestamp::{TakeTimestampError, Timestamp};
use derive_more::{Display, Error};

/// A subtitle cue with a resolved end time, ready for rendering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubtitleCue {
    /// Timestamp at which the cue begins to display. Read directly
    /// from the `MM:SS.mmm` prefix on the cue-opening line.
    pub start: Timestamp,
    /// Timestamp at which the cue stops displaying. Taken from the
    /// timestamp of the next event in the source file, whether that
    /// is the next cue or a `clr` sentinel; `parse_lyrics` fails with
    /// [`ParseLyricsError::UnclosedCue`] if no such event exists.
    pub end: Timestamp,
    /// The leading marker token that the cue-opening line declared, for
    /// example `ttl` in `ttl: 《Song》`. Every cue-opening line in the
    /// source format carries a marker; lines that appear to lack one
    /// cause [`ParseLyricsError::MissingMarker`].
    pub marker: String,
    /// Cue text, with line breaks preserved between the opening line
    /// and any continuation lines.
    pub text: String,
}

/// An intermediate event extracted from a source file before end times
/// are resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Event {
    Cue {
        start: Timestamp,
        marker: String,
        text: String,
    },
    Clear {
        start: Timestamp,
    },
}

impl Event {
    fn start(&self) -> Timestamp {
        match self {
            Event::Cue { start, .. } => *start,
            Event::Clear { start } => *start,
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

    for (line_index, raw_line) in content.lines().enumerate() {
        let line_number = line_index + 1;
        let trimmed = raw_line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let header = match Timestamp::take(trimmed) {
            Ok((start, after_prefix)) => {
                let body = after_prefix.trim_start();
                if body.len() == after_prefix.len() {
                    return Err(ParseLyricsError::MissingSeparatorAfterTimestamp(
                        MissingSeparatorAfterTimestamp {
                            line_number,
                            content: trimmed.to_string(),
                        },
                    ));
                }
                Some((start, body))
            }
            Err(TakeTimestampError::ShapeMismatch) => None,
            Err(cause) => {
                return Err(ParseLyricsError::InvalidTimestamp(InvalidTimestamp {
                    line_number,
                    cause,
                }));
            }
        };

        match header {
            Some((start, body)) => {
                let first_token = body.split_whitespace().next().unwrap_or("");

                if first_token == END_OF_VIDEO_MARKER || first_token == CLEAR_MARKER {
                    if body.len() > first_token.len() {
                        return Err(ParseLyricsError::ExtraTextAfterControlMarker(
                            ExtraTextAfterControlMarker {
                                line_number,
                                marker: first_token.to_string(),
                                trailing: body[first_token.len()..].trim_start().to_string(),
                            },
                        ));
                    }
                    if first_token == CLEAR_MARKER {
                        events.push(Event::Clear { start });
                    }
                    last_cue_index = None;
                    continue;
                }

                let (marker, text) = split_marker(body).ok_or_else(|| {
                    ParseLyricsError::MissingMarker(MissingMarker {
                        line_number,
                        content: body.to_string(),
                    })
                })?;
                if text.is_empty() {
                    last_cue_index = None;
                    continue;
                }

                let event = Event::Cue {
                    start,
                    marker: marker.to_string(),
                    text: text.to_string(),
                };
                events.push(event);
                last_cue_index = Some(events.len() - 1);
            }
            None => {
                let Some(cue_index) = last_cue_index else {
                    return Err(ParseLyricsError::StrayContinuation(StrayContinuation {
                        line_number,
                        content: trimmed.to_string(),
                    }));
                };
                if let Event::Cue { text, .. } = &mut events[cue_index] {
                    text.push('\n');
                    text.push_str(trimmed);
                } else {
                    unreachable!("last_cue_index must point at a Cue event");
                }
            }
        }
    }

    for window in events.windows(2) {
        let previous = window[0].start();
        let next = window[1].start();
        if next < previous {
            return Err(ParseLyricsError::OutOfOrder(OutOfOrder { previous, next }));
        }
    }

    Ok(events)
}

fn resolve_cues(events: Vec<Event>) -> Result<Vec<SubtitleCue>, ParseLyricsError> {
    let mut cues: Vec<SubtitleCue> = Vec::new();

    for (index, event) in events.iter().enumerate() {
        let Event::Cue {
            start,
            marker,
            text,
        } = event
        else {
            continue;
        };

        let end = events
            .get(index + 1)
            .map(Event::start)
            .ok_or(ParseLyricsError::UnclosedCue(*start))?;

        cues.push(SubtitleCue {
            start: *start,
            end,
            marker: marker.clone(),
            text: text.clone(),
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

/// Payload for [`ParseLyricsError::StrayContinuation`].
#[derive(Debug, Display, Clone, PartialEq, Eq)]
#[display("line {line_number}: continuation text {content:?} before any cue")]
pub struct StrayContinuation {
    pub line_number: usize,
    pub content: String,
}

/// Payload for [`ParseLyricsError::MissingMarker`].
#[derive(Debug, Display, Clone, PartialEq, Eq)]
#[display("line {line_number}: cue body {content:?} carries no marker")]
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

#[derive(Debug, Display, Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ParseLyricsError {
    #[display("{_0}")]
    InvalidTimestamp(#[error(not(source))] InvalidTimestamp),
    #[display("{_0}")]
    StrayContinuation(#[error(not(source))] StrayContinuation),
    #[display("{_0}")]
    MissingMarker(#[error(not(source))] MissingMarker),
    #[display("{_0}")]
    MissingSeparatorAfterTimestamp(#[error(not(source))] MissingSeparatorAfterTimestamp),
    #[display("{_0}")]
    ExtraTextAfterControlMarker(#[error(not(source))] ExtraTextAfterControlMarker),
    #[display("{_0}")]
    OutOfOrder(#[error(not(source))] OutOfOrder),
    #[display("cue at {_0} has no following cue or `clr`")]
    UnclosedCue(#[error(not(source))] Timestamp),
}

#[cfg(test)]
mod tests;
