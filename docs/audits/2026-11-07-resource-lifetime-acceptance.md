# Resource lifetime acceptance — 2026-11-07

## Verdict

The 13 resource-lifetime and memory-diagnostic tasks pass the executed source-contract and isolated regression checks. No functional failure was observed in those selections. This is not unconditional release acceptance: the current worktree fails the global formatting and strict Desktop Clippy gates, native/package acceptance remains outstanding, and the historical approximately 19 GiB PlatformHost allocation remains unattributed.

Fresh execution produced **590 passing isolated tests**: 81 C# fixtures and 509 Rust tests. One additional Windows MFT boundary test passed with an ambiguous success-versus-access-denied result; it is recorded separately below. One existing million-row measurement remained ignored. Ten static/script gates passed.

Scope is the previous 13 tasks, not every dirty file in the worktree. Acceptance changed no product code, scripts or existing specifications. The new deliverable is this audit.

## Per-task assessment

Counts in shared Rust suites overlap across tasks and must not be added across rows.

| Task | Invariant assessed | Fresh evidence | Assessment |
| --- | --- | --- | --- |
| 1. Clipboard staging ownership | Capture owns files before writes; failures, paused/stale captures and retired publication clean only owned files; publication transfers referenced files; inactive recovery is bounded | 18 staging/pause/compatibility C# fixtures; clipboard lifecycle gate | Isolated regression passed |
| 2. Image budgets | Dimensions are checked before costly native encoding/file decoding; encoded PNG output and canonical DIB remain bounded; independent formats survive | 16 image-budget/compatibility C# fixtures | Isolated regression passed |
| 3. Publication-tail budget | Original 64-entry tail normally retains at most 32 MiB; a larger legal item is exclusive; bytes/files remain owned until actual retirement | 6 C# fixtures covering blocked output, order, exclusivity, UTF-16 charge, cancellation and handoff/failure | Isolated regression passed |
| 4. OneCommander cache | At most 64 recent HWND anchors; idle native-availability sweep; stale work cannot replace/delete a newer anchor; stop prevents repopulation | 9 tracker/adapter C# fixtures | Isolated regression passed; live UIA retention not measured |
| 5. Native allocation ownership | First allocation is freed when the second fails; original exceptions/native results and existing handle ownership survive | 10 C# native-allocation/compatibility fixtures, including six allocation-boundary regressions | Isolated regression passed |
| 6. Search cancellation | Clear/hide carries original request and generation; wrong lease/stale cancellation cannot retire a replacement; actual worker/backpressured completion releases snapshots | Protocol 111 and IPC 59 tests; Indexer active/pending/backpressure retirement, cooperative cancellation and lease tests; 194 Desktop quick-launch tests | Isolated regression passed |
| 7. Mapped MFT publication | Completed scan/compaction reopens the exact mapped base after owned rows retire; old leases, last-good snapshots, delta continuation and real failures survive | Six `mapped_publication_*` tests inside the Indexer selection, with cache/journal/persistence compatibility tests | Isolated regression passed; packaged-service/million-row acceptance outstanding |
| 8. Serial compaction | One original global worker; pending work retains volume identities rather than snapshot inputs; next work waits for actual retirement; reader EOF drains discovered work | Three `serial_compaction_*` tests plus coordinator/maintenance/lease tests inside the Indexer selection | Isolated regression passed; installed-service acceptance outstanding |
| 9. Temporary .NET build trees | Actual owner retains trees through downstream use and cleans in `finally`; borrowed trees and required public inputs/evidence survive | Eight build-tree wrapper unittest groups using stub tools | Wrapper regression passed; no actual package/publisher run |
| 10. Blob shutdown/recovery | Explicit shutdown waits for physical deletion and original owner/executor exit; bounded recovery excludes live owners; reservations survive failed deletion/cancelled I/O until real return | 14 Extensions library tests, six `blob_store` integration tests and three Desktop service/composition tests; Blob exit-wiring gate | Isolated regression passed |
| 11. Floating Drop release | Original connection/session/generation owns cleanup; full queues retain one coalesced cleanup; C# retries physical deletion; the one-token/16 MiB capacity survives logical stop | IPC 59 tests; 41 Desktop floating-assistant tests plus quick-launch integration coverage; 13 C# fixtures; 43 source contracts | Isolated regression passed; real OLE/Shell interaction outstanding |
| 12. DOpus export cleanup | Helper exit is not writer completion; exact directory/output and actual helper remain owned until exit/deletion; at most two active/retiring owners; delayed cleanup retries | Nine C# fixtures: six real owned-helper/export cleanup cases and three observation/compatibility cases | Owned-resource regression passed; installed DOpus interaction outstanding |
| 13. PlatformHost memory evidence | Exact ordinary Host identity, bounded same-window process/GC counters, replacement stop and incomplete-counter reporting; no payload logging or forced GC | 21 synthetic sampler assertions; documented entry returned `no_matching_ordinary_host`, zero samples, exit 2 | Diagnostic boundary passed; historical allocation attribution blocked |

