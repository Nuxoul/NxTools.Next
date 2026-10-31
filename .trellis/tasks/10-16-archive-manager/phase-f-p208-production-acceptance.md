# Phase F P208 production-row hookup — independent acceptance

Date: 2026-10-23. **NOT PASS: one confirmed connection-lifetime admission defect, F-P208-AC01.**
The remaining inspected bounded hookup and controlled content/cancellation boundaries pass the
independent checks below. This is not full P208, Phase F, real Explorer or first-release acceptance.
Current state is owned only by [the checkpoint](phase-a-checkpoint.md). The
[implementation outcomes](phase-f-p208-production-outcomes.md), [contract](phase-f-p208-production-contract.md)
and all historical negative/spike/repair reports remain unchanged.

## Confirmed blocker — F-P208-AC01

**High: a browser context from a retired PlatformHost connection can submit a start to the new
connection before the old Disconnected notification reaches the view.**

Source boundary:
- `rust/apps/nxtools-desktop/src/archive/browser_drag.rs:28-61` retains a shared PlatformBridge and
  browser/source context, but no origin connection generation.
- `browser_drag.rs:113-128,165-173` checks view generation/projection and `is_connected()` only;
  a reconnected bridge is already true. Browser generation is not the IPC connection generation.
- `rust/crates/nxtools-ipc/src/archive_virtual_drag.rs:57-65,104-123` takes the current active
  connection under the router lock and assigns that generation to the new pending/start reservation.
  There is no caller-origin generation comparison, so a valid old-view start is admitted to it.
- `rust/crates/nxtools-ipc/src/lib.rs:388-427,882-895` installs/enables the new connection before
  the queued old Disconnected notification is consumed by the Desktop application/view.
- `rust/apps/nxtools-desktop/src/archive/application.rs:79-83,815-839` defers that notification,
  and `browser_drag.rs:290-295` then clears local browser/pending without cancelling a start already
  admitted to the new connection. The new C# session is not the retired pipe's cleanup owner.

Independent scratch probe calls the unchanged public production-router contract seams:
1. Open connection 1 and construct a valid frozen old-view start.
2. Open connection 2 without consuming the queued connection-1 Disconnected notification.
3. Submit through the same bridge used by the old view.
4. Observe the connection-2 receiver and then consume the queued notification.

Actual output in `14-connection-probe.log`:
```text
old_view_start_send_ok=true
new_connection_received_old_start=true
queued_notification=Some(Disconnected { generation: 1, pending_command_ids: [] })
```
This probe exited 0 because it asserts/reports the observed erroneous admission; it is NOT a passing
regression assertion for the intended contract. No product source was reverted or edited. It does
not enter OLE, use a real HWND, or prove a physical drop. The demonstrated admission, together with
the real deferred view-clearing path, violates the explicit connection-retirement requirement and
can leave the new operation untracked by its view during native materialization/drop.

**Narrow repair recommendation:** bind the completed browser/source context to its original IPC
connection generation; compare that origin with the active connection atomically at start admission.
Reject retired context even when the shared bridge is connected again. Check both replacement before
context delivery and replacement between gesture preparation and enqueue; retain exact cancellation
and actual-cleanup reservations. Add a red-capable regression at the formal row/application plus
original bridge boundary: swap connection while notification is queued, assert no old start on the
new receiver, then prove a freshly read context can start normally. Do not solve by synchronous
cross-entity updates, arbitrary retries or another OLE/state framework.

No repair was silently made in this acceptance: this concerns the origin/context and original
bridge admission interface, not a mechanical formatting/test-input fix. Stop for a bounded repair.

## Inspected passing source/controlled scope

- Actual official ListItem Down records a candidate; held movement >=4 DIP commits once before
  release. Ordinary click/doubleclick and original BrowserModel selection/focus ownership remain.
  Product uses the original actual Window HWND resolver; override is cfg(test) only.
- Full visible selection freezes original ordinal/source/snapshot/generation/codepage/depth/MOTW.
  Current scope is 1..32 known-size unencrypted regular files, aggregate <=512 MiB. Unsupported
  members reject the whole selection through official Alert; no truncation or prepare-then-drag.
- Application hydration/results are deferred; row submission does not synchronously update the
  application. Projection/refresh/source/close retire candidates and request exact cancel.
- Formal and legacy C# aliases use the same parser, peer-HWND validation, bounded synchronous
  admission, detached receipt, dedicated STA and original content/worker/Job/cache/stream owners.
- Ordinary queue saturation does not lose the reserved exact cancel. Start precedes cancel;
  cancel ACK is not cleanup; fixed cleanup-incomplete retains the slot; only held drag receipts
  avoid ordinary 10-second expiration. Existing replacement tests prove old cancels cannot cancel
  a new reservation, but do not cover the old-context new-start admission above.
- Tiny owned ZIP fixtures demonstrate zero descriptor materialization, indexed original-worker
  contents, independent repeated cursors, changed-source rejection and target-held streams after
  retirement. Slow worker cancellation/Escape/source-close/disconnect waits for actual cleanup.

One bounded read-only IPC review was used, then parent validation; no second review loop. Its other
candidate, missing virtual-drag names in HelloAck.accepted_capabilities, was independently observed
as `hello_accepted=[]`. Current C# client validates only ACK type/correlation and does not gate this
route on that list (`PlatformPipeClient.cs:129-133`); current formal enqueue is not negotiated by it.
Therefore the suggested current-route outage was not established and is NOT another blocker here.

