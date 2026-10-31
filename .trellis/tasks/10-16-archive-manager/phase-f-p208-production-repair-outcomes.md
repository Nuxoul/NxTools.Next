# Phase F P208 — F-P208-AC01 repair outcomes

Date2026-10-23. **F-P208-AC01 repair complete, pending independent retest.** Same canonical
**Phase F — Explorer integration and genuine native drag**, no new phase or spike. This is an
implementation repair, NOT independent closure/fullP208/PhaseF/first-release PASS. User authorized
only the confirmed defect in [independent acceptance](phase-f-p208-production-acceptance.md).
Authority/boundary recorded in the sole [checkpoint](phase-a-checkpoint.md) before code; current
[production contract](phase-f-p208-production-contract.md) adds the origin-connection invariant.
Historical acceptance, production/spike outcomes and failure logs remain unchanged.

## Root cause and minimal interface change

Browser/source/view generation and IPC connection generation are DIFFERENT identities. Shared
PlatformBridge becomes connected on replacement before queued old Disconnected reaches the view.
Previously the original source had no retained IPC origin and the router stamped the new connection
onto its start. Later old view retirement then lost local tracking of that new operation.

Existing read admission ALREADY captured `connection` and atomically compared it in
`PlatformBridge::send_archive_read` under CommandRouter. Repair reuses this exact value:

```text
ArchiveReadClient::start captures connection -> original send_archive_read locked admission
 -> ArchiveReadOperation { connection } / connection_generation() immutable getter
 -> PendingRead.operation retains original identity through native read completion
 -> ArchiveApplication::consume captures operation.connection_generation BEFORE cx.defer
 -> deferred install -> configure_drag(platform,browser,ORIGINALconnection,depth,MOTW)
 -> formal held-threshold row submits send_archive_virtual_drag(command,receipt,ORIGINALconnection)
 -> SAME router lock: active.generation == origin, BEFORE pending/reservation/enqueue
```

- `nxtools-archive/src/product.rs`: one immutable operation field/read-only getter initialized from
  original successful read admission. Survives removal of read Pending and replacement of shared bridge.
- `archive/application.rs`: two lines carry operation origin across the existing deferred install.
  No configure/move sampling of the current connection, no earlier notification delivery workaround.
- `archive/browser_drag.rs`: source state retains origin; Pending also freezes it for later cancel
  even if a newer hydration overwrites source state. Start/cancel call the narrow original bridge API.
  Local pending still precedes observable send and is cleared on refusal. Same official Alert/fixed
  readable failure, HWND resolver/4DIP/still-pressed/once-only/click/selection/geometry/focus unchanged.
- `nxtools-ipc/src/archive_virtual_drag.rs`: public
  `send_archive_virtual_drag(command,command_id,origin_connection)` builds the SAME formal envelope
  and preserves operation correlation. Existing private send checks origin under the original router
  lock before ANY pending or reservation mutation. Mismatch -> Disconnected, no queue/pending/slot leak.
- `nxtools-ipc/src/lib.rs`: generic routing passes no origin. An unbound FORMAL start fails closed
  instead of bypassing the new boundary; historical isolated `ArchiveVirtualDragSpike` remains
  compatible. No wire schema/version/capability, C#, native or materialization-chain changes.

Original one bounded start/exact-cancel slot, start-before-cancel, fair ordinary/cancel turns,
full-queue retained cancel, cancel-ACK-not-cleanup, cleanup_incomplete occupancy, original-connection
retirement and ordinary10second timeout policy remain. Formal old-origin cancel cannot reach a new
reservation even if operation/session/view-generation identities match. No retry tasks/new transport/
second controller/OLE framework, cross-entity synchronous updates or feature/budget expansion.
Diagnostics reuse existing submitted=false/fixed local refusal and original operation correlation;
no new pointer/tick/secret/payload logging or temporary DEBUG probes.

## Red → green at formal application/row and original bridge

Evidence root:
`C:\Users\Administrator\.pi-desktop\scratch\7c2ebec3-f1a7-4623-9ed1-760f9adb94f5\phase-f-p208-ac01-20261023-170650`.
`commands.jsonl`, per-command full logs, lane observations and full-source hashes retain all attempts.

