/// Which of the three title columns is meant, counting from the left.
///
/// It indexes the table's layout rather than the screen columns that layout
/// occupies.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct TitleColumn(usize);

impl TitleColumn {
    /// The column `position` places among the three.
    pub(super) fn new(position: usize) -> TitleColumn {
        TitleColumn(position)
    }

    /// The column's place among the three.
    pub(crate) fn position(self) -> usize {
        self.0
    }
}
