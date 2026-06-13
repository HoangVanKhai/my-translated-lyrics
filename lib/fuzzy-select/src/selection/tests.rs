use crate::selection::{Searchable, Selector};
use pretty_assertions::assert_eq;

struct Row {
    en: &'static str,
    vi: &'static str,
    zh: &'static str,
}

impl Searchable for Row {
    fn search_keys(&self) -> Vec<&str> {
        vec![self.en, self.vi, self.zh]
    }
}

fn sample() -> Vec<Row> {
    vec![
        Row {
            en: "Example Song One",
            vi: "Example Translation One",
            zh: "示例歌曲一",
        },
        Row {
            en: "Example Song Two",
            vi: "Example Translation Two",
            zh: "示例歌曲二",
        },
        Row {
            en: "Sample Tune Three",
            vi: "Sample Melody Three",
            zh: "示例歌曲三",
        },
    ]
}

#[test]
fn empty_query_shows_every_row() {
    let rows = sample();
    let selector = Selector::new(&rows);
    assert_eq!(selector.filtered(), &[0, 1, 2]);
    assert_eq!(selector.cursor(), 0);
}

#[test]
fn typing_filters_by_substring_across_all_columns() {
    let rows = sample();
    let mut selector = Selector::new(&rows);
    for character in "melody".chars() {
        selector.push_char(character);
    }
    // "melody" appears in the Vietnamese title of the third row only, so a
    // match comes from a column other than the English title.
    assert_eq!(selector.filtered(), &[2]);
    assert_eq!(selector.selected_index(), Some(2));
    assert_eq!(rows[2].en, "Sample Tune Three");
}

#[test]
fn filtering_matches_a_non_english_column() {
    let rows = sample();
    let mut selector = Selector::new(&rows);
    for character in "translation".chars() {
        selector.push_char(character);
    }
    // The Vietnamese titles of the first and second rows both contain it.
    assert_eq!(selector.filtered(), &[0, 1]);
}

#[test]
fn backspacing_restores_rows() {
    let rows = sample();
    let mut selector = Selector::new(&rows);
    for character in "one".chars() {
        selector.push_char(character);
    }
    assert_eq!(selector.filtered(), &[0]);
    for _ in 0..3 {
        selector.pop_char();
    }
    assert_eq!(selector.query(), "");
    assert_eq!(selector.filtered(), &[0, 1, 2]);
}

#[test]
fn cursor_moves_within_bounds() {
    let rows = sample();
    let mut selector = Selector::new(&rows);
    selector.move_up(); // already at the top, stays put
    assert_eq!(selector.cursor(), 0);
    selector.move_down();
    selector.move_down();
    assert_eq!(selector.cursor(), 2);
    selector.move_down(); // already at the bottom, stays put
    assert_eq!(selector.cursor(), 2);
    selector.move_up();
    assert_eq!(selector.cursor(), 1);
}

#[test]
fn refiltering_resets_the_cursor_to_the_top() {
    let rows = sample();
    let mut selector = Selector::new(&rows);
    selector.move_down();
    selector.move_down();
    assert_eq!(selector.cursor(), 2);
    selector.push_char('e');
    assert_eq!(selector.cursor(), 0);
}

#[test]
fn no_match_leaves_nothing_selected() {
    let rows = sample();
    let mut selector = Selector::new(&rows);
    for character in "zzz".chars() {
        selector.push_char(character);
    }
    assert!(selector.filtered().is_empty());
    assert!(selector.selected_index().is_none());
}
