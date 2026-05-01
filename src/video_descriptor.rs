use derive_more::{AsRef, Deref, Display, Error, Into};
use itertools::Itertools;
use pipe_trait::Pipe;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Component, Path};
use std::str::FromStr;
use strum::{AsRefStr, EnumString, VariantArray};

pub const SEPARATED_COLLECTIONS: &[&str] = &[
    "Feng Ling Yu Xiu",
    "Luo Tianyi, Yuezheng Ling/洛天依_乐正绫",
    "Touhou Hero of Ice Fairy",
];

pub const UNIFIED_COLLECTION: &str = "Short Relaxing Playlist 2025";

pub const VIDEO_CONFIG_FILE_NAME: &str = "video.toml";

/// Parsed contents of a `video.toml` file.
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct VideoDesc {
    /// Target collection this video belongs to.
    pub collection: CollectionName,
    /// Title of the video to which this subtitle set applies.
    /// It is used as the stem of target subtitle filenames.
    pub video_title: VideoTitle,
    /// Titles of the song in each supported language.
    pub song_titles: HashMap<Language, String>,
    #[serde(default)]
    pub visibility: Visibility,
}

/// Name of a managed target-collection directory. Can only be
/// constructed from values listed in [`SEPARATED_COLLECTIONS`].
#[derive(Clone, AsRef, Deref, Display, Into, Deserialize, Serialize)]
#[as_ref(forward)]
#[deref(forward)]
#[serde(try_from = "String", into = "String")]
pub struct CollectionName(
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
pub enum ParseCollectionNameError {
    #[display("unknown collection: {_0:?}")]
    UnknownCollection(#[error(not(source))] String),
}

/// Title of a video. The constructor enforces two invariants on the
/// title: it must be a single normal path component (so it can be used
/// directly as the stem of an output filename), and it must contain
/// no backslashes (for cross-platform consistency).
#[derive(Clone, AsRef, Deref, Display, Into, Deserialize, Serialize)]
#[as_ref(forward)]
#[deref(forward)]
#[serde(try_from = "String", into = "String")]
pub struct VideoTitle(String);

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
pub enum ParseVideoTitleError {
    #[display("video title must not contain backslashes")]
    ContainsBackslash,
    #[display("video title must be a single normal path component")]
    NotSingleComponent,
}

#[derive(
    Debug,
    Clone,
    Copy,
    strum::Display,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    AsRefStr,
    EnumString,
    Deserialize,
    Serialize,
)]
#[serde(try_from = "String", into = "String")]
pub enum Language {
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

impl From<Language> for String {
    fn from(value: Language) -> Self {
        value.to_string()
    }
}

#[derive(Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Visibility {
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
    #[display(r#"filename does not start with "lyrics.""#)]
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
mod tests;
