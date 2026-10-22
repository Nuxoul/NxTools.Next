# Independent acceptance — approved atomic-path overwrite

## Verdict

**Original overwrite self-lock: RESOLVED under the user-approved amended contract on the tested local NTFS path. Overall overwrite slice: CONDITIONAL / two concrete outcome-propagation defects remain. Phase A and first release: NOT ACCEPTED (remaining scope unchanged).**

This review does not reinstate strict target-ID CAS: target recheck close→single path publication TOCTOU was expressly accepted. No demand to eliminate it or add backup-first recovery. No product code changed during this review. One bounded independent read-only review was performed, then both findings were independently reproduced using a scratch-only actual-worker probe.

## Independently executed checks

Required Rust memory variables were set in the same command environment, serial execution, no build timeout.

| Check | Actual result |
| --- | --- |
| cargo test -p nxtools-archive --features isolated-codecs --offline |31 passed/0failed/0ignored,exit0 |
| Same-feature actual worker build |exit0 |
| C# test host build --no-restore |0warnings/0errors,exit0 |
| Exact safe16+remediation6+overwrite7 fixtures |list selected29/run0; execution selected29/run29/pass29/skip0/fail0,exit0 |
| Original renamed rechecked atomic-path overwrite test |included and PASS in the29, not removed/skipped |
| Strict archive Clippy including tests |exit0 |
| cargo fmt --all -- --check |exit0 |
| cargo check --workspace --tests --offline |exit0; not full workspace tests |
| Extensions ZIP archive_import target |7/7PASS |
| Clipboard --lib archive_ |5/5PASS,39filtered |
| spec-contracts/component-catalog/task-context/test-selection gate |PASS100docs/14owners;79/79/78catalog;17implement/14check,no truncation;fixture selector source gatePASS |

No full/default C# suite, UI/Shell user gestures, FAT/SMB/old Windows, power-loss, DPI/UIA, packaging/install/service/registry was exercised. No leftover test helper/worker/Cargo/rustc/test-host observed after the probe; original fixed helper absent. Known small attributed fixtures and owned temporary directories only.

## Confirmed delivered behavior

Source-reviewed `ArchiveOutputSession.cs`: exclusive sibling partial; integrity remains codec-owned; flush then target snapshot recheck with full128-bit identity/hash; target check handle closes before Win32 FileRenameInfoEx publication while parent/source/temp protection remains. SDK enum22 vs NT65 and explicit WCHAR NUL implemented. No MoveFileEx fallback, delete-first, truncate-first or backup-first.

Native success sets entry.Committed/State and publishes idempotent token count before close/postcheck. Session cleanup refuses partial deletion of CommittedButVerificationFailed/TargetChangedAfterCommit/Unknown; cancelled operations retain actual committed count. Tests include actual occupation32, target changes, reparse, known faults, unknown retention, native success/late cancellation and ABI case. These results support removal of the old self-lock blocker, not a broad filesystem guarantee.

## Remaining P2 findings (both reproduced)

### 1. Specific target recheck errors lost at worker terminal

Sources:
- `csharp/NxTools.PlatformHost/Windows/Archive/ArchiveWorkerOutputBridge.cs:96-99` records typed commit category.
- `rust/crates/nxtools-archive/src/worker_output.rs:160-170` maps target_changed/missing/directory/reparse/busy/denied to IoFailure fallback.
- `csharp/NxTools.PlatformHost/Windows/Archive/ArchiveWorkerRunner.cs:366-372,281-291` sets worker_failure and only recovers selected commit categories (not ordinary recheck errors).

Actual tiny lzma.7z request with a confirmed overwrite; scratch hook changes target to `external` at before_recheck. Native session reports target_changed. Observed final:

```text
Category=worker_failure
CommitState=FailedBeforeCommit
NativeErrorCode=null
FilesCommitted=0
CleanupComplete=true
final bytes=external
partials=0
archive.output.failed stage=target_changed
```

Data protection is correct, but the promised typed target-change outcome is not available as the terminal category. Future conflict UX cannot reliably choose re-prompt vs generic retry from this terminal. Existing ChangesBeforeRecheck asserts session typing only; ActualWorkerOverwriteLifecycle lacks corresponding end-to-end cases. Preserve authoritative current native commit fault through terminal, without treating stale earlier per-file categories as the current failure or overwriting cancellation/unknown/postcommit precedence. Add real worker changed/missing/type/busy/denied cases and assert category/state/native/count and protected output.

### 2. Diagnostic terminal category contradicts emitted result

`ArchiveWorkerRunner.cs:284-292,419-426`: emitted terminal computes authoritative category, but Diagnostic uses run.Failure.Category (worker_failure), not that resolved result. Reproduced actual-worker commit-boundary unsupported50:

```text
exited.Category=overwrite_not_supported
exited.NativeErrorCode=50
exited.CommitState=FailedBeforeCommit
archive.output.failed stage=overwrite_not_supported
archive.worker.failed category=worker_failure
final bytes=original,partial count=0
```

Same structural mismatch applies to resolved unknown/postcommit categories. No data-loss evidence; diagnostics no longer describe the same operation outcome. Resolve the terminal once into typed metadata and use it for both event/log, preserving correlation/state/count/native and low-noise/privacy. Add captured-log vs terminal equality regression for unsupported/unknown/postcommit/cancel-with-output and normal success. Do not log raw frames/paths/passwords.

## Probe evidence / next step

Scratch-only project `overwrite-review-probe/` references the built PlatformHost and its existing internal test seam; actual C# Job→Rust worker→native output. Tiny source lzma.7z, owned temporary output, no product test registration/source changes. Initial scratch compilation missed System.IO import (preparation issue only), corrected before either scenario executed. Probe returns0 because it records outcomes; observed mismatches above are failures of expected semantics, not green acceptance tests.

Only these two bounded fixes are requested for this slice; no new scope/contract approval or native primitive investigation needed. Preserve29passing fixtures, add regression for these results and rerun relevant tests. Do not reopen broad reviews or claim Phase A finished. Full source and byte semantics of the amended publication path are locally verified; current remaining slice gap is typed result/diagnostic propagation.
