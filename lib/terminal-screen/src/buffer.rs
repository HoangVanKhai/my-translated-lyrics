//! The in-memory grid of character cells a frame is drawn into.

use crate::geometry::{Column, Height, Row, Width};
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
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum Cell {
    #[default]
    Empty,
    Glyph(Glyph),
    Trailing,
}

/// A styled glyph occupying a cell: its character, an optional trailing
/// variation selector, and the style to draw it with.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Glyph {
    pub(crate) char: char,
    /// A variation selector following `char`, when one was given.
    pub(crate) variation_selector: Option<char>,
    pub(crate) style: Style,
}

/// The display width of a glyph. A variation selector can widen a symbol to its
/// emoji form, but terminals keep the base width even for a text-form selector
/// (they change the color, not the width), so the wider of the two is used.
pub(crate) fn glyph_width(char: char, variation_selector: Option<char>) -> Width {
    let base = Width::new(char.width().unwrap_or(0) as u16);
    match variation_selector {
        Some(selector) => {
            let mut grapheme = String::with_capacity(char.len_utf8() + selector.len_utf8());
            grapheme.push(char);
            grapheme.push(selector);
            base.max(Width::new(grapheme.width() as u16))
        }
        None => base,
    }
}

/// Whether `char` is a variation selector (U+FE00..=U+FE0F), which adjusts the
/// presentation of the preceding character rather than standing on its own.
fn is_variation_selector(char: char) -> bool {
    ('\u{FE00}'..='\u{FE0F}').contains(&char)
}

/// A grid of character cells, in row-major order.
pub struct Buffer {
    pub(crate) width: Width,
    pub(crate) height: Height,
    pub(crate) cells: Vec<Cell>,
}

impl Buffer {
    pub fn new(width: Width, height: Height) -> Self {
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

    fn index(&self, col: Column, row: Row) -> Option<usize> {
        (self.width.contains(col) && self.height.contains(row))
            .then(|| usize::from(row) * usize::from(self.width) + usize::from(col))
    }

    /// Writes a glyph at `col`, `row`, marking the columns a wide glyph covers
    /// as trailing, and returns the number of columns it spans. Nothing is
    /// written when the glyph would run past the right edge.
    fn place_glyph(
        &mut self,
        col: Column,
        row: Row,
        char: char,
        variation_selector: Option<char>,
        style: Style,
    ) -> Width {
        let width = glyph_width(char, variation_selector);
        // A zero-width glyph, such as a combining mark or a control character,
        // has no column of its own, matching how `set_string` skips it.
        if width == Width::ZERO {
            return Width::ZERO;
        }
        if self.width.fits(col, width)
            && let Some(index) = self.index(col, row)
        {
            self.cells[index] = Cell::Glyph(Glyph {
                char,
                variation_selector,
                style,
            });
            for offset in 1..width.get() {
                if let Some(trailing) = self.index(col + Width::new(offset), row) {
                    self.cells[trailing] = Cell::Trailing;
                }
            }
        }
        width
    }

    /// Writes `char` at `col`, `row` with `style`. Returns the number of columns
    /// the glyph spans, so a caller laying out a line can advance past it.
    pub fn set_glyph(&mut self, col: Column, row: Row, char: char, style: Style) -> Width {
        self.place_glyph(col, row, char, None, style)
    }

    /// Writes `text` starting at `col`, `row` with a uniform `style`, advancing
    /// by each glyph's display width and stopping at the right edge. A variation
    /// selector is kept with the glyph it follows. Returns the column just after
    /// the text, so a caller can place the next segment without measuring widths
    /// itself.
    pub fn set_string(&mut self, col: Column, row: Row, text: &str, style: Style) -> Column {
        let mut cursor = col;
        let mut chars = text.chars().peekable();
        while let Some(char) = chars.next() {
            // A lone variation selector or other zero-width character has no
            // column of its own; composed (NFC) text has none on their own.
            if is_variation_selector(char) || char.width().unwrap_or(0) == 0 {
                continue;
            }
            let variation_selector = chars.next_if(|&next| is_variation_selector(next));
            // Stop rather than write a wide glyph past the right edge.
            if !self
                .width
                .fits(cursor, glyph_width(char, variation_selector))
            {
                break;
            }
            cursor += self.place_glyph(cursor, row, char, variation_selector, style);
        }
        cursor
    }

    /// The text of a row, with empty and trailing cells shown as blanks, to
    /// read back what a frame drew without inspecting the terminal.
    pub fn row_text(&self, row: Row) -> String {
        self.width
            .columns()
            .filter_map(|col| self.index(col, row))
            .map(|index| match self.cells[index] {
                Cell::Glyph(glyph) => glyph.char,
                Cell::Empty | Cell::Trailing => ' ',
            })
            .collect()
    }

    /// The style of the glyph at `col`, `row`, or the default style for an
    /// empty or trailing cell.
    pub fn style_at(&self, col: Column, row: Row) -> Style {
        match self.index(col, row).map(|index| self.cells[index]) {
            Some(Cell::Glyph(glyph)) => glyph.style,
            _ => Style::PLAIN,
        }
    }
}

#[cfg(test)]
mod tests;
