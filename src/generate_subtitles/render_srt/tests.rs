use super::{RenderSrtError, render_file};
use crate::credits_descriptor::CreditsDesc;
use crate::generate_subtitles::parse::SubtitleCue;
use crate::line_markers_descriptor::LineMarkersDesc;
use crate::timestamp::Timestamp;
use crate::video_descriptor::Language;
use maplit::btreemap;
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
        marker: "plain".to_string(),
        text: "<a> & <b>".to_string(),
    }];
    let output = render_file(
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
fn unknown_role_in_credit_line_produces_credits_error() {
    let cues = vec![SubtitleCue {
        start: Timestamp::new(0, 0, 0).unwrap(),
        end: Timestamp::new(0, 5, 0).unwrap(),
        marker: "cre".to_string(),
        text: "unknown-role name-a".to_string(),
    }];
    let err = render_file(
        &cues,
        &markers_with_credit_trigger(),
        &credits_with_one_role(),
        &Language::Vietnamese,
    )
    .unwrap_err();
    let RenderSrtError::Credits(payload) = err;
    assert_eq!(payload.start, Timestamp::new(0, 0, 0).unwrap());
}
