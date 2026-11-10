# OPT-02 — independent acceptance

## Decision

**Accept OPT-02 — Add namespace correctness at its scoped automated/owned-native stop point.** G-11 and the touched G-04/G-19 integrity, fidelity, source-preservation and publication assertions pass. No confirmed in-scope blocker remains.

This does not close G-00, OPT-00/01 overall acceptance, the complete G-04 optimization, Phase E or the whole archive task. OPT-03 through OPT-07 remain not started. No new product code, test-source changes, format support, UI or native authority was introduced by this acceptance review.

Scope is the OPT-02 working-tree changes against HEAD `0940eec23286c1a750b3aff4284f32e76da68164`, including the new incoming namespace resolver and actual-worker/native fixtures. Previously accepted read/cache changes and unrelated workspace modifications are preserved, not re-reviewed. Requirements and delivery: [implementation](D:/Code/NxTools.Next-main/.trellis/tasks/11-02-archive-workflow-optimization/implement.md), [acceptance](D:/Code/NxTools.Next-main/.trellis/tasks/11-02-archive-workflow-optimization/acceptance.md), [OPT-02 delivery](D:/Code/NxTools.Next-main/.trellis/tasks/11-02-archive-workflow-optimization/opt02-add-namespace.md).

## Standards

**No confirmed violation in the accepted scope.**

- [IncomingPlan](D:/Code/NxTools.Next-main/rust/crates/nxtools-archive/src/creation.rs:69) retains original protected IDs, requested names, types and descendants until decisions complete. Ordinary Create and ReplaceContent still use strict [CreationPlan::build](D:/Code/NxTools.Next-main/rust/crates/nxtools-archive/src/creation.rs:127); duplicate roots are rejected before traversal/payload as before.
- Collection and namespace admission remain finite under the existing count/path/metadata/work constraints. The collector uses conservative per-entry/depth reservations; [namespace reservation](D:/Code/NxTools.Next-main/rust/crates/nxtools-archive/src/worker_conflict/add.rs:185) checks before allocating reserved/live/result structures. This acceptance does not claim an aggregate peak-memory benchmark or close G-00.
- The [shared resolver](D:/Code/NxTools.Next-main/rust/crates/nxtools-archive/src/worker_conflict/add.rs:30) returns protected input indices and original replacement indices, not filesystem mutation authority. It retains the original InteractionPort/tickets, cancellation and attempt-local policy.
- ZIP Add uses the locked writer's public ZIP64 reservation option with the same checked expansion policy as Create. [The actual option/header test](D:/Code/NxTools.Next-main/rust/crates/nxtools-archive/src/worker_zip_edit.rs:22) passes without pretending to test multi-GiB publication.
- Accepting the existing ZIP `modifying` password state in [DiagnosticPassword](D:/Code/NxTools.Next-main/csharp/NxTools.PlatformHost/Windows/Archive/ArchiveWorkerRunner.Password.cs:84) matches the actual original worker route. The shared known-state diagnostic whitelist does not grant source/output/commit authority. The same native tests retain terminal ambiguous authentication and explicit password cancellation; no blind retry or guessed WrongPassword is added.
- The source-writer fixtures now recognize the actual early reservation refusal (`source_unavailable`, native32/33) while retaining zero publication, original-byte and cleanup checks. Subsequent stale binding, cancellation, unknown commit and late-cancel assertions remain strict and were independently rerun.

## Spec

**G-11 / ADD-03 through ADD-05 match the requested behavior.**

