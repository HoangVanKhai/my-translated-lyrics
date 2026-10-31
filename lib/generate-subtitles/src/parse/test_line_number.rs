//! Tests for the line number the parser locates every diagnostic by.

use crate::parse::LineNumber;
use pretty_assertions::assert_eq;

/// The first line of a file is line one, so a zero-based index and the line
/// number naming it are one apart. The conversion lives in one place, which
/// is what keeps a zero-based position out of a diagnostic.
#[test]
fn an_index_names_the_line_after_it() {
    assert_eq!(LineNumber::from_index(0), LineNumber::new(1));
    assert_eq!(LineNumber::from_index(41), LineNumber::new(42));
}

/// A line number renders as the bare number, so a diagnostic written as
/// `line {line_number}:` prints the text it always has.
#[test]
fn a_line_number_renders_as_its_number() {
    assert_eq!(LineNumber::new(7).to_string(), "7");
    assert_eq!(format!("line {}:", LineNumber::from_index(0)), "line 1:");
}
