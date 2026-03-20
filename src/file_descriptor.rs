use core::fmt;
use std::cell::OnceCell;
use std::fs::{read_to_string, symlink_metadata};
use std::io;
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;

use pipe_trait::Pipe;

#[derive(Clone)]
pub struct FileDescriptor {
    path: PathBuf,
    dev: u64,
    inode: u64,
    size: u64,
    content: OnceCell<String>,
}

/// Debug implementation that omits [`FileDescriptor::content`].
impl fmt::Debug for FileDescriptor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FileDescriptor")
            .field("path", &self.path)
            .field("dev", &self.dev)
            .field("inode", &self.inode)
            .field("size", &self.size)
            .finish()
    }
}

impl FileDescriptor {
    pub(crate) fn new(path: PathBuf) -> io::Result<Self> {
        let stats = symlink_metadata(&path)?;
        Ok(FileDescriptor {
            path,
            dev: stats.dev(),
            inode: stats.ino(),
            size: stats.len(),
            content: OnceCell::new(),
        })
    }

    pub(crate) fn load(&self) -> io::Result<&str> {
        if let Some(content) = self.content.get() {
            return Ok(content);
        }
        let content = read_to_string(&self.path)?;
        self.content.get_or_init(|| content).as_str().pipe(Ok)
    }

    pub(crate) fn content_eq(&self, other: &Self) -> bool {
        if self.inode == other.inode && self.dev == other.dev {
            return true;
        }

        if self.size != other.size {
            return false;
        }

        match (self.load(), other.load()) {
            (Ok(a), Ok(b)) => a == b,
            (Err(error), Ok(_)) => panic!("error: Cannot load file {:?}: {error}", &self.path),
            (Ok(_), Err(error)) => panic!("error: Cannot load file {:?}: {error}", &other.path),
            (Err(error_a), Err(error_b)) => panic!(
                "error: Cannot load file {:?} ({error_a}) and {:?} ({error_b})",
                &self.path, &other.path,
            ),
        }
    }
}
