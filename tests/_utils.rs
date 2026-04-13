use derive_more::{AsRef, Deref};
use pipe_trait::Pipe;
use rand::{RngExt, distr::Alphanumeric, rng};
use std::env::temp_dir;
use std::fs::{create_dir, remove_dir_all};
use std::path::PathBuf;

/// RAII temporary directory that is deleted on drop.
#[derive(Debug, AsRef, Deref)]
#[as_ref(forward)]
#[deref(forward)]
pub struct Temp(PathBuf);

impl Temp {
    /// Create a temporary directory.
    pub fn new_dir() -> Self {
        let path = rng()
            .sample_iter(&Alphanumeric)
            .take(15)
            .map(char::from)
            .collect::<String>()
            .pipe(|name| temp_dir().join(name));
        if path.exists() {
            return Self::new_dir();
        }
        create_dir(&path).unwrap_or_else(|error| panic!("failed to create {path:?}: {error}"));
        Temp(path)
    }
}

impl Drop for Temp {
    fn drop(&mut self) {
        let path = &self.0;
        if let Err(error) = remove_dir_all(path) {
            eprintln!("warning: Failed to delete {path:?}: {error}");
        }
    }
}
