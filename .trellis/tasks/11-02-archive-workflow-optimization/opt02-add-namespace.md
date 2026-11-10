# OPT-02 — Add namespace correctness

## Result

**G-11 and touched G-04/G-19 automated/owned-native preservation passed.** Parent phase remains **Phase E — Format expansion and ZIP editing**. This independently authorized package is complete at its scoped stop point. OPT-00/01 and G-00 remain open; OPT-03 through OPT-07 are not started. The original independent verdict, AC repair evidence and historical measurements are retained.

## Implementation

| Requirement | Source owner and delivered behavior |
| --- | --- |
| Complete incoming plan | `creation.rs::IncomingPlan` reuses bounded native-ID traversal, lexical/identity/metadata/count/byte admission and source revalidation. Every protected input, requested path/type and descendant survives until conflict resolution. Ordinary `CreationPlan::build` and ReplaceContent retain strict collision refusal at the original pre-traversal/pre-payload boundary. |
| Original conflict decisions | `worker_conflict::add::resolve` uses existing `InteractionPort`, `ConflictPrompt`, numbered-name helper and attempt-local ApplyAll. Incoming/incoming and incoming/original conflicts complete before external payload opens. Overwrite selects the accepted protected input index and retains original replacement binding; discarded inputs are not encoded. |
| Complete name reservation | Original members/implicit parents, all requested incoming paths/types and generated names reserve one case-insensitive namespace. Actual original/accepted occupancy stays separate. Directory merge preserves accepted spelling and old directory metadata; file/directory and ancestor mismatches refuse. |
| No-change and one publication | Original ZIP, qualified ordinary single-volume7z and TAR/TGZ/TXZ adapters consume resolved indices. Merge-only/allSkip returns without a candidate, including skipped-count zero. Accepted new empty directories remain changes. Format finish/verification/readback/reader-close/native source-target checks and one-shot publication retain their owners. |

Source: `rust/crates/nxtools-archive/src/{creation.rs,worker_conflict/add.rs,worker_zip_edit.rs,worker_sevenz_edit.rs,worker_tar_edit.rs}`. ZIP preserves old encoded records and its new-entry plain Deflate policy; Add reserves ZIP64 local headers using checked size plus the existing Create expansion allowance. 7z/TAR overwrite preserves original metadata. No generic Create collision relaxation, format qualification expansion, public DTO, UI, scheduler, sharing-policy or publisher replacement is included.

`ArchiveWorkerRunner.Password.cs::DiagnosticPassword` accepts the existing ZIP `modifying` password-state transition. It retains correlated attempt/elapsed events and the existing secret-free diagnostic fields. ZIP/7z wrong passwords remain terminal `authentication_or_data_error`; they are not guessed `WrongPassword` or automatically retried. Source-writer preservation fixtures assert the existing early reservation refusal (`source_unavailable`, native32/33), then independently exercise stale-source and publication boundaries.

## Reference and original lifecycle

Saved Bandizip evidence was inspected as reconstructed pseudocode and assembly/call relationships: `ark-create-p-0x18001dd30.json` (old-index import), `ark-create-p-0x18001e1d0.json` (member/name planning and planned replacement), `bandizip-progress-code-0x14011a320.json` (decision versus retained policy), and `bandizip-operations-dossier-0x140105370.json` (constructor and callers). This is static shipped-artifact evidence, not proprietary source or reference execution. The archive reference note and ZIP/7z/TAR/native contracts own NxTools-specific fidelity and publication.

Native route: `ArchiveJobSession.Dispatch` → immutable admission → `ArchiveSourceLease.OpenForEdit` + `ArchiveCreationSources.Open` → opaque ID enumeration/read/close/check → format candidate verification → `edit_reads_complete` → `MarkCandidateVerified/CompleteEditReads` → `CommitVerifiedEdit` → source/target/cancel recheck → FILE-only release → one `CommitCore`. Distinct filesystem roots already receive distinct native IDs. Source and output ownership are unchanged.

## RED/GREEN evidence

