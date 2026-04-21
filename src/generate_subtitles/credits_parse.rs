//! Vocabulary-driven credit-line parser.
//!
//! The parser takes the role vocabulary for one language from the
//! song's `credits.yaml` and walks the line left to right, matching
//! the longest registered role at every cursor position. The bytes
//! between a role match and the next role match (or end of line)
//! form the associated name region, and name regions are scanned
//! for bracketed highlight runs that become
//! [`NameSegment::Special`] values; anything else becomes
//! [`NameSegment::Plain`].
//!
//! A credit line whose first non-whitespace token is not a known
//! role raises [`ParseCreditError::UnknownRole`]. This lets the
//! integration tests catch typos such as `作詞` vs `作词` before
//! they ever reach `dist/`.

use crate::credits_descriptor::CreditsDesc;
use crate::video_descriptor::Language;
use core::fmt;
use derive_more::{Display, Error};
use std::collections::BTreeSet;

/// A structural role/name pair extracted from one credit line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreditPair {
    /// The role cell, exactly as it appeared in the source line.
    pub role: String,
    /// Literal bytes to emit between the role tag and the name tag.
    pub separator: String,
    /// Decomposed name region, with bracketed highlights promoted
    /// to structural segments.
    pub name_segments: Vec<NameSegment>,
}

/// A unit within the name region of a credit pair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NameSegment {
    /// Plain text that did not match a highlight.
    Plain(String),
    /// A bracketed highlight such as `【studio】`, `[note]`, or
    /// `(remark)`.
    Special(Bracketed),
}

/// A string that is guaranteed to open with a recognized bracket,
/// close with its matching counterpart, and contain no further
/// bracket characters in between. The type can only be obtained via
/// [`Bracketed::take`], which follows the parse-don't-validate
/// pattern: it consumes a prefix of the input and returns both the
/// parsed value and the remaining unparsed tail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bracketed(String);

impl Bracketed {
    /// Consumes a leading bracketed span. The three recognized
    /// pairs are `【...】`, `[...]`, and `(...)`. The bytes between
    /// the opening and closing characters must contain none of
    /// those six characters; if another bracket is encountered
    /// before the matching close, no value is produced and the
    /// caller is free to re-interpret the input as ordinary text.
    pub fn take(input: &str) -> Option<(Self, &str)> {
        let mut chars = input.char_indices();
        let (_, open) = chars.next()?;
        let close = matching_close(open)?;
        for (offset, ch) in chars {
            if ch == close {
                let end = offset + ch.len_utf8();
                return Some((Bracketed(input[..end].to_string()), &input[end..]));
            }
            if is_bracket_char(ch) {
                return None;
            }
        }
        None
    }

    /// The bracketed text, including the surrounding brackets.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Bracketed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
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

/// The role vocabulary for one language, built from `credits.yaml`
/// and reused across every credit cue in the song.
pub struct CreditsVocabulary {
    roles: Vec<String>,
}

impl CreditsVocabulary {
    /// Collects the language-specific labels from the
    /// [`credit-roles`] list in the descriptor and sorts them by
    /// descending length so the parser matches the longest role
    /// that still fits at the current cursor position.
    ///
    /// [`credit-roles`]: CreditsDesc::credit_roles
    pub fn from_descriptor(descriptor: &CreditsDesc, language: &Language) -> Self {
        let roles = deduplicate_longest_first(
            descriptor
                .credit_roles
                .iter()
                .filter_map(|entry| entry.get(language)),
        );
        CreditsVocabulary { roles }
    }

    fn take_role<'a>(&self, input: &'a str) -> Option<(&'a str, &'a str)> {
        self.roles.iter().find_map(|role| {
            let rest = input.strip_prefix(role.as_str())?;
            is_role_boundary(rest).then(|| input.split_at(role.len()))
        })
    }

