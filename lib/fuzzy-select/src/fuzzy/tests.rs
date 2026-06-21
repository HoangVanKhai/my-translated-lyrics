use crate::fuzzy::{
    ResolveError, contains_substring, fuzzy_subsequence, match_mask, resolve_unique,
};
use pretty_assertions::assert_eq;

/// The indices a highlight mask marks, for readable assertions.
fn marked(mask: &[bool]) -> Vec<usize> {
    mask.iter()
        .enumerate()
        .filter(|&(_, &on)| on)
        .map(|(index, _)| index)
        .collect()
}

#[test]
fn subsequence_matches_in_order() {
    assert!(fuzzy_subsequence("cell", "celluloid"));
    assert!(fuzzy_subsequence("mpv", "mpv"));
    assert!(fuzzy_subsequence("", "anything"));
}

/// An all-lowercase or all-uppercase query matches any case of the text.
#[test]
fn a_uniformly_cased_query_folds_case() {
    for query in ["foo bar baz", "FOO BAR BAZ"] {
        for text in ["Foo Bar Baz", "foo bar baz", "FOO BAR BAZ"] {
            let matched = contains_substring(text, query);
            assert!(matched, "{query:?} should match {text:?}");
        }
    }
    assert!(fuzzy_subsequence("MPV", "mpv"));
    assert!(fuzzy_subsequence("cell", "CELLULOID"));
}

/// A deliberately mixed-case query matches only that exact case.
#[test]
fn a_mixed_case_query_is_taken_literally() {
    assert!(contains_substring("Foo Bar Baz", "Foo Bar Baz"));
    assert!(!contains_substring("foo bar baz", "Foo Bar Baz"));
    assert!(!contains_substring("FOO BAR BAZ", "Foo Bar Baz"));
}

#[test]
fn subsequence_rejects_out_of_order_or_missing() {
    assert!(!fuzzy_subsequence("vpm", "mpv"));
    assert!(!fuzzy_subsequence("xyz", "celluloid"));
}

#[test]
fn contains_is_substring_not_subsequence() {
    assert!(contains_substring("celluloid", "cell"));
    // "cld" is a subsequence of "celluloid" but not a contiguous
    // substring, so the substring test rejects what the fuzzy test accepts.
    assert!(fuzzy_subsequence("cld", "celluloid"));
    assert!(!contains_substring("celluloid", "cld"));
    assert!(contains_substring("anything", ""));
}

/// Typing without diacritics finds a title that carries them.
#[test]
fn an_unmarked_query_matches_marked_text() {
    // cspell:locale en vi
    assert!(contains_substring("Mưa Xuân", "mua xuan"));
    assert!(contains_substring("Mùa Xuân", "mua xuan"));
    assert!(fuzzy_subsequence("mua", "Mưa Xuân"));
    assert!(fuzzy_subsequence("mua", "Mùa Xuân"));
}

/// Typing a diacritic narrows the match to exactly that form, so it
/// matches a title that carries the mark but not a bare one.
#[test]
fn a_marked_query_matches_only_that_mark() {
    // cspell:locale en vi
    assert!(contains_substring("Mưa Xuân", "mưa"));
    assert!(!contains_substring("Mua Xuan", "mưa"));
    assert!(contains_substring("Mùa Xuân", "mùa"));
    assert!(!contains_substring("Mua Xuan", "mùa"));
    assert!(fuzzy_subsequence("xuân", "Mưa Xuân"));
    assert!(!fuzzy_subsequence("xuân", "Mua Xuan"));
    assert!(fuzzy_subsequence("xuân", "Mùa Xuân"));
    assert!(!fuzzy_subsequence("xuân", "Mua Xuan"));
}

#[test]
fn resolve_unique_returns_the_single_match() {
    let items = ["mpv", "celluloid"];
    let (index, value) = resolve_unique("cell", &items, |item| vec![*item]).unwrap();
    assert_eq!(index, 1);
    assert_eq!(*value, "celluloid");
}

