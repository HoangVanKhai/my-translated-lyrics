//! The text attributes a cell is drawn with.

/// The text attributes a cell is drawn with.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Style(u8);

impl Style {
    pub const PLAIN: Style = Style(0);
    pub const BOLD: Style = Style(1 << 0);
    pub const DIM: Style = Style(1 << 1);
    pub const UNDERLINE: Style = Style(1 << 2);
    pub const REVERSE: Style = Style(1 << 3);
    pub const ITALIC: Style = Style(1 << 4);

    /// The union of two attribute sets.
    pub fn with(self, other: Style) -> Style {
        Style(self.0 | other.0)
    }

    /// Whether every attribute in `other` is also set here.
    pub(crate) fn contains(self, other: Style) -> bool {
        self.0 & other.0 == other.0
    }
}
