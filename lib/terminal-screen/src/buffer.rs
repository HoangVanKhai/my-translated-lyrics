//! The in-memory grid of character cells a frame is drawn into.

use crate::style::Style;
use unicode_width::UnicodeWidthChar;

/// One character cell of the screen: an empty cell, a styled glyph, or the
/// trailing column of a double-width glyph.
///
/// `Empty` cells are drawn as blanks with the default style. A double-width
/// glyph occupies two columns, held as the glyph followed by `Trailing`, so the
/// grid keeps one cell per terminal column.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Cell {
    #[default]
    Empty,
    Glyph {
        ch: char,
        style: Style,
    },
    /// The second column of the double-width glyph in the cell to its left.
    Trailing,
}

/// A grid of character cells, in row-major order.
pub struct Buffer {
    pub(crate) width: u16,
    pub(crate) height: u16,
    pub(crate) cells: Vec<Cell>,
}

impl Buffer {
    pub fn new(width: u16, height: u16) -> Self {
        let count = usize::from(width) * usize::from(height);
        Buffer {
            width,
            height,
            cells: vec![Cell::Empty; count],
        }
    }

    /// Resets every cell to empty, to draw a fresh frame.
    pub(crate) fn clear(&mut self) {
        self.cells.iter_mut().for_each(|cell| *cell = Cell::Empty);
    }

    fn index(&self, col: u16, row: u16) -> Option<usize> {
        (col < self.width && row < self.height)
            .then(|| usize::from(row) * usize::from(self.width) + usize::from(col))
    }

    /// Writes `ch` at `col`, `row` with `style`, marking the trailing column
    /// when the glyph is double-width. Returns the number of columns it spans,
    /// so a caller laying out a line can advance past it.
    pub fn set_glyph(&mut self, col: u16, row: u16, ch: char, style: Style) -> u16 {
        let width = ch.width().unwrap_or(0) as u16;
        if let Some(index) = self.index(col, row) {
            self.cells[index] = Cell::Glyph { ch, style };
            if width == 2
                && let Some(trailing) = self.index(col + 1, row)
            {
                self.cells[trailing] = Cell::Trailing;
            }
        }
        width.max(1)
    }

    /// Writes `text` starting at `col`, `row` with a uniform `style`, advancing
    /// by each character's display width and stopping at the right edge.
    pub fn set_string(&mut self, col: u16, row: u16, text: &str, style: Style) {
        let mut cursor = col;
        for ch in text.chars() {
            // A zero-width character has no column of its own, so skip it;
            // composed (NFC) text has no standalone combining marks.
            if ch.width().unwrap_or(0) == 0 {
                continue;
            }
            if cursor >= self.width {
                break;
            }
            cursor += self.set_glyph(cursor, row, ch, style);
        }
    }

    /// The text of a row, with empty and trailing cells shown as blanks, to
    /// read back what a frame drew without inspecting the terminal.
    pub fn row_text(&self, row: u16) -> String {
        (0..self.width)
            .filter_map(|col| self.index(col, row))
            .map(|index| match self.cells[index] {
                Cell::Glyph { ch, .. } => ch,
                Cell::Empty | Cell::Trailing => ' ',
            })
            .collect()
    }

    /// The style of the glyph at `col`, `row`, or the default style for an
    /// empty or trailing cell.
    pub fn style_at(&self, col: u16, row: u16) -> Style {
        match self.index(col, row).map(|index| self.cells[index]) {
            Some(Cell::Glyph { style, .. }) => style,
            _ => Style::PLAIN,
        }
    }
}

#[cfg(test)]
mod tests;
