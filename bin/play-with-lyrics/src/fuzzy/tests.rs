// cspell:ignore cloudside clld vbmt biên mộng thoại

use crate::fuzzy::{ResolveError, contains_ci, fuzzy_subsequence, resolve_unique};
use pretty_assertions::assert_eq;

#[test]
fn subsequence_matches_in_order() {
    assert!(fuzzy_subsequence("clld", "celluloid"));
    assert!(fuzzy_subsequence("mpv", "mpv"));
    assert!(fuzzy_subsequence("", "anything"));
}

#[test]
fn subsequence_is_case_insensitive() {
    assert!(fuzzy_subsequence("MPV", "mpv"));
    assert!(fuzzy_subsequence("Cloud", "cloudside dreams"));
}

#[test]
fn subsequence_rejects_out_of_order_or_missing() {
    assert!(!fuzzy_subsequence("vpm", "mpv"));
    assert!(!fuzzy_subsequence("xyz", "celluloid"));
}

#[test]
fn contains_is_substring_not_subsequence() {
    assert!(contains_ci("cloudside dreams", "side"));
    assert!(contains_ci("Vân Biên Mộng Thoại", "biên"));
    assert!(!contains_ci("celluloid", "clld"));
    assert!(contains_ci("anything", ""));
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
    let error = resolve_unique("vlc", &items, |item| vec![*item]).unwrap_err();
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
    let items = ["cloudside dreams"];
    // The second key is matched even though the first does not.
    let resolved = resolve_unique("vbmt", &items, |item| vec![*item, "Vân Biên Mộng Thoại"]);
    assert!(resolved.is_ok());
}
