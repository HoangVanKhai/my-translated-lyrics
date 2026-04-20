//! Credit-line segmentation.
//!
//! Credit blocks separate one role cell from the next by a run of
//! whitespace whose width is language-specific. For ASCII-spaced
//! data a run of two or more ASCII spaces marks a cell boundary.
//! For Chinese data authored with the ideographic space (`\u{3000}`)
//! a run of one or more ideographic spaces marks the same boundary.
//! Within a cell an optional `:` or `：` separates role from name,
//! and `【...】` runs inside a name become structural highlights.
//!
//! Two cell conventions are recognized:
//!
//! * **Colon-separated.** At least one cell contains `:` or `：`.
//!   `split_cells` produces the cells according to the
//!   language-specific separator rule above, and each cell becomes
//!   its own role-name pair. Cells that happen to lack a colon
//!   fall through as plain name-only entries.
//! * **First-cell-is-role.** The line contains no `:` or `：` at
//!   all. The first run of two or more ASCII spaces splits the
//!   line into a role prefix and a name suffix. The name suffix
//!   stays verbatim, including any internal space runs. This
//!   convention uses ASCII spacing regardless of language because
//!   it matches the data authored that way today.

use crate::video_descriptor::Language;

/// A structural role/name pair extracted from one credit line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreditPair {
    /// Role cell, or `None` when a colon-convention cell has no
    /// colon-separated role.
    pub role: Option<String>,
    /// Literal bytes to emit between the role tag and the name tag.
    /// Empty when `role` is `None`.
    pub separator: String,
    /// Decomposed name region, with `【...】` highlights promoted
    /// to structural segments.
    pub name_segments: Vec<NameSegment>,
}

/// A unit within the name region of a credit pair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NameSegment {
    /// Plain text that did not match a highlight.
    Plain(String),
    /// A `【...】` highlight used to emphasize a studio or release.
    /// The inner bytes include the surrounding brackets so the
    /// renderer can reproduce them verbatim.
    Special(String),
}

/// Parses a credit line into ordered role-name pairs following the
/// per-language split rule described in the module docs.
pub fn parse_credit_line(line: &str, language: &Language) -> Vec<CreditPair> {
    if line.contains(':') || line.contains('：') {
        split_cells(line, language)
            .into_iter()
            .map(parse_colon_cell)
            .collect()
    } else {
        parse_role_plus_rest(line).into_iter().collect()
    }
}

fn parse_colon_cell(cell: &str) -> CreditPair {
    for (offset, ch) in cell.char_indices() {
        if ch == ':' || ch == '：' {
            let role = cell[..offset].trim();
            let name = cell[offset + ch.len_utf8()..].trim();
            if !role.is_empty() && !name.is_empty() {
                return CreditPair {
                    role: Some(role.to_string()),
                    separator: " ".to_string(),
                    name_segments: wrap_specials(name),
                };
            }
        }
    }
    CreditPair {
        role: None,
        separator: String::new(),
        name_segments: wrap_specials(cell.trim()),
    }
}

fn parse_role_plus_rest(line: &str) -> Option<CreditPair> {
    let (role, separator, rest) = find_first_space_run(line, 2)?;
    let role = role.trim();
    if role.is_empty() || rest.is_empty() {
        return None;
    }
    Some(CreditPair {
        role: Some(role.to_string()),
        separator: separator.to_string(),
        name_segments: wrap_specials(rest),
    })
}

/// Locates the first run of `min_run` or more ASCII spaces and
/// returns the slice before, the run itself, and the slice after.
fn find_first_space_run(line: &str, min_run: usize) -> Option<(&str, &str, &str)> {
    let bytes = line.as_bytes();
    let mut cursor = 0;
    while cursor < bytes.len() {
        if bytes[cursor] == b' ' {
            let start = cursor;
            while cursor < bytes.len() && bytes[cursor] == b' ' {
                cursor += 1;
            }
            if cursor - start >= min_run {
                return Some((&line[..start], &line[start..cursor], &line[cursor..]));
            }
        } else {
            cursor += 1;
        }
    }
    None
}

fn split_cells<'a>(line: &'a str, language: &Language) -> Vec<&'a str> {
    let (sep, min_run) = cell_separator_rule(language);
    split_on_char_run(line, sep, min_run)
}

fn cell_separator_rule(language: &Language) -> (char, usize) {
    match language {
        Language::Chinese => ('\u{3000}', 1),
        _ => (' ', 2),
    }
}

fn split_on_char_run(line: &str, sep: char, min_run: usize) -> Vec<&str> {
    let mut cells = Vec::new();
    let mut cell_start = 0;
    let mut iter = line.char_indices().peekable();
    while let Some(&(offset, ch)) = iter.peek() {
        if ch == sep {
            let run_start = offset;
            let mut run_len = 0;
            while let Some(&(_, next_ch)) = iter.peek() {
                if next_ch != sep {
                    break;
                }
                iter.next();
                run_len += 1;
            }
            if run_len >= min_run {
                let run_end = iter.peek().map(|&(offset, _)| offset).unwrap_or(line.len());
                let cell = line[cell_start..run_start].trim();
                if !cell.is_empty() {
                    cells.push(cell);
                }
                cell_start = run_end;
            }
        } else {
            iter.next();
        }
    }
    let last = line[cell_start..].trim();
    if !last.is_empty() {
        cells.push(last);
    }
    cells
}

