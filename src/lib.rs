//! larry: blobs for invertdb. A [`Store`] holds named, append-only blobs, read through memory-mapped [`View`]s, and
//! small documents replaced atomically.
//!
//! Blobs are how every layer above keeps bulk data: postings, stored sources, vectors, coordinate columns, chunks.
//! Bytes, once committed, never change: a blob only grows by appending, and shrinks only when recovery drops what a
//! crash left past the last commit (the layer above knows the committed length; larry doesn't). Immutable bytes are
//! what let readers share them without locks, and let a distributed store copy them to replicas without ever
//! invalidating anything.
//!
//! The local implementation is a directory of files. Each blob is a file, a view is a read-only memory map of the
//! blob as it was when the view was taken, and a document is written to a temporary file, synced and renamed over
//! the old one (shadowing; Lorie, 1977), so a crash leaves either the old document or the new.

mod blob;
mod store;

pub use blob::{Blob, View};
pub use store::Store;
