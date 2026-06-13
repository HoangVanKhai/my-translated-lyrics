use core::fmt;
use core::str::FromStr;
use derive_more::{Display, From, Into};

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
/// `rendered_length_matches_timestamp_str_len` test in
/// `tests.rs` and the `[..TIMESTAMP_STR_LEN]` slices below
/// keep the constant honest.
pub const TIMESTAMP_STR_LEN: usize = 9;

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
#[derive(Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
    /// `None` is returned for two distinct reasons: when the
    /// weighted total reaches or exceeds one hour (the cap
    /// invariant), and when any intermediate `u64` multiplication
    /// or addition would overflow. The latter would otherwise
    /// silently wrap in release builds and the wrapped value
    /// could happen to land below the cap, so the constructor
    /// uses checked arithmetic and propagates the overflow as
    /// `None` rather than risking a stray valid `Timestamp` from
    /// nonsense input. Callers that need the strict `MM < 60` /
    /// `SS < 60` / `mmm < 1_000` component ranges of the
    /// `MM:SS.mmm` source format must perform those checks before
    /// calling `new`; [`Timestamp::take`] does so.
    pub fn new(minutes: u64, seconds: u64, milliseconds: u64) -> Option<Self> {
        let total = minutes
            .checked_mul(MILLISECONDS_PER_MINUTE)?
            .checked_add(seconds.checked_mul(MILLISECONDS_PER_SECOND)?)?
            .checked_add(milliseconds)?;
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
    /// - `Err(TakeTimestampError::MinutesOutOfRange { ... })` indicates
    ///   the prefix has timestamp shape but the minutes component
    ///   reaches or exceeds 60. `Timestamp` caps at one hour, so a
    ///   two-digit `MM` field of 60 or more is rejected rather than
    ///   rolled over.
    /// - `Err(TakeTimestampError::SecondsOutOfRange { ... })` indicates
    ///   the prefix has timestamp shape but the seconds component
    ///   exceeds 59. Three-digit milliseconds can never exceed 999.
    ///   Both out-of-range errors carry a copy of the offending
    ///   9-character prefix for diagnostics.
    ///
    /// When both `MM` and `SS` fields are out of range, the
    /// seconds variant is reported. The seconds guard fires before
    /// the cap check inside [`Timestamp::new`], so the more local
    /// component-range diagnostic wins over the cap-derived
    /// minutes diagnostic.
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

        if seconds >= 60 {
            return Err(TakeTimestampError::SecondsOutOfRange(SecondsOutOfRange {
                raw: input[..TIMESTAMP_STR_LEN].to_string(),
                value: seconds,
            }));
        }

        let timestamp = Timestamp::new(minutes, seconds, milliseconds).ok_or_else(|| {
            // With `seconds < 60` (filtered by the guard above) and
            // the 2/3-digit ASCII parses capping all three
            // components, the `checked_*` paths inside `new` cannot
            // overflow on any input that reaches this line. The
            // only remaining way for `new` to return `None` is the
            // cap check (`total < MILLISECONDS_PER_HOUR`), which
            // can only fire when `minutes >= 60`.
            TakeTimestampError::MinutesOutOfRange(MinutesOutOfRange {
                raw: input[..TIMESTAMP_STR_LEN].to_string(),
                value: minutes,
            })
        })?;

        Ok((timestamp, chars.as_str()))
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
#[derive(Display, Clone, Copy, From, Into)]
#[display(
    "{hours:02}:{minutes:02}:{seconds:02},{milliseconds:03}",
    hours = _0.0 / MILLISECONDS_PER_HOUR,
    minutes = (_0.0 % MILLISECONDS_PER_HOUR) / MILLISECONDS_PER_MINUTE,
    seconds = (_0.0 % MILLISECONDS_PER_MINUTE) / MILLISECONDS_PER_SECOND,
    milliseconds = _0.0 % MILLISECONDS_PER_SECOND,
)]
pub struct SrtTime(Timestamp);

/// Thin wrapper around [`Timestamp`] that renders in the WebVTT
/// `HH:MM:SS.mmm` format. See [`SrtTime`] for the same construction
/// and extraction story.
#[derive(Display, Clone, Copy, From, Into)]
#[display(
    "{hours:02}:{minutes:02}:{seconds:02}.{milliseconds:03}",
    hours = _0.0 / MILLISECONDS_PER_HOUR,
    minutes = (_0.0 % MILLISECONDS_PER_HOUR) / MILLISECONDS_PER_MINUTE,
    seconds = (_0.0 % MILLISECONDS_PER_MINUTE) / MILLISECONDS_PER_SECOND,
    milliseconds = _0.0 % MILLISECONDS_PER_SECOND,
)]
pub struct VttTime(Timestamp);

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
#[derive(Debug, Display, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ParseTimestampError {
    /// The input does not begin with an `MM:SS.mmm` shape.
    #[display("input does not begin with an MM:SS.mmm timestamp")]
    ShapeMismatch,
    /// The input begins with an `MM:SS.mmm` shape but the minutes
    /// component is out of range (reaches or exceeds 60).
    MinutesOutOfRange(MinutesOutOfRange),
    /// The input begins with an `MM:SS.mmm` shape but the seconds
    /// component is out of range.
    SecondsOutOfRange(SecondsOutOfRange),
    /// The input begins with a valid `MM:SS.mmm` prefix but has
    /// an unexpected character where end of input was required.
    #[display(
        "unexpected character {_0:?} after the `MM:SS.mmm` prefix; `FromStr` requires end of input there"
    )]
    UnexpectedCharacter(char),
}

impl FromStr for Timestamp {
    type Err = ParseTimestampError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match Timestamp::take(input) {
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
#[derive(Debug, Display, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum TakeTimestampError {
    /// The input does not begin with an `MM:SS.mmm` shape.
    #[display("input does not begin with an MM:SS.mmm timestamp")]
    ShapeMismatch,
    /// The input begins with an `MM:SS.mmm` shape but the minutes
    /// component reaches or exceeds 60, breaking the one-hour cap.
    MinutesOutOfRange(MinutesOutOfRange),
    /// The input begins with an `MM:SS.mmm` shape but the seconds
    /// component is out of range.
    SecondsOutOfRange(SecondsOutOfRange),
}

#[cfg(test)]
mod tests;
