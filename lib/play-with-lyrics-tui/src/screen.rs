//! A double-buffered screen that diffs frames so only the cells that change
//! are written to the terminal.
//!
//! The selectors used to clear the whole screen and repaint it on every
//! keystroke, which the terminal showed as a flicker. Instead, a frame is
//! drawn into an in-memory [`Buffer`] of character cells, that buffer is
//! compared against the one currently on screen, and only the differing cells
//! are sent to the terminal. The two buffers are swapped after each frame, so
//! no per-cell copy is needed.

use crossterm::QueueableCommand;
use crossterm::cursor::MoveTo;
use crossterm::style::{Attribute, Print, SetAttribute};
use crossterm::terminal::{Clear, ClearType};
use std::io::{self, Write};
use std::mem;
use unicode_width::UnicodeWidthChar;

/// The text attributes a cell is drawn with, as a small set of flags so a cell
/// stays cheap to store and compare.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Style(u8);

impl Style {
    pub(crate) const PLAIN: Style = Style(0);
    pub(crate) const BOLD: Style = Style(1 << 0);
    pub(crate) const DIM: Style = Style(1 << 1);
    pub(crate) const UNDERLINE: Style = Style(1 << 2);
    pub(crate) const REVERSE: Style = Style(1 << 3);

    /// The union of two attribute sets, for combining a base style with an
    /// extra attribute such as an underline on a matched character.
    pub(crate) fn with(self, other: Style) -> Style {
        Style(self.0 | other.0)
    }

    fn contains(self, other: Style) -> bool {
        self.0 & other.0 == other.0
    }
}

/// One character cell of the screen.
///
/// A bare `char` is not enough: a cell also carries its [`Style`], and the
/// trailing column of a double-width glyph needs to be marked so the diff does
/// not overwrite the glyph's right half with a blank. An `Empty` cell, the
/// equivalent of a `NUL` character, is drawn as a blank with the default style.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum Cell {
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
pub(crate) struct Buffer {
    width: u16,
    height: u16,
    cells: Vec<Cell>,
}

impl Buffer {
    pub(crate) fn new(width: u16, height: u16) -> Self {
        let count = usize::from(width) * usize::from(height);
        Buffer {
            width,
            height,
            cells: vec![Cell::Empty; count],
        }
    }

    /// Resets every cell to empty, to draw a fresh frame.
    fn clear(&mut self) {
        self.cells.iter_mut().for_each(|cell| *cell = Cell::Empty);
    }

    fn index(&self, col: u16, row: u16) -> Option<usize> {
        (col < self.width && row < self.height)
            .then(|| usize::from(row) * usize::from(self.width) + usize::from(col))
    }

    /// Writes `ch` at `col`, `row` with `style`, marking the trailing column
    /// when the glyph is double-width. Returns the number of columns it spans,
    /// so a caller laying out a line can advance past it.
    pub(crate) fn set_glyph(&mut self, col: u16, row: u16, ch: char, style: Style) -> u16 {
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
    pub(crate) fn set_string(&mut self, col: u16, row: u16, text: &str, style: Style) {
        let mut cursor = col;
        for ch in text.chars() {
            // A zero-width character cannot stand in its own cell; the rendered
            // titles are composed (NFC), so none arrive on their own here.
            if ch.width().unwrap_or(0) == 0 {
                continue;
            }
            if cursor >= self.width {
                break;
            }
            cursor += self.set_glyph(cursor, row, ch, style);
        }
    }

    /// The text of a row, with empty and trailing cells shown as blanks. For
    /// tests, to read back what a frame drew without inspecting the terminal.
    #[cfg(test)]
    pub(crate) fn row_text(&self, row: u16) -> String {
        (0..self.width)
            .filter_map(|col| self.index(col, row))
            .map(|index| match self.cells[index] {
                Cell::Glyph { ch, .. } => ch,
                Cell::Empty | Cell::Trailing => ' ',
            })
            .collect()
    }

    /// The style of the glyph at `col`, `row`, or the default style for an
    /// empty or trailing cell. For tests.
    #[cfg(test)]
    pub(crate) fn style_at(&self, col: u16, row: u16) -> Style {
        match self.index(col, row).map(|index| self.cells[index]) {
            Some(Cell::Glyph { style, .. }) => style,
            _ => Style::PLAIN,
        }
    }
}

/// The double-buffered screen: the `front` buffer holds what is on the
/// terminal, and a frame is drawn into the `back` buffer before the two are
/// compared and swapped.
pub(crate) struct Screen {
    front: Buffer,
    back: Buffer,
}

impl Screen {
    pub(crate) fn new() -> Self {
        Screen {
            front: Buffer::new(0, 0),
            back: Buffer::new(0, 0),
        }
    }

