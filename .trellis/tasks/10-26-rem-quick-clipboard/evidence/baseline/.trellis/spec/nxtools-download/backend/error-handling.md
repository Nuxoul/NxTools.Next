# Download errors

Evidence: `repo:rust/crates/nxtools-download/src/message.rs#DownloadFault`, `repo:rust/crates/nxtools-download/src/http.rs#HttpDownloadError`, `repo:rust/crates/nxtools-download/src/storage.rs#DownloadStorageError`, `repo:rust/crates/nxtools-download/src/media.rs#MediaProcessError`.

Typed domain faults preserve code/message/retryable; engine maps HTTP/status/artifact/SQLite/native permission/disk-full/file-in-use/cancel/unavailable distinctly. Expected pause/cancel stops writers before state commit; never erase resumable checkpoint on failed cleanup. Permanent protocol identity mismatch isn't retryable transient network failure. Retry bounds and stalled/body deadlines belong to transfer owner, not UI. reqwest errors use without_url; raw storage/context/child output isn't log safe. [Shared errors](../../shared/engineering.md). Tests obstruct real synthetic artifacts/store commits, not generic catch-all success.