**01-origin-row-red.log** before implementation: actual completed original read -> application deferred
install -> official production ListItem Down/Move -> same production receiver. **RUN2/PASS0/FAIL2,
exit101**, both fail specifically `retired source MUST NOT enqueue start on replacement connection`:
1. Read completion consumed but connection replaced INSIDE the application callback before deferred
   install runs. The old origin must not become the replacement generation during configure.
2. Original view installed and row Down prepared; connection replaced before threshold Move/enqueue.
   A connected shared bridge and current view generation cannot authorize that source.
In BOTH cases old ordinary Disconnected stays queued during the gesture/assertion. Test pauses only
application drain to make the first gap deterministic; real original client/transport/consume/deferred
install and official row callbacks stay. Test HWND311 is the existing cfg(test) dependency only.

**03-origin-row-green.log retains a fixture-only failure** after the product refusal was fixed: both
passed the old-start empty-receiver assertion, then failed an overstrong empty old-disconnect-receipts
assertion. This controlled read omitted ordinary receipts for original reads. Corrected test verifies
original notification generation and its old IDs no longer pending; no production change for that
fixture. No failure log overwritten or relabelled PASS. **06-origin-row-green.log PASS2/FAIL0**.
Both tests then deliver old Disconnected through the original deferred owner, REFRESH/RE-READ on the
new connection and assert exact fresh browser context starts immediately before release, matching
pending receipt, Down alone sends nothing and repeated held Move sends no second start.

Original router regression checks stale-origin refusal leaves `router.pending` empty AND
`active.archive_drag` absent, generic unbound formal start refused, SAME rejected receipt reusable on
fresh origin, fresh start exactly once, old-origin matching cancel rejected. Existing queue-pressure/
ACK/incomplete/cleanup/ordinary-timeout tests remain. Public read consumer regression verifies operation
origin before and AFTER terminal removes read Pending and a new connection replaces the shared bridge.

## Original independent probe reused honestly

Original probe at
`C:\Users\Administrator\.pi-desktop\scratch\ceebe351-d540-4ca3-a460-dadc71f3a7df\p208-connection-probe`
and historical `run2/14-connection-probe.log` are unchanged. That original exit0 asserted the WRONG
admission and is NOT reused as a passing result. This round creates `connection-probe` inside its own
evidence root, same command/source/old queued-notification scenario, now correct refusal assertions:

```text
old_view_start_send_ok=false
new_connection_received_old_start=false
fresh_origin_start_send_ok=true
queued_notification=Disconnected { generation: 1, pending_command_ids: [] }
```

`17-corrected-connection-probe.log` exit0 also checks no rejected pending, unbound formal start refused,
same receipt/fresh source readmission and no second envelope. It intentionally does NOT inspect/fix
Hello accepted capabilities, a nonblocking observation explicitly excluded by user.

## Actual executed verification

Every Cargo command checks/waits for the shared Cargo/rustc lane, never kills another owner; jobs1,
incremental0, `RUSTFLAGS=-C codegen-units=256`. No Release build/package. Exact invocations and elapsed
wall times are in commands.jsonl; all test commands serial `-- --test-threads=1` with `--jobs 1`.

