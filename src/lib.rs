// Register the `perfectionist` tool namespace so that
// `#[allow / expect(perfectionist::...)]` attributes in this crate are
// accepted. Both attributes are gated on the `dylint_lib` cfg, which is
// only set when the Dylint driver loads the `perfectionist` library, so a
// plain `cargo build` or `cargo check` ignores them and needs no nightly
// toolchain.
#![cfg_attr(dylint_lib = "perfectionist", feature(register_tool))]
#![cfg_attr(dylint_lib = "perfectionist", register_tool(perfectionist))]

mod file_snapshot;

pub mod credits_descriptor;
pub mod generate_subtitles;
pub mod install_local_lyrics;
pub mod line_markers_descriptor;
pub mod timestamp;
pub mod video_descriptor;
