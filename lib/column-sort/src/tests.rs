use super::{ColumnSort, Direction};
use core::cmp::Ordering;
use pretty_assertions::assert_eq;

use Direction::{Ascending, Descending};

/// Clicking promotes a column, inverts the leading one, and keeps the demoted
/// columns' order and directions, following the worked example.
#[test]
fn clicking_headers_reorders_and_inverts() {
    let mut sort = ColumnSort::new(["en", "vi", "zh"]);
    assert_eq!(
        sort.order(),
        &[("en", Ascending), ("vi", Ascending), ("zh", Ascending)],
    );

    // Promoting a non-leading column sets it ascending and keeps the rest.
    sort.click("vi");
    assert_eq!(
        sort.order(),
        &[("vi", Ascending), ("en", Ascending), ("zh", Ascending)],
    );

    // Clicking the leading column inverts its direction.
    sort.click("vi");
    assert_eq!(
        sort.order(),
        &[("vi", Descending), ("en", Ascending), ("zh", Ascending)],
    );

    // Promoting another column keeps the demoted column's inverted direction.
    sort.click("zh");
    assert_eq!(
        sort.order(),
        &[("zh", Ascending), ("vi", Descending), ("en", Ascending)],
    );

    // Promoting a descending column resets it to ascending.
    sort.click("vi");
    assert_eq!(
        sort.order(),
        &[("vi", Ascending), ("zh", Ascending), ("en", Ascending)],
    );
}

/// Clicking the leading column a second time inverts it from descending back
/// to ascending.
#[test]
fn clicking_the_leading_column_twice_returns_to_ascending() {
    let mut sort = ColumnSort::new(["en", "vi"]);
    sort.click("en");
    assert_eq!(sort.order()[0], ("en", Descending));
    sort.click("en");
    assert_eq!(sort.order()[0], ("en", Ascending));
}

/// A function standing in for a row, mapping each column to its cell.
fn row(en: &'static str, vi: &'static str) -> impl Fn(&'static str) -> &'static str {
    move |column| match column {
        "en" => en,
        "vi" => vi,
        _ => "",
    }
}

/// The leading column decides; the next is consulted only on a tie.
#[test]
fn compare_falls_through_to_the_next_column_on_a_tie() {
    let sort = ColumnSort::new(["en", "vi"]);
    // Different English titles: the English column decides.
    let by_english = sort.compare(row("apple", "z"), row("banana", "a"));
    assert_eq!(by_english, Ordering::Less);
    // Equal English titles: the Vietnamese column breaks the tie.
    let by_vietnamese = sort.compare(row("apple", "z"), row("apple", "a"));
    assert_eq!(by_vietnamese, Ordering::Greater);
    // Equal in every column.
    let identical = sort.compare(row("apple", "a"), row("apple", "a"));
    assert_eq!(identical, Ordering::Equal);
}

/// Comparison ignores case, matching the earlier sort.
#[test]
fn compare_ignores_case() {
    let sort = ColumnSort::new(["en"]);
    let same_word = sort.compare(row("Apple", ""), row("apple", ""));
    assert_eq!(same_word, Ordering::Equal);
    let earlier_word = sort.compare(row("Apple", ""), row("banana", ""));
    assert_eq!(earlier_word, Ordering::Less);
}

/// A present cell sorts before an empty one when ascending.
#[test]
fn a_present_cell_sorts_before_an_empty_one() {
    let sort = ColumnSort::new(["en"]);
    let present_before_empty = sort.compare(row("apple", ""), row("", ""));
    assert_eq!(present_before_empty, Ordering::Less);
}

/// Ascending puts the empty cell last; descending puts it first.
#[test]
fn empty_cells_fall_to_the_far_end() {
    let mut sort = ColumnSort::new(["en"]);
    // Ascending: the empty English title sorts after a present one.
    let ascending = sort.compare(row("", "x"), row("apple", "y"));
    assert_eq!(ascending, Ordering::Greater);
    // Descending: the empty English title sorts before a present one.
    sort.click("en");
    assert!(sort.order()[0].1 == Descending);
    let descending = sort.compare(row("", "x"), row("apple", "y"));
    assert_eq!(descending, Ordering::Less);
}