| Command / log | Actual result |
| --- | --- |
| `cargo test -p nxtools-desktop --bin nxtools-desktop production_row_drag` /09 | **10PASS**,0FAIL; overlaps browser/application collections, not10 disjoint extra tests |
| Same binary filter `archive::browser::` /10 | **26PASS**,0FAIL |
| Same binary filter `archive::application::tests::` /11 | **12PASS**,0FAIL, including BOTH new gaps/fresh rereads |
| `cargo test -p nxtools-ipc --lib` /07 | **40PASS**,0FAIL, no ignored/filtered |
| `cargo test -p nxtools-archive --test product_read_consumer --test product_read_cancellation` /12 | **7+10PASS**,0FAIL, original-origin/completion/control boundaries; no codecs/native worker |
| `cargo test -p nxtools-protocol --lib` /13 | **69PASS**,0FAIL, unchanged wire compatible |
| `cargo clippy -p nxtools-desktop -p nxtools-ipc -p nxtools-protocol --tests -- -D warnings` /14 | PASS,exit0 |
| `cargo clippy -p nxtools-archive --lib --test product_read_consumer --test product_read_cancellation -- -D warnings` /15 | PASS,exit0 |
| `cargo check --workspace --tests` /16 | PASS,exit0, compilation NOT all workspace test execution |
| Corrected public probe, offline manifest in scratch, shared workspace target /17 | PASS,exit0, output above |
| Scoped rustfmt affected8Rustfiles /18 | PASS, no global/vendor format |
| ONE full Desktop after convergence /19 | **1044PASS/0FAIL/0ignored/0filtered**,128.74s,exit0. **684/684** Rust/C# source/project hashes unchanged during run. No repeat. |
| Source/document gates | Phase F row/origin/reserved-cancel/oneSTA/content ownership, Shell gate, GPUI catalog/release/editor provenance, spec109documents/14owners, task validation and scoped diff; source gates NOT native acceptance |

### Historical evidence explicitly NOT re-executed

C#/native/content files unchanged. Per user no new host/worker build, no repeated C#5fixture set, no
native Ole32 fixture or real content-chain rerun. Existing independently executed evidence is
[production acceptance](phase-f-p208-production-acceptance.md): host-first builds0warnings/errors,
LIST5=RUN5=PASS5, skip/fail/not_run0, matching worker SHA
`5996E3AE615A1233BAD709983D1D79FEDB57446C65943977264293470A4CD16A` stable before/after. Its descriptor/
indexed-content/stream/worker/cancel/actualcleanup results remain HISTORICAL, not this round's counts.
No codec/password/conflict cross-matrix,100k/large-file/performance tests or unrelated fixes/reviews.

## Protection and stop

Per-round opening baseline **3006** files, including preexisting dirty/untracked work, not cumulative
HEAD diff. `final-preservation.json` identifies only owned changes/new repair report and any concurrent
changes; no baseline deletions. **62/62 historical archive outcomes/acceptance reports unchanged**.
Final audit: **2987 unchanged +14 owned-changed +5 concurrent-document-changed =3006**, missing0;
new owned1 (this report). Concurrent floating-assistant task/reference documents and3new task docs
were preserved, never edited by this repair. Source hashes remain unchanged AFTER the full suite too.
Task validation passed82implement/79check entries with existing32KiBcontext-injection size warnings
for native-contracts/archive reference/implement/checkpoint; no unrelated document refactor.
Preserved reports include independent NOTPASS and implementation/spike evidence; original probe hashes unchanged.
**84/84 sequence25 artifact files unchanged**, no extras. Installer SHA256 remains
`1CF5FCCC2ACD62B96A6128018642A87C2D21DA07BED9BC206355E21C663D07E5`.
C#/native, Protocol source, manifests/lock/dependencies/vendor, installer and scripts unchanged.
Existing sequence25 binaries do NOT contain this repair. Installer resource duplication stays deferred
until separately authorized packaging. ProductApplyAuthorized=false; no production trust changes.

Current scope stays1..32known-sizeunencryptedregularfiles,total512MiB. Directory/unknown-size/password
interaction and prior A–E/JOB07/SAFE08/native/user obligations are NOT closed by this fix. No real
product launch, GUI security bypass, Explorer/physical drop, Registry/COM registration, service/
bootstrap, package/sign/install, source recycle, dependency/vendor change, commit/push or broad review.
Actual HWND/capture/button continuity, physical gestures, external OLE/foreign-process COM/Explorer,
DPI/UIA/user visuals remain **NOT RUN / unaccepted**.

**Stop: F-P208-AC01 repair complete, pending independent retest.** Both required race gaps are formally
regression-covered; fresh new-connection reads start normally. Independent closure/fullP208/PhaseF/
first-release acceptance is NOT claimed; no further packaging or functionality is started.
