//! Build subtitle files from the structured lyrics sources.
//!
//! This module implements the `build-subtitles` binary. For each song
//! directory in `sources/`, it reads the video descriptor, the
//! line-marker descriptor, the credits descriptor, and the per-language
//! `lyrics.{lang}.txt` files. It parses the text files into cues,
//! renders each cue through the marker-aware VTT and SRT emitters, and
//! writes the result to the corresponding `dist/` directory.
//!
//! The entry point [`main`] is called from `cli/build_subtitles.rs`.

pub mod credits_parse;
pub mod parse;
pub mod render_srt;
pub mod render_vtt;
pub mod styles;

mod driver;

pub use driver::{BuildError, Song, load_song, main, render_song_to_disk};
