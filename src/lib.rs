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
//!
//! **Telemetry:** spans for syncs and document writes (`tracing`) and the metric `invertdb.storage.sync.duration`
//! (the `opentelemetry` API); both do nothing until the program installs a subscriber and a meter provider.

mod blob;
mod store;

use std::sync::OnceLock;

use opentelemetry::metrics::Histogram;

/// Time spent making storage durable: blob syncs and document writes. Made on first use (after a service installs
/// its meter provider; a no-op until then).
pub(crate) fn sync_duration() -> &'static Histogram<f64> {
    static HISTOGRAM: OnceLock<Histogram<f64>> = OnceLock::new();
    HISTOGRAM.get_or_init(|| {
        opentelemetry::global::meter("larry")
            .f64_histogram("invertdb.storage.sync.duration")
            .with_unit("s")
            .with_description("Time to make blobs and documents durable.")
            .with_boundaries(vec![0.00005, 0.0001, 0.00025, 0.0005, 0.001, 0.0025, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 1.0, 2.5])
            .build()
    })
}

pub use blob::{Blob, View};
pub use store::Store;
