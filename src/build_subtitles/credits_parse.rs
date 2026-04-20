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
    /// Decomposed name region, where `【...】` highlights and known
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
    /// A `【...】` highlight used to emphasize a studio or release.
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
    /// further scanned for known names and `【...】` highlight runs.
    pub fn parse_line(&self, line: &str) -> Result<ParsedCreditLine, ParseCreditError> {
        let mut pairs: Vec<CreditPair> = Vec::new();
        let bytes = line.as_bytes();
        let length = bytes.len();
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
                let step = char_len_at(bytes, name_end);
                name_end += step;
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
        for role in &self.roles {
            let role_bytes = role.as_bytes();
            if suffix.as_bytes().starts_with(role_bytes) {
                let following = &suffix[role.len()..];
                if is_role_boundary(following) {
                    return Some(role);
                }
            }
        }
        None
    }

    fn parse_name_region(&self, region: &str) -> Vec<NameSegment> {
        let mut segments: Vec<NameSegment> = Vec::new();
        let mut buffer = String::new();
        let bytes = region.as_bytes();
        let length = bytes.len();
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

            let step = char_len_at(bytes, cursor);
            buffer.push_str(&region[cursor..cursor + step]);
            cursor += step;
        }

        flush(&mut buffer, &mut segments);
        segments
    }

    fn match_name_at<'a>(&'a self, suffix: &str) -> Option<&'a str> {
        self.names
            .iter()
            .find(|name| suffix.as_bytes().starts_with(name.as_bytes()))
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
    while cursor < input.len() {
        let char_end = char_len_at(input.as_bytes(), cursor);
        let char_str = &input[cursor..cursor + char_end];
        if char_str == ":"
            || char_str == "\u{FF1A}"
            || char_str.chars().all(|ch| ch.is_whitespace())
        {
            cursor += char_end;
        } else {
            break;
        }
    }
    cursor
}

fn count_leading_whitespace(input: &str) -> usize {
    let mut cursor = 0;
    while cursor < input.len() {
        let char_end = char_len_at(input.as_bytes(), cursor);
        let char_str = &input[cursor..cursor + char_end];
        if char_str.chars().all(|ch| ch.is_whitespace()) {
            cursor += char_end;
        } else {
            break;
        }
    }
    cursor
}

fn trim_end_separator(input: &str) -> usize {
    let bytes = input.as_bytes();
    let mut end = bytes.len();
    while end > 0 {
        let mut char_start = end - 1;
        while char_start > 0 && (bytes[char_start] & 0xC0) == 0x80 {
            char_start -= 1;
        }
        let char_str = &input[char_start..end];
        if char_str == ":"
            || char_str == "\u{FF1A}"
            || char_str.chars().all(|ch| ch.is_whitespace())
        {
            end = char_start;
        } else {
            break;
        }
    }
    end
}

fn is_role_boundary(following: &str) -> bool {
    if following.is_empty() {
        return true;
    }
    let Some(first) = following.chars().next() else {
        return true;
    };
    first.is_whitespace() || first == ':' || first == '\u{FF1A}'
}

fn match_special_at(suffix: &str) -> Option<usize> {
    if !suffix.starts_with('\u{3010}') {
        return None;
    }
    let open_len = '\u{3010}'.len_utf8();
    let rest = &suffix[open_len..];
    let close_offset = rest.find('\u{3011}')?;
    Some(open_len + close_offset + '\u{3011}'.len_utf8())
}

fn char_len_at(bytes: &[u8], offset: usize) -> usize {
    let first = bytes[offset];
    match first {
        0x00..=0x7F => 1,
        0xC0..=0xDF => 2,
        0xE0..=0xEF => 3,
        0xF0..=0xF7 => 4,
        _ => 1,
    }
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
        let vocab = make_vocab(
            &["演唱", "作曲", "作词", "编曲"],
            &["洛天依", "乐正绫", "雨观"],
        );
        let parsed = vocab.parse_line("演唱  洛天依  乐正绫").unwrap();
        assert_eq!(parsed.pairs.len(), 1);
        assert_eq!(parsed.pairs[0].role, "演唱");
        assert_eq!(parsed.pairs[0].separator, "  ");
        assert_eq!(
            parsed.pairs[0].name_segments,
            vec![
                NameSegment::Name("洛天依".to_string()),
                NameSegment::Plain("  ".to_string()),
                NameSegment::Name("乐正绫".to_string()),
            ],
        );
    }

    #[test]
    fn splits_ideographic_colon_separated_line() {
        let vocab = make_vocab(
            &["作词", "编曲", "VSINGER"],
            &["雨観", "雨观", "洛天依X楽正绫"],
        );
        let parsed = vocab
            .parse_line("作词：雨観\u{3000}编曲：雨观\u{3000}VSINGER：洛天依X楽正绫")
            .unwrap();
        assert_eq!(parsed.pairs.len(), 3);
        assert_eq!(parsed.pairs[0].role, "作词");
        assert_eq!(parsed.pairs[0].separator, "\u{FF1A}");
        assert_eq!(
            parsed.pairs[0].name_segments,
            vec![NameSegment::Name("雨観".to_string())],
        );
        assert_eq!(parsed.pairs[2].role, "VSINGER");
    }

    #[test]
    fn recognizes_special_highlight() {
        let vocab = make_vocab(&["视频"], &["Ａ影羌", "良月十八"]);
        let parsed = vocab.parse_line("视频  Ａ影羌【璇玑坊Studio】").unwrap();
        assert_eq!(parsed.pairs.len(), 1);
        assert_eq!(
            parsed.pairs[0].name_segments,
            vec![
                NameSegment::Name("Ａ影羌".to_string()),
                NameSegment::Special("【璇玑坊Studio】".to_string()),
            ],
        );
    }

    #[test]
    fn unknown_leading_text_errors() {
        let vocab = make_vocab(&["演唱"], &["洛天依"]);
        assert!(matches!(
            vocab.parse_line("unknown  洛天依"),
            Err(ParseCreditError::UnknownRole { .. }),
        ));
    }

    #[test]
    fn longer_role_wins_over_shorter_role_prefix() {
        let vocab = make_vocab(&["演", "演唱"], &["洛天依"]);
        let parsed = vocab.parse_line("演唱  洛天依").unwrap();
        assert_eq!(parsed.pairs[0].role, "演唱");
    }
}
