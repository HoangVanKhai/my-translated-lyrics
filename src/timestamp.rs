use core::fmt;
use core::num::ParseIntError;
use core::str::FromStr;
use derive_more::{Display, Error};

/// A point in time inside the video, measured as milliseconds from
/// `00:00.000`. Cues use it for start and end positions and for
/// ordering comparisons. The millisecond resolution is an internal
/// implementation detail; callers compose and destructure via the
/// minute / second / millisecond API surface.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Timestamp(u64);

impl Timestamp {
    /// Composes a `Timestamp` total from minutes, seconds, and
    /// milliseconds components. The result is
    /// `minutes * 60_000 + seconds * 1_000 + milliseconds`, so this
    /// constructor doubles as a single-unit conversion:
    /// `Timestamp::new(n, 0, 0)` yields `n` minutes,
    /// `Timestamp::new(0, n, 0)` yields `n` seconds, and
    /// `Timestamp::new(0, 0, n)` yields `n` milliseconds.
    ///
    /// The components are intentionally not range-checked. Supporting
    /// the single-unit patterns above requires the same arithmetic
    /// that would normalize an out-of-range cue component, and that
    /// same arithmetic falls out of the composition naturally.
    /// Callers that need strict `SS < 60` / `mmm < 1_000` validation
    /// must perform it before calling `new`; the [`FromStr`] impl
    /// does so for `MM:SS.mmm` source strings.
    pub const fn new(minutes: u64, seconds: u64, milliseconds: u64) -> Self {
        Timestamp(minutes * 60_000 + seconds * 1_000 + milliseconds)
    }
}

impl FromStr for Timestamp {
    type Err = ParseTimestampError;

    /// Parses the `MM:SS.mmm` form that opens each cue in
    /// `lyrics.*.txt`. The caller is expected to have extracted the
    /// 9-byte `MM:SS.mmm` prefix beforehand. Songs longer than 99
    /// minutes would require widening both this parser and the
    /// tokenizer in [`crate::generate_subtitles::parse`].
    ///
    /// Seconds must satisfy `0 <= SS < 60` and milliseconds
    /// `0 <= mmm < 1_000`. Out-of-range components raise
    /// [`ParseTimestampError::SecondsOutOfRange`] or
    /// [`ParseTimestampError::MillisecondsOutOfRange`] rather than
    /// being folded silently into the total.
    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let (minutes_part, seconds_part) = input
            .split_once(':')
            .ok_or(ParseTimestampError::MissingColon)?;
        let (seconds_part, milliseconds_part) = seconds_part
            .split_once('.')
            .ok_or(ParseTimestampError::MissingDot)?;
        let minutes =
            minutes_part
                .parse::<u64>()
                .map_err(|source| ParseTimestampError::InvalidMinutes {
                    value: minutes_part.to_string(),
                    source,
                })?;
        let seconds =
            seconds_part
                .parse::<u64>()
                .map_err(|source| ParseTimestampError::InvalidSeconds {
                    value: seconds_part.to_string(),
                    source,
                })?;
        if seconds >= 60 {
            return Err(ParseTimestampError::SecondsOutOfRange { value: seconds });
        }
        let milliseconds = milliseconds_part.parse::<u64>().map_err(|source| {
            ParseTimestampError::InvalidMilliseconds {
                value: milliseconds_part.to_string(),
                source,
            }
        })?;
        if milliseconds >= 1_000 {
            return Err(ParseTimestampError::MillisecondsOutOfRange {
                value: milliseconds,
            });
        }
        Ok(Timestamp::new(minutes, seconds, milliseconds))
    }
}

/// Renders `Timestamp` in the `MM:SS.mmm` source format. Error
/// messages that quote a timestamp use this implementation so the
/// output matches the form the source file used.
impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Timestamp(total) = *self;
        write!(
            f,
            "{minutes:02}:{seconds:02}.{milliseconds:03}",
            minutes = total / 60_000,
            seconds = (total % 60_000) / 1_000,
            milliseconds = total % 1_000,
        )
    }
}

/// `Debug` reuses `Display` so panic messages and assertion failures
/// quote timestamps in the same `MM:SS.mmm` shape as the rest of the
/// pipeline.
impl fmt::Debug for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

/// Thin wrapper around [`Timestamp`] that renders in the SubRip
/// `HH:MM:SS,mmm` format.
#[derive(Clone, Copy)]
pub struct SrtTime(pub Timestamp);

impl fmt::Display for SrtTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Timestamp(total) = self.0;
        write!(
            f,
            "{hours:02}:{minutes:02}:{seconds:02},{milliseconds:03}",
            hours = total / 3_600_000,
            minutes = (total % 3_600_000) / 60_000,
            seconds = (total % 60_000) / 1_000,
            milliseconds = total % 1_000,
        )
    }
}

