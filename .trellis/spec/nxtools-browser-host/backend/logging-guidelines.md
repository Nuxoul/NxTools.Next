# Browser host diagnostics

Evidence: `repo:rust/apps/nxtools-browser-host/src/lib.rs#ALLOWED_EXTENSION_LOG_EVENTS`, `repo:rust/apps/nxtools-browser-host/src/blob_upload.rs`.

Only allowlisted bounded extension event/correlation metadata is accepted for logs; raw browser payloads and request objects never printed. stdin/stdout is protocol channel, not debugging console. No URL/path/name/headers/cookies/tokens/Blob bytes/native response dumps. Log handoff accepted/start/terminal/duration/count/staging-cleanup fixed code, no byte ticks. Preserve one correlation into engine command. [Shared privacy](../../shared/engineering.md).

Browser workflow events `link_collection_*`, `media_inspection_*`, `connection_check_*` admit only the fixed started/completed/failed names with UUID correlation, required nonnegative elapsedMs and event-specific fixed reason codes. Completed protected is valid only for media inspection; connection failures use fixed host/desktop classifications. The logger emits these bounded fields and ignores payload/message secrets. `handoff_state` also admits unsupported_request_method for browser-retained non-GET downloads. Framed production-route regression: `browser_wire_tests::b2_capture_pause_logs_use_bounded_correlated_native_frames` validates reason/correlation/elapsed requirements and absence of payload fields.
