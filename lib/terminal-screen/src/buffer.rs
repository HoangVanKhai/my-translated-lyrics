//! The in-memory grid of character cells a frame is drawn into.

use crate::style::Style;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// One character cell of the screen: an empty cell, a styled glyph, or a column
/// covered by a wide glyph to its left.
///
/// `Empty` cells are drawn as blanks with the default style. A glyph may carry
/// a trailing variation selector, which can switch a symbol between its narrow
/// text form and its wide emoji form. A wide glyph occupies more than one
/// column, with `Trailing` in the columns after its first, so the grid keeps
/// one cell per terminal column.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Cell {
    #[default]
    Empty,
    Glyph {
        ch: char,
        /// A variation selector following `ch`, when one was given.
        vs: Option<char>,
        style: Style,
    },
    Trailing,
}

/// The display width of a glyph. A variation selector can widen a symbol to its
/// emoji form, but terminals keep the base width even for a text-form selector
/// (they change the color, not the width), so the wider of the two is used.
pub(crate) fn glyph_width(ch: char, vs: Option<char>) -> u16 {
    let base = ch.width().unwrap_or(0) as u16;
    match vs {
        Some(selector) => {
            let mut grapheme = String::with_capacity(ch.len_utf8() + selector.len_utf8());
            grapheme.push(ch);
            grapheme.push(selector);
            base.max(grapheme.width() as u16)
        }
        None => base,
    }
}

/// Whether `ch` is a variation selector (U+FE00..=U+FE0F), which adjusts the
/// presentation of the preceding character rather than standing on its own.
fn is_variation_selector(ch: char) -> bool {
    ('\u{FE00}'..='\u{FE0F}').contains(&ch)
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

    /// Writes a glyph at `col`, `row`, marking the columns a wide glyph covers
    /// as trailing, and returns the number of columns it spans. Nothing is
    /// written when the glyph would run past the right edge.
    fn place_glyph(&mut self, col: u16, row: u16, ch: char, vs: Option<char>, style: Style) -> u16 {
        let width = glyph_width(ch, vs);
        if usize::from(col) + usize::from(width) <= usize::from(self.width)
            && let Some(index) = self.index(col, row)
        {
            self.cells[index] = Cell::Glyph { ch, vs, style };
            for offset in 1..width {
                if let Some(trailing) = self.index(col + offset, row) {
                    self.cells[trailing] = Cell::Trailing;
                }
            }
        }
        width.max(1)
    }

    /// Writes `ch` at `col`, `row` with `style`. Returns the number of columns
    /// the glyph spans, so a caller laying out a line can advance past it.
    pub fn set_glyph(&mut self, col: u16, row: u16, ch: char, style: Style) -> u16 {
        self.place_glyph(col, row, ch, None, style)
    }

    /// Writes `text` starting at `col`, `row` with a uniform `style`, advancing
    /// by each glyph's display width and stopping at the right edge. A variation
    /// selector is kept with the glyph it follows. Returns the column just after
    /// the text, so a caller can place the next segment without measuring widths
    /// itself.
    pub fn set_string(&mut self, col: u16, row: u16, text: &str, style: Style) -> u16 {
        let mut cursor = col;
        let mut chars = text.chars().peekable();
        while let Some(ch) = chars.next() {
            // A lone variation selector or other zero-width character has no
            // column of its own; composed (NFC) text has none on their own.
            if is_variation_selector(ch) || ch.width().unwrap_or(0) == 0 {
                continue;
            }
            let vs = chars.next_if(|&next| is_variation_selector(next));
            // Stop rather than write a wide glyph past the right edge.
            if usize::from(cursor) + usize::from(glyph_width(ch, vs)) > usize::from(self.width) {
                break;
            }
            cursor += self.place_glyph(cursor, row, ch, vs, style);
        }
        cursor
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
