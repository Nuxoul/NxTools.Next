# Browser host staging (SQL not applicable)

Evidence: `repo:rust/apps/nxtools-browser-host/src/lib.rs`, `repo:rust/apps/nxtools-browser-host/src/blob_upload.rs#BlobUploadService`.

Host uses Core config and Download IPC; no independent task SQL. Blob capability registry is bounded ephemeral state, staging files are task-owned synthetic/received resources with cleanup; engine store owns handoff idempotency. Receiving exclusive socket lease cannot be reaped mid-write; owner total/per-read deadlines release state on abandonment. Registry/manifest installation belongs C# transaction, not this process. Don't add persistence replay of user URLs/cookies. [Quality lease contract](quality-guidelines.md).
