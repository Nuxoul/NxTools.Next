# Clipboard structure

Evidence: `repo:rust/crates/nxtools-clipboard/src/lib.rs#ClipboardHistory`, `repo:rust/crates/nxtools-clipboard/src/archive.rs`, `repo:rust/crates/nxtools-clipboard/src/storage.rs`.

lib owns typed capture/items/native formats, connection initialization, actions, usage/retention and SQLite validation. Private categories.rs owns category/source maintenance and built-in seeding; query.rs owns recent/search/count/category pagination and cursor SQL. Both extend the same ClipboardHistory and do not create another connection/repository or change root exports. archive owns bounded portable ZIP/resources; storage owns online backup, VACUUM and migration staging/path rewrite. Desktop ClipboardCaptureWorker serializes live connection/jobs and application owns operation/session/config commit. No native Windows capture/paste or GPUI in this crate. Owner-specific cleanup must distinguish existing shared from new resources. [Architecture](../../shared/architecture.md).