    /// Begins a frame at the given terminal size, returning the back buffer to
    /// draw into. On a size change the terminal and both buffers are reset, so
    /// the next diff repaints everything at the new size; otherwise only the
    /// back buffer is cleared.
    pub(crate) fn begin(
        &mut self,
        width: u16,
        height: u16,
        output: &mut impl Write,
    ) -> io::Result<&mut Buffer> {
        if self.back.width != width || self.back.height != height {
            output.queue(Clear(ClearType::All))?;
            self.front = Buffer::new(width, height);
            self.back = Buffer::new(width, height);
        } else {
            self.back.clear();
        }
        Ok(&mut self.back)
    }

    /// Sends the cells that changed since the last frame to the terminal, then
    /// swaps the buffers so the drawn frame becomes the one on screen.
    pub(crate) fn flush(&mut self, output: &mut impl Write) -> io::Result<()> {
        diff(&self.front, &self.back, output)?;
        mem::swap(&mut self.front, &mut self.back);
        output.flush()
    }
}

/// Writes the cells of `back` that differ from `front` to the terminal.
///
/// Each row is scanned for runs of changed cells. A run is drawn with one
/// cursor move to its start followed by its characters, so an unchanged region
/// costs nothing and a changed region costs one move rather than one per cell.
fn diff(front: &Buffer, back: &Buffer, output: &mut impl Write) -> io::Result<()> {
    let width = back.width;
    let mut current = Style::PLAIN;
    for row in 0..back.height {
        let mut col = 0;
        while col < width {
            let index = usize::from(row) * usize::from(width) + usize::from(col);
            if back.cells[index] == front.cells[index] {
                col += 1;
                continue;
            }
            output.queue(MoveTo(col, row))?;
            while col < width {
                let index = usize::from(row) * usize::from(width) + usize::from(col);
                if back.cells[index] == front.cells[index] {
                    break;
                }
                match back.cells[index] {
                    Cell::Empty => {
                        set_style(output, &mut current, Style::PLAIN)?;
                        output.queue(Print(' '))?;
                        col += 1;
                    }
                    Cell::Glyph { ch, style } => {
                        set_style(output, &mut current, style)?;
                        output.queue(Print(ch))?;
                        col += ch.width().unwrap_or(1).max(1) as u16;
                    }
                    // The leading glyph already covered this column.
                    Cell::Trailing => col += 1,
                }
            }
        }
    }
    set_style(output, &mut current, Style::PLAIN)
}

/// Switches the terminal's active attributes to `target` when they differ from
/// `current`, by resetting and re-applying, which keeps the logic the same for
/// every attribute combination.
fn set_style(output: &mut impl Write, current: &mut Style, target: Style) -> io::Result<()> {
    if *current == target {
        return Ok(());
    }
    output.queue(SetAttribute(Attribute::Reset))?;
    if target.contains(Style::BOLD) {
        output.queue(SetAttribute(Attribute::Bold))?;
    }
    if target.contains(Style::DIM) {
        output.queue(SetAttribute(Attribute::Dim))?;
    }
    if target.contains(Style::UNDERLINE) {
        output.queue(SetAttribute(Attribute::Underlined))?;
    }
    if target.contains(Style::REVERSE) {
        output.queue(SetAttribute(Attribute::Reverse))?;
    }
    *current = target;
    Ok(())
}

#[cfg(test)]
mod tests;
