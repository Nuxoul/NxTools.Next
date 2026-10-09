# Download diagnostics

Evidence: `repo:rust/crates/nxtools-download/src/engine/segmented_runner.rs`, `repo:rust/crates/nxtools-download/src/engine/dash_runner.rs`, `repo:rust/crates/nxtools-download/src/storage.rs#sqlite_extended_code`.

Carry command correlation/task ID/attempt and fixed stage; record probe phase durations, request client reuse, retry/terminal state and native/SQLite code at owning boundary. No source URL/query/path/file/headers/cookies/media keys/FFmpeg output. Never log byte progress/sample ticks as events; coalesce milestones/state. Temporary DEBUG marker removed after causal regression. Store retry logs are bounded and don't alter error. [Shared policy](../../shared/engineering.md); accepted/in-progress is not completed artifact publication.
