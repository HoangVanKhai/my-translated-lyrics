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
mod tests;
