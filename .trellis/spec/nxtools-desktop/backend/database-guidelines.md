# Desktop persistence boundary

Evidence: `repo:rust/apps/nxtools-desktop/src/clipboard_worker.rs`, `repo:rust/apps/nxtools-desktop/src/clipboard.rs#save_clipboard_section`, `repo:rust/apps/nxtools-desktop/src/runtime.rs#DesktopRuntime`.

Desktop composes DownloadEngine/store, FinderCatalog and ClipboardHistory; it is not a new SQL/schema owner. Clipboard jobs own live history connection and stage migration then durable config commit before switching. Query reply validates generation/query/category/scope; retain bounded four-page window and immutable cursor. Ordinary UI save merges only owned Core section; storage_directory remains migration authority. No SQLite wait or file/native IO in render. Tests use actual synthetic worker/config/DB delay/failure; direct rusqlite dependency doesn't authorize UI-owned ad hoc queries. [Shared persistence contract](../../shared/engineering.md).
