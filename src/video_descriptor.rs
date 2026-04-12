use derive_more::{AsRef, Deref, Display, Error, Into};
use pipe_trait::Pipe;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Component, Path};
use strum::EnumString;

pub(crate) const SEPARATED_COLLECTIONS: &[&str] = &[
    "Feng Ling Yu Xiu",
    "Luo Tianyi, Yuezheng Ling/洛天依_乐正绫",
    "Touhou Hero of Ice Fairy",
];

pub(crate) const VIDEO_CONFIG_FILENAME: &str = "video.toml";

#[derive(Deserialize)]
pub(crate) struct VideoDesc {
    /// Target collection this video belongs to.
    pub(crate) collection: CollectionName,
    /// Title of the video to which this subtitle set applies.
    /// It is used as the stem of target subtitle filenames.
    pub(crate) video_title: VideoTitle,
    /// Titles of the song in each supported language.
    #[serde(rename = "song-titles")]
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
            Ok(Self(value))
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
            (Some(Component::Normal(_)), None) => Ok(Self(value)),
            _ => Err(ParseVideoTitleError::NotSingleComponent),
        }
    }
}

#[derive(Debug, Display, Error)]
#[non_exhaustive]
pub(crate) enum ParseVideoTitleError {
    #[display("video_title must not contain backslashes")]
    ContainsBackslash,
    #[display("video_title must be a single normal path component")]
    NotSingleComponent,
}

#[derive(Debug, PartialEq, Eq, Hash, EnumString, Deserialize)]
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
pub(crate) enum Visibility {
    /// The target subtitle files should be created and
    /// synchronized with the source.
    #[default]
    #[serde(rename = "visible")]
    Visible,
    /// The target subtitle files should not exist. They are removed
    /// if present.
    #[serde(rename = "hidden")]
    Hidden,
    /// The target subtitle files are edited manually. They should
    /// neither be deleted, created, nor synchronized.
    #[serde(rename = "manual")]
    Manual,
}

#[cfg(test)]
mod tests {
    use super::{
        CollectionName, Language, ParseCollectionNameError, ParseVideoTitleError,
        SEPARATED_COLLECTIONS, VideoTitle,
    };
    use pipe_trait::Pipe;

    #[test]
    fn collection_name_accepts_known_values() {
        for &value in SEPARATED_COLLECTIONS {
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
            "【示例表演者】《示例歌曲》Example Song [ExampleVideoID]",
            "【示例表演者 | 日本語タグ】《示例歌曲名》 [ExampleID]",
            "【示例表演者】示例歌(Example Song)——“示例歌词”【示例标签】 [ExampleID]",
            "【示例表演者】回舟《示例歌》(Example)，归去示例。【示例标签】 [ExampleID]",
            "【示例表演者】示例歌(Example Song)  【示例标签】 [ExampleID]",
            "【示例表演者】《示例歌曲》SuffixText020 [ExampleID]",
            "【FULL ver.】Example Performer 示例表演者 - Example Song 示例歌曲【示例标签】",
        ];
        for input in cases {
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
        assert_eq!(
            "en".to_string().pipe(Language::try_from).unwrap(),
            Language::English
        );
        assert_eq!(
            "vi".to_string().pipe(Language::try_from).unwrap(),
            Language::Vietnamese
        );
        assert_eq!(
            "zh".to_string().pipe(Language::try_from).unwrap(),
            Language::Chinese
        );
    }

    #[test]
    fn language_rejects_unknown_code() {
        assert!("ja".to_string().pipe(Language::try_from).is_err());
        assert!("xx".to_string().pipe(Language::try_from).is_err());
        assert!(String::new().pipe(Language::try_from).is_err());
    }
}
