# Browser host diagnostics

Evidence: `repo:rust/apps/nxtools-browser-host/src/lib.rs#ALLOWED_EXTENSION_LOG_EVENTS`, `repo:rust/apps/nxtools-browser-host/src/blob_upload.rs`.

Only allowlisted bounded extension event/correlation metadata is accepted for logs; raw browser payloads and request objects never printed. stdin/stdout is protocol channel, not debugging console. No URL/path/name/headers/cookies/tokens/Blob bytes/native response dumps. Log handoff accepted/start/terminal/duration/count/staging-cleanup fixed code, no byte ticks. Preserve one correlation into engine command. [Shared privacy](../../shared/engineering.md).
