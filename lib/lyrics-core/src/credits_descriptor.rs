use crate::video_descriptor::Language;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

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
    pub credit_roles: Vec<BTreeMap<Language, CreditRole>>,
    /// Ordered list of credited person or studio name entries. Each entry
    /// maps one or more language codes to the name as it appears in the
    /// credit block.
    #[serde(default)]
    pub credit_names: Vec<BTreeMap<Language, CreditName>>,
}

/// The label of a credit role, such as the word a credit line opens with
/// before its names. It imposes no shape of its own.
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct CreditRole(String);

impl CreditRole {
    /// The underlying role text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for CreditRole {
    fn from(source: String) -> Self {
        CreditRole(source)
    }
}

/// The name of a credited person or studio, as it appears in the credit
/// block. Like [`CreditRole`], it imposes no shape of its own.
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct CreditName(String);

impl CreditName {
    /// The underlying name text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for CreditName {
    fn from(source: String) -> Self {
        CreditName(source)
    }
}
