//! Declarative list of the collections in the target media library.
//!
//! The library is a flat set of collection directories. Each video
//! belongs to one separated collection, named by the `collection` field
//! of its `video.toml`, and every video additionally appears in each
//! unified collection. Both kinds are declared by a single
//! `collections.toml` manifest that sits beside the video directories it
//! describes, rather than being hardcoded in Rust, so adding a
//! collection is a data edit.
//!
//! Declaring the separated collections in one place also makes a typo in
//! an individual `video.toml` detectable: a descriptor that names a
//! collection the manifest does not declare is an
//! [`UndeclaredCollection`] error rather than a silently created
//! directory.

use core::fmt;
use derive_more::{AsRef, Deref, Display, Into};
use serde::{Deserialize, Serialize};
use strsim::levenshtein;

/// Name of the collections manifest, relative to the directory of video
/// descriptors it describes.
pub const COLLECTIONS_CONFIG_FILE_NAME: &str = "collections.toml";

/// Parsed contents of a `collections.toml` file.
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct CollectionsDesc {
    /// The collections that receive a copy of every subtitle file, in
    /// addition to the separated collection the video belongs to. A
    /// manifest that omits the field declares none of them.
    #[serde(default)]
    pub unified: Vec<CollectionName>,
    /// The collections a video descriptor may name. A manifest that
    /// omits the field declares none of them.
    #[serde(default)]
    pub separated: Vec<CollectionName>,
}

impl CollectionsDesc {
    /// Every declared collection: the separated ones in the order the
    /// manifest lists them, followed by the unified ones.
    pub fn names(&self) -> impl Iterator<Item = &CollectionName> {
        self.separated.iter().chain(&self.unified)
    }

    /// Checks that `name` is one of the declared separated collections.
    /// A video descriptor may only name a collection the manifest
    /// declares, so a misspelled name fails the run.
    pub fn check_separated(&self, name: &CollectionName) -> Result<(), UndeclaredCollection> {
        if self.separated.contains(name) {
            return Ok(());
        }
        Err(UndeclaredCollection {
            name: name.clone(),
            closest: self.closest_separated(name).cloned(),
        })
    }

    /// The declared separated collection that `name` differs from by the
    /// fewest single-character edits, provided the difference is small
    /// enough to read as a typo rather than as a different name. The
    /// tolerance is one edit per three characters of `name`, so a long
    /// name may absorb a longer slip than a short one, and a name that
    /// resembles nothing declared yields nothing to suggest.
    ///
    /// Edit distance suits the job because a manifest declares a handful
    /// of names of a few dozen characters each, so the quadratic cost of
    /// each comparison is negligible, and because the mistakes it
    /// measures, a dropped letter or a swapped pair, are the mistakes a
    /// hand-written descriptor makes.
    fn closest_separated(&self, name: &CollectionName) -> Option<&CollectionName> {
        let tolerance = (name.chars().count() / 3).max(1);
        self.separated
            .iter()
            .map(|declared| (levenshtein(declared, name), declared))
            .filter(|(distance, _)| *distance <= tolerance)
            .min_by_key(|(distance, _)| *distance)
            .map(|(_, declared)| declared)
    }
}

/// A collection name that the manifest does not declare among its
/// separated collections.
#[derive(Debug)]
pub struct UndeclaredCollection {
    /// The name that was looked up.
    name: CollectionName,
    /// The declared name closest to it, when one is close enough to be
    /// worth suggesting.
    closest: Option<CollectionName>,
}

impl fmt::Display for UndeclaredCollection {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Both names are quoted through the text they wrap, so the
        // message reads as the manifest spells them.
        let UndeclaredCollection { name, closest } = self;
        write!(formatter, "unknown collection: {:?}", &**name)?;
        match closest {
            Some(closest) => write!(formatter, ", did you mean {:?}?", &**closest),
            None => Ok(()),
        }
    }
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
#[derive(AsRef, Clone, Debug, Deref, Deserialize, Display, Eq, Into, PartialEq, Serialize)]
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
            "" => Err(ParseCollectionNameError::StraySlash),
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
    #[display(r#"collection name must not contain a leading, trailing, or repeated "/""#)]
    StraySlash,
    #[display(r#"collection name must not contain a "." or ".." path component"#)]
    RelativeComponent,
}

#[cfg(test)]
mod tests;
