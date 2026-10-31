# Release archive test-only fields — bounded runtime verification contract

2026-10-24. Authorized validation slice within Phase A — Engine contracts and safety foundation.
B2/B3/SAFE04 reconciliation is [independently accepted](phase-a-b2-b3-safe04-closeout-acceptance.md)
and is not reopened. Only [the checkpoint](phase-a-checkpoint.md) owns current state. This contract
precedes new regression code and execution; none of the planned results below is already PASS.

## Exact boundary and expected behavior

[worker_main.rs](../../../rust/crates/nxtools-archive/src/worker_main.rs) accepts only
`--archive-worker`, invokes original `worker::run`, and exits1 without stderr if that call fails.
[worker.rs](../../../rust/crates/nxtools-archive/src/worker.rs) first reads a <=1MiB length-prefixed
JSON WorkerRequest with `deny_unknown_fields`, then validates, then creates control/reader state and
emits accepted. `zip_risk_fixture` exists only under `cfg(debug_assertions)`; disabled means UNKNOWN,
including a JSON null value. Decode failure is InvalidData -> process exit1, not a mandatory business
failure frame. Source acquisition happens only later in execute. No new parser/error contract.

Production [zip_risk.rs](../../../rust/crates/nxtools-archive/src/zip_risk.rs) retains strictly
>64MiB AND >1000x known-positive packed bytes, overflow-safe u128 arithmetic and exact immutable
admitted-ceiling approval. No thresholds/cfg/binary patches or synthetic large expansion are allowed.
C# [original Risk serializer](../../../csharp/NxTools.PlatformHost/Windows/Archive/ArchiveWorkerRunner.Risk.cs)
adds the field only inside `#if DEBUG`; its property is also conditional. Original runner Pack
request branches omit it, call AddRiskFixture, and later forward the same private input. Review is
SOURCE evidence only unless this round explicitly runs the original C# serializer; no shadow clone.

## Smallest change / non-goals

The missing evidence is actual Release-process first-frame rejection, not missing product behavior.
Add an explicit-worker Python regression driver and bounded driver unit tests under tests/integration.
Expected workspace writes: those two new files, this contract, additive outcomes, current checkpoint
only. Product/parser/threshold/C#/protocol/spec/dependency/vendor/history changes are NOT planned.
Only if an actual Release worker accepts the disabled field or executes will a red result justify a
minimal original-owner repair and retest. No refactor/filter/retry/extra protocol is authorized.

## Six process cases, one independently produced tiny ZIP

Use a fresh UUID child of one scratch evidence root, a CPython ZIP with one fixed small regular
payload and one empty file, literal sizes/CRC independently known. Use the normally built Release
isolated-codecs worker and unmodified original four-byte-LE-length JSON frames.

1. `control_test`: schema2 TEST without test field; original source_acquire is acknowledged by this
   raw driver, actual decoder/integrity completes with exact expected file/byte counts and exit0.
   stdin remains open until terminal; ACK/accepted alone is not success. No output request allowed.
2. `test_fixture_object`: same TEST request plus valid-shape `{bytes:1,ratio:1}`.
3. `test_fixture_null`: same TEST request plus `zip_risk_fixture:null`.
4. `extract_fixture_object`: same source/options/ID basis, only operation=extract and valid DEBUG
   shape; guards against a DEBUG build rejecting the TEST object merely at semantic validation.
5. `extract_fixture_null`: same extract shape plus null.
6. `unknown_field`: one representative ordinary unknown field; no field matrix.

Cases2-6 must finish in a finite deadline with exit1, stdout0bytes/0frames and stderr0bytes, no
accepted/source_acquire/native/output calls. Expected nonzero rejection is PASS only when all facts
match; timeout/forced termination, decode failure in the driver, accepted/failure frames or unexpected
exit are not rejection proof. The driver must fail if a forbidden field reaches accepted or source
acquisition; never issue output/native capability replies to a negative case. Keep each exact request,
raw wire and stdout/stderr/decoded sequence/elapsed/exit/forced facts. Preserve failed logs.

The source-acquire success reply in the control is a transport fact injection, NOT a real C# lease,
Job, helper, native-source or cleanup acceptance. Decode completion and owned process exit are real
worker facts. All native output requests are denied, never forwarded to Windows. No extraction is
allowed to complete or publish. Scratch manifest/file/worker SHA256 before/after checks retain scope.

## Build / validation / stop

Before EVERY Cargo command check ONLY the cargo/rustc shared lane; wait, do not kill/overlap/inspect
product processes. Set CARGO_BUILD_JOBS=1, CARGO_INCREMENTAL=0, RUSTFLAGS=-C codegen-units=256 and allow
full-operation timeout. Build only the Release worker with original source/features, no package.ps1,
artifacts output, signing, sequence reservation or installed/protected state. Retain actual Cargo
artifact profile (debug_assertions=false,opt_level=3), features, path,size,hash and matching source
hashes. Fresh worker SHA256 before/after all six cases must match. Run driver-specific unit tests,
Python strict syntax/format checks and necessary docs/spec/task/scoped-diff/link/preservation gates;
no full Desktop/CLI/accepted format/password/conflict/resource/large/performance collections.

No ordinary product launch, process-list/product-status audit, service/updater/installation/release
ledger observation, Registry/COM/UNC/recycling/P209/elevation/commit/push. UI/Explorer remains DEFERRED /
NOT RUN. Worker-only evidence and C# source-only evidence stay separate. No full SAFE04/Phase A/F/
first-release claim, automatic packaging, UI resumption or next debt.

**Stop: Release测试专用字段运行时拒绝验证完成，等待独立验收。**