## Fresh execution results

| Selection/check | Result |
| --- | --- |
| C# Debug fixture build | Exit 0; zero warnings and errors |
| Exact C# fixture selection | Selected/run/pass 81; skip/fail/not-run 0; every selected entry classified `Fixture` |
| Protocol library | 111 passed |
| IPC library | 59 passed |
| Indexer selected library tests | 82 passed, one ignored; 81 isolated passes plus one real MFT boundary pass |
| Extensions selected library tests | 14 passed |
| Extensions `blob_store` integration | 6 passed |
| Desktop selected tests | 238 passed: 194 quick-launch, 41 floating-assistant, three Extensions composition tests |
| Scoped Rust formatting | All 31 scoped files passed with `skip_children=true` |
| Strict Protocol/IPC/Indexer/Extensions Clippy, including tests | Exit 0 |
| Strict Desktop Clippy, including tests | Exit 101; one `unnecessary_lazy_evaluations` error |
| Workspace compile check, including tests | Exit 0 |
| Global Rust formatting | Exit 1; differences in four archive-related files outside the 13-task source scope |

The Desktop invocation also selected zero tests in a separate ten-test target. That target contributes no passing tests to the totals. C# `--list` output was used only to audit selection; its `run=0` is not execution evidence.

### Commands and environment

All Rust invocations inherited `CARGO_BUILD_JOBS=1`, `CARGO_INCREMENTAL=0` and `RUSTFLAGS=-C codegen-units=256` through `Initialize-LowMemoryRustBuildEnvironment`. Cargo was offline and serial. Desktop compilation used `Initialize-GpuiReleaseEnvironment`. Rust test `TEMP`/`TMP` pointed to the acceptance-owned scratch directory, including default BlobStore recovery tests.

```powershell
cargo test --offline -p nxtools-protocol --lib -- --test-threads=1
cargo test --offline -p nxtools-ipc --lib -- --test-threads=1

cargo test --offline -p nxtools-indexer --lib -- `
    mft::cache:: mft::tests:: search_retirement_ maintenance_tests::u4_ `
    tests::m4_ tests::launcher_ tests::r8_ tests::persistence_fault_ `
    tests::compaction_ tests::one_poll_schedules_compaction `
    tests::a_restored_oversized_overlay usn::tests::an_oversized_live_overlay `
    tests::closing_search_executor_cancels_a_completion_blocked_by_backpressure `
    tests::a_search_stops_work_when_its_generation_is_cancelled --test-threads=1

cargo test --offline -p nxtools-extensions --lib -- `
    blobs:: blob_shutdown_runtime_ --test-threads=1
cargo test --offline -p nxtools-extensions --test blob_store -- --test-threads=1

cargo test --offline -p nxtools-desktop -- `
    quick_launch:: tools::floating_assistant:: blob_shutdown_service_ `
    expansion_storage_service_drop_releases_real_owner_after_executor_grace `
    shared_gui_and_headless_composition_defaults_to_noop_and_does_not_retain_engine `
    --test-threads=1

cargo clippy --offline -p nxtools-protocol -p nxtools-ipc `
    -p nxtools-indexer -p nxtools-extensions --tests -- -D warnings
cargo clippy --offline -p nxtools-desktop --tests -- -D warnings
cargo check --offline --workspace --tests
cargo fmt --all -- --check
# The exact 31-file list is retained in rust-scoped-fmt-results.json.
rustfmt --edition 2024 --check --config skip_children=true <scoped-files>
```

The Indexer command above records what ran, including the real-boundary test described under Limitations. An isolated-only rerun must add `--skip the_real_windows_mft_boundary_is_either_searchable_or_actionably_denied` after the test-runner separator. The ignored million-row measurement was not enabled.

### Static/script gates

All ten exited 0:

