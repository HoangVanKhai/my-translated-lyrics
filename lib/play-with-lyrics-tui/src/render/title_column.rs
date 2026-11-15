use super::TITLE_CELLS;
use std::ops::Index;

/// Which of the title columns is meant, counting from the left.
///
/// It indexes the table's layout rather than the screen columns that layout
/// occupies.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct TitleColumn(usize);

impl TitleColumn {
    /// The column `position` places among the title cells.
    pub(super) fn new(position: usize) -> TitleColumn {
        TitleColumn(position)
    }
}

/// Reads the entry a [`TitleColumn`] names from a list that holds one entry
/// per title cell. A list of any other length is not one of these columns to
/// index, so it cannot be indexed by one.
impl<Item> Index<TitleColumn> for [Item; TITLE_CELLS] {
    type Output = Item;

    fn index(&self, column: TitleColumn) -> &Item {
        &self[column.0]
    }
}
