use core::fmt;
use core::num::ParseIntError;
use core::str::FromStr;
use derive_more::{Display, Error, Into};

/// Duration in milliseconds from the start of the video. Cues use it
/// for start and end times and for ordering comparisons.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Into)]
pub struct Milliseconds(u64);

impl Milliseconds {
    /// Builds a `Milliseconds` from an already-decomposed
    /// `MM:SS.mmm` triple. Tests and the string parser both use this
    /// constructor to avoid opaque literals such as `Milliseconds(2_960)`.
    pub const fn new(minutes: u64, seconds: u64, milliseconds: u64) -> Self {
        Milliseconds(minutes * 60_000 + seconds * 1_000 + milliseconds)
    }
}

impl FromStr for Milliseconds {
    type Err = ParseTimestampError;

    /// Parses the `MM:SS.mmm` form that opens each cue in
    /// `lyrics.*.txt`. The caller is expected to have extracted the
    /// 9-byte prefix `DD:DD.DDD` beforehand. Songs longer than 99
    /// minutes would require widening both this parser and the
    /// tokenizer in [`crate::build_subtitles::parse`].
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
        let milliseconds = milliseconds_part.parse::<u64>().map_err(|source| {
            ParseTimestampError::InvalidMilliseconds {
                value: milliseconds_part.to_string(),
                source,
            }
        })?;
        Ok(Milliseconds::new(minutes, seconds, milliseconds))
    }
}

/// Renders `Milliseconds` in the `MM:SS.mmm` source format. Error
/// messages that quote a timestamp use this implementation so the
/// output matches the form the source file used.
impl fmt::Display for Milliseconds {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Milliseconds(total) = *self;
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
impl fmt::Debug for Milliseconds {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

/// Thin wrapper around [`Milliseconds`] that renders in the SubRip
/// `HH:MM:SS,mmm` format.
#[derive(Clone, Copy)]
pub struct SrtTime(pub Milliseconds);

impl fmt::Display for SrtTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Milliseconds(total) = self.0;
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

/// Thin wrapper around [`Milliseconds`] that renders in the WebVTT
/// `HH:MM:SS.mmm` format.
#[derive(Clone, Copy)]
pub struct VttTime(pub Milliseconds);

impl fmt::Display for VttTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Milliseconds(total) = self.0;
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_basic_timestamp() {
        assert_eq!(
            "00:02.960".parse::<Milliseconds>().unwrap(),
            Milliseconds::new(0, 2, 960),
        );
    }

    #[test]
    fn parses_timestamp_with_high_minutes() {
        assert_eq!(
            "01:59.715".parse::<Milliseconds>().unwrap(),
            Milliseconds::new(1, 59, 715),
        );
    }

    #[test]
    fn display_round_trips() {
        let cases = ["00:02.960", "01:59.715", "02:07.075", "04:46.000"];
        for input in cases {
            let value: Milliseconds = input.parse().unwrap();
            assert_eq!(value.to_string(), input);
        }
    }

    #[test]
    fn srt_time_uses_comma() {
        assert_eq!(
            SrtTime(Milliseconds::new(0, 2, 960)).to_string(),
            "00:00:02,960",
        );
    }

    #[test]
    fn vtt_time_uses_dot() {
        assert_eq!(
            VttTime(Milliseconds::new(0, 2, 960)).to_string(),
            "00:00:02.960",
        );
    }

    #[test]
    fn hour_boundary() {
        let value = Milliseconds::new(61, 2, 15);
        assert_eq!(SrtTime(value).to_string(), "01:01:02,015");
        assert_eq!(VttTime(value).to_string(), "01:01:02.015");
    }

    #[test]
    fn rejects_missing_colon() {
        assert!(matches!(
            "0002.960".parse::<Milliseconds>(),
            Err(ParseTimestampError::MissingColon),
        ));
    }

    #[test]
    fn rejects_missing_dot() {
        assert!(matches!(
            "00:02".parse::<Milliseconds>(),
            Err(ParseTimestampError::MissingDot),
        ));
    }
}
