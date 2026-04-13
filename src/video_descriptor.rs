use derive_more::{AsRef, Deref, Display, Error, Into};
use itertools::Itertools;
use pipe_trait::Pipe;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Component, Path};
use std::str::FromStr;
use strum::{AsRefStr, EnumString, VariantArray};

pub(crate) const SEPARATED_COLLECTIONS: &[&str] = &[
    "Feng Ling Yu Xiu",
    "Luo Tianyi, Yuezheng Ling/洛天依_乐正绫",
    "Touhou Hero of Ice Fairy",
];

pub(crate) const UNIFIED_COLLECTION: &str = "Short Relaxing Playlist 2025";

pub const VIDEO_CONFIG_FILE_NAME: &str = "video.toml";

/// Parsed contents of a `video.toml` file.
#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct VideoDesc {
    /// Target collection this video belongs to.
    pub(crate) collection: CollectionName,
    /// Title of the video to which this subtitle set applies.
    /// It is used as the stem of target subtitle filenames.
    pub(crate) video_title: VideoTitle,
    /// Titles of the song in each supported language.
    #[expect(dead_code, reason = "not used for now, may be used in the future")]
    song_titles: HashMap<Language, String>,
    #[serde(default)]
    pub(crate) visibility: Visibility,
}

/// Name of a managed target-collection directory. Can only be
/// constructed from values listed in [`SEPARATED_COLLECTIONS`].
#[derive(AsRef, Deref, Display, Into, Deserialize)]
#[as_ref(forward)]
#[deref(forward)]
#[serde(try_from = "String")]
pub(crate) struct CollectionName(
    /// Owned `String` rather than `&'static str`: every valid value is
    /// known statically today, but owning the string leaves room to
    /// replace [`SEPARATED_COLLECTIONS`] with a runtime source later
    /// without breaking the crate API.
    String,
);

impl TryFrom<String> for CollectionName {
    type Error = ParseCollectionNameError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if SEPARATED_COLLECTIONS.contains(&value.as_str()) {
            Ok(CollectionName(value))
        } else {
            Err(ParseCollectionNameError::UnknownCollection(value))
        }
    }
}

#[derive(Debug, Display, Error)]
#[non_exhaustive]
pub(crate) enum ParseCollectionNameError {
    #[display("unknown collection: {_0:?}")]
    UnknownCollection(#[error(not(source))] String),
}

/// Title of a video. The constructor enforces two invariants on the
/// title: it must be a single normal path component (so it can be used
/// directly as the stem of an output filename), and it must contain
/// no backslashes (for cross-platform consistency).
#[derive(AsRef, Deref, Display, Into, Deserialize)]
#[as_ref(forward)]
#[deref(forward)]
#[serde(try_from = "String")]
pub(crate) struct VideoTitle(String);

impl TryFrom<String> for VideoTitle {
    type Error = ParseVideoTitleError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.contains('\\') {
            return Err(ParseVideoTitleError::ContainsBackslash);
        }
        let mut components = value.pipe_ref(Path::new).components();
        match (components.next(), components.next()) {
            (Some(Component::Normal(_)), None) => Ok(VideoTitle(value)),
            _ => Err(ParseVideoTitleError::NotSingleComponent),
        }
    }
}

#[derive(Debug, Display, Error)]
#[non_exhaustive]
pub(crate) enum ParseVideoTitleError {
    #[display("video title must not contain backslashes")]
    ContainsBackslash,
    #[display("video title must be a single normal path component")]
    NotSingleComponent,
}

#[derive(Debug, strum::Display, PartialEq, Eq, Hash, AsRefStr, EnumString, Deserialize)]
#[serde(try_from = "String")]
pub(crate) enum Language {
    #[strum(serialize = "en")]
    English,
    #[strum(serialize = "vi")]
    Vietnamese,
    #[strum(serialize = "zh")]
    Chinese,
}

impl TryFrom<String> for Language {
    type Error = strum::ParseError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

#[derive(Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Visibility {
    /// The target subtitle files should be created and
    /// synchronized with the source.
    #[default]
    Visible,
    /// The target subtitle files should not exist. They are removed
    /// if present.
    Hidden,
    /// The target subtitle files are edited manually. They should
    /// neither be deleted, created, nor synchronized.
    Manual,
}

/// A validated subtitle filename in the `lyrics.{lang}.{ext}` format.
pub struct LyricsFileName {
    language: Language,
    format: SubtitleFormat,
}

impl LyricsFileName {
    /// Combines with a video title to produce the target subtitle
    /// file name.
    pub(crate) fn target_file_name<'a>(
        &'a self,
        video: &'a VideoTitle,
    ) -> impl std::fmt::Display + 'a {
        let LyricsFileName { language, format } = self;
        TargetFileName {
            video,
            language,
            format,
        }
    }
}

impl FromStr for LyricsFileName {
    type Err = ParseLyricsFileNameError;

    fn from_str(file_name: &str) -> Result<Self, Self::Err> {
        let suffix = file_name
            .strip_prefix("lyrics.")
            .ok_or(ParseLyricsFileNameError::NotLyricsFile)?;
        let Some((language, extension)) = suffix.rsplit_once('.') else {
            return Err(ParseLyricsFileNameError::MissingLanguageCode);
        };
        let format = extension
            .parse::<SubtitleFormat>()
            .map_err(drop::<strum::ParseError>)
            .map_err(|()| extension)
            .map_err(<str>::to_string)
            .map_err(ParseLyricsFileNameError::UnsupportedFormat)?;
        if language.is_empty() {
            return Err(ParseLyricsFileNameError::MissingLanguageCode);
        }
        let language = language
            .parse::<Language>()
            .map_err(drop::<strum::ParseError>)
            .map_err(|()| language)
            .map_err(<str>::to_string)
            .map_err(ParseLyricsFileNameError::UnrecognizedLanguage)?;
        Ok(LyricsFileName { language, format })
    }
}

#[derive(strum::Display, Clone, Copy, AsRefStr, EnumString, VariantArray)]
enum SubtitleFormat {
    #[strum(serialize = "srt")]
    SubRip,
    #[strum(serialize = "vtt")]
    WebVtt,
}

#[derive(Debug, Display, Error)]
#[non_exhaustive]
pub enum ParseLyricsFileNameError {
    #[display("filename does not start with \"lyrics.\"")]
    NotLyricsFile,
    #[display("missing language code in lyrics filename")]
    MissingLanguageCode,
    #[display("unsupported subtitle format: {_0:?} (expected one of {})", SubtitleFormat::VARIANTS.iter().format(", "))]
    UnsupportedFormat(#[error(not(source))] String),
    #[display("unrecognized language code: {_0:?}")]
    UnrecognizedLanguage(#[error(not(source))] String),
}

#[derive(Display)]
#[display("{video}.{language}.{format}")]
struct TargetFileName<'a> {
    video: &'a VideoTitle,
    language: &'a Language,
    format: &'a SubtitleFormat,
}

#[cfg(test)]
mod tests {
    use super::{
        CollectionName, Language, LyricsFileName, ParseCollectionNameError,
        ParseLyricsFileNameError, ParseVideoTitleError, SEPARATED_COLLECTIONS, VideoTitle,
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
            "foo\\bar".to_string().pipe(VideoTitle::try_from),
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
}
