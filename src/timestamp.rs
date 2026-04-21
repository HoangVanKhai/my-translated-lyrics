use core::fmt;
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
    /// must perform it before calling `new`; [`Timestamp::take`] does
    /// so for `MM:SS.mmm` source strings.
    pub const fn new(minutes: u64, seconds: u64, milliseconds: u64) -> Self {
        Timestamp(minutes * 60_000 + seconds * 1_000 + milliseconds)
    }

    /// Consumes a leading `MM:SS.mmm` prefix (9 ASCII characters)
    /// from `input` and returns the parsed `Timestamp` along with the
    /// unconsumed tail. Follows the parse-don't-validate pattern:
    ///
    /// - `Ok(Some((ts, tail)))` — the prefix matched the shape and
    ///   every component fits its range. `tail` is `input` past the
    ///   nine consumed characters, untouched.
    /// - `Ok(None)` — the first nine characters of `input` do not
    ///   form an `MM:SS.mmm` shape (too short, wrong punctuation, or
    ///   a non-digit where a digit is required). Callers typically
    ///   treat this as "no timestamp here" and route the line
    ///   elsewhere.
    /// - `Err(_)` — the prefix has timestamp shape but a component
    ///   is out of range (currently only `seconds >= 60`; three-digit
    ///   milliseconds can never exceed 999, and two-digit minutes
    ///   are uncapped by design). The error carries a copy of the
    ///   offending 9-character prefix for diagnostics.
    ///
    /// The caller is responsible for anything past the prefix: if
    /// the cue format requires whitespace between the timestamp and
    /// the body, the caller inspects `tail` for it.
    pub fn take(input: &str) -> Result<Option<(Self, &str)>, ParseTimestampError> {
        let digit = |next: Option<char>| -> Option<u8> {
            let ch = next.filter(char::is_ascii_digit)?;
            Some((ch as u8) - b'0')
        };

        let mut chars = input.chars();
        let Some(tens_min) = digit(chars.next()) else {
            return Ok(None);
        };
        let Some(ones_min) = digit(chars.next()) else {
            return Ok(None);
        };
        if !matches!(chars.next(), Some(':')) {
            return Ok(None);
        }
        let Some(tens_sec) = digit(chars.next()) else {
            return Ok(None);
        };
        let Some(ones_sec) = digit(chars.next()) else {
            return Ok(None);
        };
        if !matches!(chars.next(), Some('.')) {
            return Ok(None);
        }
        let Some(hundreds_ms) = digit(chars.next()) else {
            return Ok(None);
        };
        let Some(tens_ms) = digit(chars.next()) else {
            return Ok(None);
        };
        let Some(ones_ms) = digit(chars.next()) else {
            return Ok(None);
        };

        let seconds = u64::from(tens_sec) * 10 + u64::from(ones_sec);
        if seconds >= 60 {
            return Err(ParseTimestampError::SecondsOutOfRange {
                raw: input[..9].to_string(),
                value: seconds,
            });
        }
        let minutes = u64::from(tens_min) * 10 + u64::from(ones_min);
        let milliseconds =
            u64::from(hundreds_ms) * 100 + u64::from(tens_ms) * 10 + u64::from(ones_ms);
        Ok(Some((
            Timestamp::new(minutes, seconds, milliseconds),
            chars.as_str(),
        )))
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
    #[display("invalid timestamp {raw:?}: seconds component {value} must be less than 60")]
    SecondsOutOfRange {
        #[error(not(source))]
        raw: String,
        #[error(not(source))]
        value: u64,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn takes_basic_timestamp_with_tail() {
        let (ts, tail) = Timestamp::take("00:02.960 hello").unwrap().unwrap();
        assert_eq!(ts, Timestamp::new(0, 2, 960));
        assert_eq!(tail, " hello");
    }

    #[test]
    fn takes_timestamp_at_exact_length() {
        let (ts, tail) = Timestamp::take("01:59.715").unwrap().unwrap();
        assert_eq!(ts, Timestamp::new(1, 59, 715));
        assert_eq!(tail, "");
    }

    #[test]
    fn new_composes_weighted_components() {
        assert_eq!(Timestamp::new(0, 0, 1), Timestamp::new(0, 0, 1));
        assert_eq!(Timestamp::new(0, 1, 0), Timestamp::new(0, 0, 1_000));
        assert_eq!(Timestamp::new(1, 0, 0), Timestamp::new(0, 60, 0));
        assert_eq!(Timestamp::new(0, 0, 2_500), Timestamp::new(0, 2, 500));
    }

    #[test]
    fn display_round_trips() {
        let cases = ["00:02.960", "01:59.715", "02:07.075", "04:46.000"];
        for input in cases {
            let (value, tail) = Timestamp::take(input).unwrap().unwrap();
            assert_eq!(tail, "");
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
    fn shape_mismatch_returns_ok_none() {
        // Missing colon.
        assert!(matches!(Timestamp::take("0002.960"), Ok(None)));
        // Missing dot.
        assert!(matches!(Timestamp::take("00:02"), Ok(None)));
        // Fewer than three millisecond digits.
        assert!(matches!(Timestamp::take("00:02.96"), Ok(None)));
        // Empty input.
        assert!(matches!(Timestamp::take(""), Ok(None)));
        // Non-digit where a digit is required.
        assert!(matches!(Timestamp::take("ab:cd.efg"), Ok(None)));
    }

    #[test]
    fn rejects_seconds_out_of_range() {
        let Err(ParseTimestampError::SecondsOutOfRange { raw, value }) =
            Timestamp::take("00:60.000")
        else {
            panic!("expected SecondsOutOfRange");
        };
        assert_eq!(raw, "00:60.000");
        assert_eq!(value, 60);

        let Err(ParseTimestampError::SecondsOutOfRange { raw, value }) =
            Timestamp::take("00:99.000trailing")
        else {
            panic!("expected SecondsOutOfRange");
        };
        assert_eq!(raw, "00:99.000");
        assert_eq!(value, 99);
    }
}