    fn take_until_role<'a>(&self, input: &'a str) -> (&'a str, &'a str) {
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
/// provided vocabulary. See the module docs for the algorithm.
pub fn parse_credit_line(
    line: &str,
    vocabulary: &CreditsVocabulary,
) -> Result<Vec<CreditPair>, ParseCreditError> {
    let mut pairs: Vec<CreditPair> = Vec::new();
    let (_, mut rest) = take_leading_whitespace(line);

    while !rest.is_empty() {
        let (role, after_role) =
            vocabulary
                .take_role(rest)
                .ok_or_else(|| ParseCreditError::UnknownRole {
                    line: line.to_string(),
                    offset: line.len() - rest.len(),
                })?;
        let (separator, after_separator) = take_cell_separator(after_role);
        let (raw_name_region, after_name) = vocabulary.take_until_role(after_separator);
        let name_region = trim_end_separator(raw_name_region);
        let name_segments = parse_name_region(name_region);

        pairs.push(CreditPair {
            role: role.to_string(),
            separator: separator.to_string(),
            name_segments,
        });

        let (_, after_trailing) = take_cell_separator(after_name);
        rest = after_trailing;
    }

    Ok(pairs)
}

fn parse_name_region(region: &str) -> Vec<NameSegment> {
    let mut segments: Vec<NameSegment> = Vec::new();
    let mut plain = String::new();
    let mut rest = region;

    let flush = |plain: &mut String, segments: &mut Vec<NameSegment>| {
        if !plain.is_empty() {
            segments.push(NameSegment::Plain(std::mem::take(plain)));
        }
    };

    while !rest.is_empty() {
        if let Some((bracketed, next_rest)) = Bracketed::take(rest) {
            flush(&mut plain, &mut segments);
            segments.push(NameSegment::Special(bracketed));
            rest = next_rest;
            continue;
        }
        let Some(next_char) = rest.chars().next() else {
            break;
        };
        let step = next_char.len_utf8();
        plain.push_str(&rest[..step]);
        rest = &rest[step..];
    }
    flush(&mut plain, &mut segments);
    segments
}

fn deduplicate_longest_first<Iter, Item>(values: Iter) -> Vec<String>
where
    Iter: IntoIterator<Item = Item>,
    Item: AsRef<str>,
{
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut collected: Vec<String> = Vec::new();
    for value in values {
        let owned = value.as_ref().to_string();
        if seen.insert(owned.clone()) {
            collected.push(owned);
        }
    }
    collected.sort_by(|a, b| b.len().cmp(&a.len()).then_with(|| a.cmp(b)));
    collected
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

#[derive(Debug, Display, Error)]
#[non_exhaustive]
pub enum ParseCreditError {
    #[display(
        "credit line {line:?} contains unrecognized text at byte offset {offset}; expected a known credit role from `credits.yaml`"
    )]
    UnknownRole { line: String, offset: usize },
}

#[cfg(test)]
mod tests {
    use super::*;
    use maplit::btreemap;

    fn vocabulary(roles: &[&str]) -> CreditsVocabulary {
        let descriptor = CreditsDesc {
            credit_roles: roles
                .iter()
                .map(|role| btreemap! { Language::Vietnamese => role.to_string() })
                .collect(),
            credit_names: Vec::new(),
        };
        CreditsVocabulary::from_descriptor(&descriptor, &Language::Vietnamese)
    }

    fn bracketed(source: &str) -> Bracketed {
        let (value, rest) =
            Bracketed::take(source).expect("test fixture must be a valid bracketed prefix");
        assert!(
            rest.is_empty(),
            "test fixture must not carry trailing bytes past the closing bracket",
        );
        value
    }

