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

/// Reads the entry a [`TitleColumn`] names, panicking past the end of the
/// slice as indexing by a number does.
impl<Item> Index<TitleColumn> for [Item] {
    type Output = Item;

    fn index(&self, column: TitleColumn) -> &Item {
        &self[column.0]
    }
}
