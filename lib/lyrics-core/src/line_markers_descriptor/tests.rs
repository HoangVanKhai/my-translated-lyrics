use super::{
    CssClassName, InvalidCssClassName, InvalidMarkerName, InvalidVoiceName, LineMarkersDesc,
    MarkerName, ReservedMarker, VoiceName,
};
use pipe_trait::Pipe;
use pretty_assertions::assert_eq;
use serde::{Deserialize, Serialize};
use strum::VariantArray;
use text_block_macros::text_block_fnl;

#[test]
fn accepts_simple_ascii_names() {
    assert_eq!(
        "title"
            .to_string()
            .pipe(CssClassName::new)
            .unwrap()
            .as_str(),
        "title",
    );
    assert_eq!(
        "creditRole"
            .to_string()
            .pipe(CssClassName::new)
            .unwrap()
            .as_str(),
        "creditRole",
    );
    assert_eq!(
        "_hidden"
            .to_string()
            .pipe(CssClassName::new)
            .unwrap()
            .as_str(),
        "_hidden",
    );
    assert_eq!(
        "kebab-name_42"
            .to_string()
            .pipe(CssClassName::new)
            .unwrap()
            .as_str(),
        "kebab-name_42",
    );
}

#[test]
fn rejects_empty() {
    assert_eq!(
        String::new().pipe(CssClassName::new).unwrap_err(),
        InvalidCssClassName::Empty,
    );
}

#[test]
fn rejects_leading_digit_hyphen_or_non_ascii() {
    assert_eq!(
        "1name".to_string().pipe(CssClassName::new).unwrap_err(),
        InvalidCssClassName::InvalidLeadingCharacter('1'),
    );
    assert_eq!(
        "-name".to_string().pipe(CssClassName::new).unwrap_err(),
        InvalidCssClassName::InvalidLeadingCharacter('-'),
    );
    assert_eq!(
        "名字".to_string().pipe(CssClassName::new).unwrap_err(),
        InvalidCssClassName::InvalidLeadingCharacter('名'),
    );
}

#[test]
fn rejects_unsafe_continue_characters() {
    assert_eq!(
        "bad name".to_string().pipe(CssClassName::new).unwrap_err(),
        InvalidCssClassName::InvalidCharacter(' '),
    );
    assert_eq!(
        "bad.name".to_string().pipe(CssClassName::new).unwrap_err(),
        InvalidCssClassName::InvalidCharacter('.'),
    );
    assert_eq!(
        r#"bad"name"#.to_string().pipe(CssClassName::new).unwrap_err(),
        InvalidCssClassName::InvalidCharacter('"'),
    );
}

#[test]
fn voice_name_accepts_cjk_latin_and_embedded_space() {
    assert_eq!(
        "名字一".to_string().pipe(VoiceName::new).unwrap().as_str(),
        "名字一",
    );
    assert_eq!(
        "Voz Ñ".to_string().pipe(VoiceName::new).unwrap().as_str(),
        "Voz Ñ",
    );
    assert_eq!(
        "voice-a".to_string().pipe(VoiceName::new).unwrap().as_str(),
        "voice-a",
    );
}

#[test]
fn voice_name_rejects_empty() {
    assert_eq!(
        String::new().pipe(VoiceName::new).unwrap_err(),
        InvalidVoiceName::Empty,
    );
}

#[test]
fn voice_name_rejects_webvtt_and_css_meta_characters() {
    for char in ['<', '>', '"', '\\'] {
        assert_eq!(
            format!("bad{char}name").pipe(VoiceName::new).unwrap_err(),
            InvalidVoiceName::ForbiddenCharacter(char),
        );
    }
}

#[test]
fn voice_name_rejects_control_and_line_separator_characters() {
    for char in ['\n', '\r', '\t', '\u{2028}', '\u{2029}'] {
        assert_eq!(
            format!("bad{char}name").pipe(VoiceName::new).unwrap_err(),
            InvalidVoiceName::ForbiddenCharacter(char),
        );
    }
}