    #[test]
    fn colon_separated_line_yields_one_pair_per_cell() {
        let v = vocabulary(&["role-a", "role-b", "role-c"]);
        let parsed = parse_credit_line(
            "role-a：name-a\u{3000}role-b：name-b\u{3000}role-c：name-c",
            &v,
        )
        .unwrap();
        assert_eq!(parsed.len(), 3);
        assert_eq!(parsed[0].role, "role-a");
        assert_eq!(parsed[0].separator, "：");
        assert_eq!(
            parsed[0].name_segments,
            vec![NameSegment::Plain("name-a".into())],
        );
        assert_eq!(parsed[1].role, "role-b");
        assert_eq!(parsed[2].role, "role-c");
    }

    #[test]
    fn two_space_separated_line_yields_one_pair_with_embedded_spaces() {
        let v = vocabulary(&["role-a"]);
        let parsed = parse_credit_line("role-a  name-a  name-b", &v).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].role, "role-a");
        assert_eq!(parsed[0].separator, "  ");
        assert_eq!(
            parsed[0].name_segments,
            vec![NameSegment::Plain("name-a  name-b".into())],
        );
    }

    #[test]
    fn tolerates_runs_wider_than_two_spaces() {
        let v = vocabulary(&["role-a"]);
        let parsed = parse_credit_line("role-a   name-a", &v).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].role, "role-a");
        assert_eq!(parsed[0].separator, "   ");
        assert_eq!(
            parsed[0].name_segments,
            vec![NameSegment::Plain("name-a".into())],
        );
    }

    #[test]
    fn longer_role_wins_over_shorter_prefix() {
        let v = vocabulary(&["role", "role-a"]);
        let parsed = parse_credit_line("role-a  name-a", &v).unwrap();
        assert_eq!(parsed[0].role, "role-a");
    }

    #[test]
    fn unknown_leading_text_errors() {
        let v = vocabulary(&["role-a"]);
        assert!(matches!(
            parse_credit_line("unknown  name-a", &v),
            Err(ParseCreditError::UnknownRole { .. }),
        ));
    }

    #[test]
    fn recognizes_lenticular_highlight() {
        let v = vocabulary(&["role-a"]);
        let parsed = parse_credit_line("role-a  name-a【label-a】", &v).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(
            parsed[0].name_segments,
            vec![
                NameSegment::Plain("name-a".into()),
                NameSegment::Special(bracketed("【label-a】")),
            ],
        );
    }

    #[test]
    fn multiple_highlights_interleave_with_plain_text() {
        let v = vocabulary(&["role-a"]);
        let parsed = parse_credit_line("role-a  【label-a】name-a 【label-b】name-b", &v).unwrap();
        assert_eq!(
            parsed[0].name_segments,
            vec![
                NameSegment::Special(bracketed("【label-a】")),
                NameSegment::Plain("name-a ".into()),
                NameSegment::Special(bracketed("【label-b】")),
                NameSegment::Plain("name-b".into()),
            ],
        );
    }

    #[test]
    fn bracketed_accepts_three_pair_kinds() {
        let (lenticular, rest) = Bracketed::take("【gold】tail").unwrap();
        assert_eq!(lenticular.as_str(), "【gold】");
        assert_eq!(rest, "tail");

        let (square, rest) = Bracketed::take("[silver]tail").unwrap();
        assert_eq!(square.as_str(), "[silver]");
        assert_eq!(rest, "tail");

        let (round, rest) = Bracketed::take("(bronze)tail").unwrap();
        assert_eq!(round.as_str(), "(bronze)");
        assert_eq!(rest, "tail");
    }

    #[test]
    fn bracketed_rejects_non_bracket_prefix_and_nested_or_unclosed_brackets() {
        assert!(Bracketed::take("no bracket").is_none());
        assert!(Bracketed::take("【open only").is_none());
        assert!(Bracketed::take("[open only").is_none());
        assert!(Bracketed::take("(open only").is_none());
        assert!(Bracketed::take("【a【b】c】").is_none());
        assert!(Bracketed::take("[mismatch】").is_none());
        assert!(Bracketed::take("(also [nested])").is_none());
    }
}
