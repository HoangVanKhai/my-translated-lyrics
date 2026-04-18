use crate::video_descriptor::Language;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const LINE_MARKERS_CONFIG_FILE_NAME: &str = "line-markers.toml";

/// Parsed contents of a `line-markers.toml` file.
///
/// Each source directory may contain a `line-markers.toml` file that lists
/// the short codes that may appear as prefixes in the song's subtitle
/// files, grouped by the kind of marker they represent. All sections are
/// optional and default to empty collections.
#[derive(Default, Deserialize, Serialize)]
pub struct LineMarkersDesc {
    /// Mapping from dialogue-role code (for example `LTY`, `Y+L`) to the
    /// display name rendered as a voice label in VTT cues.
    #[serde(default)]
    pub speakers: BTreeMap<String, BTreeMap<Language, String>>,
}