## Independent execution

Evidence root:
`C:\Users\Administrator\.pi-desktop\scratch\ceebe351-d540-4ca3-a460-dadc71f3a7df\p208-production-independent-20261023-162729\run2`.
Runners in the parent session scratch: `p208-independent-verify.ps1`, `p208-independent-final.ps1`.
Probe: `p208-connection-probe/Cargo.toml` and `src/main.rs`, referring directly to current IPC/protocol
source with offline dependencies. No workspace dependency/lock/source edit. Unique per-command logs,
`commands.jsonl`, `independent-results.json`, `full-meta.json` and C# metadata retain actual results.

| Check | Independent actual result |
| --- | --- |
| Fresh isolated PlatformHost then Tests builds | PASS, 0 warnings/errors; `01-host-build.log`, `02-tests-build.log` |
| Formal production drag tests, contained in browser/application collections | 8 distinct named PASS; identity list in `independent-results.json`; not an extra disjoint 8 tests |
| Browser actual-view collection | 26 PASS, 0 FAIL, 1016 filtered; `03-browser.log` |
| Application lifecycle collection | 10 PASS, 0 FAIL, 1032 filtered; `04-application.log` |
| Protocol library | 69 PASS, 0 FAIL/ignored/filtered; `05-protocol.log` |
| IPC library | 39 PASS, 0 FAIL/ignored/filtered; `06-ipc.log` |
| Strict Desktop/Protocol/IPC Clippy --tests -- -D warnings | PASS, exit0; `07-strict.log` |
| Workspace --tests | Compilation PASS, exit0; not execution of all workspace tests; `08-workspace.log` |
| Feature trees and matching isolated-codecs worker after preceding checks | PASS; `09` through `11` logs; no feature/dependency change |
| Exact controlled C# set | LIST5 = RUN5 = PASS5, selected5/run5/pass5/skip0/fail0/not_run0; names AND order equal |
| Matching worker SHA256 before/after C# | `5996E3AE615A1233BAD709983D1D79FEDB57446C65943977264293470A4CD16A`, identical |
| Connection-origin probe | Exit0 reproduces F-P208-AC01 erroneous admission; NOT contract PASS |
| ONE full Desktop | 1042 PASS, 0 FAIL/ignored/filtered, 134.58 seconds test time, exit0; `15-desktop-full-once.log` |
| Full-run drift | 684 Rust/C# source/project files monitored, none changed during the run |
| Scoped rustfmt | PASS on 10 affected Rust entry/module files, skip_children=true; no global reformat |
| Source/document gates | PASS: Phase F row/STA/content/reserved cancel, Shell, GPUI catalog/release, editor provenance, specs, task, fixture selection and diff |

C# exact names:
- `archive Phase F virtual formal wire alias source peer rejection`
- `archive Phase F virtual descriptor indexed streams lifetime`
- `archive Phase F virtual STA gesture cancellation responsiveness`
- `archive Phase F virtual actual worker delayed indexed contents`
- `archive Phase F virtual slow worker cancel source close disconnect`

All Cargo commands inherited jobs1/incremental0/RUSTFLAGS codegen-units256 and checked/waited for the
shared lane. No unrelated process was killed. The initial background wrapper disappeared before its
first command produced output; its empty attempt logs remain in the parent evidence directory and
count as no checks. A fresh `run2` foreground attempt did actual verification. One full-suite attempt
was refused BEFORE launch while another Cargo owner was active; after waiting it ran once only.
No green-hunting repeat, old Ole32 pre-cancel fixture, archive codec/password/conflict cross-product,
100k/large-file/performance/volume/edit matrix, or installed-product startup was run.

Task validation passed with existing context-size warnings, including implement/checkpoint exceeding
32 KiB injection limits. Source gates do not prove this connection-origin contract, native behavior,
actual provider accessibility or visual acceptance. Full green tests do not close F-P208-AC01.

## Preservation and remaining scope

Before this report/checkpoint writes, **2034/2034 baseline files unchanged**, none missing, including
historical archive reports, installer/package scripts, Cargo manifests/lock, vendor and Generated.
**84/84 sequence25 artifacts unchanged**. Installer SHA256 remains
`1CF5FCCC2ACD62B96A6128018642A87C2D21DA07BED9BC206355E21C663D07E5`.
This acceptance changes only this additive report, the unique checkpoint and scratch evidence.
Sequence25 still does not contain the new row-hookup source. Payload duplication is deferred to the
next separately authorized packaging task; no packaging/sign/install/service/bootstrap change.

Actual GPUI HWND/capture release, physical button continuity, real external DoDragDrop/foreign-process
COM marshaling/Explorer receive-cancel, DPI/UIA/user visuals remain **NOT RUN**. No security-denial
bypass, Registry/COM registration, source recycle, dependency/vendor/backend change, commit or push.
Directories, unknown sizes and password interaction remain full-P208 capability debts; prior A–E /
JOB07/SAFE08/native/user debts are unchanged. Stop at **F-P208-AC01 pending bounded repair**, not a
new phase and not permission to package, install, register or advance optional P209.
