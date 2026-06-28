use super::{
    MinutesOutOfRange, ParseTimestampError, SecondsOutOfRange, SrtTime, TIMESTAMP_STR_LEN,
    TakeTimestampError, Timestamp, VttTime,
};
use pretty_assertions::assert_eq;

#[test]
fn takes_basic_timestamp_with_tail() {
    let (ts, tail) = Timestamp::take("00:02.960 hello").unwrap();
    assert_eq!(ts, Timestamp::new(0, 2, 960).unwrap());
    assert_eq!(tail, " hello");
}

#[test]
fn takes_timestamp_at_exact_length() {
    let (ts, tail) = Timestamp::take("01:59.715").unwrap();
    assert_eq!(ts, Timestamp::new(1, 59, 715).unwrap());
    assert_eq!(tail, "");
}

#[test]
fn new_composes_weighted_components() {
    assert_eq!(
        Timestamp::new(0, 0, 1).unwrap(),
        Timestamp::new(0, 0, 1).unwrap(),
    );
    assert_eq!(
        Timestamp::new(0, 1, 0).unwrap(),
        Timestamp::new(0, 0, 1_000).unwrap(),
    );
    assert_eq!(
        Timestamp::new(1, 0, 0).unwrap(),
        Timestamp::new(0, 60, 0).unwrap(),
    );
    assert_eq!(
        Timestamp::new(0, 0, 2_500).unwrap(),
        Timestamp::new(0, 2, 500).unwrap(),
    );
}

#[test]
fn display_round_trips() {
    let cases = ["00:02.960", "01:59.715", "02:07.075", "04:46.000"];
    for input in cases {
        let (value, tail) = Timestamp::take(input).unwrap();
        assert_eq!(tail, "");
        assert_eq!(value.to_string(), input);
    }
}

#[test]
fn srt_time_uses_comma() {
    assert_eq!(
        SrtTime::from(Timestamp::new(0, 2, 960).unwrap()).to_string(),
        "00:00:02,960",
    );
}

#[test]
fn vtt_time_uses_dot() {
    assert_eq!(
        VttTime::from(Timestamp::new(0, 2, 960).unwrap()).to_string(),
        "00:00:02.960",
    );
}

/// The display impl for [`Timestamp`] formats two ASCII digits, a
/// colon, two ASCII digits, a dot, and three ASCII digits, so
/// every cap-respecting value renders to exactly
/// [`TIMESTAMP_STR_LEN`] bytes. Lock that invariant at both
/// ends of the legal range plus one mid-range value, so a future
/// tweak to the format string trips here before any caller that
/// slices on the constant produces silent UTF-8 panics.
#[test]
fn rendered_length_matches_timestamp_str_len() {
    for ts in [
        Timestamp::new(0, 0, 0).unwrap(),
        Timestamp::new(0, 0, 1).unwrap(),
        Timestamp::new(12, 34, 567).unwrap(),
        Timestamp::new(59, 59, 999).unwrap(),
    ] {
        assert_eq!(ts.to_string().len(), TIMESTAMP_STR_LEN);
    }
}

/// [`Timestamp`] is capped at one hour, so the largest value the
/// constructor accepts is 59:59.999. The SRT and VTT wrappers
/// still emit `HH:MM:SS`, but the hour field at that value is
/// always `00`.
#[test]
fn maximum_representable_value_renders_just_below_one_hour() {
    let value = Timestamp::new(59, 59, 999).unwrap();
    assert_eq!(value.to_string(), "59:59.999");
    assert_eq!(SrtTime::from(value).to_string(), "00:59:59,999");
    assert_eq!(VttTime::from(value).to_string(), "00:59:59.999");
}

/// `Timestamp::new` accepts arbitrary `u64` components, so the
/// weighted total can overflow `u64` before the cap check has a
/// chance to look at it. In release builds an unchecked
/// multiplication would wrap silently and the wrapped value
/// could happen to land below `MILLISECONDS_PER_HOUR`, leaking
/// a stray `Some(Timestamp(_))` from nonsense input. The
/// `checked_mul` / `checked_add` chain forwards every overflow
/// as `None` instead.
#[test]
fn new_rejects_arithmetic_overflow_before_the_cap_check() {
    assert_eq!(Timestamp::new(u64::MAX, 0, 0), None);
    assert_eq!(Timestamp::new(0, u64::MAX, 0), None);
    assert_eq!(Timestamp::new(u64::MAX, u64::MAX, u64::MAX), None);
    // The largest `minutes` that can multiply by
    // `MILLISECONDS_PER_MINUTE` without overflowing is
    // `u64::MAX / MILLISECONDS_PER_MINUTE`. Adding any non-zero
    // milliseconds to that product overflows the subsequent add.
    let max_safe_minutes = u64::MAX / 60_000;
    assert_eq!(Timestamp::new(max_safe_minutes, 0, u64::MAX), None);
}

