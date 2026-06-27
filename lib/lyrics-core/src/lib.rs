#![cfg_attr(dylint_lib = "perfectionist", feature(register_tool))]
#![cfg_attr(dylint_lib = "perfectionist", register_tool(perfectionist))]

//! Shared data model for the lyrics tooling.
//!
//! This crate holds the descriptor types and primitives that the
//! binaries and the subtitle generation library build upon: the video,
//! credits, and line-marker descriptors, the timestamp primitives, and
//! the file snapshot helper.

pub mod credits_descriptor;
pub mod file_snapshot;
pub mod line_markers_descriptor;
pub mod timestamp;
pub mod video_descriptor;
