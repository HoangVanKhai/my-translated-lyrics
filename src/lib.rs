#![cfg_attr(dylint_lib = "perfectionist", feature(register_tool))]
#![cfg_attr(dylint_lib = "perfectionist", register_tool(perfectionist))]

mod file_snapshot;

pub mod credits_descriptor;
pub mod generate_subtitles;
pub mod install_local_lyrics;
pub mod line_markers_descriptor;
pub mod timestamp;
pub mod video_descriptor;
