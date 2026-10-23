use std::fs::{self, File, OpenOptions};
use std::io::{self, ErrorKind, Write};
use std::path::{Path, PathBuf};

use crate::blob::Blob;

/// A set of named blobs and documents, and nested stores (for a part with its own blobs, such as an index inside
/// a collection).
#[derive(Clone, Debug)]
pub struct Store {
    directory: PathBuf,
}

fn valid(name: &str) -> io::Result<()> {
    let ok = !name.is_empty() && !name.starts_with('.') && name.bytes().all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b));
    if ok { Ok(()) } else { Err(io::Error::new(ErrorKind::InvalidInput, format!("not a valid name: {name:?}"))) }
}

impl Store {
    /// Opens a store, creating it if need be.
    pub fn open(directory: impl AsRef<Path>) -> io::Result<Store> {
        fs::create_dir_all(directory.as_ref())?;
        Ok(Store { directory: directory.as_ref().to_path_buf() })
    }

    /// A nested store, created if need be.
    pub fn store(&self, name: &str) -> io::Result<Store> {
        valid(name)?;
        Store::open(self.directory.join(name))
    }

    /// Whether a nested store exists.
    pub fn has_store(&self, name: &str) -> bool {
        valid(name).is_ok() && self.directory.join(name).is_dir()
    }

    /// The names of the nested stores.
    pub fn stores(&self) -> io::Result<Vec<String>> {
        let mut names: Vec<String> =
            fs::read_dir(&self.directory)?.flatten().filter(|e| e.path().is_dir()).filter_map(|e| e.file_name().into_string().ok()).filter(|n| valid(n).is_ok()).collect();
        names.sort();
        Ok(names)
    }

    /// Opens a blob, creating an empty one if need be.
    pub fn blob(&self, name: &str) -> io::Result<Blob> {
        valid(name)?;
        let file = OpenOptions::new().read(true).write(true).create(true).truncate(false).open(self.directory.join(name))?;
        Blob::new(file)
    }

    /// A document's bytes, or None if it was never written.
    pub fn read(&self, name: &str) -> io::Result<Option<Vec<u8>>> {
        valid(name)?;
        match fs::read(self.directory.join(name)) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Replaces a document atomically and durably: a crash leaves either the old bytes or the new.
    pub fn write(&self, name: &str, bytes: &[u8]) -> io::Result<()> {
        valid(name)?;
        let temporary = self.directory.join(format!(".{name}.new"));
        let mut file = File::create(&temporary)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        fs::rename(&temporary, self.directory.join(name))?;
        File::open(&self.directory)?.sync_all() // the rename itself
    }

    /// Removes a blob or document (nothing if it doesn't exist).
    pub fn remove(&self, name: &str) -> io::Result<()> {
        valid(name)?;
        match fs::remove_file(self.directory.join(name)) {
            Err(e) if e.kind() != ErrorKind::NotFound => Err(e),
            _ => Ok(()),
        }
    }

    /// Deletes the store and everything in it.
    pub fn destroy(self) -> io::Result<()> {
        fs::remove_dir_all(&self.directory)
    }

    /// Where the store lives on disk, for a library that keeps its own files inside it (the B+tree).
    pub fn path(&self, name: &str) -> io::Result<PathBuf> {
        valid(name)?;
        Ok(self.directory.join(name))
    }

    /// Bytes on disk, everything included.
    pub fn size(&self) -> u64 {
        // the space files take on disk (as `du` counts it), not their lengths: a file grown ahead of its data (as
        // b-positive's is) is sparse, and its length would count space it doesn't use
        #[cfg(unix)]
        fn on_disk(m: &fs::Metadata) -> u64 {
            use std::os::unix::fs::MetadataExt;
            m.blocks() * 512
        }
        #[cfg(not(unix))]
        fn on_disk(m: &fs::Metadata) -> u64 {
            m.len()
        }
        fn walk(path: &Path) -> u64 {
            fs::read_dir(path).map(|entries| entries.flatten().map(|e| if e.path().is_dir() { walk(&e.path()) } else { e.metadata().map(|m| on_disk(&m)).unwrap_or(0) }).sum()).unwrap_or(0)
        }
        walk(&self.directory)
    }
}
