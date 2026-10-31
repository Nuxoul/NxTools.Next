# Archive CLI — approved noninteractive slice

2026-10-24 user approves switching from manual P208 progression to archive CLI list/test/extract and
structured results. Same Phase F — Explorer integration and genuine native drag; this is automation
access to already implemented archive operations, not a renumbered phase or native-drag substitute.
The sole current state remains [checkpoint](phase-a-checkpoint.md). Real drag partial evidence and
sequence26 identity remain [live evidence](manual-package26-live-acceptance.md); installed26 is not
modified. No installer/release packaging, elevation, service/bootstrap/Registry/COM, commit or push.

## Smallest gap and owner

Current nxtools-cli parses only downloads. Existing ArchiveReadClient/ArchiveReadSession and
ArchiveJobSession already own bounded headless read/test/extract and actual worker cleanup. Complete
WindowsPlatformAdapter would initialize unrelated UI/native listeners, so CLI requires an early
archive-only composition of those same original sessions. No private worker protocol, second codec,
Job/source/output owner, direct filesystem extraction or Desktop startup is permitted.

Expected changes: CLI parser/main and new archive modules/tests; internal dependencies on existing
archive/protocol/IPC plus Tokio signal handling; C# early mode branch/new archive-only host/tests;
reference/spec/usage and task evidence. Existing download syntax/outputs must remain compatible.
Original archive engine/native transaction, credentials, budgets and installed artifacts stay unchanged.

## Commands and invariants

- `archive list <source> [--json] [--codepage N]`: original v1 bounded4096/4MiB metadata read,
  preserve ordinals, explicit unknown sizes, warnings and source snapshot; ACK only after retention.
- `archive test <source> [--json] [--codepage N]`: original noninteractive no-output read TEST,
  typed failures and actual exited/cleanup terminal, not listing-as-test.
- `archive extract <source> --dest <directory> [--json] [--codepage N]`
  plus explicit `--conflict fail|skip|overwrite|rename` and `--create-destination`.
  Here layout preserves archive paths, full archive only, MOTW enabled, original default finite budgets.
  Default fail maps original Ask to cancel-on-prompt, never overwrite. Explicit destination creation
  approves only create_destination, never low space/risk/output-budget escalation. No source deletion.
- Noninteractive only: no password in argv/environment/config/log, password/risk/conflict/low-space
  requests return a typed interaction requirement after cancellation and actual cleanup. No GUI dialog.
  No P209, archive create/modify, budget expansion, shell-open, virtual drag or auto-retry in this slice.
- One private GUID pipe and fixed sibling PlatformHost per invocation; exact sibling CLI parent,
  actual parent relationship/same ordinary user+session/live process. No normal Desktop pipe or adapter.
  Original platform envelopes/read credits/job receipts and original worker helper trust remain.
- Ctrl+C/deadline sends one original cancel, waits bounded actual cleanup; EOF/parent death stops
  original C# sessions. ACK/accepted/private Complete is never success. Preserve committed/skipped/
  cleanup/native facts even on cancellation/failure. Missing helper/mismatch fail closed.
- `--connect-only` remains download-only; archive must not silently ignore it or start Desktop.
- JSON: one schema-versioned result object with original operation/request ID, command, success,
  authoritative typed terminal/result and fixed error; no mixed progress/stdout noise or duplicate error.
  Text mode is human-readable with escaped member names. Preserve existing exit codes0/2/3/6/7/8.
- Diagnostics: existing original correlation through Rust/IPC/C#/worker; new host session events are
  start/connected/stopped/failure with session correlation, elapsed and safe stage/native code only.
  No payload names/paths/passwords, high-frequency logs, or raw child errors.

## Verification

First red-capable actual parser regression before command implementation. Tests exercise synthetic
protocol sessions for stale identities/receipt-versus-terminal/cleanup/cancel/prompt/output behavior.
Then source-layout Debug or fixed sibling isolated build using original worker approval machinery;
real CLI subprocess against owned tiny independently generated archives: list exact metadata, TEST
CRC failure, extract literal bytes/empty+implicit tree, conflict preservation, missing destination,
source-missing/unsupported input and JSON single-result/exit. Relevant existing download output tests,
CLI strict checks and affected C# safe fixtures; no full Desktop/codec cross-product/performance matrix.
One shared Cargo lane with jobs1/incremental0/codegen256, no duplicate long build. Preserve per-round
source hashes/concurrent dirty work and failed logs. No full native P208/PhaseF/first-release claim.

## Bounded lifecycle evidence contract — 2026-10-24

Ordinary public commands are independently accepted; lifecycle validation is a separate slice.
`tests/integration/prepare-archive-cli-lifecycle.ps1` creates one fresh scratch source-copy with only
explicit fixture observation wiring; the real CLI image and original worker/Job/source/output owners
remain unchanged. `ArchiveCliLifecycleFixture.cs` is NEVER compiled into product projects. Existing
runner output-write/native_published/control-input/cleanup checkpoints control tiny samples with
finite named-event barriers, not sleeps or large workloads. Exact fixture diffs and identities stay
with evidence. A successful controlled host is not an uninstrumented-host/physical-keyboard claim.

`archive-cli-lifecycle.py` creates a NEW owned Windows console and refuses CTRL_C_EVENT until its
process list is exactly the driver plus its own real CLI. Signal0/group0 is confined to that console,
never the current tool terminal or another task. Once sent, observe original cancel write, keep CLI
pending before actual cleanup, then require one non-success JSON and exact child/lease release.
Native signal, protocol fixture and physical keyboard evidence are reported separately. Published
cancel preserves cancelled_with_committed_output/count/bytes/state/journal and the committed file.
Parent termination has no JSON; EOF/Stop must drain originals, preserve commits, release owned
partials/source/helper. Whichever parent-monitor/EOF branch wins is recorded, not both asserted.

Original-runner short Drain(false) fixture proves only its held slots/source/helper/no terminal
credit and release-after-checkpoint. It does NOT prove every public CLI/session DisposeAsync timeout
branch, nor authorize changing inherited Drain(false) handling without public reproduction.
Diagnostics first use isolated sink, then bounded own-PID/date default-sink capture. Record actual
session and operation correlations, elapsed/native fields; do not rewrite old shared logs. Missing
one observation is not reproducible diagnostic failure if a complete scoped capture later passes.
No generalized cleanup/logging framework, dependency change, ordinary-command rerun or packaging.
