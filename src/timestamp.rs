use core::fmt;
use core::str::FromStr;
use derive_more::{Display, Error, From, Into};

/// Milliseconds in one second. The inner `u64` of [`Timestamp`] and
/// the format widths of the `MM:SS.mmm` and `HH:MM:SS.mmm` shapes
/// are all expressed in multiples of this unit.
const MILLISECONDS_PER_SECOND: u64 = 1_000;
/// Milliseconds in one minute, derived from [`MILLISECONDS_PER_SECOND`]
/// so the relationship between the units is visible at the definition.
const MILLISECONDS_PER_MINUTE: u64 = 60 * MILLISECONDS_PER_SECOND;
/// Milliseconds in one hour, derived from [`MILLISECONDS_PER_MINUTE`]
/// for the same reason.
const MILLISECONDS_PER_HOUR: u64 = 60 * MILLISECONDS_PER_MINUTE;

/// Byte length of a rendered `MM:SS.mmm` timestamp. Two ASCII
/// digits, a colon, two ASCII digits, a dot, and three ASCII
/// digits add up to nine. Every cap-respecting [`Timestamp`]
/// renders to exactly this many bytes; the
/// [`rendered_length_matches_byte_length_constant`] test in
/// `tests.rs` and the `[..MM_SS_MMM_BYTE_LENGTH]` slices below
/// keep the constant honest.
pub const MM_SS_MMM_BYTE_LENGTH: usize = 9;

/// A point in time inside the video, measured as milliseconds from
/// `00:00.000`. Cues use it for start and end positions and for
/// ordering comparisons. The millisecond resolution is an internal
/// implementation detail; callers compose and destructure via the
/// minute / second / millisecond API surface.
///
/// The type carries an upper bound: every `Timestamp` represents a
/// point strictly earlier than `01:00:00.000`, i.e. less than one
/// hour from video start. The `MM:SS.mmm` source format does not
/// have an hour field, and no song in this repository is long
/// enough to need one; enforcing the bound at construction keeps
/// that invariant visible at every call site.
///
/// Renders through `Display` in the `MM:SS.mmm` source format. Error
/// messages that quote a timestamp use this implementation so the
/// output matches the form the source file used.
#[derive(Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[display(
    "{minutes:02}:{seconds:02}.{milliseconds:03}",
    minutes = _0 / MILLISECONDS_PER_MINUTE,
    seconds = (_0 % MILLISECONDS_PER_MINUTE) / MILLISECONDS_PER_SECOND,
    milliseconds = _0 % MILLISECONDS_PER_SECOND,
)]
pub struct Timestamp(u64);

impl Timestamp {
    /// Composes a `Timestamp` total from minutes, seconds, and
    /// milliseconds components. The result is
    /// `minutes * MILLISECONDS_PER_MINUTE + seconds * MILLISECONDS_PER_SECOND + milliseconds`,
    /// so this constructor doubles as a single-unit conversion:
    /// `Timestamp::new(n, 0, 0)` yields `n` minutes,
    /// `Timestamp::new(0, n, 0)` yields `n` seconds, and
    /// `Timestamp::new(0, 0, n)` yields `n` milliseconds.
    ///
    /// Individual components are not range-checked; the constructor
    /// only validates the composed total. `new(0, 120, 0)` is
    /// accepted and yields the same `Timestamp` as `new(2, 0, 0)`.
    /// The `None` return value is reserved for inputs whose total
    /// reaches or exceeds one hour, which the type invariant
    /// forbids. Callers that need the strict `MM < 60` /
    /// `SS < 60` / `mmm < 1_000` component ranges of the
    /// `MM:SS.mmm` source format must perform those checks before
    /// calling `new`; [`Timestamp::take`] does so.
    pub fn new(minutes: u64, seconds: u64, milliseconds: u64) -> Option<Self> {
        let total =
            minutes * MILLISECONDS_PER_MINUTE + seconds * MILLISECONDS_PER_SECOND + milliseconds;
        (total < MILLISECONDS_PER_HOUR).then_some(Timestamp(total))
    }

