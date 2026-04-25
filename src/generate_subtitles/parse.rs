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

/// Payload of an [`Event::Cue`]. The start time is the one declared
/// in the source file; the end time is resolved later by looking at
/// the next event in the stream.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Cue {
    start: Timestamp,
    marker: String,
    text: String,
}

/// An intermediate event extracted from a source file before end times
/// are resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Event {
    Cue(Cue),
    Clear(Timestamp),
}

impl Event {
    fn start(&self) -> Timestamp {
        match self {
            Event::Cue(cue) => cue.start,
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

    for (line_index, raw_line) in content.lines().enumerate() {
        let line_number = line_index + 1;
        let trimmed = raw_line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Indentation must use ASCII spaces only; a tab in the
        // leading whitespace would interact unpredictably with the
        // forthcoming column-exact indentation rules and is rejected
        // here so the prohibition shows up at the boundary rather
        // than in a downstream "indent does not match expected
        // width" message.
        if raw_line
            .bytes()
            .take_while(|&b| b == b' ' || b == b'\t')
            .any(|b| b == b'\t')
        {
            return Err(ParseLyricsError::TabIndentation(TabIndentation {
                line_number,
            }));
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
                    let trailing = body[first_token.len()..].trim();
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
                        events.push(Event::Clear(start));
                        last_cue_index = None;
                    }
                    continue;
                }

                // Run the reserved-character check on the full body
                // before `split_marker`. A line such as `<v>foo</v>`
                // has no `:` separator, so `split_marker` would
                // return `None` and the error would surface as
                // `MissingMarker` even though the real problem is
                // the angle brackets. Checking here lets the more
                // specific `CueTextReservedCharacter` diagnostic
                // win. The control-marker branch above is
                // deliberately left above this check because its
                // own `ExtraTextAfterControlMarker` diagnostic is
                // more specific than the reserved-character error
                // for the `clr`/`eov` cases.
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

                events.push(Event::Cue(Cue {
                    start,
                    marker: marker.to_string(),
                    text: text.to_string(),
                }));
                last_cue_index = Some(events.len() - 1);
            }
            None => {
                let Some(cue_index) = last_cue_index else {
                    return Err(ParseLyricsError::StrayContinuation(StrayContinuation {
                        line_number,
                        content: trimmed.to_string(),
                    }));
                };
                reject_reserved_cue_text_characters(trimmed, line_number)?;
                if let Event::Cue(Cue { text, .. }) = &mut events[cue_index] {
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
        let Event::Cue(Cue {
            start,
            marker,
            text,
        }) = event
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

#[derive(Debug, Display, Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ParseLyricsError {
    // Per-line failures, in the order they are raised inside the
    // `collect_events` loop.
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
    ReservedControlMarker(#[error(not(source))] ReservedControlMarker),
    #[display("{_0}")]
    EmptyCueBody(#[error(not(source))] EmptyCueBody),
    #[display("{_0}")]
    TabIndentation(#[error(not(source))] TabIndentation),
    #[display("{_0}")]
    CueTextReservedCharacter(#[error(not(source))] CueTextReservedCharacter),
    // Post-pass failures, raised after `collect_events` returns.
    #[display("{_0}")]
    OutOfOrder(#[error(not(source))] OutOfOrder),
    #[display("cue at {_0} has no following cue or `clr`")]
    UnclosedCue(#[error(not(source))] Timestamp),
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
