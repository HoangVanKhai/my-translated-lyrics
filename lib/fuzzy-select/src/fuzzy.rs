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
/// order but not necessarily contiguously. Case is ignored and diacritics
/// are handled asymmetrically: an unmarked query character matches the base
/// letter and all its accented forms, while a marked one matches only that
/// exact form. An empty query matches everything.
///
/// This is the "fuzzy" match used to resolve a command-line flag value to
/// a single candidate. For example, the query `cld` matches `celluloid`.
pub fn fuzzy_subsequence(query: &str, text: &str) -> bool {
    let mut haystack = text.chars();
    query
        .chars()
        .all(|needle| haystack.any(|candidate| char_matches(needle, candidate)))
}

/// Returns `true` when `text` contains `query` as a contiguous run of
/// characters. Case is ignored and diacritics are handled asymmetrically:
/// an unmarked query character matches the base letter and all its accented
/// forms, while a marked one matches only that exact form. An empty query
/// matches everything.
///
/// This is the "contains the word" filter used by the interactive table.
pub fn contains_ci(text: &str, query: &str) -> bool {
    let query: Vec<char> = query.chars().collect();
    if query.is_empty() {
        return true;
    }
    let text: Vec<char> = text.chars().collect();
    text.windows(query.len()).any(|window| {
        window
            .iter()
            .zip(&query)
            .all(|(&candidate, &needle)| char_matches(needle, candidate))
    })
}

/// Returns `true` when `text_char` matches `query_char`.
///
/// Case is always ignored. Diacritics are matched asymmetrically, because
/// Vietnamese titles are routinely typed without their marks: a query
/// character written without a diacritic matches the base letter and every
/// accented form of it (so "a" matches "a", "á", "à", "â", and so on),
/// while a query character written with a diacritic matches only that exact
/// form (so "à" matches only "à"). "đ" is the base letter "d" with a mark,
/// so "d" matches "đ" but "đ" matches only "đ".
fn char_matches(query_char: char, text_char: char) -> bool {
    let query_lower = lowercase(query_char);
    let text_lower = lowercase(text_char);
    if base(query_lower) == query_lower {
        // The query carries no diacritic: match the text character's base.
        base(text_lower) == query_lower
    } else {
        // The query carries a diacritic: require an exact match.
        text_lower == query_lower
    }
}

/// Lowercases a single character without allocating.
fn lowercase(character: char) -> char {
    // `to_lowercase` can expand to several characters (for example the
    // German sharp S). The inputs handled here never contain such
    // characters, so taking the first element keeps this a `char -> char`
    // mapping.
    character.to_lowercase().next().unwrap_or(character)
}

/// The unaccented base letter of an already-lowercased character. Every
/// accented Vietnamese letter maps to its base, "đ" maps to "d", and any
/// other character (plain ASCII, CJK) maps to itself.
fn base(lower: char) -> char {
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
