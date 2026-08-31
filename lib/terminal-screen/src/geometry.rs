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

    /// A span of `rows` rows.
    pub const fn new(rows: u16) -> Height {
        Height(rows)
    }

    /// The number of rows spanned.
    pub const fn get(self) -> u16 {
        self.0
    }

    /// Every row of a grid this tall, from top to bottom.
    pub fn rows(self) -> impl Iterator<Item = Row> {
        (0..self.0).map(Row)
    }

    /// Whether `row` lies inside a grid this tall.
    pub fn contains(self, row: Row) -> bool {
        row.0 < self.0
    }
}

impl From<Height> for usize {
    fn from(height: Height) -> usize {
        usize::from(height.0)
    }
}

#[cfg(test)]
mod tests;
