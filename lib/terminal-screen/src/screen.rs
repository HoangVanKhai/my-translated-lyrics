//! The double buffer and the diff that turns a drawn frame into terminal writes.

use crate::buffer::{Buffer, Cell, glyph_width};
use crate::style::Style;
use crossterm::QueueableCommand;
use crossterm::cursor::MoveTo;
use crossterm::style::{Attribute, Print, SetAttribute};
use crossterm::terminal::{Clear, ClearType};
use std::io::{self, Write};
use std::mem;

/// The double-buffered screen: the `front` buffer holds what is on the
/// terminal, and a frame is drawn into the `back` buffer before the two are
/// compared and swapped.
pub struct Screen {
    front: Buffer,
    back: Buffer,
}

impl Default for Screen {
    fn default() -> Self {
        Screen::new()
    }
}

impl Screen {
    pub fn new() -> Self {
        Screen {
            front: Buffer::new(0, 0),
            back: Buffer::new(0, 0),
        }
    }

    /// Begins a frame at the given terminal size, returning the back buffer to
    /// draw into. On a size change the terminal and both buffers are reset, so
    /// the next diff repaints everything at the new size; otherwise only the
    /// back buffer is cleared.
    pub fn begin(
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
    pub fn flush(&mut self, output: &mut impl Write) -> io::Result<()> {
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
                    Cell::Glyph(glyph) => {
                        set_style(output, &mut current, glyph.style)?;
                        output.queue(Print(glyph.char))?;
                        if let Some(selector) = glyph.variation_selector {
                            output.queue(Print(selector))?;
                        }
                        col += glyph_width(glyph.char, glyph.variation_selector).max(1);
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
    if target.contains(Style::ITALIC) {
        output.queue(SetAttribute(Attribute::Italic))?;
    }
    *current = target;
    Ok(())
}

#[cfg(test)]
mod tests;
