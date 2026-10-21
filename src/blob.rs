use std::fs::File;
use std::io;
use std::ops::Deref;
use std::os::unix::fs::FileExt;
use std::sync::{Arc, Mutex};

use memmap2::{Mmap, MmapOptions};

/// An append-only sequence of bytes. One writer appends; any number of views read.
pub struct Blob {
    file: File,
    length: Mutex<u64>,
}

impl Blob {
    pub(crate) fn new(file: File) -> io::Result<Blob> {
        let length = file.metadata()?.len();
        Ok(Blob { file, length: Mutex::new(length) })
    }

    pub fn len(&self) -> u64 {
        *self.length.lock().unwrap()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Appends bytes and returns where they start. They're readable at once, and durable after [`Blob::sync`].
    pub fn append(&self, bytes: &[u8]) -> io::Result<u64> {
        let mut length = self.length.lock().unwrap();
        let at = *length;
        self.file.write_all_at(bytes, at)?;
        *length += bytes.len() as u64;
        Ok(at)
    }

    /// Reads exactly `out.len()` bytes from `offset`.
    pub fn read_at(&self, offset: u64, out: &mut [u8]) -> io::Result<()> {
        self.file.read_exact_at(out, offset)
    }

    /// The whole blob.
    pub fn read_all(&self) -> io::Result<Vec<u8>> {
        let mut out = vec![0u8; self.len() as usize];
        self.read_at(0, &mut out)?;
        Ok(out)
    }

    /// Drops everything past `length`: what a crash left beyond the last commit. A blob never shrinks otherwise.
    pub fn truncate(&self, length: u64) -> io::Result<()> {
        let mut current = self.length.lock().unwrap();
        if length > *current {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, format!("can't truncate a blob of {} bytes to {length}", *current)));
        }
        self.file.set_len(length)?;
        *current = length;
        Ok(())
    }

    /// Makes everything appended so far durable.
    pub fn sync(&self) -> io::Result<()> {
        self.file.sync_data()
    }

    /// A read-only view of the blob as it is now. Later appends aren't in it; take a new view to see them.
    pub fn view(&self) -> io::Result<View> {
        let length = self.len();
        let map = if length == 0 { MmapOptions::new().len(0).map_anon()?.make_read_only()? } else { unsafe { MmapOptions::new().len(length as usize).map(&self.file)? } };
        Ok(View { map: Arc::new(map) })
    }
}

/// A read-only view of a blob's bytes as of when it was taken; cheap to clone and share across threads.
#[derive(Clone)]
pub struct View {
    map: Arc<Mmap>,
}

impl Deref for View {
    type Target = [u8];

    #[inline]
    fn deref(&self) -> &[u8] {
        &self.map
    }
}

impl std::fmt::Debug for View {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "View({} bytes)", self.map.len())
    }
}
