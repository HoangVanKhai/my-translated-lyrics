use super::{VoiceSelector, VoiceSpan};
use crate::line_markers_descriptor::VoiceName;
use pipe_trait::Pipe;
use pretty_assertions::assert_eq;

fn sample_voice_name(text: &str) -> VoiceName {
    text.to_string()
        .pipe(VoiceName::new)
        .expect("test fixture passes the voice-name validator")
}

#[test]
fn voice_span_emits_cue_tag_wrapping_pre_escaped_inner() {
    let voice_name = sample_voice_name("名字一");
    let rendered = VoiceSpan {
        voice_name: &voice_name,
        inner: "Hello &amp; world",
    }
    .to_string();
    assert_eq!(rendered, "<v 名字一>Hello &amp; world</v>");
}

#[test]
fn voice_span_preserves_ascii_apostrophes_in_the_name() {
    // `'` is not a meta character of the WebVTT cue tag, so it
    // passes through unchanged.
    let voice_name = sample_voice_name("O'Brien");
    let rendered = VoiceSpan {
        voice_name: &voice_name,
        inner: "line",
    }
    .to_string();
    assert_eq!(rendered, "<v O'Brien>line</v>");
}

#[test]
fn voice_selector_emits_double_quoted_attribute_selector() {
    let voice_name = sample_voice_name("名字一");
    assert_eq!(
        VoiceSelector(&voice_name).to_string(),
        "v[voice=\"名字一\"]",
    );
}

#[test]
fn voice_selector_preserves_ascii_apostrophes_inside_double_quotes() {
    // Double-quoted CSS strings accept `'` verbatim, so the
    // selector can splat a name containing `'` without any escape.
    let voice_name = sample_voice_name("O'Brien");
    assert_eq!(
        VoiceSelector(&voice_name).to_string(),
        "v[voice=\"O'Brien\"]",
    );
}

#[test]
fn voice_name_containing_ampersand_is_not_html_escaped_in_either_context() {
    // Regression for the bug that `fix(render-vtt): drop redundant
    // voice-name escape` repaired. A prior revision wrapped the
    // voice name in the HTML-entity escape at both interpolation
    // sites, which broke the match between cue tag and CSS selector:
    // the
    // WebVTT cue-text parser decodes `&amp;` back to `&` inside
    // `<v ...>`, but CSS does not decode entity references in
    // attribute-value strings, so the selector would match the
    // literal `amp;` form and stop targeting its cue. Both
    // wrappers must emit the raw `&` character verbatim; if either
    // one reintroduces the HTML escape this test fails immediately
    // rather than only the dist golden. The synthetic name
    // `Alpha & Beta` keeps the fixture independent of any voice
    // name actually present in `sources/*/line-markers.toml`.
    let voice_name = sample_voice_name("Alpha & Beta");
    assert_eq!(
        VoiceSpan {
            voice_name: &voice_name,
            inner: "body",
        }
        .to_string(),
        "<v Alpha & Beta>body</v>",
    );
    assert_eq!(
        VoiceSelector(&voice_name).to_string(),
        "v[voice=\"Alpha & Beta\"]",
    );
}
