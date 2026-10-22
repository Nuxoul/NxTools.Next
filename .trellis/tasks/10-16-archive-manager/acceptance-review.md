# Independent acceptance review — Phase A partial extraction slice

## Verdict

**First release: NOT ACCEPTED. Phase A: NOT ACCEPTED. Current RAR/7z extraction slice: useful verified progress, conditional acceptance only; defects below require correction.**

Scope: current working-tree sources and pasted continuation report, not a commit-range PR. No product source changed by this review. One directed independent read-only review was used; findings were checked against actual sources. Approved A–F first-release scope is unchanged. The continuation correctly labels itself partial and discloses its red test; no evidence that it claimed completed UI or first release.

## Independently executed verification

All Rust commands inherited CARGO_BUILD_JOBS=1, CARGO_INCREMENTAL=0, RUSTFLAGS=-C codegen-units=256 and ran serially.

| Check | Actual review result |
| --- | --- |
| cargo test -p nxtools-archive --features isolated-codecs --offline |30 passed,0 failed,0 ignored; exit0 |
| cargo build -p nxtools-archive --bin nxtools-archive-worker --features isolated-codecs --offline |exit0 |
| dotnet build csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-restore |exit0,0 warnings/errors |
| Exact checkpoint16-filter C# subset, explicit worker/codec/ZIP fixture paths |list16/run0; execute16/run16/pass16/skip0/fail0 |
| archive output fixture identity bound atomic overwrite |list1/run0; execute1/run1/pass0/skip0/fail1,exit1,replace_file:32 |
| cargo fmt --all -- --check |exit0 |
| cargo clippy -p nxtools-archive --tests --features isolated-codecs --offline -- -D warnings |exit0 |
| cargo check --workspace --tests --offline |exit0; not a full workspace test run |
| spec-contracts.py |PASS100documents/14owners |
| gpui-component-catalog.py |PASS79pages/79rows/78locked entries |
| task.py validate current task |PASS15implement/12check,no truncation warnings |

No leftover Cargo/rustc/archive-worker/test-host process or test-installed fixed helper observed after execution. No full/default C# suite was executed in this review; its known registered red means it cannot be called all-green. No UI, Shell user gesture, DPI/UIA, packaging, install, registry or service operation performed.

## Required findings

### P1 — Existing-target overwrite is genuinely blocked

`csharp/NxTools.PlatformHost/Windows/Archive/ArchiveOutputSession.cs:182-187,395-404`; real test `ArchiveOutputTests.cs:171-192` fails at Commit with Win32 sharing violation32. The old target is held without delete sharing while NtSetInformationFile replacement tries to replace its name. Independent single-fixture rerun confirms failure. This is not a cosmetic gate and cannot be excluded to close Phase A.

The proposed backup→no-replace publish→restore changes the product contract: name visibility gap and crash recovery, not atomic replacement. This acceptance review does **not** approve the amendment. If chosen by the user, require durable owned recovery journal, publish/restore crash-point tests, external name competition, retained backup reporting, cancellation/disconnect and exact-handle cleanup; never silently delete an unrecovered backup or overwrite a competing file. Otherwise keep atomic contract and continue bounded investigation. No evidence from this review proves all other Windows primitives impossible.

### P1 — Source hashes are not bound to the actual decoded file object (static confirmed)

`rust/crates/nxtools-archive/src/worker_codecs.rs:49-66,171-175,357-365,430-445`; `worker_codecs/rar.rs:174-202`.

`fingerprint(path)` independently opens the name.7z later opens it again for decoding; RAR opens by name separately for enumeration/decode. Final checks reopen the name. A path can switch A→B after fingerprint, decode/stage B, then return to A before final fingerprint; equal before/after hashes do not prove staged content came from A. RAR second-pass header comparisons narrow but do not bind source identity to one file object. No source guard/lease is established by the C# output-root handles.

Required: bind codec reads and revision checks to the same stable source capability/handle or an equivalent protected immutable source representation, preserving Windows API ownership. Add deterministic barriers for source replacement between fingerprint/open/decode/precommit. Do not rely only on another path stat/hash. **No live ABA race fixture was executed in this review**; this finding is based on direct control-flow/source-open inspection.

### P1 —7z ignores accepted per-request working-memory budget (reproduced protocol discrepancy)

`worker.rs:98-105` accepts positive max_working_bytes; `worker_codecs.rs:173-197` never applies it to the7z reader. C# `ArchiveWorkerRunner.cs:60-73` derives Job memory from separate MemoryBytes/default512MiB, independently serializing request.Limits. Thus the Job ceiling does not automatically honor a lower domain working budget.

A tiny trusted lzma.7z direct worker test with `limits.max_working_bytes=1` returned complete/files_verified1/bytes_verified15, identical to baseline, rather than ResourceLimit or admission rejection. This was a small protocol probe, **not memory exhaustion testing and not a C# Job-memory measurement**. The independently inspected C# mapping confirms the missing relationship.

Required: define dictionary/work budget vs process-memory-overhead semantics, reject inconsistent requests and map the approved limits coherently to native Job admission and backend preflight. Test both low requested budget and actual owned-Job enforcement. Never label an unrelated fixed process ceiling as honoring every accepted working budget.

### P2 — Out-of-file7z header is misclassified as resource exhaustion (reproduced)

`worker_codecs.rs:180-189` combines next-header size budget and extent-over-EOF into ResourceLimit. A copied tiny lzma fixture with next_offset beyond file end and recomputed valid start-header CRC returned ResourceLimit. Structural truncation/range overflow should be CorruptArchive; budget exceeded should remain ResourceLimit. Separate checks and add independent fixtures so the UI does not tell users to increase memory for damaged archives.

### Evidence limitation — encrypted extraction interoperability remains partial

`ArchiveExtractionWorkerTests.cs:99,160-163`: encrypted7z/RAR5 check terminal success; encryptedRAR4 checks a file length, not independent exact bytes/hash. Plain fixtures compare known bytes/hash. Retain these encrypted checks as chain smoke evidence only; add independently verified encrypted output hashes/counts and corrupted authenticated-payload cases before marking full encrypted compatibility/integrity accepted. The checkpoint already acknowledges small-sample limitations.

## Scope status and next action

Still incomplete by its own checkpoint: ZIP encryption/extra methods, TAR/compound/streams, creation/source traversal, MOTW/attributes/recycle, product IPC/backpressure/diagnostics/resource-performance evidence; B–F settings/browser/forms/ZIP editing/splits/Explorer/real drag remain required. No archive desktop UI source was found by scoped filename search; no live product UI was inspected.

Recommended next development order: add red-capable regressions and fix source binding, memory-contract mismatch and7z classification within A; keep the overwrite red until an actually proved atomic solution or explicit semantic amendment. Continue other independent A work without treating one unresolved primitive as permission to stop the entire approved task. Do not start B before canonical A exit and do not claim first-release completion.

Review scratch evidence: archive-acceptance-{rust-tests,worker-build,csharp-build,fmt,clippy,workspace}.log and archive_acceptance_probes.py in the review session scratch. Direct probe mutated only a temporary fixture copy, removed on completion; no user archives modified.
