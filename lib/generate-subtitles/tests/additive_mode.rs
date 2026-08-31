//! End-to-end coverage for additive regions.
//!
//! The parser's own tests assert the shape of the accumulated cues;
//! these assert the bytes that reach a subtitle file. A region is
//! written the way an author would write it, run through the same
//! parse-then-render path the generator uses, and compared against the
//! complete WebVTT and SubRip documents it is expected to produce.
//! Neither subtitle format has a notion of a cue that extends the one
//! before it, so the accumulation has to be resolved before rendering,
//! and that is exactly what these files show: each cue repeats every
//! line above it in the region.

use generate_subtitles::parse::parse_lyrics;
use generate_subtitles::render_srt::render_srt;
use generate_subtitles::render_vtt::render_vtt;
use generate_subtitles::{StylePalette, load_palette};
use lyrics_core::credits_descriptor::CreditsDesc;
use lyrics_core::line_markers_descriptor::LineMarkersDesc;
use lyrics_core::video_descriptor::Language;
use pretty_assertions::assert_eq;
use text_block_macros::text_block_fnl;

/// The source text under test, in the shape the specification
/// describes: four cues wrapped in a region, closed by a `clr` that
/// ends the last of them.
fn additive_source() -> &'static str {
    text_block_fnl! {
        "<additive>"
        "07:11.111 LRC: first line"
        "07:22.222 LRC: second line"
        "07:33.333 LRC: third line"
        "07:44.444 LRC: fourth line"
        "</additive>"
        ""
        "07:55.555 clr"
    }
}

/// The workspace palette, which the generator itself renders with. No
/// marker in the fixture resolves to a voice, a class, or a credit
/// block, so no palette entry reaches the output; the palette is
/// supplied because the renderers require one.
fn workspace_palette() -> StylePalette {
    load_palette(&test_utils::workspace_dir().join("styles.toml"))
}

#[test]
fn an_additive_region_renders_as_accumulating_vtt_cues() {
    let cues = parse_lyrics(additive_source()).unwrap();
    let output = render_vtt(
        &cues,
        &LineMarkersDesc::default(),
        &CreditsDesc::default(),
        &workspace_palette(),
        &Language::Vietnamese,
    )
    .unwrap();
    assert_eq!(
        output,
        text_block_fnl! {
            "WEBVTT"
            "Language: vi"
            ""
            "STYLE"
            "::cue {"
            "  background-color: transparent;"
            "  text-shadow: 2px 2px 2px black;"
            "}"
            ""
            "00:07:11.111 --> 00:07:22.222"
            "first line"
            ""
            "00:07:22.222 --> 00:07:33.333"
            "first line"
            "second line"
            ""
            "00:07:33.333 --> 00:07:44.444"
            "first line"
            "second line"
            "third line"
            ""
            "00:07:44.444 --> 00:07:55.555"
            "first line"
            "second line"
            "third line"
            "fourth line"
        },
    );
}

#[test]
fn an_additive_region_renders_as_accumulating_srt_cues() {
    let cues = parse_lyrics(additive_source()).unwrap();
    let output = render_srt(
        &cues,
        &LineMarkersDesc::default(),
        &CreditsDesc::default(),
        &workspace_palette(),
        &Language::Vietnamese,
    )
    .unwrap();
    assert_eq!(
        output,
        text_block_fnl! {
            "1"
            "00:07:11,111 --> 00:07:22,222"
            "first line"
            ""
            "2"
            "00:07:22,222 --> 00:07:33,333"
            "first line"
            "second line"
            ""
            "3"
            "00:07:33,333 --> 00:07:44,444"
            "first line"
            "second line"
            "third line"
            ""
            "4"
            "00:07:44,444 --> 00:07:55,555"
            "first line"
            "second line"
            "third line"
            "fourth line"
        },
    );
}