    /// Consumes a leading `MM:SS.mmm` prefix (9 ASCII characters)
    /// from `input` and returns the parsed `Timestamp` along with the
    /// unconsumed tail. Follows the parse-don't-validate pattern:
    ///
    /// - `Ok((ts, tail))` indicates the prefix matched the shape and
    ///   every component fits its range. `tail` is `input` past the
    ///   nine consumed characters, untouched.
    /// - `Err(TakeTimestampError::ShapeMismatch)` indicates the first
    ///   nine characters of `input` do not form an `MM:SS.mmm` shape
    ///   (too short, wrong punctuation, or a non-digit where a digit
    ///   is required). Callers typically treat this as "no timestamp
    ///   here" and route the line elsewhere.
    /// - `Err(TakeTimestampError::MinutesOutOfRange { … })` indicates
    ///   the prefix has timestamp shape but the minutes component
    ///   reaches or exceeds 60. `Timestamp` caps at one hour, so a
    ///   two-digit `MM` field of 60 or more is rejected rather than
    ///   rolled over.
    /// - `Err(TakeTimestampError::SecondsOutOfRange { … })` indicates
    ///   the prefix has timestamp shape but the seconds component
    ///   exceeds 59. Three-digit milliseconds can never exceed 999.
    ///   Both out-of-range errors carry a copy of the offending
    ///   9-character prefix for diagnostics.
    ///
    /// When both `MM` and `SS` fields are out of range, the
    /// minutes variant is reported because the one-hour cap is the
    /// tighter invariant; a seconds diagnostic on a prefix the
    /// type would reject anyway would be misleading.
    ///
    /// The caller is responsible for anything past the prefix: if
    /// the cue format requires whitespace between the timestamp and
    /// the body, the caller inspects `tail` for it.
    pub fn take(input: &str) -> Result<(Self, &str), TakeTimestampError> {
        // The closure body cannot become eager (`.then_some(next as
        // u8 - b'0')`) because `next as u8` truncates non-ASCII
        // chars to their low byte, and the subtraction would
        // overflow before `is_ascii_digit()` filters the value out.
        let digit = |next: char| next.is_ascii_digit().then(|| next as u8 - b'0');

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

        if minutes >= 60 {
            return Err(TakeTimestampError::MinutesOutOfRange(MinutesOutOfRange {
                raw: input[..MM_SS_MMM_BYTE_LENGTH].to_string(),
                value: minutes,
            }));
        }
        if seconds >= 60 {
            return Err(TakeTimestampError::SecondsOutOfRange(SecondsOutOfRange {
                raw: input[..MM_SS_MMM_BYTE_LENGTH].to_string(),
                value: seconds,
            }));
        }

        // The two guards above plus the three-digit ASCII parse
        // (`milliseconds <= 999`) bound the weighted total at
        // `59 * 60_000 + 59 * 1_000 + 999 = 3_599_999`, which is
        // strictly less than `MILLISECONDS_PER_HOUR`. Construct the
        // value directly through the private tuple constructor so
        // the `Option` from `Timestamp::new` does not enter the call
        // graph here; the alternative would be an `.expect` whose
        // safety argument lives in two places at once.
        let total =
            minutes * MILLISECONDS_PER_MINUTE + seconds * MILLISECONDS_PER_SECOND + milliseconds;
        Ok((Timestamp(total), chars.as_str()))
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
/// `HH:MM:SS,mmm` format. Construction and extraction go through
/// `From`/`Into`; the inner `Timestamp` is not exposed directly so
/// that every call site is a named conversion rather than a
/// positional tuple access.
#[derive(From, Into, Clone, Copy)]
pub struct SrtTime(Timestamp);

impl fmt::Display for SrtTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Timestamp(total) = (*self).into();
        write!(
            f,
            "{hours:02}:{minutes:02}:{seconds:02},{milliseconds:03}",
            hours = total / MILLISECONDS_PER_HOUR,
            minutes = (total % MILLISECONDS_PER_HOUR) / MILLISECONDS_PER_MINUTE,
            seconds = (total % MILLISECONDS_PER_MINUTE) / MILLISECONDS_PER_SECOND,
            milliseconds = total % MILLISECONDS_PER_SECOND,
        )
    }
}

