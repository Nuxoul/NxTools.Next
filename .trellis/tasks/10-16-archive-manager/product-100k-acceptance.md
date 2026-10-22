# Explicit finite product100k — independent acceptance

Date: 2026-10-20. Canonical **Phase A — Engine contracts and safety foundation** remains in progress.
Scope: independent acceptance of [finite v2 contract](product-100k-contract.md) and
[implementation evidence](product-100k-evidence.md). Product source/tests were NOT modified.
One bounded read-only review, one fresh actual100k observation, no automatic measurement repeat.

## Verdict

**Product100k primary read correctness: PASS within this fixture/Debug scope. Full combined
resource/lifecycle observation: NOT ACCEPTED, one P2 observation-tool finding remains.**

The independent actual chain successfully streamed and compared all100000members, produced the
same owner/remote successful terminal and confirmed original source/helper/Job cleanup. However,
the driver encountered PermissionError reading status.json and requested driver_exception stop
during follow-up reuse. Its immutable verdict is **Failed**, operation wrapper **Stopped**, not
resource PASS. Do not relabel it from the successful inner terminal or complete PID samples.
The complete requested slice cannot be signed off as having no remaining blockers.

No concrete product v2/protocol/IPC defect was found in this review or these regressions.
No PhaseA/ENG08/A–F/UI/native-user/Release/deployment completion is implied.

## P2 — status polling converts a recoverable sharing refusal into a whole-attempt stop

- Owners: `tests/performance/archive-resources/control.py:10-15`,
  `driver_loop.py:110-113,146-148`; sampler also reads the same status in `run.py:241`.
- Actual attempt: `read_status()` raised `PermissionError: [Errno13] Permission denied` for the
  original run's status.json. Driver records this exception and immediately requests
  `driver_exception`. Attempt CLI exit1, probe exit0, non-forced cleanup Confirmed; measurement
  Complete. The original100k terminal is succeeded/cleanuptrue, but `same_router_reused` was NOT
  recorded. Do not claim follow-up reuse succeeded in this independent observation.
- Independently reproduced at the unchanged real reader boundary: create a tiny valid schema2
  status file; retain an exact Win32 CreateFileW handle with share mode0; call read_status =>
  PermissionError/errno13; close that handle => the same file/run identity reads successfully.
  This uses no product process, archive, worker, large data or benchmark repeat.
- Source: read_status has no handling for transient open refusal; driver treats every exception
  as fatal. Rust status writer uses bounded status.tmp followed by rename under its state lock
  (`examples/resource_probe_support/control.rs:112-153`). A Windows rename/open sharing race is
  plausible, but the original Python exception did not retain its Win32 code or competing handle;
  **the exact contender/cause of this attempt is not proved**. The tiny exclusive-handle probe
  proves the recovery boundary, not that its held-handle timing equals the original event.
- Required correction: narrowly handle transient Windows sharing/open races within a finite
  polling/retry policy, preserving last-good observation and real error/native-code accounting.
  Do not silently accept stale/malformed/oversized/wrong-run status, erase persistent permission
  failures, claim cleanup from process exit, or weaken resource PASS. Cover reader/writer and
  driver behavior with small deterministic regressions. Product budget/timeout changes are not
  warranted. This acceptance intentionally does not implement a fix or run another100k attempt.

## Independent actual100k result

Fresh root under session scratch:
`archive-resources-0586f1bc-7576-42df-aa6c-09e7c10c338e`.
Run `8ea055ef-5f11-4cbc-8e65-3957f96396f9`; original100k operation
`89c593ba-7379-450c-9834-a13f0d7f21be`.
Actual chain: Rust ArchiveReadClient -> original typed pipe -> production ArchiveReadSession in
isolated test apphost -> original runner/Job/source lease -> actual isolated-codecs Rust worker.

`fixtures.py ROOT --catalog-only` generated CPython100000members/100directories/100Unicodefiles/
9081emptyfiles/1362285payloadbytes. No1GiB or largeZIP64 generated. ZIP12161983bytes SHA256
`34a9f9fe2e1d669b825d3557e523cadf00aeabf1ebfe187c99fcd4475cda956d`;
streamed manifest SHA256 `4fd4bdca3f5ceb0f388206c1c2a12cf827be97856313f3be85c5efb31e2768dc`.
Independent post-run hashes match fixture-summary, including credit.zip.

| Observed fact | Independent result |
| --- | --- |
| Streamed manifest / snapshot |100000exact,391contiguous batches; b6483d5b-574b-47df-894f-3650693c1f05 |
| Default request |ResourceLimit, confirmed cleanup |
| First business metadata |2306.2309ms |
| Full original product terminal/cleanup |7207.2709ms; runner7201ms; unchanged30s |
| Private cumulative transport including headers |16745255bytes |
| C#business cumulative receipt |16708854bytes |
| Rustbusiness cumulative receipt |16707854bytes |
| Max batch / entries / inflight |42817bytes /256 /1 |
| Approved Job process budget |268435456bytes, not observed process memory |
| Inner original terminal |succeeded,100000entries,391batches,cleanuptrue |
| Full attempt |Failed; Stopped/driver_exception, no forced targets |
| Measurement / cleanup |Complete /Confirmed |
| Same-router follow-up reuse |not completed/asserted; no success claim |

