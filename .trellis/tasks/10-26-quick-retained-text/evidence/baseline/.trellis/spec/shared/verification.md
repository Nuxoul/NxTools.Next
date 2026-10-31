# Verification and evidence levels

## Commands and workstation policy
Evidence: `repo:Cargo.toml`, `repo:scripts/build.ps1`, `repo:scripts/package.ps1`, `repo:tests/architecture/package-memory-policy.ps1`. Root Cargo.toml is the workspace manifest; rust/ has none. No command here grants permission to package/install/launch a product or mutate a real service.

Every direct or background Rust command on this workstation must inherit:
```powershell
$env:CARGO_BUILD_JOBS='1'
$env:CARGO_INCREMENTAL='0'
$env:RUSTFLAGS='-C codegen-units=256'
```
Coordinate one Cargo process. Use full-operation timeout, inspect surviving cargo/rustc after timeout, never duplicate builds. If building was declined for memory pressure, compiled verification requires separate approval. Release wrappers require committed-memory headroom preflight and exact process/service ownership.

## Check selection
| Change | Relevant verification |
| --- | --- |
| Specs/docs only | `python tests/architecture/spec-contracts.py`, scoped diff and source evidence; no Cargo needed |
| Rust owner | `cargo fmt --all -- --check`; `cargo clippy -p <owner> --tests -- -D warnings`; `cargo check --workspace --tests`; owner tests and affected contracts |
| Desktop surface | `cargo test -p nxtools-desktop -- --test-threads=1`; relevant production GPUI bounds/actions and lifecycle gates; visual/native checklist |
| Download | `cargo test -p nxtools-download --lib`; relevant tests under tests/; engine_contract serial `--test-threads=1` for integrated boundary evidence |
| Indexer | Focused synthetic/admission/cache tests; do not run unrestricted real-volume tests when scan prohibited |
| C# | Build/test paths under csharp; no args/--fixtures uses explicitly audited effects, or exact safe name filters; native requires --native --authorize-native plus filters after separate approval. --list has RUN0; zero/unclassified/cross-effect selection exit2; summary reports pass/skip/fail separately |
| Extension | PowerShell expands `(Get-ChildItem browser_extension/nxtools-interceptor/tests -Filter '*.test.js').FullName` then `node --test $extensionTests`; direct Blob assert script and Native Messaging gate |

Use command exit status and actual RUN/test counts, not a truncated native stdout pipeline. For long Cargo output use `cmd /d /c "cargo ... 2>&1" > $log` and capture LASTEXITCODE before other commands. Avoid piping running native commands to Select-Object -First. PowerShell statement foreach output is assigned before Format-Table; avoid automatic variable names such as Host/PID/Error/HOME.

## Safe boundary vs real acceptance
- Static/source gate: structure/call ownership/token use only. Regex cannot prove UI completeness, native behavior or symbol meaning.
- Compiled unit/integration: typed state/results, temporary pipes/sockets/SQLite/actual fixture processes. Production GPUI render bounds and custom HWND API tests are stronger than helper predicates, but not a live user gesture.
- Visual/interactive: supported DPI screenshots plus mouse/keyboard/focus/window lifecycle. Screenshots alone cannot certify interaction or accessibility provider exposure.
- Release/native service: approved package/protected runtime tuple and LocalSystem restart/search/CPU. `Ready` or synthetic search is insufficient.

Actual gates: `repo:tests/architecture/download-ui-lifecycle.ps1`, `repo:tests/architecture/quick-launch-lifecycle.ps1`, `repo:tests/architecture/clipboard-lifecycle.ps1`, `repo:tests/architecture/clipboard-ui-lifecycle.ps1`, `repo:tests/architecture/desktop-shell-lifecycle.ps1`, `repo:tests/architecture/browser-native-messaging.ps1`, `repo:tests/architecture/ui-token-boundaries.ps1`, `repo:tests/architecture/ui-consistency.ps1`, `repo:tests/architecture/ui-state-lifecycle.ps1`, `repo:tests/architecture/file-dialog-window-follow.ps1`, `repo:tests/architecture/indexer-lifecycle.ps1`. Gates tied to old dimensions are observed current guards, not override of approved target; update assertions with measured regression in the owning UI stage, never weaken just to pass.

## Test quality and failures
Add bug regression before fix at the real pattern/call-site seam. Boundary input/output assertions must go red on the symptom. Inject only the variable being controlled; e.g. sampled speed injection retains real writers/store/output. Synthetic Windows socket listeners explicitly restore accepted blocking mode with bounded IO; marker waits also check actual task terminal status. Test secrets and artifacts are temporary synthetic data.

A failing full suite stays failing even if single rerun passes; bounded causal diagnosis, then one integrated verification, not repeated whole-suite green hunting. Record exact run date/count/skips/failures. No optional reviewer loops beyond one directed pass unless asked. Preserve user dirty state; no reset or implicit commit/archive.

## Restricted release gates
Indexer changes to MFT/USN/service/cache require, when separately authorized, `tests/performance/indexer-idle-cpu.ps1 -RestartService -AssertRegression` against packaged LocalSystem then real service search. Restart restores versioned mmap base+persisted delta; packaged/protected approved/runtime hashes must match. Never kill same-name unrelated processes or stop unowned services. C# real DOpus/Files/Explorer, registry/tray/global hooks, external GPUI acceptance and currently unisolated cache/signing fixtures are explicitly outside default fixture selection. They are retained, listed with scope, and require separate native authorization/filters; missing supported live environment is SKIP, not PASS. Owned synthetic HWND/fake operations remain eligible fixtures. This classification does not grant release acceptance.

Current evidence is [dated SOL report](../../../docs/audits/2026-10-08-sol-review-remediation.md), including 2026-10-09 fixture-only 103/103 engine closeout. It is not a promise that all current changes were packaged or that pending real pointer/DPI/LocalSystem checks passed.
