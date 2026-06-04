use super::{RenderVttError, render_vtt};
use crate::credits_descriptor::CreditsDesc;
use crate::generate_subtitles::parse::{CuePart, SubtitleCue};
use crate::line_markers_descriptor::{LineMarkersDesc, VoiceName};
use crate::timestamp::Timestamp;
use crate::video_descriptor::Language;

use maplit::btreemap;
use pipe_trait::Pipe;
use pretty_assertions::assert_eq;

fn credits_with_one_role() -> CreditsDesc {
    CreditsDesc {
        credit_roles: vec![btreemap! { Language::Vietnamese => "role-a".to_string() }],
        ..Default::default()
    }
}

fn markers_with_credit_trigger() -> LineMarkersDesc {
    LineMarkersDesc {
        credits: vec!["cre".to_string()],
        ..Default::default()
    }
}

#[test]
fn cue_text_html_meta_characters_are_escaped() {
    let cues = vec![SubtitleCue {
        start: Timestamp::new(0, 0, 0).unwrap(),
        end: Timestamp::new(0, 5, 0).unwrap(),
        parts: vec![CuePart {
            marker: "plain".to_string(),
            text: "<a> & <b>".to_string(),
        }],
    }];
    let output = render_vtt(
        &cues,
        &LineMarkersDesc::default(),
        &CreditsDesc::default(),
        &Language::Vietnamese,
    )
    .unwrap();
    assert!(
        output.contains("&lt;a&gt; &amp; &lt;b&gt;"),
        "expected escaped cue text in output:\n{output}",
    );
    assert!(
        !output.contains("<a>"),
        "raw `<a>` must not appear in the rendered output:\n{output}",
    );
}

#[test]
fn voice_name_containing_ampersand_is_emitted_verbatim_in_cue_tag() {
    // Regression for the bug where the voice name was passed
    // through HTML-entity escape on the cue-tag side. The WebVTT
    // cue-text parser decodes `&amp;` back to `&`, but the CSS
    // selector side does not, so an HTML-escaped cue tag falls
    // out of step with its STYLE-block selector. Both sides now emit the raw
    // `&` character verbatim; the CSS-side companion lives in
    // `render_vtt/voice_span/tests.rs`.
    let voice_name = "Alpha & Beta"
        .to_string()
        .pipe(VoiceName::new)
        .expect("test fixture passes the voice-name validator");
    let markers = LineMarkersDesc {
        markers: vec!["vca".to_string()],
        voices: btreemap! {
            "vca".to_string() => btreemap! { Language::Vietnamese => voice_name },
        },
        ..Default::default()
    };
    let cues = vec![SubtitleCue {
        start: Timestamp::new(0, 0, 0).unwrap(),
        end: Timestamp::new(0, 5, 0).unwrap(),
        parts: vec![CuePart {
            marker: "vca".to_string(),
            text: "body".to_string(),
        }],
    }];
    let output = render_vtt(
        &cues,
        &markers,
        &CreditsDesc::default(),
        &Language::Vietnamese,
    )
    .unwrap();
    assert!(
        output.contains("<v Alpha & Beta>body</v>"),
        "cue-tag side must emit raw `&`:\n{output}",
    );
    assert!(
        output.contains(r#"v[voice="Alpha & Beta"]"#),
        "CSS-selector side must emit raw `&`:\n{output}",
    );
    assert!(
        !output.contains("&amp;"),
        "no HTML-entity escape of the voice name should appear:\n{output}",
    );
}

#[test]
fn unknown_role_in_credit_line_produces_credits_error() {
    let cues = vec![SubtitleCue {
        start: Timestamp::new(0, 0, 0).unwrap(),
        end: Timestamp::new(0, 5, 0).unwrap(),
        parts: vec![CuePart {
            marker: "cre".to_string(),
            text: "unknown-role name-a".to_string(),
        }],
    }];
    let err = render_vtt(
        &cues,
        &markers_with_credit_trigger(),
        &credits_with_one_role(),
        &Language::Vietnamese,
    )
    .unwrap_err();
    let RenderVttError::Credits(payload) = err;
    assert_eq!(payload.start, Timestamp::new(0, 0, 0).unwrap());
}
