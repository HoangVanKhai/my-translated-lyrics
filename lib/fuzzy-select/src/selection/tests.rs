// cspell:locale en vi

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

// Each placeholder title says the same thing in every language:
// "Example Song" (示例歌曲 / Bài Hát Ví Dụ), "Sample Song" (样本歌曲 /
// Bài Hát Mẫu), and "Sample Tune" (样本曲调 / Giai Điệu Mẫu). They
// deliberately share words so a query can match one row or several.
fn sample() -> Vec<Row> {
    vec![
        Row {
            en: "Example Song",
            vi: "Bài Hát Ví Dụ",
            zh: "示例歌曲",
        },
        Row {
            en: "Sample Song",
            vi: "Bài Hát Mẫu",
            zh: "样本歌曲",
        },
        Row {
            en: "Sample Tune",
            vi: "Giai Điệu Mẫu",
            zh: "样本曲调",
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
    for char in "ví".chars() {
        selector.push_char(char);
    }
    // "ví" appears in the Vietnamese title of the first row only, so a
    // match comes from a column other than the English title.
    assert_eq!(selector.filtered(), &[0]);
    assert_eq!(selector.selected_index(), Some(0));
    assert_eq!(rows[0].en, "Example Song");
}

#[test]
fn filtering_ignores_diacritics() {
    let rows = sample();
    let mut selector = Selector::new(&rows);
    // The Vietnamese title "Bài Hát Ví Dụ" may be typed without its marks.
    for char in "vi du".chars() {
        selector.push_char(char);
    }
    assert_eq!(selector.filtered(), &[0]);
}

#[test]
fn filtering_matches_a_non_english_column() {
    let rows = sample();
    let mut selector = Selector::new(&rows);
    for char in "mẫu".chars() {
        selector.push_char(char);
    }
    // The Vietnamese titles of the second and third rows both contain it.
    assert_eq!(selector.filtered(), &[1, 2]);
}

#[test]
fn backspacing_restores_rows() {
    let rows = sample();
    let mut selector = Selector::new(&rows);
    for char in "example".chars() {
        selector.push_char(char);
    }
    assert_eq!(selector.filtered(), &[0]);
    for _ in 0.."example".len() {
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
    // Every English title contains an "e", so the rows stay visible and
    // only the cursor resets.
    selector.push_char('e');
    assert_eq!(selector.cursor(), 0);
}

#[test]
fn no_match_leaves_nothing_selected() {
    let rows = sample();
    let mut selector = Selector::new(&rows);
    for char in "zzz".chars() {
        selector.push_char(char);
    }
    assert!(selector.filtered().is_empty());
    assert!(selector.selected_index().is_none());
}
