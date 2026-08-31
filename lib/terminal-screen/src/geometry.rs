//! The coordinates and extents of the terminal grid.
//!
//! A cell is located by a [`Column`] and a [`Row`]; a region is measured by a
//! [`Width`] and a [`Height`]. All four wrap the same `u16`, so while they were
//! spelled as bare integers the compiler accepted any of them wherever another
//! belonged. A transposed pair of arguments is now a compile error rather than
//! a frame drawn into the wrong cells.

use std::ops::{Add, AddAssign};

/// A zero-based column, counted from the left edge of the grid.
#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct Column(u16);

impl Column {
    /// The leftmost column of any grid.
    pub const LEFT: Column = Column(0);

    /// The column `offset` columns from the left edge.
    pub const fn new(offset: u16) -> Column {
        Column(offset)
    }

    /// The column's distance from the left edge.
    pub const fn get(self) -> u16 {
        self.0
    }

    /// How many columns to the right of `origin` this column sits, or `None`
    /// when it sits to the left of `origin`. This is the inverse of advancing
    /// a column past a run, and the only way to recover a width from two
    /// positions.
    pub fn columns_after(self, origin: Column) -> Option<Width> {
        self.0.checked_sub(origin.0).map(Width)
    }
}

/// Advances past a glyph or a run of text that spans `width` columns, which is
/// the only arithmetic a column takes part in. The sum saturates at the last
/// column a `u16` can name, so a caller laying out text past the right edge
/// keeps moving rightwards rather than wrapping back to the left one.
impl Add<Width> for Column {
    type Output = Column;

    fn add(self, width: Width) -> Column {
        Column(self.0.saturating_add(width.0))
    }
}

impl AddAssign<Width> for Column {
    fn add_assign(&mut self, width: Width) {
        *self = *self + width;
    }
}

impl From<Column> for usize {
    fn from(column: Column) -> usize {
        usize::from(column.0)
    }
}

/// A zero-based row, counted from the top edge of the grid.
#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct Row(u16);

impl Row {
    /// The topmost row of any grid.
    pub const TOP: Row = Row(0);

    /// The row `offset` rows from the top edge.
    pub const fn new(offset: u16) -> Row {
        Row(offset)
    }

    /// The row's distance from the top edge.
    pub const fn get(self) -> u16 {
        self.0
    }

    /// How many rows below `origin` this row sits, or `None` when it sits
    /// above `origin`. This is the only way to recover a height from two
    /// positions, and it is what turns the row a click landed on into an
    /// offset from the first row of a block.
    pub fn rows_below(self, origin: Row) -> Option<Height> {
        self.0.checked_sub(origin.0).map(Height)
    }

    /// This row and every row below it, ending at the last row a `u16` can
    /// name. A caller laying out one screen row per item zips against this,
    /// so a list longer than the grid runs out of rows rather than wrapping
    /// back to the top.
    pub fn downwards(self) -> impl Iterator<Item = Row> {
        (self.0..=u16::MAX).map(Row)
    }
}

impl From<Row> for usize {
    fn from(row: Row) -> usize {
        usize::from(row.0)
    }
}

/// A number of columns: the width of a grid, or the span of a glyph or a run
/// of text.
#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct Width(u16);

impl Width {
    /// No columns at all, which is what a zero-width glyph spans and what an
    /// unsized grid starts out as.
    pub const ZERO: Width = Width(0);

    /// A single column, the span of an ordinary narrow glyph.
    pub const ONE: Width = Width(1);

    /// A span of `columns` columns.
    pub const fn new(columns: u16) -> Width {
        Width(columns)
    }

    /// The number of columns spanned.
    pub const fn get(self) -> u16 {
        self.0
    }

    /// Every column of a grid this wide, from left to right.
    pub fn columns(self) -> impl Iterator<Item = Column> {
        (0..self.0).map(Column)
    }

    /// Whether `column` lies inside a grid this wide.
    pub fn contains(self, column: Column) -> bool {
        column.0 < self.0
    }

    /// Whether a run that starts at `column` and spans `span` columns ends
    /// within a grid this wide. The sum is taken in `usize` so a run that would
    /// overflow a `u16` reads as not fitting rather than wrapping around.
    pub fn fits(self, column: Column, span: Width) -> bool {
        usize::from(column) + usize::from(span) <= usize::from(self)
    }

    /// How much of a grid this wide is left from `column` rightwards, which is
    /// the room a caller has for more text after drawing a prefix. A column at
    /// or past the right edge leaves no room.
    pub fn remaining_from(self, column: Column) -> Width {
        Width(self.0.saturating_sub(column.0))
    }

    /// This width less `other`, or no columns at all when `other` is the
    /// wider of the two.
    pub fn saturating_sub(self, other: Width) -> Width {
        Width(self.0.saturating_sub(other.0))
    }
}

/// Lays two runs side by side. The sum saturates at the widest a `u16` can
/// measure, so an absurdly long run reads as reaching the far edge rather than
/// wrapping around to a narrow one.
impl Add for Width {
    type Output = Width;

    fn add(self, other: Width) -> Width {
        Width(self.0.saturating_add(other.0))
    }
}

impl AddAssign for Width {
    fn add_assign(&mut self, other: Width) {
        *self = *self + other;
    }
}

impl From<Width> for usize {
    fn from(width: Width) -> usize {
        usize::from(width.0)
    }
}

/// A number of rows: the height of a grid.
#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct Height(u16);

impl Height {
    /// No rows at all, which is what an unsized grid starts out as.
    pub const ZERO: Height = Height(0);

    /// A single row.
    pub const ONE: Height = Height(1);

    /// A span of `rows` rows.
    pub const fn new(rows: u16) -> Height {
        Height(rows)
    }

    /// Every row of a grid this tall, from top to bottom.
    pub fn rows(self) -> impl Iterator<Item = Row> {
        (0..self.0).map(Row)
    }

    /// Whether `row` lies inside a grid this tall.
    pub fn contains(self, row: Row) -> bool {
        row.0 < self.0
    }

    /// This height less `other`, or no rows at all when `other` is the taller
    /// of the two.
    pub fn saturating_sub(self, other: Height) -> Height {
        Height(self.0.saturating_sub(other.0))
    }
}

impl From<Height> for usize {
    fn from(height: Height) -> usize {
        usize::from(height.0)
    }
}

#[cfg(test)]
mod tests;
