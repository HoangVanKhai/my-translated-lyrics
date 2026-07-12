use super::{CssClassName, InvalidCssClassName, InvalidVoiceName, VoiceName};
use pipe_trait::Pipe;
use pretty_assertions::assert_eq;
use serde::{Deserialize, Serialize};

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