#[test]
fn resolve_unique_reports_no_match() {
    let items = ["mpv", "celluloid"];
    let error = resolve_unique("xyz", &items, |item| vec![*item]).unwrap_err();
    assert_eq!(error, ResolveError::NoMatch);
}

#[test]
fn resolve_unique_reports_ambiguity() {
    let items = ["english", "spanish"];
    // "s" is a subsequence of both candidates.
    let error = resolve_unique("s", &items, |item| vec![*item]).unwrap_err();
    assert_eq!(error, ResolveError::Ambiguous);
}

#[test]
fn resolve_unique_matches_against_any_key() {
    let items = ["alpha"];
    // The second key is matched even though the first does not.
    let resolved = resolve_unique("beta", &items, |item| vec![*item, "beta gamma"]);
    assert!(resolved.is_ok());
}

/// A query of only spaces carries no filter and matches every row.
#[test]
fn a_whitespace_only_query_matches_everything() {
    assert!(contains_substring("Abc Def Ghi", " "));
    assert!(contains_substring("Abc Def Ghi", "  "));
    assert!(contains_substring("Abc Def Ghi", "   "));
}

/// Runs of spaces inside a query collapse to one, so loosely typed spacing
/// matches the same rows as the single-spaced form.
#[test]
fn runs_of_spaces_in_a_query_collapse_to_one() {
    assert!(contains_substring("Abc Def", "abc     def"));
    assert!(contains_substring("Abc Def", "abc def"));
}

/// A query with no spaces ignores the spacing of the text, so a run-together
/// query still finds a multi-word title.
#[test]
fn a_spaceless_query_matches_across_word_boundaries() {
    assert!(contains_substring("Abc Def Ghi", "abcdefghi"));
    assert!(contains_substring("Abc Def Ghi", "abcdef"));
}

/// A query that keeps a space treats the spacing as deliberate, so it only
/// matches text spaced the same way.
#[test]
fn a_spaced_query_requires_the_spacing_to_match() {
    // The exact spacing matches.
    assert!(contains_substring("Abc Def Ghi", "abc def ghi"));
    // A space in a place the title does not have one does not match.
    assert!(!contains_substring("Abc Def Ghi", "abcdef ghi"));
}

/// A single-character query marks every occurrence for highlighting.
#[test]
fn match_mask_marks_every_single_character_occurrence() {
    // cspell:words booo
    // "Foo Bo Booo" carries six "o" letters across its three words.
    let marks = marked(&match_mask("Foo Bo Booo", "o"));
    assert_eq!(marks, vec![1, 2, 5, 8, 9, 10]);
}

/// A multi-character query marks non-overlapping runs, scanning left to
/// right, so "oo" marks the pair in "Foo" and one pair in "Booo".
#[test]
fn match_mask_marks_non_overlapping_runs() {
    // cspell:words booo
    assert_eq!(marked(&match_mask("Foo Bo Booo", "oo")), vec![1, 2, 8, 9]);
    // "ooo" is long enough only for the run in "Booo".
    assert_eq!(marked(&match_mask("Foo Bo Booo", "ooo")), vec![8, 9, 10]);
}

/// An empty or whitespace-only query marks nothing.
#[test]
fn match_mask_marks_nothing_for_an_empty_query() {
    assert!(match_mask("Abc Def", "").iter().all(|&on| !on));
    assert!(match_mask("Abc Def", "   ").iter().all(|&on| !on));
}

/// A query with no spaces marks across word boundaries but leaves the
/// skipped space unmarked.
#[test]
fn match_mask_marks_across_word_boundaries() {
    let marks = marked(&match_mask("Abc Def", "abcdef"));
    assert_eq!(marks, vec![0, 1, 2, 4, 5, 6]);
}

/// A query that keeps a space marks the space in place along with the rest.
#[test]
fn match_mask_marks_a_spaced_query_in_place() {
    let marks = marked(&match_mask("Abc Def", "abc def"));
    assert_eq!(marks, vec![0, 1, 2, 3, 4, 5, 6]);
}
