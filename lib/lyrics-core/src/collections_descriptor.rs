//! Declarative list of the collections in the target media library.
//!
//! The library is a flat set of collection directories. Each video
//! belongs to one separated collection, named by the `collection` field
//! of its `video.toml`, and every video additionally appears in the one
//! unified collection. Both are declared by a single `collections.toml`
//! manifest that sits beside the video directories it describes, rather
//! than being hardcoded in Rust, so adding a collection is a data edit.
//!
//! Declaring the separated collections in one place also makes a typo in
//! an individual `video.toml` detectable: a descriptor that names a
//! collection the manifest does not declare is an
//! [`UndeclaredCollection`] error rather than a silently created
//! directory.

use derive_more::{AsRef, Deref, Display, Into};
use serde::{Deserialize, Serialize};
use std::iter::once;

/// Name of the collections manifest, relative to the directory of video
/// descriptors it describes.
pub const COLLECTIONS_CONFIG_FILE_NAME: &str = "collections.toml";

/// Parsed contents of a `collections.toml` file.
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct CollectionsDesc {
    /// The collection that receives a copy of every subtitle file, in
    /// addition to the separated collection of its video.
    pub unified: CollectionName,
    /// The collections a video descriptor may name. A manifest that
    /// omits the field declares none of them.
    #[serde(default)]
    pub separated: Vec<CollectionName>,
}

impl CollectionsDesc {
    /// Every declared collection: the separated ones in the order the
    /// manifest lists them, followed by the unified one.
    pub fn names(&self) -> impl Iterator<Item = &CollectionName> {
        self.separated.iter().chain(once(&self.unified))
    }

    /// Checks that `name` is one of the declared separated collections.
    /// A video descriptor may only name a collection the manifest
    /// declares, so a misspelled name fails the run.
    pub fn check_separated(&self, name: &str) -> Result<(), UndeclaredCollection> {
        if self.separated.iter().any(|declared| &**declared == name) {
            return Ok(());
        }
        Err(UndeclaredCollection {
            name: name.to_string(),
            declared: self.separated.iter().map(ToString::to_string).collect(),
        })
    }
}

/// A collection name that the manifest does not declare among its
/// separated collections.
#[derive(Debug, Display)]
#[display("unknown collection: {name:?} (expected one of {declared:?})")]
pub struct UndeclaredCollection {
    /// The name that was looked up.
    name: String,
    /// The separated collections the manifest declares, listed so the
    /// message shows what the name was expected to match.
    declared: Vec<String>,
}

/// Name of a collection directory, relative to the root of the target
/// media library.
///
/// The constructor accepts the names that stay inside that root once
/// appended to it, and rejects the rest with the
/// [`ParseCollectionNameError`] variant that states the broken rule.
///
/// Whether a name of an acceptable shape is declared anywhere is a
/// question for [`CollectionsDesc`], not for this type.
#[derive(AsRef, Clone, Deref, Deserialize, Display, Into, Serialize)]
#[as_ref(forward)]
#[deref(forward)]
#[serde(try_from = "String", into = "String")]
pub struct CollectionName(String);

impl TryFrom<String> for CollectionName {
    type Error = ParseCollectionNameError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.contains('\\') {
            return Err(ParseCollectionNameError::ContainsBackslash);
        }
        if value.is_empty() {
            return Err(ParseCollectionNameError::Empty);
        }
        value.split('/').try_for_each(|component| match component {
            "" => Err(ParseCollectionNameError::EmptyComponent),
            "." | ".." => Err(ParseCollectionNameError::RelativeComponent),
            _ => Ok(()),
        })?;
        Ok(CollectionName(value))
    }
}

#[derive(Debug, Display)]
#[non_exhaustive]
pub enum ParseCollectionNameError {
    #[display("collection name must not contain backslashes")]
    ContainsBackslash,
    #[display("collection name must not be empty")]
    Empty,
    #[display("collection name must not contain an empty path component")]
    EmptyComponent,
    #[display(r#"collection name must not contain a "." or ".." path component"#)]
    RelativeComponent,
}

#[cfg(test)]
mod tests;