First batch is not first paint. Debug elapsed is not a Release1.5s promise. Cumulative bytes are
not simultaneously retained RSS. Requested20ms sampling/50ms discovery does not guarantee peaks.

| Exact process | Valid samples / counter failures | Sampled WS / Private bytes |
| --- | --- | --- |
|7020 Rustcaller |252 /0 |9330688 /2805760 |
|4768 C#host, parent7020 |249 /0 |61775872 /30429184 |
|6720 actual100kworker, parent4768 |230 /0 |67080192 /70934528 |

All images/parents verified; all five observed processes ended, including separate default worker
21308and follow-up worker22468. Actual100kworker max valid gap33.187ms. Follow-up worker had only
one sample and was stopped by the driver; it is not evidence of successful router reuse.

## Independently rerun verification

All direct/background Rust invocations inherited CARGO_BUILD_JOBS=1, CARGO_INCREMENTAL=0,
RUSTFLAGS=-C codegen-units=256. Machine-wide Cargo/rustc checked before each build/observation.
C#isolated test apphost built first; isolated worker rebuilt after feature-unifying checks before
actual observation/C#fixture execution. No unrelated processes terminated.

| Command / gate | Actual result |
| --- | --- |
| Archive isolated offline no-fail-fast, serial |150pass/0fail/0ignored |
| Protocol / IPC lib offline |53 /30pass,0fail/ignored |
| resource_probe isolated example tests |11pass,0fail/ignored |
| Strict Clippy owner tests/examples isolated +default |both exit0 |
| cargo fmt --all -- --check |exit0 |
| cargo check --workspace --tests --offline |exit0; compilation ONLY, not workspace tests |
| Extensions archive_import / Clipboard archive_ |7 /5pass,0fail/ignored |
| C#Debug build |0warnings/0errors |
| Exact C#LIST |134selected,run0 |
| Same C#execution |selected134/run134/pass134/skip0/fail0/not_run0 |
| C#identities |selection==LIST==RUN==PASS, exact134in catalog order; largeZIP64excluded |
| Python resource tool unittest |28pass,0fail/error/skip |
| spec / catalog / context / selector / diff |all exit0 |
| Independent assertions / real reader sharing probe |PASS;24raw/log hashes saved |

Spec gate106documents/14owners; catalog79pages/79rows/78locked; context68/65entries.
Existing native-contracts.md35497bytes>32768 warning remains, not a new product failure.
One optional read-only reviewer checked Rust quotas/negotiation/batching/credit/partial-frame
cancellation: no concrete finding; runtime and C#evidence above belong to this parent, not reviewer.

Execution incidents retained, not hidden passes: initial background PowerShell disappeared with
no archive completion/test result and no observation started; its partial logs were preserved.
Foreground execution completed the above Rust checks. A scratch PS script's UTF8-without-BOM
Chinese oracle path was misdecoded before observation; continuation used Unicode codepoints and
verified the existing7-ZipSHA256. No completed test collection repeated to chase green. Assertion
script was corrected for PowerShell UTF16BOM and CRLF before its small probe; source untouched.
The single independent100k attempt remains Failed and was not rerun.

## Evidence preservation and resource boundary

Session scratch root:
`C:\Users\Administrator\.pi-desktop\scratch\ceebe351-d540-4ca3-a460-dadc71f3a7df`.
Logs: `product-100k-independent-*.log`; serial command ledger
`product-100k-independent-ledger.txt`; exact selection JSON; deterministic probe/assert script
`product-100k-independent-assert.py`; assertions JSON includes24input/log SHA256values.

- Original attempt SHA256 `ca165d886c837a78da137d0b387a4da0e44c946d37511155dbbd566812756f55`.
- Samples SHA256 `e8b291144acb16eb9041adc57cb73a06854eb6d20e25c65f4b9f91cc4078e8d3`.
- Assertions JSON SHA256 `05922ec53e4b9238b3de3892ba430b1cbf70f591f94580ab58f708b351db492b`.

Original delivery evidence/results were not modified. Post-review hashes:
evidence `3afe222ccd28fa25c1f311894e10d04affd2185c8ea87bcb21c92f74c2c7869c`;
results `a639d3ff3e56025e4009b3d7d9310e342ca84756aeb55c1c1499a8e50fe6e6db`.
These are observed closing hashes, not a claim that all historical files were independently
rehashed against an opening baseline in this turn.

Final check found0owned probe/host/worker,0archiveproduct/resource pipes, no fixed test helper,
and original observation package absent. Keep exact synthetic ZIPs/manifest/attempt/samples and
small sharing-probe status as evidence; no test-source deletion/recycle or recursive cleanup.
No formalDesktop/installedPlatformHost,Release/package/sign/install/commit/push,registry/service/
drive-map mutation, user archive operation or native source deletion. Broader PhaseA/native/
deployment gates remain as previously documented. Stop acceptance here pending the bounded tool
correction; do not advance to another feature or reinterpret this observation as resource PASS.
