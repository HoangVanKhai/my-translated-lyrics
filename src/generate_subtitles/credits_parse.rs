//! Role-driven credit-line parser.
//!
//! The parser takes the [`CreditRoles`] for one language from the
//! song's `credits.yaml` and walks the line left to right, matching
//! the longest registered role at every cursor position. The bytes
//! between a role match and the next role match (or end of line)
//! form the associated name region, and name regions are scanned
//! for bracketed spans that become [`NameSegment::Bracketed`]
//! values; anything else becomes [`NameSegment::Unbracketed`].
//!
//! A credit line whose first non-whitespace token is not a known
//! role raises [`ParseCreditError::UnknownRole`]. This lets the
//! integration tests catch typos such as `作詞` vs `作词` before
//! they ever reach `dist/`.
//!
//! Only the `credit-roles` list is consumed by this parser. The
//! `credit-names` list on [`CreditsDesc`] is loaded and carried
//! through the pipeline but is not cross-checked against the parsed
//! name regions here; see the "`credits.yaml` consistency test"
//! item in the PR description for the deferred work that would add
//! that validation.

use crate::credits_descriptor::CreditsDesc;
use crate::video_descriptor::Language;
use derive_more::Display;
use into_deduped::IntoDeduped;
use into_sorted::IntoSorted;
use pipe_trait::Pipe;

/// A structural role/name pair extracted from one credit line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreditPair<'a> {
    /// The role cell, exactly as it appeared in the source line.
    pub role: &'a str,
    /// Raw separator text captured between the role cell and the
    /// name cell, preserved verbatim for the renderer to decide how
    /// to emit it: ASCII space/tab runs survive unchanged (so a
    /// multi-space gutter round-trips), while other shapes such as
    /// `：` or `\u{3000}` collapse to a single ASCII space.
    pub separator: &'a str,
    /// Decomposed name region, with bracketed and unbracketed
    /// segments.
    pub name_segments: Vec<NameSegment<'a>>,
}

/// A unit within the name region of a credit pair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NameSegment<'a> {
    /// A run of text that contains no parseable bracketed span.
    Unbracketed(Unbracketed<'a>),
    /// A bracketed span (`【...】`, `[...]`, or `(...)`), with the
    /// surrounding brackets included in the wrapped slice.
    Bracketed(Bracketed<'a>),
}

/// A string that contains no parseable bracketed span.
#[derive(Debug, Display, Clone, Copy, PartialEq, Eq)]
pub struct Unbracketed<'a>(&'a str);

impl<'a> Unbracketed<'a> {
    /// The unbracketed text, exactly as it appeared in the source.
    pub fn as_str(&self) -> &'a str {
        self.0
    }
}

/// An item of the parsing process of [`NameSegment`]s.
///
/// This struct has no semantic significance, it is merely a
/// consequence of the structure of the credit-line: The
/// [unbracketed] segments and the [bracketed] segments are
/// interleaving. So this struct is a pair before the final
/// unpaired [unbracketed] segment.
///
/// [unbracketed]: Unbracketed
/// [bracketed]: Bracketed
#[derive(Debug, Clone, Copy)]
struct NameSegmentPair<'a>(Unbracketed<'a>, Bracketed<'a>);

impl<'a> NameSegmentPair<'a> {
    fn take(input: &'a str) -> Option<(Self, &'a str)> {
        let mut unbracketed_end: usize = 0;
        let mut chars = input.chars();
        loop {
            if let Some((bracketed, rest)) = Bracketed::take(chars.as_str()) {
                let unbracketed = Unbracketed(&input[..unbracketed_end]);
                return Some((NameSegmentPair(unbracketed, bracketed), rest));
            }
            if let Some(char) = chars.next() {
                unbracketed_end += char.len_utf8();
                continue;
            }
            return None;
        }
    }

    fn append_to(&self, target: &mut Vec<NameSegment<'a>>) {
        let NameSegmentPair(unbracketed, bracketed) = self;
        if !unbracketed.as_str().is_empty() {
            target.push(NameSegment::Unbracketed(*unbracketed));
        }
        target.push(NameSegment::Bracketed(*bracketed));
    }
}

/// A string that is guaranteed to open with a recognized bracket,
/// close with its matching counterpart, and contain no further
/// bracket characters in between.
#[derive(Debug, Display, Clone, Copy, PartialEq, Eq)]
pub struct Bracketed<'a>(&'a str);

impl<'a> Bracketed<'a> {
    /// Consumes a leading bracketed span. The three recognized
    /// pairs are `【...】`, `[...]`, and `(...)`. The bytes between
    /// the opening and closing characters must contain none of
    /// those six characters; if another bracket is encountered
    /// before the matching close, no value is produced and the
    /// caller is free to re-interpret the input as ordinary text.
    pub fn take(input: &'a str) -> Option<(Self, &'a str)> {
        let mut chars = input.char_indices();
        let (_, open) = chars.next()?;
        let close = matching_close(open)?;
        for (offset, ch) in chars {
            if ch == close {
                let end = offset + ch.len_utf8();
                return Some((Bracketed(&input[..end]), &input[end..]));
            }
            if is_bracket_char(ch) {
                return None;
            }
        }
        None
    }

    /// The bracketed text, including the surrounding brackets.
    pub fn as_str(&self) -> &'a str {
        self.0
    }
}

/// Reasons [`Bracketed::try_from`] can fail.
#[derive(Debug, Display, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ParseBracketedError {
    /// The input does not form a valid bracketed span: it is
    /// empty, does not begin with a recognized opening bracket,
    /// contains another bracket before the matching close, or
    /// ends before the closing bracket.
    #[display("input is not a valid bracketed span")]
    ShapeMismatch,
    /// The input begins with a valid bracketed span but carries
    /// an unexpected character where end of input was required.
    #[display(
        "unexpected character {_0:?} after the bracketed span; `TryFrom` requires end of input there"
    )]
    UnexpectedCharacter(char),
}

