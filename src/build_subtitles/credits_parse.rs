//! Structural parser for credit blocks.
//!
//! The per-song `make-subtitles.js` generators used fragile heuristics
//! such as "split by two spaces" or "split by `\u3000`" to locate the
//! boundaries between role and name cells in a credit line. Those
//! heuristics break as soon as a role or a name contains the separator
//! character. This module replaces them with an explicit longest-match
//! scan against the role and name vocabularies declared in
//! `credits.yaml`. The parser returns structural pairs of role and
//! name region, preserving the original inter-cell bytes so the
//! downstream renderers can decide whether to reproduce the source
//! whitespace verbatim or collapse punctuation like `：` and `\u3000`
//! into a single space.

use crate::credits_descriptor::CreditsDesc;
use crate::video_descriptor::Language;
use derive_more::{Display, Error};
use std::collections::BTreeSet;

/// A credit line parsed into ordered role-name pairs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedCreditLine {
    pub pairs: Vec<CreditPair>,
}

/// A single role cell paired with the name region that follows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreditPair {
    pub role: String,
    /// The verbatim bytes between the role's last character and the
    /// first character of the name region. Typically a run of spaces,
    /// a single `:` or `：`, or an ideographic space.
    pub separator: String,
    /// Decomposed name region, where bracketed highlights and known
    /// credited names are promoted to structural segments.
    pub name_segments: Vec<NameSegment>,
}

/// A unit within the name region of a credit line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NameSegment {
    /// Plain text that did not match any recognized name or highlight.
    Plain(String),
    /// A recognized credit name.
    Name(String),
    /// A bracketed highlight used to emphasize a studio or release.
    /// The inner bytes include the surrounding brackets so the
    /// renderer can reproduce them verbatim.
    Special(String),
}

/// A vocabulary of credit roles and names for a particular language,
/// used to drive structural parsing. Construct it once per song with
/// [`CreditsVocabulary::from_descriptor`] and reuse it across all cues
/// that share the same language.
pub struct CreditsVocabulary {
    roles: Vec<String>,
    names: Vec<String>,
}

impl CreditsVocabulary {
    /// Collects all role and name labels for `language` from the
    /// given credits descriptor.
    pub fn from_descriptor(descriptor: &CreditsDesc, language: &Language) -> Self {
        let roles = deduplicate_longest_first(
            descriptor
                .credit_roles
                .iter()
                .filter_map(|entry| entry.get(language)),
        );
        let names = deduplicate_longest_first(
            descriptor
                .credit_names
                .iter()
                .filter_map(|entry| entry.get(language)),
        );
        CreditsVocabulary { roles, names }
    }

    /// Parses a single credit line into ordered role-name pairs.
    ///
    /// The scan advances left to right by composing small
    /// `(consumed, rest)` parsers in the style popularized by `nom` and
    /// Parsec. At each position the scanner tries to match the longest
    /// role label from the vocabulary; whatever follows the role, up
    /// to the next role match or the end of the line, is the role's
    /// associated name region. Name regions are further scanned for
    /// known names and bracketed highlight runs.
    pub fn parse_line(&self, line: &str) -> Result<ParsedCreditLine, ParseCreditError> {
        let mut pairs: Vec<CreditPair> = Vec::new();
        let (_, mut rest) = take_leading_whitespace(line);

        while !rest.is_empty() {
            let (role, after_role) =
                self.take_role(rest)
                    .ok_or_else(|| ParseCreditError::UnknownRole {
                        line: line.to_string(),
                        offset: line.len() - rest.len(),
                    })?;
            let (separator, after_separator) = take_separator(after_role);
            let (raw_name_region, after_name) = self.take_until_role(after_separator);
            let name_region = trim_end_separator(raw_name_region);
            let name_segments = self.parse_name_region(name_region);

            pairs.push(CreditPair {
                role: role.to_string(),
                separator: separator.to_string(),
                name_segments,
            });

            let (_, after_trailing) = take_separator(after_name);
            rest = after_trailing;
        }

        Ok(ParsedCreditLine { pairs })
    }

