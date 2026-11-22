use super::{COLUMN_LANGUAGES, ColumnSpan, column_spans};
use lyrics_core::video_descriptor::Language;
use terminal_screen::Width;

/// Which of the title columns is meant, counting from the left.
///
/// It names a place in the table's layout rather than a screen column.
/// `span` converts it to the screen columns that place occupies.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct TitleColumn(usize);

impl TitleColumn {
    /// The column `position` places among the title cells.
    pub(super) fn new(position: usize) -> TitleColumn {
        TitleColumn(position)
    }

    /// The language this column shows.
    pub(crate) fn language(self) -> Language {
        COLUMN_LANGUAGES[self.0]
    }

    /// The screen columns this column covers in a line of `total` columns.
    pub(crate) fn span(self, total: Width) -> ColumnSpan {
        column_spans(total)[self.0]
    }
}
