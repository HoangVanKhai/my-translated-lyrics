use super::{SecondsOutOfRange, SrtTime, TakeTimestampError, Timestamp, VttTime};

#[test]
fn takes_basic_timestamp_with_tail() {
    let (ts, tail) = Timestamp::take("00:02.960 hello").unwrap();
    assert_eq!(ts, Timestamp::new(0, 2, 960));
    assert_eq!(tail, " hello");
}

#[test]
fn takes_timestamp_at_exact_length() {
    let (ts, tail) = Timestamp::take("01:59.715").unwrap();
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
        let (value, tail) = Timestamp::take(input).unwrap();
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
fn shape_mismatch_rejects_non_ascii_unicode_digits() {
    // `char::to_digit(10)` would accept these as `0`, but the
    // `MM:SS.mmm` source format is ASCII-only, and slicing the
    // input by byte index would panic mid-character.
    assert!(matches!(
        Timestamp::take("００:00.000"),
        Err(TakeTimestampError::ShapeMismatch),
    ));
    assert!(matches!(
        Timestamp::take("٠٠:00.000"),
        Err(TakeTimestampError::ShapeMismatch),
    ));
}

#[test]
fn shape_mismatch_reports_error() {
    // Missing colon.
    assert!(matches!(
        Timestamp::take("0002.960"),
        Err(TakeTimestampError::ShapeMismatch),
    ));
    // Missing dot.
    assert!(matches!(
        Timestamp::take("00:02"),
        Err(TakeTimestampError::ShapeMismatch),
    ));
    // Fewer than three millisecond digits.
    assert!(matches!(
        Timestamp::take("00:02.96"),
        Err(TakeTimestampError::ShapeMismatch),
    ));
    // Empty input.
    assert!(matches!(
        Timestamp::take(""),
        Err(TakeTimestampError::ShapeMismatch),
    ));
    // Non-digit where a digit is required.
    assert!(matches!(
        Timestamp::take("ab:cd.efg"),
        Err(TakeTimestampError::ShapeMismatch),
    ));
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
