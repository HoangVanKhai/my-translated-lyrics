#![cfg_attr(dylint_lib = "perfectionist", feature(register_tool))]
#![cfg_attr(dylint_lib = "perfectionist", register_tool(perfectionist))]

//! A double-buffered terminal screen that diffs frames so only the cells that
//! change are written out.
//!
//! A frame is drawn into an in-memory [`Buffer`] of character cells. The buffer
//! is compared against the one currently on screen, and only the differing
//! cells are sent to the terminal, leaving unchanged regions as they are. The
//! two buffers are swapped after each frame, so no per-cell copy is needed.
//!
//! The crate is the rendering core, independent of any particular interface: a
//! caller draws text with a [`Style`] into the back buffer through
//! [`Screen::begin`], then calls [`Screen::flush`] to send the diff.

mod buffer;
mod screen;
mod style;

pub use buffer::Buffer;
pub use screen::Screen;
pub use style::Style;