- `tests/architecture/spec-contracts.py`: 126 documents, 14 owners at execution.
- `tests/architecture/extension-blob-lifecycle.py`.
- `tests/architecture/floating-assistant-lifecycle.py`: 43 source contracts.
- `tests/architecture/gpui-release-contracts.py`.
- `tests/architecture/markdown-editor-contracts.py`.
- `tests/architecture/clipboard-lifecycle.ps1`.
- `tests/architecture/indexer-lifecycle.ps1`.
- `tests/architecture/quick-launch-lifecycle.ps1`.
- `tests/architecture/platformhost-memory-sampler.tests.ps1`: 21 synthetic assertions.
- `tests/architecture/build-tree-cleanup.tests.py`: eight unittest groups; tool stubs, not actual build/sign/package/service execution.

Static assertions, wrapper unittest groups and sampler assertions are separate from the 590 isolated C#/Rust passing tests.

## Current quality-gate findings

### Strict Desktop Clippy

[Archive browser drag](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/browser_drag.rs:447) uses `(result.success && pending.locate).then(|| data.as_ref())`. Strict Clippy rejects this as `unnecessary_lazy_evaluations` and recommends `then_some(data.as_ref())`. This archive-locate code is outside the 13-task modification scope and remains unchanged by acceptance. The finding prevents a clean whole-Desktop strict lint result; it is not a failed resource-lifetime test.

### Global formatting

`cargo fmt --all -- --check` reports ten diff locations across these four out-of-scope files:

- [Desktop archive application tests](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/application_tests.rs:3723).
- [Desktop archive browser drag](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/browser_drag.rs:428).
- [Desktop archive browser drag tests](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/browser_drag_tests.rs:840).
- [Protocol archive virtual drag](D:/Code/NxTools.Next-main/rust/crates/nxtools-protocol/src/archive_virtual_drag.rs:720).

The Protocol archive file is distinct from the IPC archive file whose timeout fields changed for task 11. All 31 scoped Rust files passed a direct read-only formatting check. No whole-worktree formatting was applied.

## Limitations and remaining acceptance

### Native MFT boundary result

The Indexer selection included [the real Windows MFT boundary test](D:/Code/NxTools.Next-main/rust/apps/nxtools-indexer/src/mft.rs:1206). It calls `scan_volume` for the test executable's drive. Its pass condition accepts either a searchable snapshot containing that executable or an actionable administrator/Win32 5 denial. Captured output does not distinguish those branches. This establishes that a real-volume access was attempted, not that scanning was completed or denied. It is excluded from the isolated count and does not satisfy the installed-service search gate.

### Native interaction and release

No live Desktop/Shell/OLE/DOpus user gesture, real clipboard interaction, installed service restart or Release/package operation was performed. C# fixtures used owned temporary paths, small HGlobal allocations, synthetic hidden windows and controlled helper/writer processes. Desktop tests used TestApp/fake bridge and temporary data, not a launched product session.

Indexer tasks still require the packaged LocalSystem `indexer-idle-cpu.ps1 -RestartService -AssertRegression` gate followed by real service search smoke testing, versioned base-plus-delta restart verification, and the atomic protected Desktop/PlatformHost/indexer binary checks. A passing source build or an older installed package cannot certify that release. The ignored million-row mapped/owned memory measurement is also outstanding.

Real OneCommander focus/close-window behavior, clipboard capture/publication in target applications, floating Drop/OLE retirement and DOpus export interaction remain native acceptance boundaries. DOpus delayed cleanup is owned by the running process, not persisted across process termination.

### Historical PlatformHost memory

The [memory audit](D:/Code/NxTools.Next-main/docs/audits/2026-11-06-platformhost-memory.md:40) records zero matching ordinary Hosts and zero samples at 2026-11-06 23:12:50 UTC. The new sampler passed synthetic tests, but live counter attachment and the historical allocation source were not established. The original approximately 19 GiB private commit is not a measured managed heap. Neither these lifetime regressions nor the earlier short stable observation proves the incident fixed; private commit minus GC committed is not an exact native-allocation measurement.

Attribution requires an affected ordinary Host to remain alive for same-window process/GC evidence, followed by an appropriate managed-root or native-allocation investigation. No forced GC, working-set trim or restart policy is part of this acceptance.

## Evidence retention

Full command logs and selection/result records are retained for this session under:

`D:\.pi-desktop\data\scratch\967153f6-8036-4bb3-b7a4-60f20d7ea4d6\resource-acceptance`

Key records are `csharp-fixture-selection.json`, `csharp-fixture-run.log`, `csharp-debug-build.log`, `static-results.json`, `rust-core-results.json`, `rust-resource-results.json`, `rust-desktop-results.json`, `rust-scoped-fmt-results.json` and `rust-quality-results.json`, with their corresponding full logs. Scratch evidence is session-local and is not a shipped release artifact. This audit preserves the outcomes and remaining gates in the repository.
