# Clipboard errors

Evidence: `repo:rust/crates/nxtools-clipboard/src/lib.rs#ClipboardHistoryError`, `repo:rust/crates/nxtools-clipboard/src/archive.rs#ClipboardArchiveError`, `repo:rust/crates/nxtools-clipboard/src/storage.rs#ClipboardStorageError`.

QuerySuperseded is expected cancellation; retain committed rows, no empty/error UI. Item/category missing, unsupported/empty edit and built-in immutable category are typed validation. SQLite/serialization/IO propagate cause. Late import Partial counts mean committed rows exist: retain referenced assets and remove only proven new orphans; query failure preserves conservatively. Migration failure retains source until durable commit. Caller logs fixed code/count/stage, not text/path/category payload or raw error. [Engineering](../../shared/engineering.md).
