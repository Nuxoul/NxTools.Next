# Clipboard diagnostics

Evidence: `repo:rust/crates/nxtools-clipboard/src/lib.rs#CaptureSource`, `repo:rust/crates/nxtools-clipboard/src/archive.rs`, `repo:rust/apps/nxtools-desktop/src/clipboard_worker.rs`.

Capture/history/asset values are user payload, not log fields: no text/image/path/native-format bytes/source title/icon resource/query. Worker/app owns correlation/generation/start/terminal/elapsed with kind, counts, lengths, fixed storage/database/native code. Superseded queries remain quiet; no scroll/hover/frame logs. Library pure query helpers need no extra logging. [Shared privacy](../../shared/engineering.md); source error Display isn't sanitized by type alone.