/// Thin wrapper around [`Timestamp`] that renders in the WebVTT
/// `HH:MM:SS.mmm` format.
#[derive(Clone, Copy)]
pub struct VttTime(pub Timestamp);

impl fmt::Display for VttTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Timestamp(total) = self.0;
        write!(
            f,
            "{hours:02}:{minutes:02}:{seconds:02}.{milliseconds:03}",
            hours = total / 3_600_000,
            minutes = (total % 3_600_000) / 60_000,
            seconds = (total % 60_000) / 1_000,
            milliseconds = total % 1_000,
        )
    }
}

#[derive(Debug, Display, Error)]
#[non_exhaustive]
pub enum ParseTimestampError {
    #[display("timestamp is missing ':' between minutes and seconds")]
    MissingColon,
    #[display("timestamp is missing '.' between seconds and milliseconds")]
    MissingDot,
    #[display("invalid minutes component {value:?}: {source}")]
    InvalidMinutes {
        #[error(not(source))]
        value: String,
        source: ParseIntError,
    },
    #[display("invalid seconds component {value:?}: {source}")]
    InvalidSeconds {
        #[error(not(source))]
        value: String,
        source: ParseIntError,
    },
    #[display("invalid milliseconds component {value:?}: {source}")]
    InvalidMilliseconds {
        #[error(not(source))]
        value: String,
        source: ParseIntError,
    },
    #[display("seconds component {value} is out of range 0..60")]
    SecondsOutOfRange {
        #[error(not(source))]
        value: u64,
    },
    #[display("milliseconds component {value} is out of range 0..1000")]
    MillisecondsOutOfRange {
        #[error(not(source))]
        value: u64,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_basic_timestamp() {
        assert_eq!(
            "00:02.960".parse::<Timestamp>().unwrap(),
            Timestamp::new(0, 2, 960),
        );
    }

    #[test]
    fn new_composes_weighted_components() {
        assert_eq!(Timestamp::new(0, 0, 1), Timestamp::new(0, 0, 1));
        assert_eq!(Timestamp::new(0, 1, 0), Timestamp::new(0, 0, 1_000));
        assert_eq!(Timestamp::new(1, 0, 0), Timestamp::new(0, 60, 0));
        assert_eq!(Timestamp::new(0, 0, 2_500), Timestamp::new(0, 2, 500));
    }

    #[test]
    fn parses_timestamp_with_high_minutes() {
        assert_eq!(
            "01:59.715".parse::<Timestamp>().unwrap(),
            Timestamp::new(1, 59, 715),
        );
    }

    #[test]
    fn display_round_trips() {
        let cases = ["00:02.960", "01:59.715", "02:07.075", "04:46.000"];
        for input in cases {
            let value: Timestamp = input.parse().unwrap();
            assert_eq!(value.to_string(), input);
        }
    }

    #[test]
    fn srt_time_uses_comma() {
        assert_eq!(
            SrtTime(Timestamp::new(0, 2, 960)).to_string(),
            "00:00:02,960",
        );
    }

    #[test]
    fn vtt_time_uses_dot() {
        assert_eq!(
            VttTime(Timestamp::new(0, 2, 960)).to_string(),
            "00:00:02.960",
        );
    }

    #[test]
    fn hour_boundary() {
        let value = Timestamp::new(61, 2, 15);
        assert_eq!(SrtTime(value).to_string(), "01:01:02,015");
        assert_eq!(VttTime(value).to_string(), "01:01:02.015");
    }

    #[test]
    fn rejects_missing_colon() {
        assert!(matches!(
            "0002.960".parse::<Timestamp>(),
            Err(ParseTimestampError::MissingColon),
        ));
    }

    #[test]
    fn rejects_missing_dot() {
        assert!(matches!(
            "00:02".parse::<Timestamp>(),
            Err(ParseTimestampError::MissingDot),
        ));
    }

    #[test]
    fn rejects_seconds_out_of_range() {
        assert!(matches!(
            "00:60.000".parse::<Timestamp>(),
            Err(ParseTimestampError::SecondsOutOfRange { value: 60 }),
        ));
        assert!(matches!(
            "00:99.000".parse::<Timestamp>(),
            Err(ParseTimestampError::SecondsOutOfRange { value: 99 }),
        ));
    }

    #[test]
    fn rejects_milliseconds_out_of_range() {
        assert!(matches!(
            "00:00.1000".parse::<Timestamp>(),
            Err(ParseTimestampError::MillisecondsOutOfRange { value: 1000 }),
        ));
    }
}