/// Wrapper so `toml::from_str` has a root table to deserialize into.
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct CssClassHolder {
    value: CssClassName,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct VoiceNameHolder {
    value: VoiceName,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct MarkerNameHolder {
    value: MarkerName,
}

#[test]
fn css_class_name_round_trips_through_toml() {
    let original = CssClassHolder {
        value: "kebab-name_42".to_string().pipe(CssClassName::new).unwrap(),
    };
    let serialized = toml::to_string(&original).unwrap();
    let deserialized: CssClassHolder = toml::from_str(&serialized).unwrap();
    assert_eq!(deserialized, original);
}

#[test]
fn css_class_name_toml_rejects_invalid_source() {
    let err = toml::from_str::<CssClassHolder>(r#"value = "bad name""#).unwrap_err();
    assert!(
        err.to_string().contains("class name"),
        "error message should surface the validator's diagnostic: {err}",
    );
}

#[test]
fn voice_name_round_trips_through_toml() {
    let original = VoiceNameHolder {
        value: "Voz Ñ".to_string().pipe(VoiceName::new).unwrap(),
    };
    let serialized = toml::to_string(&original).unwrap();
    let deserialized: VoiceNameHolder = toml::from_str(&serialized).unwrap();
    assert_eq!(deserialized, original);
}

#[test]
fn voice_name_toml_rejects_invalid_source() {
    let err = toml::from_str::<VoiceNameHolder>(r#"value = "bad<name""#).unwrap_err();
    assert!(
        err.to_string().contains("voice name"),
        "error message should surface the validator's diagnostic: {err}",
    );
}

#[test]
fn marker_name_accepts_ordinary_tokens() {
    for source in ["ttl", "cre", "MKA", "mk-b", "m+n", "名字"] {
        eprintln!("CASE: {source:?}");
        assert_eq!(
            source.to_string().pipe(MarkerName::new).unwrap().as_str(),
            source,
        );
    }
}

/// The rejected set is read off [`ReservedMarker`] rather than
/// restated here, so a marker added to the enum is covered by this
/// test without an edit.
#[test]
fn marker_name_rejects_every_reserved_marker() {
    for reserved in ReservedMarker::VARIANTS {
        eprintln!("CASE: {reserved:?}");
        assert_eq!(
            reserved
                .as_ref()
                .to_string()
                .pipe(MarkerName::new)
                .unwrap_err(),
            InvalidMarkerName::Reserved(*reserved),
        );
    }
}

#[test]
fn marker_name_round_trips_through_toml() {
    let original = MarkerNameHolder {
        value: "m+n".to_string().pipe(MarkerName::new).unwrap(),
    };
    let serialized = toml::to_string(&original).unwrap();
    let deserialized: MarkerNameHolder = toml::from_str(&serialized).unwrap();
    assert_eq!(deserialized, original);
}

/// A `line-markers.toml` naming a reserved marker fails to parse
/// wherever the name appears. The four groups are spelled out one by
/// one because each has its own TOML shape; the rule itself is
/// carried by [`MarkerName`], which is the type of every marker name
/// in the descriptor, so a group added later is covered by
/// construction rather than by an addition to this list.
#[test]
fn line_markers_descriptor_rejects_a_reserved_marker_in_every_group() {
    for reserved in ReservedMarker::VARIANTS {
        let marker = reserved.as_ref();
        let sources = [
            format!("markers = [{marker:?}]"),
            format!("credits = [{marker:?}]"),
            format!(r#"voices = {{ {marker:?} = {{ vi = "Voice A" }} }}"#),
            format!(r#"classes = {{ {marker:?} = "title" }}"#),
        ];
        for source in sources {
            eprintln!("CASE: {source:?}");
            let Err(error) = toml::from_str::<LineMarkersDesc>(&source) else {
                panic!("expected {source:?} to be rejected");
            };
            assert!(
                error.to_string().contains("is reserved by the parser"),
                "error message should surface the validator's diagnostic \
                 for {source:?}: {error}",
            );
        }
    }
}

/// A descriptor that declares no reserved marker still parses, with
/// each group landing where it belongs.
#[test]
fn line_markers_descriptor_accepts_ordinary_markers() {
    let source = text_block_fnl! {
        r#"markers = ["cre", "ttl", "vca"]"#
        r#"credits = ["cre"]"#
        ""
        "[voices]"
        r#"vca = { vi = "Voice A" }"#
        ""
        "[classes]"
        r#"ttl = "title""#
    };
    let descriptor: LineMarkersDesc = toml::from_str(source).unwrap();
    let markers: Vec<&str> = descriptor.markers.iter().map(MarkerName::as_str).collect();
    assert_eq!(markers, ["cre", "ttl", "vca"]);
    assert!(descriptor.is_credit("cre"));
    assert!(!descriptor.is_credit("ttl"));
    assert!(descriptor.voices.contains_key("vca"));
    assert_eq!(
        descriptor.classes.get("ttl").map(CssClassName::as_str),
        Some("title"),
    );
}
