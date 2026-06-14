//! Text matching primitives shared by the interactive selectors and the
//! command-line flag resolution.
//!
//! Two distinct matching strategies live here, because the issue asks for
//! two different behaviors:
//!
//! * The interactive table filters rows that *contain* the typed word.
//!   [`contains_ci`] implements that substring test.
//! * A command-line flag pre-selects a value by *fuzzy* matching, and the
//!   value must resolve to exactly one candidate. [`fuzzy_subsequence`]
//!   implements the subsequence test and [`resolve_unique`] enforces the
//!   "exactly one" rule.
//!
//! Both compare characters the same way: case and diacritics are matched
//! asymmetrically, so a generic query (uniform case, no marks) matches
//! broadly while a specific one (mixed case, or marks) matches exactly.

use derive_more::Display;

/// Returns `true` when every character of `query` appears in `text`, in
/// order but not necessarily contiguously. Case and diacritics are matched
/// asymmetrically (see the module docs). An empty query matches everything.
///
/// This is the "fuzzy" match used to resolve a command-line flag value to
/// a single candidate. For example, the query `cld` matches `celluloid`.
pub fn fuzzy_subsequence(query: &str, text: &str) -> bool {
    let case_insensitive = is_case_insensitive(query);
    let mut haystack = text.chars();
    query
        .chars()
        .all(|needle| haystack.any(|candidate| char_matches(needle, candidate, case_insensitive)))
}

/// Returns `true` when `text` contains `query` as a contiguous run of
/// characters. Case and diacritics are matched asymmetrically (see the
/// module docs). An empty query matches everything.
///
/// This is the "contains the word" filter used by the interactive table.
pub fn contains_ci(text: &str, query: &str) -> bool {
    let case_insensitive = is_case_insensitive(query);
    let query: Vec<char> = query.chars().collect();
    if query.is_empty() {
        return true;
    }
    let text: Vec<char> = text.chars().collect();
    text.windows(query.len()).any(|window| {
        window
            .iter()
            .zip(&query)
            .all(|(&candidate, &needle)| char_matches(needle, candidate, case_insensitive))
    })
}

/// Whether matching for `query` should ignore case.
///
/// A uniformly-cased query (all lowercase, all uppercase, or with no cased
/// letters at all) is treated as case-agnostic. A mixed-case query is taken
/// literally, because the user spelled out the case on purpose.
fn is_case_insensitive(query: &str) -> bool {
    let has_upper = query.chars().any(char::is_uppercase);
    let has_lower = query.chars().any(char::is_lowercase);
    !(has_upper && has_lower)
}

/// Returns `true` when `text_char` matches `query_char`.
///
/// Case and diacritics are matched asymmetrically, on the same principle:
/// what the user spelled out must match exactly, while what they left generic
/// matches broadly. A query character written without a diacritic matches the
/// base letter and every accented form of it (so "a" matches "a", "á", "à",
/// "â", and so on), while one written with a diacritic matches only that exact
/// form (so "à" matches only "à"); "đ" is the base "d" with a mark, so "d"
/// matches "đ" but "đ" matches only "đ". When `case_insensitive` is set the
/// comparison folds case; otherwise it is exact.
fn char_matches(query_char: char, text_char: char, case_insensitive: bool) -> bool {
    let (query_char, text_char) = if case_insensitive {
        (lowercase(query_char), lowercase(text_char))
    } else {
        (query_char, text_char)
    };
    if without_diacritics(query_char) == query_char {
        // The query carries no diacritic: match the text character's base.
        without_diacritics(text_char) == query_char
    } else {
        // The query carries a diacritic: require an exact match.
        text_char == query_char
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

/// The same letter with any diacritics removed, keeping its case. Every
/// accented Vietnamese letter maps to its base, "đ" maps to "d", and any
/// other character (plain ASCII, CJK) is returned unchanged.
fn without_diacritics(character: char) -> char {
    let stripped = match lowercase(character) {
        'à' | 'á' | 'ả' | 'ã' | 'ạ' | 'â' | 'ầ' | 'ấ' | 'ẩ' | 'ẫ' | 'ậ' | 'ă' | 'ằ' | 'ắ' | 'ẳ'
        | 'ẵ' | 'ặ' => 'a',
        'è' | 'é' | 'ẻ' | 'ẽ' | 'ẹ' | 'ê' | 'ề' | 'ế' | 'ể' | 'ễ' | 'ệ' => 'e',
        'ì' | 'í' | 'ỉ' | 'ĩ' | 'ị' => 'i',
        'ò' | 'ó' | 'ỏ' | 'õ' | 'ọ' | 'ô' | 'ồ' | 'ố' | 'ổ' | 'ỗ' | 'ộ' | 'ơ' | 'ờ' | 'ớ' | 'ở'
        | 'ỡ' | 'ợ' => 'o',
        'ù' | 'ú' | 'ủ' | 'ũ' | 'ụ' | 'ư' | 'ừ' | 'ứ' | 'ử' | 'ữ' | 'ự' => 'u',
        'ỳ' | 'ý' | 'ỷ' | 'ỹ' | 'ỵ' => 'y',
        'đ' => 'd',
        // No diacritic: keep the character as-is, preserving its case.
        _ => return character,
    };
    if character.is_uppercase() {
        stripped.to_uppercase().next().unwrap_or(stripped)
    } else {
        stripped
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
