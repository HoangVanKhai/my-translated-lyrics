use super::{RenderSrtError, render_srt};
use crate::parse::{CuePart, SubtitleCue};
use crate::styles::{Color, CreditPalette, MissingStyle, StylePalette};
use lyrics_core::credits_descriptor::CreditsDesc;
use lyrics_core::line_markers_descriptor::{CssClassName, LineMarkersDesc, VoiceName};
use lyrics_core::timestamp::Timestamp;
use lyrics_core::video_descriptor::Language;
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

fn color(value: String) -> Color {
    value
        .pipe(Color::new)
        .expect("test fixture passes the color validator")
}

fn test_palette() -> StylePalette {
    StylePalette {
        credit: CreditPalette {
            role: color("#AAAA22".to_owned()),
            name: color("#AAAAAA".to_owned()),
            special: color("#55ABCD".to_owned()),
        },
        voices: btreemap! {},
        classes: btreemap! {},
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
    let output = render_srt(
        &cues,
        &LineMarkersDesc::default(),
        &CreditsDesc::default(),
        &test_palette(),
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

/// A credit cue can mix a role-only header line with role-less bracket
/// lines. The header renders as a bare role span, and each bracketed
/// line renders its highlight in the credit-special color in place of
/// a role span.
#[test]
fn role_only_header_and_role_less_lines_render() {
    let cues = vec![SubtitleCue {
        start: Timestamp::new(0, 0, 0).unwrap(),
        end: Timestamp::new(0, 5, 0).unwrap(),
        parts: vec![CuePart {
            marker: "cre".to_string(),
            text: "role-a\n[label-a] name-a".to_string(),
        }],
    }];
    let output = render_srt(
        &cues,
        &markers_with_credit_trigger(),
        &credits_with_one_role(),
        &test_palette(),
        &Language::Vietnamese,
    )
    .unwrap();
    assert!(
        output.contains(
            "<font color=\"#AAAA22\">role-a</font>\n\
             <font color=\"#55ABCD\">[label-a]</font> <font color=\"#AAAAAA\">name-a</font>",
        ),
        "header should be a bare role span followed by the role-less line:\n{output}",
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
    let err = render_srt(
        &cues,
        &markers_with_credit_trigger(),
        &credits_with_one_role(),
        &test_palette(),
        &Language::Vietnamese,
    )
    .unwrap_err();
    match err {
        RenderSrtError::Credits(payload) => {
            assert_eq!(payload.start, Timestamp::new(0, 0, 0).unwrap());
        }
        other => panic!("expected a credits error, got {other:?}"),
    }
}

#[test]
fn class_declared_without_palette_entry_produces_style_error() {
    let class_name = "title"
        .to_string()
        .pipe(CssClassName::new)
        .expect("test fixture passes the class-name validator");
    let markers = LineMarkersDesc {
        markers: vec!["ttl".to_string()],
        classes: btreemap! { "ttl".to_string() => class_name },
        ..Default::default()
    };
    let cues = vec![SubtitleCue {
        start: Timestamp::new(0, 0, 0).unwrap(),
        end: Timestamp::new(0, 5, 0).unwrap(),
        parts: vec![CuePart {
            marker: "ttl".to_string(),
            text: "body".to_string(),
        }],
    }];
    let err = render_srt(
        &cues,
        &markers,
        &CreditsDesc::default(),
        &test_palette(),
        &Language::Vietnamese,
    )
    .unwrap_err();
    match err {
        RenderSrtError::Style(MissingStyle::Class(name)) => assert_eq!(name, "title"),
        other => panic!("expected a missing-class-style error, got {other:?}"),
    }
}

#[test]
fn voice_declared_without_palette_entry_produces_style_error() {
    let voice_name = "Some Voice"
        .to_string()
        .pipe(VoiceName::new)
        .expect("test fixture passes the voice-name validator");
    let markers = LineMarkersDesc {
        markers: vec!["unk".to_string()],
        voices: btreemap! {
            "unk".to_string() => btreemap! { Language::Vietnamese => voice_name },
        },
        ..Default::default()
    };
    let cues = vec![SubtitleCue {
        start: Timestamp::new(0, 0, 0).unwrap(),
        end: Timestamp::new(0, 5, 0).unwrap(),
        parts: vec![CuePart {
            marker: "unk".to_string(),
            text: "body".to_string(),
        }],
    }];
    let err = render_srt(
        &cues,
        &markers,
        &CreditsDesc::default(),
        &test_palette(),
        &Language::Vietnamese,
    )
    .unwrap_err();
    match err {
        RenderSrtError::Style(MissingStyle::Voice(name)) => assert_eq!(name, "unk"),
        other => panic!("expected a missing-voice-style error, got {other:?}"),
    }
}
