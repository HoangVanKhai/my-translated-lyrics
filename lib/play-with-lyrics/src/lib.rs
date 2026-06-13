#![cfg_attr(dylint_lib = "perfectionist", feature(register_tool))]
#![cfg_attr(dylint_lib = "perfectionist", register_tool(perfectionist))]

//! The data and operations behind the `play-with-lyrics` command.
//!
//! [`catalog`] loads the per-video `video.toml` descriptors that supply the
//! titles shown in the selector. [`library`] locates the video files and
//! installed subtitles inside the media library. [`player`] describes the
//! supported media players and subtitle formats and builds the player
//! invocation. The interactive front-end and the command-line wiring live
//! in the binary crate.

pub mod catalog;
pub mod library;
pub mod player;
