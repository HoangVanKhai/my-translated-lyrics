//! Parser for the tag lines that delimit a region of the source
//! format.
//!
//! A tag line stands alone at column zero and carries nothing besides
//! the tag itself. An opening tag is written `<name>` and a closing
//! tag is written `</name>`. The grammar here is deliberately generic:
//! [`Tag::take`] recognizes the shape and reports whichever name it
//! finds, leaving [`super`] to decide which names it knows and what
//! each one means. Today the only recognized name is `additive`.

use core::fmt;
use derive_more::Display;

/// Which side of a region a tag line names.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TagKind {
    /// An opening tag, written `<name>`.
    Opening,
    /// A closing tag, written `</name>`.
    Closing,
}

/// A tag line parsed out of the source, such as `<additive>` or
/// `</additive>`.
///
/// The [`fmt::Display`] implementation reproduces the tag exactly as
/// it was written, so a diagnostic can quote the source text without
/// the caller reassembling the delimiters by hand.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Tag<'a> {
    /// Whether the tag opens or closes a region.
    pub kind: TagKind,
    /// The name between the delimiters, `additive` in `<additive>`.
    pub name: &'a str,
}

impl<'a> Tag<'a> {
    /// Consumes a leading `<name>` or `</name>` from `source` and
    /// returns it together with the unconsumed tail.
    ///
    /// The name is whatever sits between the delimiters, taken
    /// verbatim. A name carrying a stray space or an unexpected
    /// character therefore parses successfully and reaches the caller
    /// as an unrecognized name, which quotes it back in full rather
    /// than reporting a shape the author did not write.
    pub fn take(source: &'a str) -> Result<(Self, &'a str), TakeTagError> {
        let after_delimiter = source
            .strip_prefix('<')
            .ok_or(TakeTagError::ShapeMismatch)?;
        let (kind, after_kind) = match after_delimiter.strip_prefix('/') {
            Some(rest) => (TagKind::Closing, rest),
            None => (TagKind::Opening, after_delimiter),
        };
        let (name, tail) = after_kind
            .split_once('>')
            .ok_or(TakeTagError::Unterminated)?;
        if name.is_empty() {
            return Err(TakeTagError::EmptyName);
        }
        Ok((Tag { kind, name }, tail))
    }
}

impl fmt::Display for Tag<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let slash = match self.kind {
            TagKind::Opening => "",
            TagKind::Closing => "/",
        };
        write!(formatter, "<{slash}{name}>", name = self.name)
    }
}

/// Failure modes of [`Tag::take`].
///
/// [`TakeTagError::ShapeMismatch`] reports that the text is not a tag
/// at all, which is how the caller learns to route the line to a
/// different parser. The remaining variants report text that opens a
/// tag and then fails to complete one, which is a hard error.
#[derive(Clone, Copy, Debug, Display, Eq, PartialEq)]
#[non_exhaustive]
pub enum TakeTagError {
    #[display("the text does not begin with `<`")]
    ShapeMismatch,
    #[display("the tag is not terminated by `>`")]
    Unterminated,
    #[display("the tag encloses no name")]
    EmptyName,
}

#[cfg(test)]
mod tests;
