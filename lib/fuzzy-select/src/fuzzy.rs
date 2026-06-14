//! Text matching primitives shared by the interactive selectors and the
//! command-line flag resolution.
//!
//! Two distinct matching strategies live here, because the issue asks for
//! two different behaviors:
//!
//! * The interactive table filters rows that *contain* the typed word,
//!   case-insensitively. [`contains_ci`] implements that substring test.
//! * A command-line flag pre-selects a value by *fuzzy* matching, and the
//!   value must resolve to exactly one candidate. [`fuzzy_subsequence`]
//!   implements the subsequence test and [`resolve_unique`] enforces the
//!   "exactly one" rule.

// cspell:ignore mưa xuân xuan

use derive_more::Display;

/// Returns `true` when every character of `query` appears in `text`, in
/// order but not necessarily contiguously, ignoring case and diacritical
/// marks. An empty query matches everything.
///
/// This is the "fuzzy" match used to resolve a command-line flag value to
/// a single candidate. For example, the query `cld` matches `celluloid`.
pub fn fuzzy_subsequence(query: &str, text: &str) -> bool {
    let mut haystack = text.chars().map(fold_char);
    query
        .chars()
        .map(fold_char)
        .all(|needle| haystack.any(|candidate| candidate == needle))
}

/// Returns `true` when `text` contains `query` as a contiguous substring,
/// ignoring case and diacritical marks. An empty query matches everything.
///
/// This is the "contains the word" filter used by the interactive table.
pub fn contains_ci(text: &str, query: &str) -> bool {
    if query.is_empty() {
        return true;
    }
    let text: String = text.chars().map(fold_char).collect();
    let query: String = query.chars().map(fold_char).collect();
    text.contains(&query)
}

/// Folds a character to a lowercase, diacritic-free form so that matching
/// ignores both case and diacritical marks.
///
/// Vietnamese titles are routinely written with or without their marks, so
/// a search for "mua xuan" should still find "Mưa Xuân". Every accented
/// Vietnamese letter folds to its base letter and "đ" folds to "d"; CJK and
/// plain ASCII characters pass through unchanged.
fn fold_char(character: char) -> char {
    // `to_lowercase` can expand to several characters (for example the
    // German sharp S). The titles handled here never contain such
    // characters, so taking the first element keeps the function a simple
    // `char -> char` mapping without allocating.
    let lower = character.to_lowercase().next().unwrap_or(character);
    match lower {
        'à' | 'á' | 'ả' | 'ã' | 'ạ' | 'â' | 'ầ' | 'ấ' | 'ẩ' | 'ẫ' | 'ậ' | 'ă' | 'ằ' | 'ắ' | 'ẳ'
        | 'ẵ' | 'ặ' => 'a',
        'è' | 'é' | 'ẻ' | 'ẽ' | 'ẹ' | 'ê' | 'ề' | 'ế' | 'ể' | 'ễ' | 'ệ' => 'e',
        'ì' | 'í' | 'ỉ' | 'ĩ' | 'ị' => 'i',
        'ò' | 'ó' | 'ỏ' | 'õ' | 'ọ' | 'ô' | 'ồ' | 'ố' | 'ổ' | 'ỗ' | 'ộ' | 'ơ' | 'ờ' | 'ớ' | 'ở'
        | 'ỡ' | 'ợ' => 'o',
        'ù' | 'ú' | 'ủ' | 'ũ' | 'ụ' | 'ư' | 'ừ' | 'ứ' | 'ử' | 'ữ' | 'ự' => 'u',
        'ỳ' | 'ý' | 'ỷ' | 'ỹ' | 'ỵ' => 'y',
        'đ' => 'd',
        other => other,
    }
}

/// Reason a fuzzy query failed to identify exactly one candidate.
#[derive(Debug, Display, Clone, Copy, PartialEq, Eq)]
pub enum ResolveError {
    /// No candidate matched the query.
    #[display("no candidate matched")]
    NoMatch,
    /// More than one candidate matched the query.
    #[display("the query is ambiguous between multiple candidates")]
    Ambiguous,
}

/// Returns the single element of `items` whose any search key fuzzily
/// matches `query`.
///
/// The `keys` function yields the strings a candidate is matched against.
/// A candidate matches when at least one of its keys fuzzily contains the
/// query as a subsequence. The result is an error when zero candidates
/// match ([`ResolveError::NoMatch`]) or when more than one does
/// ([`ResolveError::Ambiguous`]).
pub fn resolve_unique<'a, Item, Keys>(
    query: &str,
    items: &'a [Item],
    keys: Keys,
) -> Result<&'a Item, ResolveError>
where
    Keys: Fn(&'a Item) -> Vec<&'a str>,
{
    let mut found: Option<&'a Item> = None;
    for item in items {
        let matched = keys(item)
            .into_iter()
            .any(|key| fuzzy_subsequence(query, key));
        if matched {
            if found.is_some() {
                return Err(ResolveError::Ambiguous);
            }
            found = Some(item);
        }
    }
    found.ok_or(ResolveError::NoMatch)
}

#[cfg(test)]
mod tests;
