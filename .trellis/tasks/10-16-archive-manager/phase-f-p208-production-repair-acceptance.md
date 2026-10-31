# Phase F P208 — independent acceptance of F-P208-AC01 repair

Date: 2026-10-23. **F-P208-AC01 independently CLOSED at source/isolated boundaries.**
The bounded production-row source hookup is accepted after this repair. Not full P208, actual
Explorer/OLE, Phase F or first-release acceptance. Current status: [sole checkpoint](phase-a-checkpoint.md).
[Original negative acceptance](phase-f-p208-production-acceptance.md) and
[repair outcomes](phase-f-p208-production-repair-outcomes.md) remain unchanged.

## Source review and closure

Parent directly reviewed the read admission/operation, application completion/deferred install,
formal row start/cancel, original bridge/router and both formal regression paths. No additional
subagent review or broad review loop was opened. No new confirmed in-scope defect was found.

- `nxtools-archive/src/product.rs:112-164`: operation stores exactly the connection already used by
  successful original `send_archive_read` admission. Immutable getter at47-60 survives Pending removal;
  it does not sample the now-current shared connection on completion.
- `archive/application.rs:537-550`: obtains that operation origin BEFORE `cx.defer`, passes it unchanged
  into `configure_drag`. Browser generation and IPC connection generation remain distinct.
- `archive/browser_drag.rs:19-38,47-64,159-179,263-284`: retained source origin accompanies formal start;
  Pending freezes its admitted connection for later exact cancel, independent of newer hydration.
  Refused send clears local pending and shows the existing fixed error. Still-pressed/once-only,
  selection, HWND and deferred entity boundaries are unchanged.
- `nxtools-ipc/src/archive_virtual_drag.rs:48-91`: bound formal entry builds the same envelope and checks
  active connection origin UNDER the original router lock. Comparison precedes pending registration,
  queue admission and reserved-drag mutation. Generic unbound formal start fails closed; legacy
  isolated spike compatibility is unchanged. Existing start/cancel fairness, timeout and cleanup rules
  are retained rather than replaced.

**Both originally confirmed gaps independently pass:**
1. Original read completed/consumed, connection replaced inside the application callback before its
   deferred view installation. The installed source keeps the old origin, not the replacement's.
2. Original view installed and actual ListItem Down prepared, connection replaced before threshold Move.

`application_tests.rs:784-922` retains the actual client/transport/consume/deferred install and formal
row callbacks. Only the app drain is paused to place the first race deterministically; the synthetic
HWND is the existing cfg(test) seam. Both tests assert the old Disconnected remains queued BEFORE and
AFTER rejection; replacement receiver receives no old start. They then deliver that notification,
refresh/re-read on the new connection and prove immediate single fresh start with the exact fresh
browser context/pending receipt. This is not notification-order masking or a new test-only spike.

Original-router regression also checks ZERO pending and ZERO reserved drag after refusal, generic
unbound start rejection, reuse of the rejected receipt on a fresh origin and old-origin matching
cancel rejection. Read-consumer regression confirms origin survives successful terminal/Pending
retirement and replacement of the shared bridge.

Read the implementation's preserved `01-origin-row-red.log`: both named race tests failed with
`retired source MUST NOT enqueue start on replacement connection`,0PASS/2FAIL,exit101. This is
historical red evidence, not a fresh parent run against reverted code. Parent ran final code only.
The retained intermediate03 fixture failure was not counted as product failure: old read receipts
made its empty-disconnect-list expectation invalid; core old-start refusal had already passed.

## Independent verification

Evidence:
`C:\Users\Administrator\.pi-desktop\scratch\ceebe351-d540-4ca3-a460-dadc71f3a7df\p208-ac01-independent-20261023-180245`.
Runner: parent session scratch `p208-ac01-independent-verify.ps1`. Raw per-command logs,
`commands.jsonl`, `independent-results.json`, `full-meta.json` and preservation records retain results.