| Requirement | Confirmed implementation and boundary |
| --- | --- |
| Same-basename protected sources reach Add decisions | Incoming inputs remain distinct before resolution; actual worker sees original tickets before external payload opens. Generic Create is not relaxed. |
| Incoming/incoming and incoming/original conflicts | Ask replies exercise Overwrite/Skip/AutoRename/Cancel, ApplyAll and per-ticket behavior. Repeated Overwrite changes the selected input while retaining its original replacement binding. |
| Full name reservation | Existing explicit/implicit types, all incoming original requests and generated names are reserved before AutoRename. Future/reversed/case-name cases retain correct source-to-final-name content. |
| Reservation is not occupancy | Future requested names do not prove original directories. Only original or accepted directories merge, retaining accepted parent spelling and old metadata. File/directory and ancestor mismatches refuse. |
| No-change | Merge-only/allSkip, including zero skipped files, has no candidate/commit/publication and preserves original bytes. |
| Real change | A new empty directory is a real accepted addition; changed archives use one complete verified candidate and one original native publication. |
| Format-specific fidelity | ZIP old records/encryption stay and new entries remain plain Deflate. Qualified7z and TAR-family updates retain their original metadata/rewrite policies. No format or edit-action capability expansion. |

Adapter anchors: [ZIP](D:/Code/NxTools.Next-main/rust/crates/nxtools-archive/src/worker_zip_edit.rs:36), [7z](D:/Code/NxTools.Next-main/rust/crates/nxtools-archive/src/worker_sevenz_edit.rs:319), [TAR family](D:/Code/NxTools.Next-main/rust/crates/nxtools-archive/src/worker_tar_edit.rs:163). The [worker matrix](D:/Code/NxTools.Next-main/rust/crates/nxtools-archive/tests/add_namespace_worker_contract.rs:5) and [independent content oracle](D:/Code/NxTools.Next-main/rust/crates/nxtools-archive/tests/fixtures/add-namespace.py:18) test these through actual worker adapters; [native matrix](D:/Code/NxTools.Next-main/csharp/NxTools.PlatformHost.Tests/EditInteractionTests.Opt02.cs:197) adds real source/output/publication evidence.

## Independent verification performed

All commands ran from the workspace root. Every Rust invocation used `CARGO_BUILD_JOBS=1`, `CARGO_INCREMENTAL=0`, `RUSTFLAGS=-C codegen-units=256`, with one Cargo lane. TEMP/TMP and fixture outputs were confined to the current session scratch. Native selections were LISTed, effects inspected, then RUN with the identical selection. No skipped fixture is counted as passed.

| Check | Observed result |
| --- | --- |
| `add_namespace_worker_contract` | 5/5 tests; five formats ×29 scenarios =145 actual worker operations |
| `creation_plan_contract` | 5/5 |
| `edit_interaction_worker_contract` | 3/3 |
| `eic_repair_worker_contract`, `sevenz_edit_safety_contract`, `sevenz_edit_worker_contract`, `tar_edit_matrix_contract`, `tar_edit_raw_header_contract`, `tar_edit_worker_contract`, `zip_edit_contract`, `zip_edit_worker_contract` | 35/35 |
| `worker_zip_edit::tests::add_zip64_reserves_local_header_before_payload` | 1/1; actual writer option/local-header boundary |
| C# test-host build, `--no-restore -m:1 -p:UseSharedCompilation=false` | PASS; 0 warnings/errors |
| Exact native filter `archive OPT02 native Add namespace` | selected3/run3/pass3/skip0/fail0;79 fresh scenarios |
| Exact original source/cancel/unknown/late-cancel preservation selection | selected7/run7/pass7/skip0/fail0 |
| `cargo clippy -p nxtools-archive --features isolated-codecs --all-targets --offline -- -D warnings` | PASS |
| Additional original7z Add/Overwrite metadata check | PASS; exact content, source binding, one candidate/controlled commit and original Created/Accessed/Modified/readonly Attributes |

Rust total is49 test cases, including the145-operation matrix. The additional metadata probe is separate; it is not included in either the145 or79 counts. It reuses the existing actual-worker `add-namespace.py` route with `SevenZ/original/overwrite`, independently seeds nondefault access/modify times and readonly attributes using stock7Zip with ctime/atime/mtime storage enabled, and compares all four metadata fields in independent before/candidate listings. This is worker/candidate fidelity evidence, not a second real NTFS publication result.

