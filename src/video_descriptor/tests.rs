use super::{
    CollectionName, Language, LyricsFileName, ParseCollectionNameError, ParseLyricsFileNameError,
    ParseVideoTitleError, SEPARATED_COLLECTIONS, VideoTitle,
};
use pipe_trait::Pipe;

#[test]
fn collection_name_accepts_known_values() {
    for &value in SEPARATED_COLLECTIONS {
        eprintln!("CASE: {value:?}");
        let name = value.to_string().pipe(CollectionName::try_from).unwrap();
        assert_eq!(&*name, value);
    }
}

#[test]
fn collection_name_rejects_unknown_value() {
    assert!(matches!(
        "Unknown Collection"
            .to_string()
            .pipe(CollectionName::try_from),
        Err(ParseCollectionNameError::UnknownCollection(_))
    ));
}

#[test]
fn video_title_accepts_normal_component() {
    let cases = [
        "【示例表演者】《示例歌曲》Example Song [ExampleID]",
        "【示例表演者 | 日本語タグ】《示例歌曲名》 [ExampleID]",
        "【示例表演者】示例歌(Example Song)——“示例歌词”【示例标签】 [ExampleID]",
        "【示例表演者】回舟《示例歌》(Example Song)，示例描述。【示例标签】 [ExampleID]",
        "【示例表演者】示例歌(Example Song)\u{3000}【示例标签】 [ExampleID]",
        "【示例表演者】《示例歌曲》SuffixText020 [ExampleID]",
        "【FULL ver.】Example Performer 示例表演者 - Example Song 示例歌曲【示例标签】",
    ];
    for input in cases {
        eprintln!("CASE: {input:?}");
        let title = input.to_string().pipe(VideoTitle::try_from).unwrap();
        assert_eq!(&*title, input);
    }
}

#[test]
fn video_title_rejects_backslash() {
    assert!(matches!(
        r"foo\bar".to_string().pipe(VideoTitle::try_from),
        Err(ParseVideoTitleError::ContainsBackslash)
    ));
}

#[test]
fn video_title_rejects_slash() {
    assert!(matches!(
        "foo/bar".to_string().pipe(VideoTitle::try_from),
        Err(ParseVideoTitleError::NotSingleComponent)
    ));
}

#[test]
fn video_title_rejects_empty() {
    assert!(matches!(
        String::new().pipe(VideoTitle::try_from),
        Err(ParseVideoTitleError::NotSingleComponent)
    ));
}

#[test]
fn video_title_rejects_dot_dot() {
    assert!(matches!(
        "..".to_string().pipe(VideoTitle::try_from),
        Err(ParseVideoTitleError::NotSingleComponent)
    ));
}

#[test]
fn language_accepts_known_codes() {
    let cases = [
        ("en", Language::English),
        ("vi", Language::Vietnamese),
        ("zh", Language::Chinese),
    ];
    for (input, expected) in cases {
        eprintln!("CASE: {input:?} → {expected:?}");
        let actual = input.to_string().pipe(Language::try_from).unwrap();
        assert_eq!(actual, expected);
    }
}

#[test]
fn language_rejects_unknown_code() {
    assert!("ja".to_string().pipe(Language::try_from).is_err());
    assert!("xx".to_string().pipe(Language::try_from).is_err());
    assert!(String::new().pipe(Language::try_from).is_err());
}

#[test]
fn lyrics_file_name_parses_valid() {
    let video_title = "Example Title"
        .to_string()
        .pipe(VideoTitle::try_from)
        .unwrap();
    let cases = [
        ("lyrics.vi.srt", "Example Title.vi.srt"),
        ("lyrics.en.vtt", "Example Title.en.vtt"),
        ("lyrics.zh.srt", "Example Title.zh.srt"),
    ];
    for (input, expected) in cases {
        eprintln!("CASE: {input:?}");
        let file_name: LyricsFileName = input.parse().unwrap();
        let actual = file_name.target_file_name(&video_title).to_string();
        assert_eq!(actual, expected);
    }
}

#[test]
fn lyrics_file_name_rejects_no_prefix() {
    assert!(matches!(
        "continuous.srt".parse::<LyricsFileName>(),
        Err(ParseLyricsFileNameError::NotLyricsFile)
    ));
}

#[test]
fn lyrics_file_name_rejects_bad_extension() {
    assert!(matches!(
        "lyrics.vi.txt".parse::<LyricsFileName>(),
        Err(ParseLyricsFileNameError::UnsupportedFormat(_))
    ));
}

#[test]
fn lyrics_file_name_rejects_no_lang() {
    assert!(matches!(
        "lyrics.srt".parse::<LyricsFileName>(),
        Err(ParseLyricsFileNameError::MissingLanguageCode)
    ));
}

#[test]
fn lyrics_file_name_rejects_empty_lang() {
    assert!(matches!(
        "lyrics..srt".parse::<LyricsFileName>(),
        Err(ParseLyricsFileNameError::MissingLanguageCode)
    ));
}

#[test]
fn lyrics_file_name_rejects_unknown_lang() {
    assert!(matches!(
        "lyrics.ja.srt".parse::<LyricsFileName>(),
        Err(ParseLyricsFileNameError::UnrecognizedLanguage(_))
    ));
}
