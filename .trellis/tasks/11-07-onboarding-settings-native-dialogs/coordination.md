# Concurrent archive-output task boundary

The approved parallel task aligns ordinary extraction output with Bandizip: direct final-name output, CREATE_NEW conflict admission, begin/close-only synchronous exchanges, pipelined writes and current-file-only cancellation cleanup. Its source requirements were supplied in the session's pasted task brief.

## Archive-output owner

Do not edit in this onboarding task:
- `csharp/NxTools.PlatformHost/Windows/Archive/ArchiveOutputSession*`, `ArchiveWorkerOutputBridge*`, `ArchiveOutput*`, related ordinary extraction output fixtures.
- `rust/crates/nxtools-archive/src/worker_output*`, `worker_conflict*`, bulk output/protocol or ordinary extraction journal/commit fixtures.
- Existing ordinary extraction commit/cancellation/publication clauses in `docs/reference-code/archive.md`, archive specs and the archive-manager PRD.
- Archive cleanup settings/core fields while their owner may be removing the previous output-cancellation option.

Do not restore old partial-file, postpublication hash/reopen, per-file durable flush or cleanup semantics to make onboarding checks pass.

## Onboarding owner

This task owns native file picker owner/lifetime changes, appearance/download category UI, effective temporary path display, Shell creation defaults, installer cleanup and indexer first-build publication/stage diagnostics. These do not change the extraction output protocol or codecs.

Before editing a potentially shared file, re-read its live contents and use a uniquely scoped Edit. Coordinate exact regions in `main_window/archive_page.rs`, `nxtools-core/src/archive.rs`, aggregate C# test registration and archive reference/spec files; do not edit the same path concurrently. This task's shared changes are limited to the fresh Shell recommendation, Create applicability label, independent uninstall fixture registrations, and picker/Shell contract sections. Extraction output/cancellation fields and clauses remain with the archive-output owner. `archive_temp_settings.rs` contains display-only path composition without changing cleanup policy.

No workspace-wide formatting, reset, clean, commit or blanket revert. Relevant tests must use the current integrated source; an archive-owner compile failure is not permission to change its pipeline.

## Build lane

Run one Cargo invocation at a time with jobs=1, incremental=0 and codegen-units=256. Check live compiler processes immediately before admission and record a build-lane notice on `nxtools-ui-archive-coordination`. Do not start package/release/native service work. Existing unrelated Unity .NET processes are not project builds and must not be terminated.

The shared channel had no registered live worker when inspected; notices document the boundary but are not an acknowledgment from the other task. Any newly observed shared-file change requires refreshing ownership rather than assuming isolation.
