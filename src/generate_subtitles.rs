//! Generate subtitle files from the structured lyrics sources.
//!
//! This module implements the `generate-subtitles` binary. For each
//! song directory in `sources/`, it reads the video descriptor, the
//! line-marker descriptor, the credits descriptor, and the per-language
//! `lyrics.{lang}.txt` files. It parses the text files into cues,
//! renders each cue through the marker-aware VTT and SRT emitters, and
//! writes the result to the corresponding `dist/` directory.
//!
//! The credit-line parser uses the `credit-roles` list in
//! `credits.yaml` as its role vocabulary; a credit line whose first
//! non-whitespace token is not a known role fails the render with
//! [`credits_parse::ParseCreditError`].
//!
//! The entry point [`main`] is called from `cli/generate_subtitles.rs`.

pub mod credits_parse;
pub mod parse;
pub mod render_srt;
pub mod render_vtt;
pub mod styles;

mod driver;
mod escape;

pub use driver::{Song, load_song, main, render_song};
