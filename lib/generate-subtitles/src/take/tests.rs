use super::{take_leading_whitespace, take_non_whitespace};
use pretty_assertions::assert_eq;

/// Both halves may be empty, so the parser always succeeds and
/// the caller decides what an empty run means.
#[test]
fn leading_whitespace_splits_the_run_from_the_tail() {
    for (input, expected) in [
        ("  rest of it", ("  ", "rest of it")),
        ("\t\n rest", ("\t\n ", "rest")),
        ("rest", ("", "rest")),
        ("   ", ("   ", "")),
        ("", ("", "")),
    ] {
        eprintln!("CASE: {input:?}");
        assert_eq!(take_leading_whitespace(input), expected);
    }
}

#[test]
fn non_whitespace_splits_the_token_from_the_tail() {
    for (input, expected) in [
        ("clr rest of it", ("clr", " rest of it")),
        ("clr\ttail", ("clr", "\ttail")),
        ("clr", ("clr", "")),
        (" leading", ("", " leading")),
        ("", ("", "")),
    ] {
        eprintln!("CASE: {input:?}");
        assert_eq!(take_non_whitespace(input), expected);
    }
}

/// The split lands on a character boundary, so a token that ends
/// in a multi-byte character survives it intact.
#[test]
fn the_split_respects_character_boundaries() {
    assert_eq!(take_non_whitespace("役割： 名前"), ("役割：", " 名前"));
    assert_eq!(
        take_leading_whitespace("\u{3000}名前"),
        ("\u{3000}", "名前"),
    );
}