fn wrap_specials(input: &str) -> Vec<NameSegment> {
    let mut segments: Vec<NameSegment> = Vec::new();
    let mut plain = String::new();
    let mut rest = input;

    let flush = |plain: &mut String, segments: &mut Vec<NameSegment>| {
        if !plain.is_empty() {
            segments.push(NameSegment::Plain(std::mem::take(plain)));
        }
    };

    while !rest.is_empty() {
        if let Some((special, next_rest)) = take_special(rest) {
            flush(&mut plain, &mut segments);
            segments.push(NameSegment::Special(special.to_string()));
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

/// Consumes a `【...】` highlight that opens at `input[0]`. Neither
/// `【` nor `】` may appear inside the brackets; if a second `【`
/// is encountered before the closing `】`, the original opener is
/// reported as ordinary text rather than a highlight.
fn take_special(input: &str) -> Option<(&str, &str)> {
    let rest = input.strip_prefix('【')?;
    let terminator = rest
        .char_indices()
        .find(|(_, ch)| *ch == '】' || *ch == '【')?;
    if terminator.1 != '】' {
        return None;
    }
    let end = '【'.len_utf8() + terminator.0 + '】'.len_utf8();
    Some(input.split_at(end))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chinese_splits_on_ideographic_space_and_colons() {
        let parsed = parse_credit_line(
            "role-a：name-a\u{3000}role-b：name-b\u{3000}role-c：name-c",
            &Language::Chinese,
        );
        assert_eq!(parsed.len(), 3);
        assert_eq!(parsed[0].role.as_deref(), Some("role-a"));
        assert_eq!(parsed[0].separator, " ");
        assert_eq!(
            parsed[0].name_segments,
            vec![NameSegment::Plain("name-a".into())],
        );
        assert_eq!(parsed[1].role.as_deref(), Some("role-b"));
        assert_eq!(parsed[2].role.as_deref(), Some("role-c"));
    }

    #[test]
    fn vietnamese_splits_on_two_or_more_ascii_spaces_and_colons() {
        let parsed = parse_credit_line("role-a: name-a    role-b: name-b", &Language::Vietnamese);
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].role.as_deref(), Some("role-a"));
        assert_eq!(
            parsed[0].name_segments,
            vec![NameSegment::Plain("name-a".into())],
        );
        assert_eq!(parsed[1].role.as_deref(), Some("role-b"));
    }

    #[test]
    fn splits_on_three_or_more_ascii_spaces() {
        let parsed = parse_credit_line("role-a: name-a   role-b: name-b", &Language::Vietnamese);
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].role.as_deref(), Some("role-a"));
        assert_eq!(
            parsed[0].name_segments,
            vec![NameSegment::Plain("name-a".into())],
        );
        assert_eq!(parsed[1].role.as_deref(), Some("role-b"));
    }

    #[test]
    fn first_cell_is_role_without_colons_uses_ascii_split() {
        for language in [Language::Vietnamese, Language::Chinese] {
            let parsed = parse_credit_line("role-a  name-a  name-b", &language);
            assert_eq!(parsed.len(), 1);
            assert_eq!(parsed[0].role.as_deref(), Some("role-a"));
            assert_eq!(parsed[0].separator, "  ");
            assert_eq!(
                parsed[0].name_segments,
                vec![NameSegment::Plain("name-a  name-b".into())],
            );
        }
    }

    #[test]
    fn tolerates_runs_wider_than_two_spaces() {
        let parsed = parse_credit_line("role-a   name-a", &Language::Vietnamese);
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].role.as_deref(), Some("role-a"));
        assert_eq!(parsed[0].separator, "   ");
        assert_eq!(
            parsed[0].name_segments,
            vec![NameSegment::Plain("name-a".into())],
        );
    }

    #[test]
    fn recognizes_lenticular_highlight() {
        let parsed = parse_credit_line("role-a  name-a【label-a】", &Language::Vietnamese);
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].role.as_deref(), Some("role-a"));
        assert_eq!(
            parsed[0].name_segments,
            vec![
                NameSegment::Plain("name-a".into()),
                NameSegment::Special("【label-a】".into()),
            ],
        );
    }

    #[test]
    fn multiple_highlights_interleave_with_plain_text() {
        let parsed = parse_credit_line(
            "role-a  【label-a】name-a 【label-b】name-b",
            &Language::Vietnamese,
        );
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].role.as_deref(), Some("role-a"));
        assert_eq!(
            parsed[0].name_segments,
            vec![
                NameSegment::Special("【label-a】".into()),
                NameSegment::Plain("name-a ".into()),
                NameSegment::Special("【label-b】".into()),
                NameSegment::Plain("name-b".into()),
            ],
        );
    }
}
