use core::fmt;
use derive_more::{Display, Error};

/// Duration in milliseconds from the start of the video. This is the
/// common unit in which cue start and end times are compared.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub struct Milliseconds(pub u64);

impl Milliseconds {
    /// Parses the `MM:SS.mmm` form that opens each cue in
    /// `lyrics.*.txt`. The minutes and milliseconds fields may carry
    /// more than their nominal number of digits; in particular a cue
    /// may reach `60:00.000` or beyond when a song is longer than an
    /// hour.
    pub fn parse_source(input: &str) -> Result<Self, ParseTimestampError> {
        let (minutes_part, seconds_part) = input
            .split_once(':')
            .ok_or(ParseTimestampError::MissingColon)?;
        let (seconds_part, milliseconds_part) = seconds_part
            .split_once('.')
            .ok_or(ParseTimestampError::MissingDot)?;
        let minutes: u64 = minutes_part
            .parse()
            .map_err(|_| ParseTimestampError::InvalidMinutes(minutes_part.to_string()))?;
        let seconds: u64 = seconds_part
            .parse()
            .map_err(|_| ParseTimestampError::InvalidSeconds(seconds_part.to_string()))?;
        let milliseconds: u64 = milliseconds_part
            .parse()
            .map_err(|_| ParseTimestampError::InvalidMilliseconds(milliseconds_part.to_string()))?;
        Ok(Milliseconds(
            minutes * 60_000 + seconds * 1_000 + milliseconds,
        ))
    }

    /// Renders the timestamp in the `MM:SS.mmm` source format. Used
    /// by error messages to point back at the offending line.
    pub fn source_fmt(self) -> impl fmt::Display {
        let Milliseconds(total) = self;
        SourceFmt {
            minutes: total / 60_000,
            seconds: (total % 60_000) / 1_000,
            milliseconds: total % 1_000,
        }
    }

    /// Renders the timestamp in the `HH:MM:SS,mmm` SRT format.
    pub fn srt_fmt(self) -> impl fmt::Display {
        let Milliseconds(total) = self;
        HhMmSsMillisFmt {
            hours: total / 3_600_000,
            minutes: (total % 3_600_000) / 60_000,
            seconds: (total % 60_000) / 1_000,
            milliseconds: total % 1_000,
            fraction_separator: ',',
        }
    }

    /// Renders the timestamp in the `HH:MM:SS.mmm` WebVTT format.
    pub fn vtt_fmt(self) -> impl fmt::Display {
        let Milliseconds(total) = self;
        HhMmSsMillisFmt {
            hours: total / 3_600_000,
            minutes: (total % 3_600_000) / 60_000,
            seconds: (total % 60_000) / 1_000,
            milliseconds: total % 1_000,
            fraction_separator: '.',
        }
    }
}

struct SourceFmt {
    minutes: u64,
    seconds: u64,
    milliseconds: u64,
}

impl fmt::Display for SourceFmt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:02}:{:02}.{:03}",
            self.minutes, self.seconds, self.milliseconds,
        )
    }
}

struct HhMmSsMillisFmt {
    hours: u64,
    minutes: u64,
    seconds: u64,
    milliseconds: u64,
    fraction_separator: char,
}

impl fmt::Display for HhMmSsMillisFmt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:02}:{:02}:{:02}{}{:03}",
            self.hours, self.minutes, self.seconds, self.fraction_separator, self.milliseconds,
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
    #[display("invalid minutes component: {_0:?}")]
    InvalidMinutes(#[error(not(source))] String),
    #[display("invalid seconds component: {_0:?}")]
    InvalidSeconds(#[error(not(source))] String),
    #[display("invalid milliseconds component: {_0:?}")]
    InvalidMilliseconds(#[error(not(source))] String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_basic_timestamp() {
        assert_eq!(
            Milliseconds::parse_source("00:02.960").unwrap(),
            Milliseconds(2_960),
        );
    }

    #[test]
    fn parses_timestamp_with_high_minutes() {
        assert_eq!(
            Milliseconds::parse_source("01:59.715").unwrap(),
            Milliseconds(119_715),
        );
    }

    #[test]
    fn source_fmt_round_trips() {
        let cases = ["00:02.960", "01:59.715", "02:07.075", "04:46.000"];
        for input in cases {
            let value = Milliseconds::parse_source(input).unwrap();
            assert_eq!(value.source_fmt().to_string(), input);
        }
    }

    #[test]
    fn srt_fmt_uses_comma() {
        assert_eq!(Milliseconds(2_960).srt_fmt().to_string(), "00:00:02,960",);
    }

    #[test]
    fn vtt_fmt_uses_dot() {
        assert_eq!(Milliseconds(2_960).vtt_fmt().to_string(), "00:00:02.960",);
    }

    #[test]
    fn hour_boundary() {
        let value = Milliseconds(3_600_000 + 62_000 + 15);
        assert_eq!(value.srt_fmt().to_string(), "01:01:02,015");
        assert_eq!(value.vtt_fmt().to_string(), "01:01:02.015");
    }

    #[test]
    fn rejects_missing_colon() {
        assert!(matches!(
            Milliseconds::parse_source("0002.960"),
            Err(ParseTimestampError::MissingColon),
        ));
    }

    #[test]
    fn rejects_missing_dot() {
        assert!(matches!(
            Milliseconds::parse_source("00:02"),
            Err(ParseTimestampError::MissingDot),
        ));
    }
}
