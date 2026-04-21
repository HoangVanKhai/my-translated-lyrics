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
use crate::timestamp::{ParseTimestampError, Timestamp};
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

        match split_timestamp(trimmed) {
            Some((timestamp_str, rest)) => {
                let start = timestamp_str.parse::<Timestamp>().map_err(|source| {
                    ParseLyricsError::InvalidTimestamp {
                        line_number,
                        raw: timestamp_str.to_string(),
                        source,
                    }
                })?;
                let rest = rest.trim();
                let first_token = rest.split_whitespace().next().unwrap_or("");

                if first_token == END_OF_VIDEO_MARKER || first_token == CLEAR_MARKER {
                    if rest.len() > first_token.len() {
                        return Err(ParseLyricsError::ExtraTextAfterControlMarker {
                            line_number,
                            marker: first_token.to_string(),
                            trailing: rest[first_token.len()..].trim_start().to_string(),
                        });
                    }
                    if first_token == CLEAR_MARKER {
                        events.push(Event::Clear { start });
                    }
                    last_cue_index = None;
                    continue;
                }

                let (marker, text) =
                    split_marker(rest).ok_or_else(|| ParseLyricsError::MissingMarker {
                        line_number,
                        content: rest.to_string(),
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
                    return Err(ParseLyricsError::StrayContinuation {
                        line_number,
                        content: trimmed.to_string(),
                    });
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
            return Err(ParseLyricsError::OutOfOrder { previous, next });
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
            .iter()
            .skip(index + 1)
            .map(Event::start)
            .next()
            .ok_or(ParseLyricsError::UnclosedCue { start: *start })?;

        cues.push(SubtitleCue {
            start: *start,
            end,
            marker: marker.clone(),
            text: text.clone(),
        });
    }

    Ok(cues)
}

/// Splits a line into a timestamp prefix and the remainder. Returns
/// `None` when the line does not start with an `MM:SS.mmm` timestamp.
fn split_timestamp(line: &str) -> Option<(&str, &str)> {
    let bytes = line.as_bytes();
    if bytes.len() < 9 {
        return None;
    }
    if !bytes[0].is_ascii_digit() || !bytes[1].is_ascii_digit() {
        return None;
    }
    if bytes[2] != b':' {
        return None;
    }
    if !bytes[3].is_ascii_digit() || !bytes[4].is_ascii_digit() {
        return None;
    }
    if bytes[5] != b'.' {
        return None;
    }
    if !bytes[6].is_ascii_digit() || !bytes[7].is_ascii_digit() || !bytes[8].is_ascii_digit() {
        return None;
    }
    let timestamp = &line[..9];
    let rest = &line[9..];
    let rest_trim_start = rest.trim_start();
    if rest.len() == rest_trim_start.len() {
        return None;
    }
    Some((timestamp, rest_trim_start))
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

#[derive(Debug, Display, Error)]
#[non_exhaustive]
pub enum ParseLyricsError {
    #[display("line {line_number}: invalid timestamp {raw:?}: {source}")]
    InvalidTimestamp {
        line_number: usize,
        raw: String,
        source: ParseTimestampError,
    },
    #[display("line {line_number}: continuation text {content:?} before any cue")]
    StrayContinuation {
        #[error(not(source))]
        line_number: usize,
        #[error(not(source))]
        content: String,
    },
    #[display("line {line_number}: cue body {content:?} carries no marker")]
    MissingMarker {
        #[error(not(source))]
        line_number: usize,
        #[error(not(source))]
        content: String,
    },
    #[display(
        "line {line_number}: control marker {marker:?} must stand alone but is followed by {trailing:?}"
    )]
    ExtraTextAfterControlMarker {
        #[error(not(source))]
        line_number: usize,
        #[error(not(source))]
        marker: String,
        #[error(not(source))]
        trailing: String,
    },
    #[display("events out of order: cue at {previous} is followed by an earlier cue at {next}")]
    OutOfOrder {
        #[error(not(source))]
        previous: Timestamp,
        #[error(not(source))]
        next: Timestamp,
    },
    #[display("cue at {start} has no following cue or `clr`")]
    UnclosedCue {
        #[error(not(source))]
        start: Timestamp,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use text_block_macros::text_block_fnl;

    #[test]
    fn parses_simple_sequence() {
        let input = text_block_fnl! {
            "00:00.000 ttl: Hello"
            "00:02.000 LRC: world"
            "00:04.000 clr"
        };
        let cues = parse_lyrics(input).unwrap();
        assert_eq!(cues.len(), 2);
        assert_eq!(cues[0].start, Timestamp::new(0, 0, 0));
        assert_eq!(cues[0].end, Timestamp::new(0, 2, 0));
        assert_eq!(cues[0].marker, "ttl");
        assert_eq!(cues[0].text, "Hello");
        assert_eq!(cues[1].start, Timestamp::new(0, 2, 0));
        assert_eq!(cues[1].end, Timestamp::new(0, 4, 0));
        assert_eq!(cues[1].marker, "LRC");
        assert_eq!(cues[1].text, "world");
    }

    #[test]
    fn comments_and_blank_lines_are_skipped() {
        let input = text_block_fnl! {
            "# this is ignored"
            ""
            "00:00.000 ttl: Hello"
            "# still ignored"
            "00:02.000 clr"
        };
        let cues = parse_lyrics(input).unwrap();
        assert_eq!(cues.len(), 1);
        assert_eq!(cues[0].text, "Hello");
    }

    #[test]
    fn continuation_lines_append_to_current_cue() {
        let input = text_block_fnl! {
            "00:00.000 cre: first line"
            "            second line"
            "            third line"
            "00:05.000 clr"
        };
        let cues = parse_lyrics(input).unwrap();
        assert_eq!(cues.len(), 1);
        assert_eq!(cues[0].text, "first line\nsecond line\nthird line");
    }

    #[test]
    fn control_markers_accept_trailing_whitespace_only() {
        let input = text_block_fnl! {
            "00:00.000 ttl: Hello"
            "00:02.000 clr \t "
            "00:05.000 eov\t"
        };
        let cues = parse_lyrics(input).unwrap();
        assert_eq!(cues.len(), 1);
        assert_eq!(cues[0].end, Timestamp::new(0, 2, 0));
    }

    #[test]
    fn control_markers_reject_trailing_text() {
        let clr_input = text_block_fnl! {
            "00:00.000 ttl: Hello"
            "00:02.000 clr some trailing text"
        };
        assert!(matches!(
            parse_lyrics(clr_input),
            Err(ParseLyricsError::ExtraTextAfterControlMarker {
                marker,
                ..
            }) if marker == "clr",
        ));

        let eov_input = text_block_fnl! {
            "00:00.000 ttl: Hello"
            "00:02.000 clr"
            "00:05.000 eov\tend of video"
        };
        assert!(matches!(
            parse_lyrics(eov_input),
            Err(ParseLyricsError::ExtraTextAfterControlMarker {
                marker,
                ..
            }) if marker == "eov",
        ));
    }

    #[test]
    fn eov_marker_does_not_produce_a_cue() {
        let input = text_block_fnl! {
            "00:00.000 ttl: Hello"
            "00:02.000 clr"
            ""
            "00:05.000 eov"
        };
        let cues = parse_lyrics(input).unwrap();
        assert_eq!(cues.len(), 1);
        assert_eq!(cues[0].end, Timestamp::new(0, 2, 0));
    }

    #[test]
    fn rejects_cue_line_without_marker() {
        let input = text_block_fnl! {
            "00:00.000 Plain text without marker"
            "00:02.000 clr"
        };
        assert!(matches!(
            parse_lyrics(input),
            Err(ParseLyricsError::MissingMarker { .. }),
        ));
    }

    #[test]
    fn cue_ends_at_next_cue_when_no_clr() {
        let input = text_block_fnl! {
            "00:00.000 ttl: A"
            "00:01.000 ttl: B"
            "00:02.000 clr"
        };
        let cues = parse_lyrics(input).unwrap();
        assert_eq!(cues[0].end, Timestamp::new(0, 1, 0));
        assert_eq!(cues[1].end, Timestamp::new(0, 2, 0));
    }

    #[test]
    fn rejects_cue_without_following_event() {
        let input = "00:00.000 ttl: Hello\n";
        assert!(matches!(
            parse_lyrics(input),
            Err(ParseLyricsError::UnclosedCue { .. }),
        ));
    }

    #[test]
    fn rejects_out_of_order_events() {
        let input = text_block_fnl! {
            "00:02.000 ttl: A"
            "00:01.000 ttl: B"
            "00:03.000 clr"
        };
        assert!(matches!(
            parse_lyrics(input),
            Err(ParseLyricsError::OutOfOrder { .. }),
        ));
    }
}
