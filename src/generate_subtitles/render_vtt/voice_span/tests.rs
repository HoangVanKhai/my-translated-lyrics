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
