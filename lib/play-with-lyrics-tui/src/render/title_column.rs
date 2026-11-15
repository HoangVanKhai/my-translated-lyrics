use super::ColumnSpan;
use lyrics_core::video_descriptor::Language;
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

/// Reads the language a `TitleColumn` names.
impl Index<TitleColumn> for [Language] {
    type Output = Language;

    fn index(&self, column: TitleColumn) -> &Language {
        &self[column.0]
    }
}

/// Reads the screen columns a `TitleColumn` names.
impl Index<TitleColumn> for [ColumnSpan] {
    type Output = ColumnSpan;

    fn index(&self, column: TitleColumn) -> &ColumnSpan {
        &self[column.0]
    }
}

/// Reads the header label a `TitleColumn` names.
impl Index<TitleColumn> for [String] {
    type Output = String;

    fn index(&self, column: TitleColumn) -> &String {
        &self[column.0]
    }
}