### Native result reconciliation

The79 new `result.json` records were read independently after the RUN. Every record has `cleanup_complete=true`; flush/commit/publish/FilesCommitted agree; nonzero publication is exactly one committed journal fact. Every zero-publication result has an empty journal and unchanged original asserted by the fixture. No `.nxpart-*` remains in each target archive directory.

| Format | Scenarios | Single publication | Zero publication |
| --- | ---: | ---: | ---: |
| ZIP | 17 | 9 | 8 |
| Qualified ordinary single-volume7z | 20 | 10 | 10 |
| TAR | 14 | 8 | 6 |
| TGZ | 14 | 8 | 6 |
| TXZ | 14 | 8 | 6 |
| **Total** | **79** | **43** | **36** |

Native cases include correct/wrong/cancelled passwords for ZIP AES and7z AES/header AES. Wrong passwords retain `authentication_or_data_error`, zero conflict prompts/publication and unchanged sources. The original seven preservation fixtures independently pass held-source/writer refusal, stale source, cancellation, native errors, unknown result/no retry and postpublication late cancellation. Unknown-result fixtures intentionally retain owned evidence under their original policy.

### Reproduction and identity

The native acceptance used the worker built from the current workspace, frozen at `opt02-independent-acceptance-worker/NxTools.ArchiveWorker.exe` beneath the current scratch root. SHA256: `290FDABEADFE3CC7B303A1EB055888F5A0890AF0BD487196786D3A8A9B997DEC`. The current and frozen copies matched before every native selection. This is this review's build identity, not the delivery report's earlier frozen binary identity.

Oracle: existing stock7Zip26.00 recorded by [the original interaction evidence](D:/Code/NxTools.Next-main/.trellis/tasks/10-16-archive-manager/edit-interaction-outcomes.md:108), verified against its existing approved hash `4BCC005758C5E604C0AAA257B6097846A4A708E62A9C0C7471FA360FAD863F1F`; current Python and system bsdtar supplied the existing independent readers. No download, installation, PATH mutation or product-backend change occurred.

Commands and environment selection are reproducible using [delivery replay](D:/Code/NxTools.Next-main/.trellis/tasks/11-02-archive-workflow-optimization/opt02-add-namespace.md:72), adding `--offline -- --test-threads=1` to the test selections. The seven native names are unchanged from that record. This review's logs are `opt02-acceptance-rust.log`, `opt02-acceptance-zip64.log`, `opt02-acceptance-format-preservation.log`, `opt02-acceptance-csharp-build.log`, `opt02-acceptance-native.log`, `opt02-acceptance-native-preservation.log` and `opt02-acceptance-clippy.log` under `D:\.pi-desktop\data\scratch\aab2f188-352d-4f1c-b7a3-5375fe0df2d8`. Fresh marked fixtures, sources, candidates/results, copied helpers and the frozen worker remain owned evidence in that scratch tree; actual operation leases are retired and successful unpublished partials are absent.

## Limits and next boundary

- G-11 is accepted; only touched G-04/G-19 preservation is accepted. The full duplicate-payload-verification optimization remains OPT-05 work.
- Total I/O, internal phase/block measurement and the original large ZIP comparison remain G-00 issues, not silently closed by these tests.
- The7z native `implicit_skip` row retains its disclosed explicit-directory seed; true implicit7z semantics are tested by the actual-worker matrix. Native source-open counts are not inferred from publication probes.
- ZIP64 is verified at real encoding-option/local-header admission, not multi-GiB encoding/publication. Resource peak, physical GUI/Explorer/DPI/accessibility and Release acceptance were not performed.
- No confirmed OPT-02 repair is requested. Stop at the current package; OPT-03 requires its own implementation instruction. No package, signing, installation, user archive mutation, source recycling/HZ5, service/registry changes, commit or push occurred.
