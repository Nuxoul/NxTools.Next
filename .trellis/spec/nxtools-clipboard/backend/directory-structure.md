# Clipboard structure

Evidence: `repo:rust/crates/nxtools-clipboard/src/lib.rs#ClipboardHistory`, `repo:rust/crates/nxtools-clipboard/src/archive.rs`, `repo:rust/crates/nxtools-clipboard/src/storage.rs`.

lib owns typed capture/items/native formats, history/category queries/actions, usage/retention and SQLite validation. archive owns bounded portable ZIP/resources; storage owns online backup, VACUUM and migration staging/path rewrite. Desktop ClipboardCaptureWorker serializes live connection/jobs and application owns operation/session/config commit. No native Windows capture/paste or GPUI in this crate. Owner-specific cleanup must distinguish existing shared from new resources. [Architecture](../../shared/architecture.md).
