# larry

Blobs for invertdb. A `Store` holds named, append-only blobs, read through memory-mapped views, and small documents
replaced atomically.

```rust
let store = larry::Store::open("collection")?;
let vectors = store.blob("vectors")?;
let at = vectors.append(&bytes)?;          // where the bytes start
vectors.sync()?;                           // durable
let view = vectors.view()?;                // &[u8] of the blob as it is now; cheap to clone and share
store.write("options", b"{...}")?;         // a whole document, replaced atomically
let index = store.store("units")?;         // a nested store
```

## Model

- **Blobs are append-only.** Committed bytes never change. A blob shrinks only when recovery drops what a crash
  left past the last commit; the layer above knows the committed length, larry doesn't. Immutable bytes let readers
  share them without locks.
- **Views** are read-only memory maps of a blob as of when they were taken. Appends after that need a new view.
- **Documents** are small and replaced whole: written to a temporary file, synced, and renamed over the old one
  (shadowing; Lorie, 1977). A crash leaves either the old document or the new.
- **Stores nest**, so a part with its own blobs (an index inside a collection) gets its own namespace.

## Why a library of its own

All of invertdb's bulk data goes through larry: postings, positions and sources (introvert), stored documents,
vectors, coordinate columns, time-series chunks. The B+tree ([b-positive](https://github.com/invertdb/b-positive))
holds pages; the log ([blammo](https://github.com/invertdb/blammo)) holds writes not yet committed.

Each needs a different strategy when distributed. Blobs are immutable segments, so a cluster can keep them in
object storage with full local copies on each replica, memory-mapped as here. A replica only ever adds bytes, so
reads stay local and nothing is ever invalidated.

## Tests

`cargo test`: blobs append, keep views stable, truncate (never grow by truncation) and reopen; documents are
replaced whole with no temporary left behind; nested stores list, refuse names that would escape, and are destroyed.

## References

- Lorie, R. A. (1977). Physical integrity in a large segmented database. *ACM Transactions on Database Systems*
  2(1).
