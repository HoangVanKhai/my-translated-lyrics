use super::{Tag, TagKind, TakeTagError};
use pretty_assertions::assert_eq;

#[test]
fn takes_an_opening_tag() {
    assert_eq!(
        Tag::take("<additive>").unwrap(),
        (
            Tag {
                kind: TagKind::Opening,
                name: "additive",
            },
            "",
        ),
    );
}

#[test]
fn takes_a_closing_tag() {
    assert_eq!(
        Tag::take("</additive>").unwrap(),
        (
            Tag {
                kind: TagKind::Closing,
                name: "additive",
            },
            "",
        ),
    );
}

/// The parser consumes a prefix, so whatever follows the `>` comes
/// back untouched for the caller to interpret.
#[test]
fn returns_the_unconsumed_tail() {
    let (tag, tail) = Tag::take("<additive> trailing text").unwrap();
    assert_eq!(tag.name, "additive");
    assert_eq!(tail, " trailing text");
}

/// A line that does not open with `<` is not a tag, and the shape
/// mismatch tells the caller to try a different parser rather than
/// reporting an authoring error.
#[test]
fn reports_a_shape_mismatch_without_the_opening_delimiter() {
    assert_eq!(
        Tag::take("00:00.000 ttl: Hello").unwrap_err(),
        TakeTagError::ShapeMismatch,
    );
    assert_eq!(Tag::take("").unwrap_err(), TakeTagError::ShapeMismatch);
}

#[test]
fn rejects_a_tag_that_is_never_terminated() {
    assert_eq!(
        Tag::take("<additive").unwrap_err(),
        TakeTagError::Unterminated,
    );
    assert_eq!(
        Tag::take("</additive").unwrap_err(),
        TakeTagError::Unterminated,
    );
}

#[test]
fn rejects_a_tag_that_encloses_no_name() {
    assert_eq!(Tag::take("<>").unwrap_err(), TakeTagError::EmptyName);
    assert_eq!(Tag::take("</>").unwrap_err(), TakeTagError::EmptyName);
}

/// A name is taken verbatim rather than trimmed, so a stray space
/// stays part of the name and the caller can quote the exact text
/// back to the author.
#[test]
fn keeps_the_name_verbatim() {
    let (tag, _) = Tag::take("<additive >").unwrap();
    assert_eq!(tag.name, "additive ");
}

/// The `Display` implementation reproduces the source text, which is
/// what the diagnostics quote.
#[test]
fn displays_the_tag_as_it_was_written() {
    let (opening, _) = Tag::take("<additive>").unwrap();
    assert_eq!(opening.to_string(), "<additive>");
    let (closing, _) = Tag::take("</additive>").unwrap();
    assert_eq!(closing.to_string(), "</additive>");
}