/// Every composition whose weighted total lands at exactly one
/// hour or beyond must be rejected, regardless of which component
/// pushed the total past the cap. The just-below-cap composition
/// (59:59.999, one millisecond short of one hour) is the
/// symmetric "still `Some`" boundary that locks the cap at its
/// exact threshold.
#[test]
fn new_rejects_totals_that_reach_or_exceed_one_hour() {
    assert!(Timestamp::new(59, 59, 999).is_some());
    assert_eq!(Timestamp::new(60, 0, 0), None);
    assert_eq!(Timestamp::new(59, 60, 0), None);
    assert_eq!(Timestamp::new(59, 59, 1_000), None);
    assert_eq!(Timestamp::new(120, 0, 0), None);
}

#[test]
fn shape_mismatch_rejects_non_ascii_unicode_digits() {
    assert_eq!(
        Timestamp::take("００:00.000").unwrap_err(),
        TakeTimestampError::ShapeMismatch,
    );
    assert_eq!(
        Timestamp::take("٠٠:00.000").unwrap_err(),
        TakeTimestampError::ShapeMismatch,
    );
}

#[test]
fn shape_mismatch_reports_error() {
    // Missing colon.
    assert_eq!(
        Timestamp::take("0002.960").unwrap_err(),
        TakeTimestampError::ShapeMismatch,
    );
    // Missing dot.
    assert_eq!(
        Timestamp::take("00:02").unwrap_err(),
        TakeTimestampError::ShapeMismatch,
    );
    // Fewer than three millisecond digits.
    assert_eq!(
        Timestamp::take("00:02.96").unwrap_err(),
        TakeTimestampError::ShapeMismatch,
    );
    // Empty input.
    assert_eq!(
        Timestamp::take("").unwrap_err(),
        TakeTimestampError::ShapeMismatch,
    );
    // Non-digit where a digit is required.
    assert_eq!(
        Timestamp::take("ab:cd.efg").unwrap_err(),
        TakeTimestampError::ShapeMismatch,
    );
}

#[test]
fn rejects_seconds_out_of_range() {
    assert_eq!(
        Timestamp::take("00:60.000").unwrap_err(),
        TakeTimestampError::SecondsOutOfRange(SecondsOutOfRange {
            raw: "00:60.000".to_string(),
            value: 60,
        }),
    );
    assert_eq!(
        Timestamp::take("00:99.000trailing").unwrap_err(),
        TakeTimestampError::SecondsOutOfRange(SecondsOutOfRange {
            raw: "00:99.000".to_string(),
            value: 99,
        }),
    );
}

#[test]
fn rejects_minutes_out_of_range() {
    assert_eq!(
        Timestamp::take("60:00.000").unwrap_err(),
        TakeTimestampError::MinutesOutOfRange(MinutesOutOfRange {
            raw: "60:00.000".to_string(),
            value: 60,
        }),
    );
    assert_eq!(
        Timestamp::take("99:59.999trailing").unwrap_err(),
        TakeTimestampError::MinutesOutOfRange(MinutesOutOfRange {
            raw: "99:59.999".to_string(),
            value: 99,
        }),
    );
}

/// When both components are out of range, the seconds diagnostic
/// fires first: `Timestamp::take` runs an explicit
/// `seconds >= 60` guard before delegating the cap check to
/// `Timestamp::new`, so the more local component-range error
/// wins over the cap-derived minutes error.
///
/// This test is load-bearing for the precedence claim in
/// `Timestamp::take`'s doc comment: a future change that moves
/// the explicit guard, or that swaps the guard for a different
/// shape, would silently flip which diagnostic fires for
/// `60:60.000` and the doc would no longer match the behavior.
/// Keep this test in lockstep with that paragraph.
#[test]
fn seconds_out_of_range_takes_precedence_over_minutes_out_of_range() {
    assert_eq!(
        Timestamp::take("60:60.000").unwrap_err(),
        TakeTimestampError::SecondsOutOfRange(SecondsOutOfRange {
            raw: "60:60.000".to_string(),
            value: 60,
        }),
    );
}

#[test]
fn from_str_accepts_exact_mm_ss_mmm_shape() {
    let parsed: Timestamp = "01:23.456".parse().unwrap();
    assert_eq!(parsed, Timestamp::new(1, 23, 456).unwrap());
}

#[test]
fn from_str_rejects_shape_mismatch() {
    assert_eq!(
        "not-a-timestamp".parse::<Timestamp>().unwrap_err(),
        ParseTimestampError::ShapeMismatch,
    );
}

#[test]
fn from_str_rejects_seconds_out_of_range() {
    assert_eq!(
        "00:60.000".parse::<Timestamp>().unwrap_err(),
        ParseTimestampError::SecondsOutOfRange(SecondsOutOfRange {
            raw: "00:60.000".to_string(),
            value: 60,
        }),
    );
}

#[test]
fn from_str_rejects_minutes_out_of_range() {
    assert_eq!(
        "60:00.000".parse::<Timestamp>().unwrap_err(),
        ParseTimestampError::MinutesOutOfRange(MinutesOutOfRange {
            raw: "60:00.000".to_string(),
            value: 60,
        }),
    );
}

#[test]
fn from_str_rejects_unexpected_character_after_prefix() {
    assert_eq!(
        "00:02.960 tail".parse::<Timestamp>().unwrap_err(),
        ParseTimestampError::UnexpectedCharacter(' '),
    );
}
