#![cfg_attr(dylint_lib = "perfectionist", feature(register_tool))]
#![cfg_attr(dylint_lib = "perfectionist", register_tool(perfectionist))]

//! The interactive terminal selectors built on `crossterm`.
//!
//! This crate renders the fuzzy table of titles and the simple list
//! selectors for the player, language, and subtitle format. It reads key
//! events and drives a [`Selector`] for the table. Column widths follow the
//! Unicode display width, so a wide glyph such as a CJK ideograph occupies
//! two columns and the columns stay aligned.
//!
//! All drawing goes to standard error, so standard output stays clean. The
//! commands are sent through the `QueueableCommand` and `ExecutableCommand`
//! trait methods rather than the `queue!` and `execute!` macros.
//!
//! [`Selector`]: fuzzy_select::selection::Selector

mod host;
mod render;
mod selectors;
mod terminal;

pub use selectors::{select_one, select_video};

/// The outcome of an interactive selector.
#[derive(Debug, Eq, PartialEq)]
pub enum Navigation {
    /// The user chose the item at this index.
    Selected(usize),
    /// The user asked to return to the previous page.
    Back,
    /// The user asked to quit the program.
    Quit,
}
