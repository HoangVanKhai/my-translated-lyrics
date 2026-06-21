#![cfg_attr(dylint_lib = "perfectionist", feature(register_tool))]
#![cfg_attr(dylint_lib = "perfectionist", register_tool(perfectionist))]

//! Multi-column sorting whose priority and direction the user changes by
//! clicking column headers.
//!
//! A [`ColumnSort`] holds the columns in priority order, each with an
//! ascending or descending [`Direction`]. Clicking a column promotes it to the
//! highest priority, or, when it is already highest, inverts its direction.
//! [`ColumnSort::compare`] orders two rows by the columns in turn, so the first
//! column that distinguishes them decides, and the rest are compared only on a
//! tie.
//!
//! Cells are strings. Ascending order places empty cells last and descending
//! places them first, so rows without a value in the leading column fall to the
//! far end either way.

use core::cmp::Ordering;

/// Whether a column is sorted ascending or descending.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Ascending,
    Descending,
}

impl Direction {
    /// The opposite direction.
    pub fn reversed(self) -> Direction {
        match self {
            Direction::Ascending => Direction::Descending,
            Direction::Descending => Direction::Ascending,
        }
    }
}

/// The columns to sort by, in priority order with the highest first, each
/// paired with its direction. `Column` is the caller's column identifier.
#[derive(Debug, Clone)]
pub struct ColumnSort<Column> {
    priorities: Vec<(Column, Direction)>,
}

impl<Column> ColumnSort<Column>
where
    Column: Copy + PartialEq,
{
    /// A sort over `columns` in the given priority order, each ascending.
    pub fn new(columns: impl IntoIterator<Item = Column>) -> Self {
        ColumnSort {
            priorities: columns
                .into_iter()
                .map(|column| (column, Direction::Ascending))
                .collect(),
        }
    }

    /// Applies a click on `column`. Clicking the highest-priority column
    /// inverts its direction; clicking any other column promotes it to the
    /// highest priority, ascending, while the rest keep their order and their
    /// directions.
    pub fn click(&mut self, column: Column) {
        if let Some(top) = self.priorities.first_mut()
            && top.0 == column
        {
            top.1 = top.1.reversed();
            return;
        }
        let position = self
            .priorities
            .iter()
            .position(|&(candidate, _)| candidate == column);
        if let Some(position) = position {
            let (promoted, _) = self.priorities.remove(position);
            self.priorities.insert(0, (promoted, Direction::Ascending));
        }
    }

    /// The columns in priority order, the highest first, with their directions.
    pub fn order(&self) -> &[(Column, Direction)] {
        &self.priorities
    }

    /// Orders two rows. For each column in priority order, `left` and `right`
    /// supply that row's cell; the first column whose cells differ decides, and
    /// the rest are consulted only on a tie.
    pub fn compare<'cell>(
        &self,
        left: impl Fn(Column) -> &'cell str,
        right: impl Fn(Column) -> &'cell str,
    ) -> Ordering {
        self.priorities
            .iter()
            .fold(Ordering::Equal, |decided, &(column, direction)| {
                decided.then_with(|| cell_ordering(left(column), right(column), direction))
            })
    }
}

/// Orders two cells, case-insensitively, with the empty cell at the far end:
/// last when ascending, first when descending.
fn cell_ordering(left: &str, right: &str, direction: Direction) -> Ordering {
    let ascending = match (left.is_empty(), right.is_empty()) {
        (true, true) => Ordering::Equal,
        (true, false) => Ordering::Greater,
        (false, true) => Ordering::Less,
        (false, false) => left.to_lowercase().cmp(&right.to_lowercase()),
    };
    match direction {
        Direction::Ascending => ascending,
        Direction::Descending => ascending.reverse(),
    }
}

#[cfg(test)]
mod tests;
