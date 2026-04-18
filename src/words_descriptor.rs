use crate::video_descriptor::Language;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const WORDS_CONFIG_FILE_NAME: &str = "words.toml";

/// Parsed contents of a `words.toml` file.
///
/// Each source directory may contain a `words.toml` file that records
/// the special vocabulary used in its subtitle files: credit roles,
/// credit names, songstress names, and song titles. All fields are
/// optional and default to empty collections.
#[derive(Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct WordsDesc {
    /// Per-language list of credit role labels. Credit roles are the
    /// labels that precede a colon or a column separator in a credit
    /// block, such as "演唱" or "Trình bày".
    #[serde(default)]
    pub credit_roles: HashMap<Language, Vec<String>>,
    /// Per-language list of credit names. Credit names are the values
    /// that follow a credit role, such as person or studio names.
    #[serde(default)]
    pub credit_names: HashMap<Language, Vec<String>>,
    /// Per-language mapping from marker code to the songstress display
    /// name used as a voice label in VTT files.
    #[serde(default)]
    pub songstress_names: HashMap<Language, HashMap<String, String>>,
    /// Per-language song title text.
    #[serde(default)]
    pub song_titles: HashMap<Language, String>,
}
