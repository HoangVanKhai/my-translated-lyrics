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
//! A credit line normally opens with a registered role, but a
//! role-less line may instead open with a bracketed span (a
//! [`CreditLead::Special`] highlight) that stands in for the role. A
//! line whose first non-whitespace token is neither a known role nor a
//! bracketed span raises [`ParseCreditError::UnknownRole`], which lets
//! the integration tests catch typos such as `作詞` vs `作词` before
//! they ever reach `dist/`.
//!
//! Only the `credit-roles` list is consumed by this parser. The
//! `credit-names` list on [`CreditsDesc`] is loaded and carried
//! through the pipeline but is not cross-checked against the parsed
//! name regions here; see the "`credits.yaml` consistency test"
//! item in the PR description for the deferred work that would add
//! that validation.

use derive_more::Display;
use into_deduped::IntoDeduped;
use into_sorted::IntoSorted;
use lyrics_core::credits_descriptor::CreditsDesc;
use lyrics_core::video_descriptor::Language;
use pipe_trait::Pipe;

/// A structural lead/name pair extracted from one credit line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreditPair<'a> {
    /// What opens the line: a role, or a role-less bracket highlight.
    pub lead: CreditLead<'a>,
    /// Raw separator text captured between the lead cell and the
    /// name cell, preserved verbatim so the renderer can decide how
    /// to emit it. [`CreditPair::separator_style`] reads this field
    /// to choose between a CJK colon, a Latin colon, or a verbatim
    /// space gutter.
    pub separator: &'a str,
    /// Decomposed name region, with bracketed and unbracketed
    /// segments. Empty for a role-only header line.
    pub name_segments: Vec<NameSegment<'a>>,
}

/// What opens a credit line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreditLead<'a> {
    /// A registered role, rendered in the credit role color.
    Role(&'a str),
    /// A bracketed span that opens a role-less line, for example
    /// `[疏楼曲]` ahead of a contributor name, rendered in the credit
    /// highlight color in place of a role span.
    Special(Bracketed<'a>),
}

impl<'a> CreditPair<'a> {
    /// Classifies this pair's separator into the layout the renderers
    /// should produce. The choice of colon glyph in the source line
    /// selects the layout, so the policy stays data-driven and
    /// symmetric with the separator-tolerant parser: a full-width
    /// colon yields [`SeparatorStyle::FullWidthColon`], a lone ASCII
    /// colon yields [`SeparatorStyle::AsciiColon`], and a colon-free
    /// separator yields [`SeparatorStyle::Spaces`].
    pub fn separator_style(&self) -> SeparatorStyle<'a> {
        if self.separator.contains('：') {
            SeparatorStyle::FullWidthColon
        } else if self.separator.contains(':') {
            SeparatorStyle::AsciiColon
        } else {
            SeparatorStyle::Spaces(self.separator)
        }
    }
}

/// How a credit pair's role-to-name separator is presented in the
/// rendered output. The variant is derived from [`CreditPair::separator`]
/// by [`CreditPair::separator_style`]. Authors pick the layout by
/// typing the matching colon in the source: full-width `：` in CJK
/// lyrics, ASCII `:` in Latin-script lyrics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeparatorStyle<'a> {
    /// The separator carried a full-width colon (`：`). The renderer
    /// emits a full-width colon between the lead and name spans with
    /// no surrounding spaces, the convention for CJK credit lines.
    FullWidthColon,
    /// The separator carried an ASCII colon (`:`) and no full-width
    /// colon. The renderer tucks an ASCII colon inside the lead span
    /// and follows it with one ASCII space, the convention for
    /// Latin-script credit lines.
    AsciiColon,
    /// The separator was free of colons. The captured run is carried
    /// through verbatim so an ASCII space or tab gutter round-trips;
    /// any other whitespace shape collapses to a single ASCII space.
    /// See [`SeparatorStyle::append_between_spans`].
    Spaces(&'a str),
}

impl SeparatorStyle<'_> {
    /// The colon, if any, that belongs inside the lead's styled span
    /// (a role or a role-less bracket highlight). A Latin-script credit
    /// line ([`SeparatorStyle::AsciiColon`]) tucks an ASCII colon
    /// inside the lead's color; the CJK and colon-free layouts
    /// contribute nothing here and place their separator between the
    /// spans with [`SeparatorStyle::append_between_spans`].
    pub fn lead_span_suffix(self) -> &'static str {
        match self {
            SeparatorStyle::AsciiColon => ":",
            SeparatorStyle::FullWidthColon | SeparatorStyle::Spaces(_) => "",
        }
    }

    /// Appends the separator that sits between the lead span and the
    /// name span: one ASCII space after a Latin colon, a full-width
    /// colon for the CJK layout, or the colon-free gutter. An ASCII
    /// space or tab gutter round-trips verbatim; any other whitespace
    /// shape collapses to a single ASCII space.
    pub fn append_between_spans(self, output: &mut String) {
        match self {
            SeparatorStyle::AsciiColon => output.push(' '),
            SeparatorStyle::FullWidthColon => output.push('：'),
            SeparatorStyle::Spaces(raw) => {
                if !raw.is_empty() && raw.chars().all(|ch| ch == ' ' || ch == '\t') {
                    output.push_str(raw);
                } else {
                    output.push(' ');
                }
            }
        }
    }
}

/// A unit within the name region of a credit pair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NameSegment<'a> {
    /// A run of text that contains no parseable bracketed span.
    Unbracketed(Unbracketed<'a>),
    /// A bracketed span (`【...】`, `[...]`, `(...)`, or `（...）`),
    /// with the surrounding brackets included in the wrapped slice.
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
    /// Consumes a leading bracketed span. The four recognized
    /// pairs are `【...】`, `[...]`, `(...)`, and the full-width
    /// `（...）`. The bytes between the opening and closing
    /// characters must contain none of those eight characters; if
    /// another bracket is encountered before the matching close, no
    /// value is produced and the caller is free to re-interpret the
    /// input as ordinary text.
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
        '（' => Some('）'),
        _ => None,
    }
}

fn is_bracket_char(ch: char) -> bool {
    matches!(ch, '【' | '】' | '[' | ']' | '(' | ')' | '（' | '）')
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
        while cursor < input.len() {
            // A registered role only opens a new cell when it begins at
            // a cell boundary: the start of the name region (which
            // already follows the previous cell's separator) or right
            // after a separator character. A role token sitting mid-name,
            // such as the `二胡` inside the personal name `陆二胡`, is part
            // of the name and must not split it.
            let at_cell_boundary = cursor == 0
                || input[..cursor]
                    .chars()
                    .next_back()
                    .is_some_and(is_separator_char);
            if at_cell_boundary && self.take_role(&input[cursor..]).is_some() {
                break;
            }
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
        // A cell opens with a registered role, or, on a role-less
        // line, with a bracketed span that the renderers highlight in
        // place of a role. Anything else is unrecognized text.
        let (lead, after_lead) = if let Some((role, after_role)) = roles.take_role(rest) {
            (CreditLead::Role(role), after_role)
        } else if let Some((bracket, after_bracket)) = Bracketed::take(rest) {
            (CreditLead::Special(bracket), after_bracket)
        } else {
            return Err(ParseCreditError::UnknownRole(UnknownRole {
                line: line.to_string(),
                offset: line.len() - rest.len(),
            }));
        };
        let (separator, after_separator) = take_cell_separator(after_lead);
        let (raw_name_region, after_name) = roles.take_until_role(after_separator);
        let name_region = trim_end_separator(raw_name_region);
        let name_segments = parse_name_region(name_region);

        pairs.push(CreditPair {
            lead,
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
