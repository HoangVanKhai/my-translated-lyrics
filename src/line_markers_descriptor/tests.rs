use super::{CssClassName, InvalidCssClassName, InvalidVoiceName, VoiceName};

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
    assert!(matches!(
        CssClassName::new(String::new()),
        Err(InvalidCssClassName::Empty),
    ));
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
    assert!(matches!(
        CssClassName::new("名字".to_string()),
        Err(InvalidCssClassName::InvalidLeadingCharacter(_)),
    ));
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
