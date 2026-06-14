#![cfg_attr(dylint_lib = "perfectionist", feature(register_tool))]
#![cfg_attr(dylint_lib = "perfectionist", register_tool(perfectionist))]

//! Render subtitle files from the structured lyrics sources.
//!
//! This crate is the library behind the `generate-subtitles` binary.
//! For each song directory in `sources/`, [`load_song`] reads the video
//! descriptor, the line-marker descriptor, the credits descriptor, and
//! the per-language `lyrics.{lang}.txt` files, parsing the text files
//! into cues. [`render_song`] renders each cue through the marker-aware
//! VTT and SRT emitters and writes the result to the corresponding
//! `dist/` directory.
//!
//! The credit-line parser uses the `credit-roles` list in
//! `credits.yaml` as the recognized role set; a credit line whose
//! first non-whitespace token is not a known role fails the render
//! with [`credits_parse::ParseCreditError`].

pub mod credits_parse;
pub mod parse;
pub mod render_srt;
pub mod render_vtt;
pub mod styles;

mod driver;
mod escape;

pub use driver::{RenderCounts, Song, load_palette, load_song, render_song};
pub use styles::StylePalette;
