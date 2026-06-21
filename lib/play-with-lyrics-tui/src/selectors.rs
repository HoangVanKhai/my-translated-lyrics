//! The interactive selector pages: the fuzzy table of titles and the simple
//! single-column list. Each page enters the terminal, then drives a loop that
//! reads events and redraws the screen after each event that changes what is
//! shown.
//!
//! The two pages live in submodules: [`video`] holds the fuzzy table and
//! [`list`] holds the single-column list. Their shared row styling stays here.

mod list;
mod video;

pub use list::select_one;
pub use video::select_video;

use terminal_screen::Style;

/// The base style for a selectable row at screen `row`: reverse video when it
/// is the current selection, with bold added when the pointer hovers over it.
fn row_style(selected: bool, hover: Option<(u16, u16)>, row: u16) -> Style {
    let mut style = if selected {
        Style::REVERSE
    } else {
        Style::PLAIN
    };
    if hover.is_some_and(|(_, hovered_row)| hovered_row == row) {
        style = style.with(Style::BOLD);
    }
    style
}

#[cfg(test)]
mod _test_utils;
#[cfg(test)]
mod buttons;
#[cfg(test)]
mod list_keyboard;
#[cfg(test)]
mod mouse;
#[cfg(test)]
mod rendering;
#[cfg(test)]
mod restore;
#[cfg(test)]
mod video_keyboard;
