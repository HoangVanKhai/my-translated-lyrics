use super::{RenderVttError, render_file};
use crate::credits_descriptor::CreditsDesc;
use crate::generate_subtitles::parse::SubtitleCue;
use crate::line_markers_descriptor::LineMarkersDesc;
use crate::timestamp::Timestamp;
use crate::video_descriptor::Language;
use maplit::btreemap;

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
fn unknown_role_in_credit_line_produces_credits_error() {
    let cues = vec![SubtitleCue {
        start: Timestamp::new(0, 0, 0),
        end: Timestamp::new(0, 5, 0),
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
    let RenderVttError::Credits(payload) = err;
    assert_eq!(payload.start, Timestamp::new(0, 0, 0));
}
