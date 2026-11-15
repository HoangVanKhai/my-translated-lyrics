use terminal_screen::{Column, Width};

/// A run of screen columns: where it begins and how wide it is.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ColumnSpan {
    start: Column,
    width: Width,
}

impl ColumnSpan {
    /// The run of `width` columns beginning at `start`.
    pub(crate) fn new(start: Column, width: Width) -> ColumnSpan {
        ColumnSpan { start, width }
    }

    /// The run from `start` up to but not including `end`, which is empty when
    /// `end` is not past `start`.
    pub(super) fn between(start: Column, end: Column) -> ColumnSpan {
        ColumnSpan::new(start, end.columns_after(start).unwrap_or(Width::ZERO))
    }

    /// The run's leftmost column.
    pub(crate) fn start(self) -> Column {
        self.start
    }

    /// The column just past the run's rightmost one.
    pub(crate) fn end(self) -> Column {
        self.start + self.width
    }

    /// How many columns the run spans.
    pub(crate) fn width(self) -> Width {
        self.width
    }

    /// Whether `column` falls inside the run.
    pub(crate) fn contains(self, column: Column) -> bool {
        self.start <= column && column < self.end()
    }
}
