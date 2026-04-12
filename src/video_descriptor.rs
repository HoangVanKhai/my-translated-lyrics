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
    pub(crate) collection: Collection,
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
pub(crate) struct Collection(
    /// Owned `String` rather than `&'static str`: every valid value is
    /// known statically today, but owning the string leaves room to
    /// replace [`SEPARATED_COLLECTIONS`] with a runtime source later
    /// without breaking the crate API.
    String,
);

impl TryFrom<String> for Collection {
    type Error = UnknownCollection;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if SEPARATED_COLLECTIONS.contains(&value.as_str()) {
            Ok(Self(value))
        } else {
            Err(UnknownCollection(value))
        }
    }
}

#[derive(Debug, Display, Error)]
#[display("unknown collection: {_0:?}")]
pub(crate) struct UnknownCollection(#[error(not(source))] String);

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
    type Error = VideoTitleError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        // Backslashes are rejected explicitly so configs behave consistently
        // regardless of platform (on Unix, `Path::components` treats `\` as
        // a normal character).
        if value.contains('\\') {
            return Err(VideoTitleError::ContainsBackslash);
        }
        let mut components = value.pipe_ref(Path::new).components();
        match (components.next(), components.next()) {
            (Some(Component::Normal(_)), None) => Ok(Self(value)),
            _ => Err(VideoTitleError::NotSingleComponent),
        }
    }
}

#[derive(Debug, Display, Error)]
pub(crate) enum VideoTitleError {
    #[display("video_title must not contain backslashes")]
    ContainsBackslash,
    #[display("video_title must be a single normal path component")]
    NotSingleComponent,
}

#[derive(PartialEq, Eq, Hash, EnumString, Deserialize)]
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
    use super::*;

    #[test]
    fn collection_accepts_known_values() {
        for value in SEPARATED_COLLECTIONS {
            let result = Collection::try_from(value.to_string());
            assert!(result.is_ok(), "expected {value:?} to be accepted");
        }
    }

    #[test]
    fn collection_rejects_unknown_value() {
        assert!(Collection::try_from("Unknown Collection".to_string()).is_err());
    }

    #[test]
    fn video_title_accepts_normal_component() {
        let result = VideoTitle::try_from(
            "【示例表演者】《示例歌曲》Example Song [ExampleVideoID]".to_string(),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn video_title_rejects_backslash() {
        let result = VideoTitle::try_from("foo\\bar".to_string());
        assert!(matches!(result, Err(VideoTitleError::ContainsBackslash)));
    }

    #[test]
    fn video_title_rejects_slash() {
        let result = VideoTitle::try_from("foo/bar".to_string());
        assert!(matches!(result, Err(VideoTitleError::NotSingleComponent)));
    }

    #[test]
    fn video_title_rejects_empty() {
        let result = VideoTitle::try_from(String::new());
        assert!(matches!(result, Err(VideoTitleError::NotSingleComponent)));
    }

    #[test]
    fn video_title_rejects_dot_dot() {
        let result = VideoTitle::try_from("..".to_string());
        assert!(matches!(result, Err(VideoTitleError::NotSingleComponent)));
    }

    #[test]
    fn language_accepts_known_codes() {
        assert!(Language::try_from("en".to_string()).is_ok());
        assert!(Language::try_from("vi".to_string()).is_ok());
        assert!(Language::try_from("zh".to_string()).is_ok());
    }

    #[test]
    fn language_rejects_unknown_code() {
        assert!(Language::try_from("ja".to_string()).is_err());
        assert!(Language::try_from("xx".to_string()).is_err());
        assert!(Language::try_from(String::new()).is_err());
    }
}
