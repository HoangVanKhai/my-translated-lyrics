use std::collections::BTreeMap;

use crate::video_descriptor::Language;

use serde::{Deserialize, Serialize};

pub const CREDITS_CONFIG_FILE_NAME: &str = "credits.yaml";

/// Parsed contents of a `credits.yaml` file.
///
/// Each source directory may contain a `credits.yaml` file that lists the
/// credit roles and credited names displayed in the song's credit block.
/// All fields are optional and default to empty collections.
#[derive(Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct CreditsDesc {
    /// Ordered list of credit role entries. Each entry maps one or more
    /// language codes to the label used in the credit block for that role.
    #[serde(default)]
    pub credit_roles: Vec<BTreeMap<Language, String>>,
    /// Ordered list of credited person or studio name entries. Each entry
    /// maps one or more language codes to the name as it appears in the
    /// credit block.
    #[serde(default)]
    pub credit_names: Vec<BTreeMap<Language, String>>,
}