    fn take_role<'a>(&self, input: &'a str) -> Option<(&'a str, &'a str)> {
        self.roles.iter().find_map(|role| {
            let rest = input.strip_prefix(role.as_str())?;
            is_role_boundary(rest).then(|| input.split_at(role.len()))
        })
    }

    fn take_name<'a>(&self, input: &'a str) -> Option<(&'a str, &'a str)> {
        self.names
            .iter()
            .find(|name| input.starts_with(name.as_str()))
            .map(|name| input.split_at(name.len()))
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

    fn parse_name_region(&self, region: &str) -> Vec<NameSegment> {
        let mut segments: Vec<NameSegment> = Vec::new();
        let mut rest = region;

        while !rest.is_empty() {
            if let Some((special, next_rest)) = take_special(rest) {
                segments.push(NameSegment::Special(special.to_string()));
                rest = next_rest;
                continue;
            }
            if let Some((name, next_rest)) = self.take_name(rest) {
                segments.push(NameSegment::Name(name.to_string()));
                rest = next_rest;
                continue;
            }
            let (plain, next_rest) = self.take_plain_run(rest);
            segments.push(NameSegment::Plain(plain.to_string()));
            rest = next_rest;
        }

        segments
    }

    fn take_plain_run<'a>(&self, input: &'a str) -> (&'a str, &'a str) {
        let mut cursor = 0;
        while cursor < input.len() {
            let suffix = &input[cursor..];
            if take_special(suffix).is_some() || self.take_name(suffix).is_some() {
                break;
            }
            let Some(next_char) = suffix.chars().next() else {
                break;
            };
            cursor += next_char.len_utf8();
        }
        input.split_at(cursor)
    }
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

/// Consumes any leading run of separator characters (ASCII or
/// full-width colon, or any whitespace including the ideographic space)
/// and returns the `(consumed, remaining)` split.
fn take_separator(input: &str) -> (&str, &str) {
    let cursor = input
        .char_indices()
        .find(|(_, ch)| !is_separator_char(*ch))
        .map(|(offset, _)| offset)
        .unwrap_or(input.len());
    input.split_at(cursor)
}

/// Consumes any leading run of whitespace and returns the
/// `(consumed, remaining)` split.
fn take_leading_whitespace(input: &str) -> (&str, &str) {
    let cursor = input
        .char_indices()
        .find(|(_, ch)| !ch.is_whitespace())
        .map(|(offset, _)| offset)
        .unwrap_or(input.len());
    input.split_at(cursor)
}

/// Trims a trailing run of separator characters from the end of a
/// slice, returning the slice up to the first non-separator character
/// when read from the right.
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

/// Consumes a bracketed highlight that opens at `input[0]`. Supports
/// the Chinese `【...】` pair, ASCII parentheses, and ASCII square
/// brackets; each bracket type must be closed by its matching
/// counterpart. Returns the `(bracket_span, remaining)` split where
/// `bracket_span` covers the opening bracket, its contents, and the
/// matching closing bracket.
fn take_special(input: &str) -> Option<(&str, &str)> {
    let first = input.chars().next()?;
    let close = match first {
        '【' => '】',
        '(' => ')',
        '[' => ']',
        _ => return None,
    };
    let open_len = first.len_utf8();
    let close_offset = input[open_len..].find(close)?;
    let end = open_len + close_offset + close.len_utf8();
    Some(input.split_at(end))
}

