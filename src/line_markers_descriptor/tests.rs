use super::{CssClassName, InvalidCssClassName, InvalidVoiceName, VoiceName};
use pretty_assertions::assert_eq;
use serde::{Deserialize, Serialize};

#[test]
fn accepts_simple_ascii_names() {
    assert_eq!(
        CssClassName::new("title".to_string()).unwrap().as_str(),
        "title",
    );
    assert_eq!(
        CssClassName::new("creditRole".to_string())
            .unwrap()
            .as_str(),
        "creditRole",
    );
    assert_eq!(
        CssClassName::new("_hidden".to_string()).unwrap().as_str(),
        "_hidden",
    );
    assert_eq!(
        CssClassName::new("kebab-name_42".to_string())
            .unwrap()
            .as_str(),
        "kebab-name_42",
    );
}

#[test]
fn rejects_empty() {
    assert_eq!(
        CssClassName::new(String::new()).unwrap_err(),
        InvalidCssClassName::Empty,
    );
}

#[test]
fn rejects_leading_digit_hyphen_or_non_ascii() {
    assert_eq!(
        CssClassName::new("1name".to_string()).unwrap_err(),
        InvalidCssClassName::InvalidLeadingCharacter('1'),
    );
    assert_eq!(
        CssClassName::new("-name".to_string()).unwrap_err(),
        InvalidCssClassName::InvalidLeadingCharacter('-'),
    );
    assert_eq!(
        CssClassName::new("名字".to_string()).unwrap_err(),
        InvalidCssClassName::InvalidLeadingCharacter('名'),
    );
}

#[test]
fn rejects_unsafe_continue_characters() {
    assert_eq!(
        CssClassName::new("bad name".to_string()).unwrap_err(),
        InvalidCssClassName::InvalidCharacter(' '),
    );
    assert_eq!(
        CssClassName::new("bad.name".to_string()).unwrap_err(),
        InvalidCssClassName::InvalidCharacter('.'),
    );
    assert_eq!(
        CssClassName::new("bad\"name".to_string()).unwrap_err(),
        InvalidCssClassName::InvalidCharacter('"'),
    );
}

#[test]
fn voice_name_accepts_cjk_latin_and_embedded_space() {
    assert_eq!(
        VoiceName::new("名字一".to_string()).unwrap().as_str(),
        "名字一",
    );
    assert_eq!(
        VoiceName::new("Voz Ñ".to_string()).unwrap().as_str(),
        "Voz Ñ",
    );
    assert_eq!(
        VoiceName::new("voice-a".to_string()).unwrap().as_str(),
        "voice-a",
    );
}

#[test]
fn voice_name_rejects_empty() {
    assert_eq!(
        VoiceName::new(String::new()).unwrap_err(),
        InvalidVoiceName::Empty,
    );
}

#[test]
fn voice_name_rejects_webvtt_and_css_meta_characters() {
    for ch in ['<', '>', '"', '\\'] {
        let source = format!("bad{ch}name");
        assert_eq!(
            VoiceName::new(source).unwrap_err(),
            InvalidVoiceName::ForbiddenCharacter(ch),
        );
    }
}

#[test]
fn voice_name_rejects_control_and_line_separator_characters() {
    for ch in ['\n', '\r', '\t', '\u{2028}', '\u{2029}'] {
        let source = format!("bad{ch}name");
        assert_eq!(
            VoiceName::new(source).unwrap_err(),
            InvalidVoiceName::ForbiddenCharacter(ch),
        );
    }
}

/// Wrapper so `toml::from_str` has a root table to deserialize into.
#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct CssClassHolder {
    value: CssClassName,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct VoiceNameHolder {
    value: VoiceName,
}

#[test]
fn css_class_name_round_trips_through_toml() {
    let original = CssClassHolder {
        value: CssClassName::new("kebab-name_42".to_string()).unwrap(),
    };
    let serialized = toml::to_string(&original).unwrap();
    let deserialized: CssClassHolder = toml::from_str(&serialized).unwrap();
    assert_eq!(deserialized, original);
}

#[test]
fn css_class_name_toml_rejects_invalid_source() {
    let err = toml::from_str::<CssClassHolder>("value = \"bad name\"").unwrap_err();
    assert!(
        err.to_string().contains("class name"),
        "error message should surface the validator's diagnostic: {err}",
    );
}

#[test]
fn voice_name_round_trips_through_toml() {
    let original = VoiceNameHolder {
        value: VoiceName::new("Voz Ñ".to_string()).unwrap(),
    };
    let serialized = toml::to_string(&original).unwrap();
    let deserialized: VoiceNameHolder = toml::from_str(&serialized).unwrap();
    assert_eq!(deserialized, original);
}

#[test]
fn voice_name_toml_rejects_invalid_source() {
    let err = toml::from_str::<VoiceNameHolder>("value = \"bad<name\"").unwrap_err();
    assert!(
        err.to_string().contains("voice name"),
        "error message should surface the validator's diagnostic: {err}",
    );
}