- **Duplicate Add inputs:** two different protected IDs with the same basename into `folder/` failed before product edits with `OutputCollision`, zero conflict prompts, and `Add same-name sources must reach original conflict ticket`. The actual-worker regression now reaches the original ticket before incoming payload reads and verifies exact source/name binding.
- **ZIP64 encoding boundary:** Add's actual encoding options failed the local-extra reservation assertion at declared size4,294,967,294. The test now verifies plain Deflate and ZIP64 local reservation for ZIP32-boundary/32GiB/checked-overflow inputs without creating a giant archive.
- **Encrypted ZIP native entry:** the actual browser→Job→worker route rejected the valid post-password `modifying` frame as `stdout_invalid_frame`/`protocol_violation`. The same route now reaches conflict resolution and exactly one native publication; password failure/cancellation still publish nothing.

## Executed verification

| Check | Final result |
| --- | --- |
| Five-format Add worker matrix | 5/5 tests;29 scenarios each,145 completed worker operations |
| Ordinary Create planning, including exact/case duplicate file/directory roots before traversal | 5/5 |
| Original ZIP interactions plus duplicate-source regression | 3/3 |
| ZIP record/crypto,7z safety/worker,TAR matrix/raw-header/worker,EIC preservation | 35/35 |
| ZIP64 actual encoding-option/local-header regression | 1/1 |
| Native Add namespace | LIST3/RUN3/PASS3/SKIP0;79 scenarios |
| Original native source/cancel/unknown/latecancel preservation | LIST7/RUN7/PASS7/SKIP0 |
| C# Debug test host build | Zero warnings/errors |
| Archive isolated-codecs all-target strict Clippy | Passed |
| Source/documentation gates | Spec contracts120documents/14owners, GPUI release, pinned editor, scoped rustfmt/diff, Python syntax/task JSON passed |

The worker matrix includes four decisions, ApplyAll/per-ticket behavior, stale/duplicate replies, future/reversed/case names, nested directory merging, implicit/explicit merge-only, new empty directories, type collisions, source-change/stale binding, corrupt readback and unknown commit. Independent Python zipfile/tarfile and stock7Zip compare exact names/content; TAR metadata is preserved. Controlled native replies expose source-open/close, candidate begin/abort and commit counts. These145 operations are not real NTFS publication evidence.

### Actual native matrix

| Format | Scenarios | Native publications | Zero-publication outcomes |
| --- | ---: | ---: | ---: |
| ZIP | 17 | 9 | 8 |
| Qualified single-volume7z | 20 | 10 | 10 |
| TAR | 14 | 8 | 6 |
| TGZ | 14 | 8 | 6 |
| TXZ | 14 | 8 | 6 |
| **Total** | **79** | **43** | **36** |

The79 saved terminal records agree on flush/commit/publish/FilesCommitted counts and completed cleanup. Each changed archive has one committed replacement journal entry. Merge-only/allSkip/cancel/type refusal retains exact original bytes; external file bytes and exclusive source reopen are checked. New empty directories publish once. The encrypted cases cover ZIP AES and7z AES/header AES with correct password, wrong password and password cancellation. Independent .NET, Python TAR/bsdtar and stock7Zip verify output names/content; new ZIP members remain plain Deflate while retained members remain encrypted.

Original seven preservation fixtures retain source-writer refusal, stale revision, held source/parent, reply-only cancellation, final cancellation, native5/64, unknown outcome with retained evidence/no retry and postpublication late cancellation. Unknown-publication fixtures intentionally retain their owned candidate evidence; ordinary failure/cancellation fixtures assert removal of unpublished partials.

### Build and evidence identity

- Current Debug `isolated-codecs` worker:12,032,512bytes; SHA256 `EB0A70A5B3DE8016365D5AC6788A4A533962DE0E13328E88629070B0D15C7977`.
- Frozen executable: session scratch `opt02-final-worker/NxTools.ArchiveWorker.exe`, copied into fresh GUID-owned `archive-product-opt02-*` roots by the original owned-package mechanism.
- Host: `csharp/NxTools.PlatformHost.Tests/bin/Debug/net8.0-windows/NxTools.PlatformHost.Tests.exe`.
- Native final log: `opt02-native-run-final.log`; preservation: `opt02-native-preservation-{list,run}.log`; per-scenario native terminal/counters: each owned root's `result.json`.
- Rust final behavior: `opt02-rust-final.log`; RED/GREEN: `opt02-first-{red,green}.log`, `opt02-zip64-{red,green}.log`; strict lint: `opt02-clippy-final.log`. Logs/fixtures are under `D:\.pi-desktop\data\scratch\a2072588-1f47-4232-be7a-3951a25df615`.

