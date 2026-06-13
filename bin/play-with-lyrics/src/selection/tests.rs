// cspell:ignore cloudside biên mộng thoại nguyệt luân hoàng

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
            en: "Cloudside Dreams",
            vi: "Vân Biên Mộng Thoại",
            zh: "云边梦话",
        },
        Row {
            en: "Lunar Cycle",
            vi: "Nguyệt Luân Hồi",
            zh: "月轮回",
        },
        Row {
            en: "Moon over the Yellow Tower",
            vi: "Hoàng Lâu Nguyệt",
            zh: "黄楼月",
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
    for character in "moon".chars() {
        selector.push_char(character);
    }
    // "moon" appears in the English title of the third row only.
    assert_eq!(selector.filtered(), &[2]);
    assert_eq!(selector.selected_index(), Some(2));
    assert_eq!(rows[2].en, "Moon over the Yellow Tower");
}

#[test]
fn filtering_matches_a_non_english_column() {
    let rows = sample();
    let mut selector = Selector::new(&rows);
    for character in "nguyệt".chars() {
        selector.push_char(character);
    }
    // The Vietnamese titles of the second and third rows both contain it.
    assert_eq!(selector.filtered(), &[1, 2]);
}

#[test]
fn backspacing_restores_rows() {
    let rows = sample();
    let mut selector = Selector::new(&rows);
    for character in "lunar".chars() {
        selector.push_char(character);
    }
    assert_eq!(selector.filtered(), &[1]);
    for _ in 0..5 {
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
    selector.push_char('o');
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