| Check | Independently executed result |
| --- | --- |
| Exact formal connection-origin race filter | **2PASS**,0FAIL,1042filtered;01-origin-races.log |
| Original browser actual-view collection | **26PASS**,0FAIL;02-browser.log |
| Original application collection | **12PASS**,0FAIL;03-application.log |
| Formal drag names in the preceding collections | **10 distinct PASS**, overlapping collections, not10 additional disjoint tests |
| IPC library | **40PASS**,0FAIL/ignored/filtered;04-ipc.log |
| Original read cancellation / consumer | **10PASS +7PASS**,0FAIL/ignored/filtered;05-read.log |
| Protocol library | **69PASS**,0FAIL/ignored/filtered;06-protocol.log |
| Desktop/IPC/Protocol strict Clippy --tests -- -D warnings | PASS,exit0;07-strict.log |
| Archive lib + two read integration targets strict Clippy | PASS,exit0;08-read-strict.log |
| Workspace --tests | Compilation PASS,exit0; not all workspace tests executed;09-workspace.log |
| Independent correct-refusal probe | PASS,exit0;10-origin-probe.log |
| ONE full Desktop | **1044PASS**,0FAIL/ignored/filtered,132.47s test time,exit0;11-desktop-full-once.log |
| Scoped rustfmt | PASS,8 affected Rust files,skip_children=true; no global/vendor reformat |
| Source/document gates | PASS: Phase F row/origin/reserved-cancel/STA/content ownership, Shell, GPUI catalog/release/editor provenance, specs, task and diff |

Scratch `p208-ac01-retest-probe` directly references current IPC/protocol source and reuses the original
frozen source/command/queued-old-notification replacement scenario, with correct refusal assertions.
It verifies bound old origin rejected, no pending after rejection, unbound formal start rejected,
SAME receipt accepted for fresh origin, exactly one fresh envelope, old matching cancel rejected and
old disconnect STILL queued. Output:
```text
PASS old_origin_rejected=true unbound_formal_rejected=true rejected_pending=false fresh_same_receipt_accepted=true old_cancel_rejected=true queued_old_disconnect=true
```
The original `p208-connection-probe` and old erroneous-admission logs were not edited or rerun as green.
The probe's fresh-origin case tests router admission, while the formal application regressions above
additionally prove the fresh context actually comes from a new original read.

All Cargo executions checked/waited for the shared lane and inherited jobs1/incremental0/
RUSTFLAGS=-C codegen-units=256. No duplicate build or unrelated process termination. One full Desktop
only; no native-worker/codec/password/conflict cross-product,100k/large-file/performance matrix.
Task validation passed with existing >32KiB context-injection warnings; no unrelated doc refactor.

## Integration drift / preservation

Full run monitored689 Rust/C# source/project files. Four concurrently changed C# files:
- `csharp/NxTools.PlatformHost/Windows/FloatingAssistantInputSurface.cs`
- `csharp/NxTools.PlatformHost.Tests/FloatingAssistantInputTests.cs`
- `csharp/NxTools.PlatformHost.Tests/Program.cs`
- `csharp/NxTools.PlatformHost.Tests/TestEffects.cs`

These belong to concurrent floating-assistant work, not the archive origin repair; no monitored
Rust source changed during the run. The1044PASS observation is retained, but is NOT a frozen entire
workspace certification. No full-suite rerun or unrelated fix was made.

Before acceptance/checkpoint writes, opening3098-file baseline had6 concurrent changes (the four
above plus FloatingAssistantDropTarget.cs and floating-assistant implement.md), missing0. Archive
repair/source/tests, historical archive reports, protocol, dependencies/vendor, installer/scripts
and original probe remained unchanged by this acceptance. **84/84 current win-x64/installer delivery
files unchanged**. Setup SHA256 remains
`1CF5FCCC2ACD62B96A6128018642A87C2D21DA07BED9BC206355E21C663D07E5`.
Only this additive acceptance report and the unique current checkpoint are edited; scratch evidence
is separate. Concurrent files are preserved. No product source fix, package/sign/install/service/
bootstrap/Registry/COM registration, commit or push.

## Historical vs unperformed native evidence / stop

C#/native/content chain was not modified by this repair. No host/worker rebuild or C# five-fixture
rerun here. Historical independent LIST5=RUN5=PASS5 and actual tiny-ZIP/content/stream/cancel evidence
remain explicitly attributed to the [prior acceptance](phase-f-p208-production-acceptance.md), not
added to this round's counts. Protocol was independently rerun for compatibility.

The bounded row source hookup is now accepted for1..32 known-size unencrypted regular files,
aggregate512MiB. Directory/tree, unknown-size and password interaction remain full-P208 capability
debts. Actual HWND capture/button continuity, physical gestures, real external DoDragDrop/foreign
COM/Explorer receive-cancel, DPI/UIA/user visuals remain **NOT RUN/unaccepted**. Existing GUI denial
was not bypassed; sequence25 does NOT contain these source changes. Installer payload duplication
remains deferred to the next explicitly authorized packaging task. Prior A–E/JOB07/SAFE08/native/user
obligations are unchanged. Stop: **F-P208-AC01 CLOSED, bounded source/isolated hookup accepted**;
no further cleanup/review loop required and no claim of fullP208/PhaseF/first-release completion.
