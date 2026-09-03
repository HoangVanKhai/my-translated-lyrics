//! Prefix parsers shared by this crate's text parsers.

/// Consumes the leading whitespace of `input`.
pub(crate) fn take_leading_whitespace(input: &str) -> (&str, &str) {
    split_at_first(input, |char| !char.is_whitespace())
}

/// Consumes the leading run of non-whitespace characters of `input`,
/// which is the first whitespace-delimited token when `input` carries
/// no leading whitespace.
pub(crate) fn take_non_whitespace(input: &str) -> (&str, &str) {
    split_at_first(input, char::is_whitespace)
}

/// Splits `input` at the first character that satisfies `boundary`,
/// or at the end of input when none does.
fn split_at_first(input: &str, boundary: impl Fn(char) -> bool) -> (&str, &str) {
    let cursor = input
        .char_indices()
        .find(|&(_, char)| boundary(char))
        .map_or(input.len(), |(offset, _)| offset);
    input.split_at(cursor)
}

#[cfg(test)]
mod tests {
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
}
