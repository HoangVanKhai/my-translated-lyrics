use crate::video_descriptor::Language;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const SUBTITLE_CONFIG_FILE_NAME: &str = "subtitle.yaml";

/// Parsed contents of a `subtitle.yaml` file.
///
/// Each source directory may contain a `subtitle.yaml` file that carries
/// the structured vocabulary and speaker-role metadata required to parse
/// and render the subtitle files for that song. All fields are optional
/// and default to empty collections.
#[derive(Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct SubtitleDesc {
    /// Ordered list of credit role entries. Each entry maps one or more
    /// language codes to the label used in the credit block for that role.
    #[serde(default)]
    pub credit_roles: Vec<BTreeMap<Language, String>>,
    /// Ordered list of credited person or studio name entries. Each entry
    /// maps one or more language codes to the name as it appears in the
    /// credit block.
    #[serde(default)]
    pub credit_names: Vec<BTreeMap<Language, String>>,
    /// Per-role mapping from dialogue-role marker code to a map of language
    /// codes to the display name used as a voice label in VTT cues.
    #[serde(default)]
    pub speaker_names: BTreeMap<String, BTreeMap<Language, String>>,
}