/// Thin wrapper around [`Timestamp`] that renders in the WebVTT
/// `HH:MM:SS.mmm` format. See [`SrtTime`] for the same construction
/// and extraction story.
#[derive(From, Into, Clone, Copy)]
pub struct VttTime(Timestamp);

impl fmt::Display for VttTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Timestamp(total) = (*self).into();
        write!(
            f,
            "{hours:02}:{minutes:02}:{seconds:02}.{milliseconds:03}",
            hours = total / MILLISECONDS_PER_HOUR,
            minutes = (total % MILLISECONDS_PER_HOUR) / MILLISECONDS_PER_MINUTE,
            seconds = (total % MILLISECONDS_PER_MINUTE) / MILLISECONDS_PER_SECOND,
            milliseconds = total % MILLISECONDS_PER_SECOND,
        )
    }
}

/// Payload for a minutes-out-of-range error. Describes an
/// `MM:SS.mmm` prefix whose minutes component reaches or exceeds 60.
#[derive(Debug, Display, Clone, PartialEq, Eq)]
#[display("invalid timestamp {raw:?}: minutes component {value} must be less than 60")]
pub struct MinutesOutOfRange {
    pub raw: String,
    pub value: u64,
}

/// Payload for a seconds-out-of-range error. Describes an
/// `MM:SS.mmm` prefix whose seconds component exceeds 59.
#[derive(Debug, Display, Clone, PartialEq, Eq)]
#[display("invalid timestamp {raw:?}: seconds component {value} must be less than 60")]
pub struct SecondsOutOfRange {
    pub raw: String,
    pub value: u64,
}

/// Reasons [`Timestamp::from_str`] can fail.
///
/// `FromStr` requires the entire input to denote a single
/// `MM:SS.mmm` timestamp. The parser reuses [`Timestamp::take`] and
/// then rejects any remaining input, so the error surface is the
/// union of [`TakeTimestampError`] with an extra
/// [`UnexpectedCharacter`](Self::UnexpectedCharacter) variant for
/// inputs that started well but carry content past the nine
/// consumed characters.
#[derive(Debug, Display, Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ParseTimestampError {
    /// The input does not begin with an `MM:SS.mmm` shape.
    #[display("input does not begin with an MM:SS.mmm timestamp")]
    ShapeMismatch,
    /// The input begins with an `MM:SS.mmm` shape but the minutes
    /// component is out of range (reaches or exceeds 60).
    #[display("{_0}")]
    MinutesOutOfRange(#[error(not(source))] MinutesOutOfRange),
    /// The input begins with an `MM:SS.mmm` shape but the seconds
    /// component is out of range.
    #[display("{_0}")]
    SecondsOutOfRange(#[error(not(source))] SecondsOutOfRange),
    /// The input begins with a valid `MM:SS.mmm` prefix but has
    /// an unexpected character where end of input was required.
    #[display(
        "unexpected character {_0:?} after the `MM:SS.mmm` prefix; `FromStr` requires end of input there"
    )]
    UnexpectedCharacter(#[error(not(source))] char),
}

impl FromStr for Timestamp {
    type Err = ParseTimestampError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match Timestamp::take(s) {
            Ok((timestamp, trailing)) => match trailing.chars().next() {
                None => Ok(timestamp),
                Some(character) => Err(ParseTimestampError::UnexpectedCharacter(character)),
            },
            Err(TakeTimestampError::ShapeMismatch) => Err(ParseTimestampError::ShapeMismatch),
            Err(TakeTimestampError::MinutesOutOfRange(inner)) => {
                Err(ParseTimestampError::MinutesOutOfRange(inner))
            }
            Err(TakeTimestampError::SecondsOutOfRange(inner)) => {
                Err(ParseTimestampError::SecondsOutOfRange(inner))
            }
        }
    }
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
    /// The input begins with an `MM:SS.mmm` shape but the minutes
    /// component reaches or exceeds 60, breaking the one-hour cap.
    #[display("{_0}")]
    MinutesOutOfRange(#[error(not(source))] MinutesOutOfRange),
    /// The input begins with an `MM:SS.mmm` shape but the seconds
    /// component is out of range.
    #[display("{_0}")]
    SecondsOutOfRange(#[error(not(source))] SecondsOutOfRange),
}

#[cfg(test)]
mod tests;
