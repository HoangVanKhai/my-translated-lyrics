use crate::video_descriptor::Language;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const SUBTITLE_CONFIG_FILE_NAME: &str = "subtitle.toml";

/// Parsed contents of a `subtitle.toml` file.
///
/// Each source directory may contain a `subtitle.toml` file that carries
/// the structured vocabulary and speaker-role metadata required to parse
/// and render the subtitle files for that song. All fields are optional
/// and default to empty collections.
#[derive(Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct SubtitleDesc {
    /// Ordered list of credit role entries. Each entry maps one or more
    /// language codes to the label used in the credit block for that role,
    /// such as "演唱" (zh) or "Trình bày" (vi).
    #[serde(default)]
    pub credit_roles: Vec<HashMap<Language, String>>,
    /// Ordered list of credited person or studio name entries. Each entry
    /// maps one or more language codes to the name as it appears in the
    /// credit block.
    #[serde(default)]
    pub credit_names: Vec<HashMap<Language, String>>,
    /// Per-language mapping from dialogue-role marker code to the display
    /// name used as a voice label in VTT cues.
    #[serde(default)]
    pub speaker_names: HashMap<Language, HashMap<String, String>>,
}
