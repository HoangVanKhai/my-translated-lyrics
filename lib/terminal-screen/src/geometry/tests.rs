use crate::geometry::{Column, Height, Row, Width};
use pretty_assertions::assert_eq;

/// A grid holds every column before its width and none at or past it, so the
/// rightmost column of a grid `n` columns wide is `n - 1`.
#[test]
fn a_width_holds_the_columns_before_it() {
    let width = Width::new(3);
    assert!(width.contains(Column::LEFT));
    assert!(width.contains(Column::new(2)));
    assert!(!width.contains(Column::new(3)));
    assert!(!Width::ZERO.contains(Column::LEFT));
}

/// A height holds the rows before it on the same rule, so the bottom row of a
/// grid `n` rows tall is `n - 1`.
#[test]
fn a_height_holds_the_rows_before_it() {
    let height = Height::new(2);
    assert!(height.contains(Row::TOP));
    assert!(height.contains(Row::new(1)));
    assert!(!height.contains(Row::new(2)));
    assert!(!Height::ZERO.contains(Row::TOP));
}

/// A run fits when it ends at the right edge and not when it would cross it,
/// which is the rule that clips a wide glyph rather than splitting it.
#[test]
fn a_run_fits_when_it_ends_at_the_right_edge() {
    let width = Width::new(4);
    assert!(width.fits(Column::new(2), Width::new(2)));
    assert!(!width.fits(Column::new(3), Width::new(2)));
    // A run of no columns fits anywhere inside the grid, and at its edge.
    assert!(width.fits(Column::new(4), Width::ZERO));
}

/// A run wide enough to overflow a `u16` reads as not fitting rather than
/// wrapping around to a small sum.
#[test]
fn an_overflowing_run_does_not_fit() {
    assert!(!Width::new(u16::MAX).fits(Column::new(u16::MAX), Width::ONE));
}

/// Advancing a column past a run moves it by that many columns, and stops at
/// the last column rather than wrapping around.
#[test]
fn advancing_a_column_saturates_at_the_last_one() {
    assert_eq!(Column::LEFT + Width::new(2), Column::new(2));
    assert_eq!(Column::new(u16::MAX) + Width::ONE, Column::new(u16::MAX));
}

/// The columns of a grid run left to right, and the rows top to bottom.
#[test]
fn a_grid_enumerates_its_columns_and_rows_in_order() {
    let columns: Vec<Column> = Width::new(3).columns().collect();
    assert_eq!(columns, [Column::LEFT, Column::new(1), Column::new(2)]);
    let rows: Vec<Row> = Height::new(2).rows().collect();
    assert_eq!(rows, [Row::TOP, Row::new(1)]);
    assert_eq!(Width::ZERO.columns().count(), 0);
    assert_eq!(Height::ZERO.rows().count(), 0);
}

/// The rows from one downwards end at the last row a `u16` can name rather
/// than wrapping back to the top, so a caller laying out one row per item
/// runs out of rows instead of drawing over the rows it already filled.
#[test]
fn the_rows_downwards_from_one_end_at_the_last() {
    let first_three: Vec<Row> = Row::new(2).downwards().take(3).collect();
    assert_eq!(first_three, [Row::new(2), Row::new(3), Row::new(4)]);
    assert_eq!(
        Row::new(u16::MAX - 1).downwards().collect::<Vec<Row>>(),
        [Row::new(u16::MAX - 1), Row::new(u16::MAX)],
    );
    assert_eq!(Row::new(u16::MAX).downwards().count(), 1);
}
