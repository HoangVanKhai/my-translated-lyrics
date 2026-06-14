// cspell:ignore mưa xuân xuan

use crate::fuzzy::{ResolveError, contains_ci, fuzzy_subsequence, resolve_unique};
use pretty_assertions::assert_eq;

#[test]
fn subsequence_matches_in_order() {
    assert!(fuzzy_subsequence("cell", "celluloid"));
    assert!(fuzzy_subsequence("mpv", "mpv"));
    assert!(fuzzy_subsequence("", "anything"));
}

#[test]
fn a_uniformly_cased_query_folds_case() {
    // An all-lowercase or all-uppercase query matches any case of the text.
    for query in ["foo bar baz", "FOO BAR BAZ"] {
        for text in ["Foo Bar Baz", "foo bar baz", "FOO BAR BAZ"] {
            assert!(contains_ci(text, query), "{query:?} should match {text:?}");
        }
    }
    assert!(fuzzy_subsequence("MPV", "mpv"));
    assert!(fuzzy_subsequence("cell", "CELLULOID"));
}

#[test]
fn a_mixed_case_query_is_taken_literally() {
    // A deliberately mixed-case query matches only that exact case.
    assert!(contains_ci("Foo Bar Baz", "Foo Bar Baz"));
    assert!(!contains_ci("foo bar baz", "Foo Bar Baz"));
    assert!(!contains_ci("FOO BAR BAZ", "Foo Bar Baz"));
}

#[test]
fn subsequence_rejects_out_of_order_or_missing() {
    assert!(!fuzzy_subsequence("vpm", "mpv"));
    assert!(!fuzzy_subsequence("xyz", "celluloid"));
}

#[test]
fn contains_is_substring_not_subsequence() {
    assert!(contains_ci("celluloid", "cell"));
    // "cld" is a subsequence of "celluloid" but not a contiguous
    // substring, so the substring test rejects what the fuzzy test accepts.
    assert!(fuzzy_subsequence("cld", "celluloid"));
    assert!(!contains_ci("celluloid", "cld"));
    assert!(contains_ci("anything", ""));
}

#[test]
fn an_unmarked_query_matches_marked_text() {
    // Typing without diacritics finds a title that carries them.
    assert!(contains_ci("Mưa Xuân", "mua xuan"));
    assert!(fuzzy_subsequence("mua", "Mưa Xuân"));
}

#[test]
fn a_marked_query_matches_only_that_mark() {
    // Typing a diacritic narrows the match to exactly that form, so it
    // matches a title that carries the mark but not a bare one.
    assert!(contains_ci("Mưa Xuân", "mưa"));
    assert!(!contains_ci("Mua Xuan", "mưa"));
    assert!(fuzzy_subsequence("xuân", "Mưa Xuân"));
    assert!(!fuzzy_subsequence("xuân", "Mua Xuan"));
}

#[test]
fn resolve_unique_returns_the_single_match() {
    let items = ["mpv", "celluloid"];
    let resolved = resolve_unique("cell", &items, |item| vec![*item]).unwrap();
    assert_eq!(*resolved, "celluloid");
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