#[derive(Debug, Display, Error)]
#[non_exhaustive]
pub enum ParseCreditError {
    #[display(
        "credit line {line:?} contains unrecognized text at byte offset {offset}; expected a known credit role from `credits.yaml`"
    )]
    UnknownRole {
        #[error(not(source))]
        line: String,
        #[error(not(source))]
        offset: usize,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use maplit::btreemap;

    const ROLE_ALPHA: &str = "示例角色甲";
    const ROLE_BETA: &str = "示例角色乙";
    const NAME_ALPHA: &str = "示例姓名甲";
    const NAME_BETA: &str = "示例姓名乙";

    fn make_vocab(roles: &[&str], names: &[&str]) -> CreditsVocabulary {
        let descriptor = CreditsDesc {
            credit_roles: roles
                .iter()
                .map(|role| btreemap! { Language::Chinese => role.to_string() })
                .collect(),
            credit_names: names
                .iter()
                .map(|name| btreemap! { Language::Chinese => name.to_string() })
                .collect(),
        };
        CreditsVocabulary::from_descriptor(&descriptor, &Language::Chinese)
    }

    #[test]
    fn splits_two_space_separated_line() {
        let vocab = make_vocab(&[ROLE_ALPHA], &[NAME_ALPHA, NAME_BETA]);
        let parsed = vocab
            .parse_line(&format!("{ROLE_ALPHA}  {NAME_ALPHA}  {NAME_BETA}"))
            .unwrap();
        assert_eq!(parsed.pairs.len(), 1);
        assert_eq!(parsed.pairs[0].role, ROLE_ALPHA);
        assert_eq!(parsed.pairs[0].separator, "  ");
        assert_eq!(
            parsed.pairs[0].name_segments,
            vec![
                NameSegment::Name(NAME_ALPHA.to_string()),
                NameSegment::Plain("  ".to_string()),
                NameSegment::Name(NAME_BETA.to_string()),
            ],
        );
    }

    #[test]
    fn splits_ideographic_colon_separated_line() {
        let vocab = make_vocab(&[ROLE_ALPHA, ROLE_BETA], &[NAME_ALPHA, NAME_BETA]);
        let parsed = vocab
            .parse_line(&format!(
                "{ROLE_ALPHA}：{NAME_ALPHA}\u{3000}{ROLE_BETA}：{NAME_BETA}"
            ))
            .unwrap();
        assert_eq!(parsed.pairs.len(), 2);
        assert_eq!(parsed.pairs[0].role, ROLE_ALPHA);
        assert_eq!(parsed.pairs[0].separator, "：");
        assert_eq!(
            parsed.pairs[0].name_segments,
            vec![NameSegment::Name(NAME_ALPHA.to_string())],
        );
        assert_eq!(parsed.pairs[1].role, ROLE_BETA);
        assert_eq!(
            parsed.pairs[1].name_segments,
            vec![NameSegment::Name(NAME_BETA.to_string())],
        );
    }

    #[test]
    fn recognizes_lenticular_highlight() {
        let vocab = make_vocab(&[ROLE_ALPHA], &[NAME_ALPHA]);
        let parsed = vocab
            .parse_line(&format!("{ROLE_ALPHA}  {NAME_ALPHA}【示例标签】"))
            .unwrap();
        assert_eq!(
            parsed.pairs[0].name_segments,
            vec![
                NameSegment::Name(NAME_ALPHA.to_string()),
                NameSegment::Special("【示例标签】".to_string()),
            ],
        );
    }

    #[test]
    fn recognizes_parenthesized_highlight() {
        let vocab = make_vocab(&[ROLE_ALPHA], &[NAME_ALPHA]);
        let parsed = vocab
            .parse_line(&format!("{ROLE_ALPHA}  {NAME_ALPHA}(example-tag)"))
            .unwrap();
        assert_eq!(
            parsed.pairs[0].name_segments,
            vec![
                NameSegment::Name(NAME_ALPHA.to_string()),
                NameSegment::Special("(example-tag)".to_string()),
            ],
        );
    }

    #[test]
    fn recognizes_square_bracketed_highlight() {
        let vocab = make_vocab(&[ROLE_ALPHA], &[NAME_ALPHA]);
        let parsed = vocab
            .parse_line(&format!("{ROLE_ALPHA}  {NAME_ALPHA}[example-tag]"))
            .unwrap();
        assert_eq!(
            parsed.pairs[0].name_segments,
            vec![
                NameSegment::Name(NAME_ALPHA.to_string()),
                NameSegment::Special("[example-tag]".to_string()),
            ],
        );
    }

    #[test]
    fn unknown_leading_text_errors() {
        let vocab = make_vocab(&[ROLE_ALPHA], &[NAME_ALPHA]);
        assert!(matches!(
            vocab.parse_line(&format!("unknown-prefix  {NAME_ALPHA}")),
            Err(ParseCreditError::UnknownRole { .. }),
        ));
    }

    #[test]
    fn longer_role_wins_over_shorter_role_prefix() {
        let vocab = make_vocab(&["示例", ROLE_ALPHA], &[NAME_ALPHA]);
        let parsed = vocab
            .parse_line(&format!("{ROLE_ALPHA}  {NAME_ALPHA}"))
            .unwrap();
        assert_eq!(parsed.pairs[0].role, ROLE_ALPHA);
    }
}
