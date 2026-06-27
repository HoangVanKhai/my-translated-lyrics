use super::VoiceSelector;
use lyrics_core::line_markers_descriptor::VoiceName;
use pipe_trait::Pipe;
use pretty_assertions::assert_eq;

fn sample_voice_name(text: impl Into<String>) -> VoiceName {
    text.into()
        .pipe(VoiceName::new)
        .expect("test fixture passes the voice-name validator")
}

#[test]
fn voice_selector_emits_double_quoted_attribute_selector() {
    let voice_name = sample_voice_name("名字一");
    assert_eq!(
        VoiceSelector(&voice_name).to_string(),
        r#"v[voice="名字一"]"#,
    );
}

/// Double-quoted CSS strings accept `'` verbatim, so the
/// selector can splat a name containing `'` without any escape.
#[test]
fn voice_selector_preserves_ascii_apostrophes_inside_double_quotes() {
    let voice_name = sample_voice_name("O'Brien");
    assert_eq!(
        VoiceSelector(&voice_name).to_string(),
        r#"v[voice="O'Brien"]"#,
    );
}

/// Regression for the bug where a prior revision wrapped the
/// voice name in HTML-entity escape on the CSS side, which
/// broke the match against the cue-tag side: CSS does not
/// decode entity references in attribute-value strings, so
/// `[voice="X &amp; Y"]` would match the literal six-byte
/// `&amp;` instead of the `&` that the WebVTT parser produces
/// from the cue-tag side. Locking the selector against any
/// future reintroduction of HTML escape on this side is the
/// job of this test; the renderer-level companion test in
/// `render_vtt/tests.rs` locks the cue-tag side.
#[test]
fn voice_selector_preserves_ampersand_verbatim() {
    let voice_name = sample_voice_name("Alpha & Beta");
    assert_eq!(
        VoiceSelector(&voice_name).to_string(),
        r#"v[voice="Alpha & Beta"]"#,
    );
}
