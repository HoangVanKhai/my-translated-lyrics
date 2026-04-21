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
    /// - `Ok((ts, tail))` — the prefix matched the shape and every
    ///   component fits its range. `tail` is `input` past the nine
    ///   consumed characters, untouched.
    /// - `Err(TakeTimestampError::ShapeMismatch)` — the first nine
    ///   characters of `input` do not form an `MM:SS.mmm` shape (too
    ///   short, wrong punctuation, or a non-digit where a digit is
    ///   required). Callers typically treat this as "no timestamp
    ///   here" and route the line elsewhere.
    /// - `Err(TakeTimestampError::SecondsOutOfRange { … })` — the
    ///   prefix has timestamp shape but the seconds component
    ///   exceeds 59. Three-digit milliseconds can never exceed 999,
    ///   and two-digit minutes are uncapped by design. The error
    ///   carries a copy of the offending 9-character prefix for
    ///   diagnostics.
    ///
    /// The caller is responsible for anything past the prefix: if
    /// the cue format requires whitespace between the timestamp and
    /// the body, the caller inspects `tail` for it.
    pub fn take(input: &str) -> Result<(Self, &str), TakeTimestampError> {
        let digit = |next: char| next.to_digit(10).map(|value| value as u8);

        let mut chars = input.chars();

        let tens = chars
            .next()
            .and_then(digit)
            .ok_or(TakeTimestampError::ShapeMismatch)?;
        let ones = chars
            .next()
            .and_then(digit)
            .ok_or(TakeTimestampError::ShapeMismatch)?;
        let minutes = u64::from(tens) * 10 + u64::from(ones);

        chars
            .next()
            .filter(|&c| c == ':')
            .ok_or(TakeTimestampError::ShapeMismatch)?;

        let tens = chars
            .next()
            .and_then(digit)
            .ok_or(TakeTimestampError::ShapeMismatch)?;
        let ones = chars
            .next()
            .and_then(digit)
            .ok_or(TakeTimestampError::ShapeMismatch)?;
        let seconds = u64::from(tens) * 10 + u64::from(ones);

        chars
            .next()
            .filter(|&c| c == '.')
            .ok_or(TakeTimestampError::ShapeMismatch)?;

        let hundreds = chars
            .next()
            .and_then(digit)
            .ok_or(TakeTimestampError::ShapeMismatch)?;
        let tens = chars
            .next()
            .and_then(digit)
            .ok_or(TakeTimestampError::ShapeMismatch)?;
        let ones = chars
            .next()
            .and_then(digit)
            .ok_or(TakeTimestampError::ShapeMismatch)?;
        let milliseconds = u64::from(hundreds) * 100 + u64::from(tens) * 10 + u64::from(ones);

        if seconds >= 60 {
            return Err(TakeTimestampError::SecondsOutOfRange(SecondsOutOfRange {
                raw: input[..9].to_string(),
                value: seconds,
            }));
        }

        Ok((
            Timestamp::new(minutes, seconds, milliseconds),
            chars.as_str(),
        ))
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

/// Payload for a seconds-out-of-range error. Describes an
/// `MM:SS.mmm` prefix whose seconds component exceeds 59.
#[derive(Debug, Display, Clone, PartialEq, Eq)]
#[display("invalid timestamp {raw:?}: seconds component {value} must be less than 60")]
pub struct SecondsOutOfRange {
    pub raw: String,
    pub value: u64,
}

#[derive(Debug, Display, Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ParseTimestampError {
    #[display("{_0}")]
    SecondsOutOfRange(#[error(not(source))] SecondsOutOfRange),
}

/// Reasons [`Timestamp::take`] can fail.
///
/// Kept distinct from [`ParseTimestampError`] so that each type can
/// accumulate its own variants over time: [`ParseTimestampError`]
/// describes ways an `MM:SS.mmm` string fails to denote a valid
/// timestamp value (used by any future `FromStr`-style parser),
/// while [`TakeTimestampError`] describes ways the `take` combinator
/// fails to produce a `Timestamp` from the start of its input.
#[derive(Debug, Display, Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum TakeTimestampError {
    /// The input does not begin with an `MM:SS.mmm` shape.
    #[display("input does not begin with an MM:SS.mmm timestamp")]
    ShapeMismatch,
    /// The input begins with an `MM:SS.mmm` shape but the seconds
    /// component is out of range.
    #[display("{_0}")]
    SecondsOutOfRange(#[error(not(source))] SecondsOutOfRange),
}

#[cfg(test)]
mod tests;
