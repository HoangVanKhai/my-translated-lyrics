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
fn subsequence_is_case_insensitive() {
    assert!(fuzzy_subsequence("MPV", "mpv"));
    assert!(fuzzy_subsequence("CELL", "celluloid"));
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
fn matching_ignores_diacritics() {
    // A Vietnamese title may be typed with or without its marks, in either
    // direction, for both the substring filter and the fuzzy match.
    assert!(contains_ci("Mưa Xuân", "mua xuan"));
    assert!(contains_ci("mua xuan", "Mưa Xuân"));
    assert!(fuzzy_subsequence("mua", "Mưa Xuân"));
    assert!(fuzzy_subsequence("mưa", "Mua Xuan"));
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