## Replay

Every Rust invocation sets `CARGO_BUILD_JOBS=1`, `CARGO_INCREMENTAL=0`, `RUSTFLAGS=-C codegen-units=256`; use one Cargo lane and adequate full-operation timeouts. TEMP/TMP point to owned session scratch. The worker matrix requires the explicit existing stock7Zip executable through `NXTOOLS_ARCHIVE_7ZIP_ORACLE`.

```text
cargo test -p nxtools-archive --features isolated-codecs --test add_namespace_worker_contract --test creation_plan_contract --test edit_interaction_worker_contract
cargo test -p nxtools-archive --features isolated-codecs --test eic_repair_worker_contract --test sevenz_edit_safety_contract --test sevenz_edit_worker_contract --test tar_edit_matrix_contract --test tar_edit_raw_header_contract --test tar_edit_worker_contract --test zip_edit_contract --test zip_edit_worker_contract
cargo test -p nxtools-archive --features isolated-codecs --lib worker_zip_edit::tests::add_zip64_reserves_local_header_before_payload -- --exact
cargo clippy -p nxtools-archive --features isolated-codecs --all-targets -- -D warnings
dotnet build csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-restore -m:1 -p:UseSharedCompilation=false
NxTools.PlatformHost.Tests.exe --fixtures --list "archive OPT02 native Add namespace"
NxTools.PlatformHost.Tests.exe --fixtures "archive OPT02 native Add namespace"
```

Native environment: `NXTOOLS_ARCHIVE_WORKER_FIXTURE` and matching `NXTOOLS_EDIT_INTERACTION_WORKER_SHA256`, `NXTOOLS_ARCHIVE_SEVENZ_EDIT_WORKER_SHA256`, `NXTOOLS_TAR_EDIT_WORKER_SHA256`; `NXTOOLS_ARCHIVE_PYTHON_ORACLE`, `NXTOOLS_ARCHIVE_7ZIP_ORACLE`, `NXTOOLS_TAR_EDIT_BSDTAR`; `NXTOOLS_ARCHIVE_ZIP_FIXTURES=rust/crates/nxtools-archive/tests/fixtures`, `NXTOOLS_TAR_EDIT_ORACLE=tests/integration/tar-edit-oracle.py`, and `PI_SCRATCH_DIR`. Resolve repository-relative oracle paths to absolute paths. Missing prerequisites/skips are not acceptance.

Preservation selection uses these exact names, LIST then identical RUN: `archive sevenz edit native source cancel handoff unknown`; `archive tar edit native source cancel handoff`; `archive tar edit native failure unknown late cancel`; `archive ZIP edit native changed revision refusal`; `archive ZIP edit native same path cancellation truth`; `archive ZIP edit native handoff failure unknown no retry`; `archive EIC AC02 native cancelled reply only ZIP TAR 7z`.

## Limitations

Native Job probes count flush/commit/publication, not individual incoming source opens. Zero unnecessary payload opens and exact protected-ID selection are observed in the actual worker protocol matrix; real native source/content/journal/publication is separately verified. The native7z `implicit_skip` case uses its explicit directory seed; true implicit behavior is covered by the7z worker matrix and native ZIP/TAR cases. ZIP64 coverage is a real writer-option/header boundary test, not a multi-gigabyte end-to-end publication.

Physical GUI/Explorer/DPI/accessibility and Release acceptance remain unexecuted. G-00 total source reads/internal format phase/block accounting and the original137,737,166-byte sample comparison remain incomplete. G-04 duplicate payload-verification optimization belongs to OPT-05. This result does not close all G-04, OPT-00/01, Phase E or the overall optimization task. No packaging/signing/install, real user archive mutation, recycling/HZ5, service/registry changes or commit/push occurred.