impl<'a> TryFrom<&'a str> for Bracketed<'a> {
    type Error = ParseBracketedError;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        let (value, trailing) = value
            .pipe(Bracketed::take)
            .ok_or(ParseBracketedError::ShapeMismatch)?;
        match trailing.chars().next() {
            None => Ok(value),
            Some(character) => Err(ParseBracketedError::UnexpectedCharacter(character)),
        }
    }
}

fn matching_close(open: char) -> Option<char> {
    match open {
        '【' => Some('】'),
        '[' => Some(']'),
        '(' => Some(')'),
        _ => None,
    }
}

fn is_bracket_char(ch: char) -> bool {
    matches!(ch, '【' | '】' | '[' | ']' | '(' | ')')
}

/// The role set for one language, built from `credits.yaml` and
/// reused across every credit cue in the song.
///
/// The roles are always sorted from longest to shortest.
///
/// There are no duplication amongst the roles.
pub struct CreditRoles<'a>(Vec<&'a str>);

impl<'a> CreditRoles<'a> {
    /// Collects the language-specific labels from a descriptor.
    pub fn from_descriptor(descriptor: &'a CreditsDesc, language: &Language) -> Self {
        descriptor
            .credit_roles
            .iter()
            .filter_map(|entry| entry.get(language).map(String::as_str))
            .collect::<Vec<_>>()
            .into_sorted_by(|a, b| b.len().cmp(&a.len()).then_with(|| a.cmp(b)))
            .into_deduped()
            .pipe(CreditRoles)
    }

    fn take_role<'input>(&self, input: &'input str) -> Option<(&'input str, &'input str)> {
        self.0.iter().find_map(|role| {
            let rest = input.strip_prefix(*role)?;
            is_role_boundary(rest).then_some(input.split_at(role.len()))
        })
    }

    fn take_until_role<'input>(&self, input: &'input str) -> (&'input str, &'input str) {
        let mut cursor = 0;
        while cursor < input.len() && self.take_role(&input[cursor..]).is_none() {
            let Some(next_char) = input[cursor..].chars().next() else {
                break;
            };
            cursor += next_char.len_utf8();
        }
        input.split_at(cursor)
    }
}

/// Parses a credit line into ordered role-name pairs using the
/// provided [`CreditRoles`]. See the module docs for the algorithm.
pub fn parse_credit_line<'a>(
    line: &'a str,
    roles: &CreditRoles,
) -> Result<Vec<CreditPair<'a>>, ParseCreditError> {
    let mut pairs = Vec::<CreditPair>::new();
    let (_, mut rest) = take_leading_whitespace(line);

    while !rest.is_empty() {
        let (role, after_role) = roles.take_role(rest).ok_or_else(|| {
            ParseCreditError::UnknownRole(UnknownRole {
                line: line.to_string(),
                offset: line.len() - rest.len(),
            })
        })?;
        let (separator, after_separator) = take_cell_separator(after_role);
        let (raw_name_region, after_name) = roles.take_until_role(after_separator);
        let name_region = trim_end_separator(raw_name_region);
        let name_segments = parse_name_region(name_region);

        pairs.push(CreditPair {
            role,
            separator,
            name_segments,
        });

        let (_, after_trailing) = take_cell_separator(after_name);
        rest = after_trailing;
    }

    Ok(pairs)
}

fn parse_name_region(region: &str) -> Vec<NameSegment<'_>> {
    let mut segments = Vec::new();
    let mut rest = region;
    while let Some((pair, next_rest)) = NameSegmentPair::take(rest) {
        pair.append_to(&mut segments);
        rest = next_rest;
    }
    if !rest.is_empty() {
        segments.push(NameSegment::Unbracketed(Unbracketed(rest)));
    }
    segments
}

fn take_leading_whitespace(input: &str) -> (&str, &str) {
    let cursor = input
        .char_indices()
        .find(|(_, ch)| !ch.is_whitespace())
        .map(|(offset, _)| offset)
        .unwrap_or(input.len());
    input.split_at(cursor)
}

fn take_cell_separator(input: &str) -> (&str, &str) {
    let cursor = input
        .char_indices()
        .find(|(_, ch)| !is_separator_char(*ch))
        .map(|(offset, _)| offset)
        .unwrap_or(input.len());
    input.split_at(cursor)
}

fn trim_end_separator(input: &str) -> &str {
    let end = input
        .char_indices()
        .rev()
        .find(|(_, ch)| !is_separator_char(*ch))
        .map(|(offset, ch)| offset + ch.len_utf8())
        .unwrap_or(0);
    &input[..end]
}

fn is_separator_char(ch: char) -> bool {
    ch == ':' || ch == '：' || ch.is_whitespace()
}

fn is_role_boundary(following: &str) -> bool {
    match following.chars().next() {
        Some(ch) => is_separator_char(ch),
        None => true,
    }
}

/// Payload for an unknown-role error. Describes a credit line
/// whose cursor rests on text that does not match any known role
/// from `credits.yaml`.
#[derive(Debug, Display, Clone, PartialEq, Eq)]
#[display(
    "credit line {line:?} contains unrecognized text at byte offset {offset}; expected a known credit role from `credits.yaml`"
)]
pub struct UnknownRole {
    pub line: String,
    pub offset: usize,
}

#[derive(Debug, Display, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ParseCreditError {
    UnknownRole(UnknownRole),
}

#[cfg(test)]
mod tests;
