// cspell:ignore mưa xuân trăng thu

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

// The three placeholder titles say the same thing in each language:
// "Spring Rain" (春雨 / Mưa Xuân), "Spring Moon" (春月 / Trăng Xuân), and
// "Autumn Moon" (秋月 / Trăng Thu). They deliberately share words so a
// query can match one row or several.
fn sample() -> Vec<Row> {
    vec![
        Row {
            en: "Spring Rain",
            vi: "Mưa Xuân",
            zh: "春雨",
        },
        Row {
            en: "Spring Moon",
            vi: "Trăng Xuân",
            zh: "春月",
        },
        Row {
            en: "Autumn Moon",
            vi: "Trăng Thu",
            zh: "秋月",
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
    for character in "mưa".chars() {
        selector.push_char(character);
    }
    // "mưa" appears in the Vietnamese title of the first row only, so a
    // match comes from a column other than the English title.
    assert_eq!(selector.filtered(), &[0]);
    assert_eq!(selector.selected_index(), Some(0));
    assert_eq!(rows[0].en, "Spring Rain");
}

#[test]
fn filtering_matches_a_non_english_column() {
    let rows = sample();
    let mut selector = Selector::new(&rows);
    for character in "trăng".chars() {
        selector.push_char(character);
    }
    // The Vietnamese titles of the second and third rows both contain it.
    assert_eq!(selector.filtered(), &[1, 2]);
}

#[test]
fn backspacing_restores_rows() {
    let rows = sample();
    let mut selector = Selector::new(&rows);
    for character in "rain".chars() {
        selector.push_char(character);
    }
    assert_eq!(selector.filtered(), &[0]);
    for _ in 0..4 {
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
    // Every title contains an "n", so the rows stay visible and only the
    // cursor resets.
    selector.push_char('n');
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
