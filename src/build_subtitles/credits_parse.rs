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
    /// The scan advances left to right. At each position it tries to
    /// match the longest role label from the vocabulary; whatever
    /// follows the role, up to the next role match or the end of the
    /// line, is the role's associated name region. Name regions are
    /// further scanned for known names and bracketed highlight runs.
    pub fn parse_line(&self, line: &str) -> Result<ParsedCreditLine, ParseCreditError> {
        let mut pairs: Vec<CreditPair> = Vec::new();
        let length = line.len();
        let mut cursor = count_leading_whitespace(line);

        while cursor < length {
            let Some(role) = self.match_role_at(&line[cursor..]) else {
                return Err(ParseCreditError::UnknownRole {
                    line: line.to_string(),
                    offset: cursor,
                });
            };
            let role_start = cursor;
            let role_end = role_start + role.len();
            cursor = role_end;

            let sep_start = cursor;
            let sep_end = sep_start + count_separator_bytes(&line[cursor..]);
            cursor = sep_end;

            let name_start = cursor;
            let mut name_end = name_start;
            while name_end < length {
                let suffix = &line[name_end..];
                if self.match_role_at(suffix).is_some() {
                    break;
                }
                let Some(next_char) = suffix.chars().next() else {
                    break;
                };
                name_end += next_char.len_utf8();
            }
            let trimmed_name_end = trim_end_separator(&line[name_start..name_end]) + name_start;
            let name_region = &line[name_start..trimmed_name_end];
            let name_segments = self.parse_name_region(name_region);

            pairs.push(CreditPair {
                role: line[role_start..role_end].to_string(),
                separator: line[sep_start..sep_end].to_string(),
                name_segments,
            });

            cursor = name_end;
            cursor += count_separator_bytes(&line[cursor..]);
        }

        Ok(ParsedCreditLine { pairs })
    }

    fn match_role_at<'a>(&'a self, suffix: &str) -> Option<&'a str> {
        self.roles
            .iter()
            .find(|role| {
                suffix.starts_with(role.as_str()) && is_role_boundary(&suffix[role.len()..])
            })
            .map(String::as_str)
    }

    fn parse_name_region(&self, region: &str) -> Vec<NameSegment> {
        let mut segments: Vec<NameSegment> = Vec::new();
        let mut buffer = String::new();
        let length = region.len();
        let mut cursor = 0;

        let flush = |buffer: &mut String, segments: &mut Vec<NameSegment>| {
            if !buffer.is_empty() {
                segments.push(NameSegment::Plain(std::mem::take(buffer)));
            }
        };

        while cursor < length {
            let suffix = &region[cursor..];

            if let Some(special_len) = match_special_at(suffix) {
                flush(&mut buffer, &mut segments);
                let special = &region[cursor..cursor + special_len];
                segments.push(NameSegment::Special(special.to_string()));
                cursor += special_len;
                continue;
            }

            if let Some(name) = self.match_name_at(suffix) {
                flush(&mut buffer, &mut segments);
                segments.push(NameSegment::Name(name.to_string()));
                cursor += name.len();
                continue;
            }

            let Some(next_char) = suffix.chars().next() else {
                break;
            };
            let step = next_char.len_utf8();
            buffer.push_str(&region[cursor..cursor + step]);
            cursor += step;
        }

        flush(&mut buffer, &mut segments);
        segments
    }

    fn match_name_at<'a>(&'a self, suffix: &str) -> Option<&'a str> {
        self.names
            .iter()
            .find(|name| suffix.starts_with(name.as_str()))
            .map(String::as_str)
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

fn count_separator_bytes(input: &str) -> usize {
    let mut cursor = 0;
    for ch in input.chars() {
        if ch == ':' || ch == '：' || ch.is_whitespace() {
            cursor += ch.len_utf8();
        } else {
            break;
        }
    }
    cursor
}

fn count_leading_whitespace(input: &str) -> usize {
    let mut cursor = 0;
    for ch in input.chars() {
        if ch.is_whitespace() {
            cursor += ch.len_utf8();
        } else {
            break;
        }
    }
    cursor
}

fn trim_end_separator(input: &str) -> usize {
    let mut end = input.len();
    for (offset, ch) in input.char_indices().rev() {
        if ch == ':' || ch == '：' || ch.is_whitespace() {
            end = offset;
        } else {
            break;
        }
    }
    end
}

fn is_role_boundary(following: &str) -> bool {
    let Some(first) = following.chars().next() else {
        return true;
    };
    first.is_whitespace() || first == ':' || first == '：'
}

/// Detects a bracketed highlight that opens at `suffix[0]`. Supports
/// the Chinese `【...】` pair, ASCII parentheses, and ASCII square
/// brackets; each bracket type must be closed by its matching
/// counterpart. Returns the length in bytes of the entire bracketed
/// span, including the open and close characters, when a match is
/// found.
fn match_special_at(suffix: &str) -> Option<usize> {
    let first = suffix.chars().next()?;
    let close = match first {
        '【' => '】',
        '(' => ')',
        '[' => ']',
        _ => return None,
    };
    let open_len = first.len_utf8();
    let rest = &suffix[open_len..];
    let close_offset = rest.find(close)?;
    Some(open_len + close_offset + close.len_utf8())
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
