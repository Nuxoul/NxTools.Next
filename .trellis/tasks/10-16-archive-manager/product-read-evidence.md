# Phase A — product read-only archive integration evidence

Date:2026-10-19. Canonical **Phase A — Engine contracts and safety foundation**.
Status: historical bounded slice verification below; no Phase A, B, UI, FIFO/write/recycle/full
A–F or release acceptance claim. Subsequent independent P2 findings remain in
product-read-acceptance.md; cancellation/epoch repair and fresh verification are in
[P2 remediation](product-read-p2-remediation.md). Original results stay historical, not
post-repair execution evidence.

## Scope and preservation

Contracts: [real chain/responsibility](product-read-contract.md), [exact wire](product-read-wire.md),
[C# directed evidence](product-read-csharp-evidence.md), [owner](../../spec/nxtools-archive/backend/product-read.md).
Core settings/storage untouched; no user configuration. No native source-share weakening,
source recheck-close->Shell TOCTOU, recycle backend or original P1/P2 modification. Original
source-deletion-remediation-acceptance.md SHA256 before/after:
47089376D106F59C267021157C630D577EB3729A4F69DF387D05DD966C3CC6EB.
All unrelated preexisting dirty changes retained. This session's baseline status/diff/scoped
hashes live in scratch archive-product-baseline. No reset/cleanup/upgrade/third-party edits.

## Actual vertical red → green

All commands use the real public product owner/typed pipe/shared C# session/original runner/
Job/actual Rust worker, except explicitly named consumer/router negative injections.

1. Initial open tracer: independent Python stored ZIP literal alpha.txt,目录/,目录/你好.txt;
   product success assertion actual RED0pass/1fail/0ignored exit101 MissingCapability at shared
   rejecting C# stub. archive-product-open-red.log. Red-ready marker read before real C# wiring.
2. First integration:2pass/2fail exit101; paused consumer and metadata budget already passed;
   normal open/test failed ProtocolViolation because private completion counters were required
   before Rust emitted them. C# verifies existing open files_verified and optional counters;
   Rust now emits independent MetadataCredit confirmed entry/batch counts for opt-in only.
   SAME original assertion green; later expanded6/6passed,0ignored,exit0. A transient Rust
   insertion compile error was corrected; it is not a behavioral red or acceptance.
3. Overflow recovery real bounded IPC consumer test RED0pass/1fail,8old queued events versus0;
   finite old-event+wake drain fix ->5consumer tests green. No native output ack used as proof.
4. Missing source real chain RED0pass/1fail,IoFailure instead of SourceUnavailable. C# now retains
   source_acquire failure/native code for opt-in and native-boundary classification outranks
   later generic worker IoFailure. SAME test green1/1,0ignored;source Win32=2|3 and cleanup true.

Current directed chain6+source1 covers literal names/types/sizes/UTF8 provenance/same snapshot+
original0..2ordinals, test2files/12bytes, mutated stored payload CorruptArchive,500ms slow
consumer refusal, exact cancel wake/idempotent cancel,1byte retained-budget reject, actual
isolated-host disconnect/Job death and consumer destruction/source write-share release.
Consumer5 covers exact move/ACK and terminal-once/stale/retired/overflow budget;IPC25 includes
original-connection enqueue rejection with no pending/queued command. Protocol50passed.
Strict Clippy Archive/Protocol/IPC isolated+tests passed; final rerun recorded below.
C#7new fixtures and4original runner fixtures pass as recorded separately; directed counts
are not added again to final totals. No ZIP64 was executed in local loops.

## Native/deployment boundary

Development resolver is DEBUG-only exact current PlatformHost source-output layout -> fixed
repository target/debug/nxtools-archive-worker.exe. Deployment resolver is fixed sibling
NxTools.ArchiveWorker.exe. Held no-reparse/read-share image lease protects selected helper
through native cleanup. No environment/PATH/current-directory/request/network selection.
Isolation package selection only exists behind guarded test apphost/GUID-owned constructor.
Package/release owner must map Cargo name, feature set and signed deployment/protected update
manifest together. Existing protected release manifest/installer do not yet carry the worker;
this slice does NOT update/install/sign/launch real product or claim deployed availability.
Actual production Debug location is source-checked; isolated product-call seam executes shared
production dispatcher with actual worker.100k/million-entry/RSS/first-paint/performance/longpath/
UNC/broad codec resources remain unmeasured/unaccepted.

## Final verification

Actual serial final ledger/logs in session scratch archive-product-final-ledger.txt and
archive-product-final-*.log. Initial final pass stopped at fmt because the last added source
regression was unformatted; corrected,fmt exit0. Workspace check then exposed two exhaustive
Desktop PlatformEvent matches; minimal ArchiveRead ignored arms added to existing unrelated-event
branches. No UI/interaction change; Desktop owner specs read,strict lint/full806tests passed.
These retained interruptions are not green final wrappers. No surviving build was duplicated.

| Gate | Actual result |
| --- | --- |
| Archive full isolated-codecs,offline,no-fail-fast,serial after final C# repair |124pass/0fail/0ignored,23nonemptygroups,exit0;2emptygroups not counted |
| Protocol --lib / IPC --lib |50/50 and25/25,0fail/ignored,exit0 |
| Desktop full serial tests (two exhaustive-event compatibility arms) |806pass/0fail/0ignored,exit0 |
| Strict Clippy tests Archive/Protocol/IPC default+isolated,Desktop |all exit0 |
| fmt --all --check / workspace check --tests --offline |exit0;workspace CHECK is NOT workspace tests |
| Extensions archive_import / clipboard --lib archive_ |7/7 and5/5,0fail/ignored,clipboard39filtered,exit0 |
| Same-feature worker / C#product+test build |exit0;C#0warnings/0errors |
| Original119+new7 precise LIST then SAME execution |LIST126/RUN0;selected126/run126/pass126/skip0/fail0/not_run0,exit0 |
| LIST/RUN/PASS identity comparison |126/126/126,0differences;original119 unchanged |
| Actual4GiB+17ZIP64 |RUN1/PASS1;only final126thisslice;realJob/nativepublication+independentPythonstreamingSHA256 |

Final C# actual2026-10-19T15:11:51.236->15:22:21.420(+08:00). Frozen-source Archive final
15:09:32->15:10:33;fmt/workspace/worker15:10:34->15:10:41. Earlier related gates recorded in
same ledger. Every Cargo command inherited jobs1/incremental0/RUSTFLAGS codegen-units256 and
waited for whole-machine lane; external wallhaven builds were never killed. The C# preflight
first stopped BEFORE tests on PowerShell ConvertFrom-Json nested-array count; flattened exact
names,LIST126verified before actual RUN. One-shot archive-product-final-zip64-attempt marker
prevents repeat. No large local loop ran. C#scenario operations are not extra harness cases.

One optional read-only reviewer was stopped after failing to converge; no report/no acceptance,
no second review. Parent direct inspection added real crosssession-admission regression to
existing C#fixture:two original admitted runner Jobs + secondsession third request REDselected1/
run1/pass0/fail1. Original runner processwide ProductReads reservation roots actual Run/helper/
source leases until real cleanup; same regression GREEN1/1,then7/7,then final126/126. A stopped
or cleanup-stuck old session cannot reset product admission. No competing process runtime.

Current chain7+consumer5 are INCLUDED in124,not added to totals. Original P1/P2 Rust9 and C#
source-session replay protections retained in full suites. Original independent reports not
rewritten. Pending100k/million-entry/RSS/performance/native/release remain unchanged.

Python3.13.2 explicit path SHA2567d96a4ed35d6e596cd9dd8933feafc66349cc21f75be3b15c89fd336e50140c1;
existing local7-Zip26.00 SHA2564bcc005758c5e604c0aaa257b6097846a4a708e62a9c0c7471fa360fad863f1f
rechecked,not historical24.09;no download/install/PATH mutation. Both oracles fixture-only.

Postfinal0observedCargo/rustc/worker/testhost/7z,0fixed product/test helper copies. Two early
failed-setup scratch chain roots containing only owned helper were inspected then removed;
remaining product roots0. Final synthetic fixtures clean finally;logs/source/markers retained
outside project. Static gates/diff/preservation final results appended below.

## Final static gates and preservation

All requested final gates exit0: spec-contracts106documents/14owners; componentcatalog79pages/
79table-viewerrows/78locked; taskvalidate56implement/53check(no truncation);PlatformHost
fixtureselector sourcegatePASS. Gitdiff--checkexit0;preexisting unrelated CRLF normalization
warnings only. Scoped source/test whitespace/conflict/temporaryDEBUGchecksPASS. Reference
index32505bytes/checkpoint32070bytes stay below32768; moved metadata/reproduction navigation
is retained in task research,not deleted history. Baseline critical source lease/control/
source-delete owners and independent acceptance SHA256 all unchanged.

At final gate inspection two external Cargo commands appeared(clippy--workspace--all-targets,
check--workspace--all-targets); no ownership was established and nothing was stopped. Reinspection
found0Cargo/rustc/archiveworker/testhost/7z. This task launched no command after final worker/C#
batch. Source/static checks do not replace native/release/user acceptance. Cfree25.9GiB/D42.3GiB.
No package/install/signing/commit/push/userarchives/realrecycle/permanentsourcedelete/registry/
association/service/drive-mapping changes. Stop at this bounded Phase A slice.
Latest resource check inspected31archive fixture prefixes,0remaining; no task worker/testhost/
fixedhelper. External wh-desktop Release Cargo/rustc under D:\Code\wallhaven remained unrelated
and untouched. Final context manifests include this evidence and C#directed evidence56/53.
