# Product read P2 backpressure remediation

Date:2026-10-19. Canonical **Phase A — Engine contracts and safety foundation**.
Both bounded P2 repairs verified; full Phase A remains in progress/not passed.
The independent product-read-acceptance.md remains immutable. P2-1 was independently reproduced
through the production bounded public router. P2-2 was source-confirmed only before this repair;
this round added and executed its first deterministic recovery-window red.

## Change boundary before implementation

P2-1: the original connection router, not a destroyed product receiver, must own cancellation.
Reserve at most two read lifecycle records on start admission (matching existing product max2).
Each record binds operation/session/generation AND original connection, remembers start dequeue,
and coalesces one cancel envelope. States: admitted/start queued -> start dequeued -> cancel
retained -> cancel dequeued; only matching actual cleanup terminal or original connection
retirement releases the lifecycle reservation. Local failure/Cancelled with cleanup_complete=false
cannot release it. Cancellation does not compete for ordinary command queue or receipt capacity.
The original command consumer services bounded cancellation and ordinary commands fairly, and
cannot send cancellation ahead of its own start. A single coalesced wake is sufficient; no worker,
retry/poll loop, blocking Drop, ordinary queue expansion or replacement-generation replay.

Expected product files: IPC lib.rs connection command consumer/router; IPC archive_read.rs
read reservations/cancel routing; Archive product.rs only if needed to route all existing cancel
paths through the same retained owner. Tests at the already requested public owner/router seam,
then actual typed pipe/shared C# session/original runner/Job/worker. Fixture lifecycle guards
must exist before fallible initialization and cover owned host/helper/directory cleanup.

P2-2 (after P2-1 red/green): preserve event8/wake1. One short synchronous epoch boundary must
serialize emit with overflow observation, old queue+wake drain and reset. No lock across await.
Both fast-path and wake-path use that same boundary. A producer inside recovery belongs to the
old epoch; genuine post-boundary overflow must still report. Test-only deterministic ordering
hooks/barriers add no product wire fields/authority.

Preserve read-only validation, exact business ACK, cumulative metadata budgets, unique terminal,
source leases and original Jobs/process-wide reservations, trusted fixed helper, source-delete
P1/P2 and native_protection_unproven/NotAttempted. No codec/output/share/recycle contract changes,
UI/FIFO/deploy/package/install/sign/commit/push or user archives/native system mutation.

## Actual directed execution evidence

All direct Cargo runs use jobs1/incremental0/RUSTFLAGS codegen-units256; no Cargo/rustc occupied
the lane before their start. Logs are in this session scratch, prefix p2-; no ZIP64 local loop.

- P2-1 original production owner/router regression, before product edits:
  `cargo test -p nxtools-archive --features isolated-codecs --offline --test product_read_cancellation -- --test-threads=1`.
  p2-1-drop-red.log:0pass/2fail/0ignored,exit101. Both operation and client Drop drained the
  filler then lacked a cancellation command. Same tests green2/2 after retained router owner.
  Extended public tests green10/10: duplicate100x coalescing/start ordering/fairness/two cleanup-
  bound slots/old connection/metadata/protocol/receipt/overflow failures/receipt pressure/wake.
- P2-2 first deterministic runtime red THIS ROUND, not retroactive independent evidence:
  `cargo test -p nxtools-ipc --lib --offline overflow_epoch -- --test-threads=1`.
  p2-2-epoch-red.log:0pass/1fail/0ignored,exit101. Test hook calls actual emit between original
  swap(false) and drain, then fresh notification is incorrectly replaced by stale Overflow.
  Same assertion green after synchronized boundary. Additional waiting select and cross-thread
  locked-drain barrier tests confirm new notification delivery and genuinely later overflow.
  IPC full directed28/28,exit0; no product wire/test permission fields or queue expansion.
- Actual typed pipe/shared production C# session/original runner/Job/actual worker chain10/10,
  p2-directed-chain.log,exit0. New pressure cases first reach real source-held metadata-credit
  barrier, pause SAME production command dequeue, fill existing32slots, verify next send Busy,
  drop operation (client stays live/unpolled) or client, release guard without retry/polling.
  Test-only host records original production terminal:Cancelled + cleanup_complete=true before
  10s credit deadline; original connection unchanged, actual source writable/helper removable.
  Thus proof is not timeout/EOF or direct runner.Cancel. Setup failure guard test covers owned
  helper directory; RAII guard exists before all fallible init and server/child are registered
  immediately. No unrelated processes or unknown fixture roots are cleanup targets.
- C# new captured-cancellation fixture LIST1/RUN0 then selected1/run1/pass1/skip0/fail0/not_run0,
  build0warnings/0errors. Uses production session Dispatch(cancel), not runner.Cancel; actual
  original source and helper leases release only after native cleanup. Final selection adds
  this fixture to exact original126,not a replacement subset.
- Isolated Archive/Protocol/IPC strict Clippy including tests directed exit0. Static/source
  checks and actual counts above do not certify native deployment/UI/full Phase A acceptance.

Original independent report SHA2563AF6051903A868226D84F2AB1C82B48374B0DBFEEECAF34944F5D7A6DEAFDF59;
source-delete acceptance SHA25647089376D106F59C267021157C630D577EB3729A4F69DF387D05DD966C3CC6EB.
Legacy tracked-source grep/package/FFmpeg entry files and Archiver core source rechecked before
product edits: no old general archive-window/worker IPC lifecycle identified; no UI invariants
introduced. Existing Archiver streaming/Vec interface is not copied as a transport framework.

## One bounded review: concrete saturation order repaired

One read-only review identified a real omitted order: ordinary notification receipt queue is
already full before Drop; handle_connection awaiting that delivery cannot poll its cancel writer.
Parent deterministic IPC test actual red0/1exit101; actual typed chain red0/1exit101 at5s
host/lease cleanup deadline(p2-review-{notification,chain}-red.log). This is recorded separately
from the original P2-1 and P2-2 evidence. No second review; independent exploration was cancelled
without report and does not count as a passed review.

Correction: ordinary notification delivery on the SAME pipe owner is nonblocking. Full/closed
receiver returns fixed failure; existing handle_connection retires original router generation,
its disconnect notification cannot await the full queue, then drops both real pipe halves.
Production C# EOF stops/drains its original shared session/runner Jobs/source/helper leases.
Impact is explicitly connection-wide: all pending commands/read runs on THAT PlatformHost pipe
are interrupted; no unrelated process/session is killed. This fail-closed fallback does not
pretend a local Cancelled/Disconnected proves cleanup or replay old intent to reconnect.
Actual chain first fills notification32, sends an ordinary command, observes its production
complete_command boundary BEFORE Drop, never drains/retries, then proves owned C# host exits,
original source writable/helper removable before10s credit timeout. Same assertion green1/1;
full IPC29/29 and product chain11/11 green. Normal command-pressure Drop tests still deliver
Cancelled+cleanup true on SAME live original connection, not via fallback.

## Final integration — exact execution and limitations

All gates foreground/serial; full-operation timeouts, no timeout/interruption/surviving build.
Scripts p2-verify.ps1,p2-review-retest.ps1; exact timestamps/commands/exit codes in
p2-final-ledger.txt; complete logs p2-final-* and p2-review-final-* in session scratch.
Initial integrated Rust16:42:24->16:44:28; C#12716:45:59->16:56:18(+08:00),exit0.
Postreview affected Rust17:02->17:04:24; same-worker C#126(no large)17:05:07->17:07:58,exit0.

| Gate | Actual result this repair |
| --- | --- |
| Archive full isolated-codecs,offline,no-fail-fast,serial AFTER final fix |138pass/0fail/0ignored,26resultgroups(24nonempty),exit0 |
| Product read cancellation / consumer / real chain included above |10/10,5/5,11/11; not added again to138 |
| Protocol --lib |50pass/0fail/0ignored,exit0; protocol source unchanged after this run |
| IPC full --lib AFTER final fix |29pass/0fail/0ignored,exit0 |
| Archive/Protocol/IPC strict Clippy incl tests default+isolated |all exit0 AFTER final fix |
| fmt --all --check / workspace check --tests --offline |exit0 AFTER final fix; check is NOT workspace tests |
| Extensions --test archive_import / clipboard --lib archive_ |7/7,5/5(39filtered),0fail/ignored,exit0 AFTER final fix |
| Actual isolated worker build / C# product+test build |exit0; C#0warnings/0errors; worker rebuilt before both final C# collections |
| Exact original126 plus new fixture first LIST then SAME RUN |LIST127/RUN0;selected127/run127/pass127/skip0/fail0/not_run0,exit0 |
| Exact LIST/RUN/PASS identities/order |127/127/127,0differences; original126retained |
| Actual4GiB+17ZIP64 |RUN1/PASS1,once only in final127; unchanged codec/output/source implementation |
| AFTER IPC-only review fix, current same-feature worker C# collection without large case |firstLIST126/RUN0;selected126/run126/pass126/skip0/fail0/not_run0,exit0; exact set=final127 minus only largeZIP64 |

C#127preceded final IPC-only fallback correction; that correction changes no C#/codec/output/
source implementation. All affected real typed pipe/native worker cases and final126rerun on
rebuilt worker pass afterwards. LargeZIP64 intentionally NOT rerun, preserving once-only bound.
Do not call C#127 a fresh postreview127run. Pre-review Archive137/IPC28 remain retained history,
superseded by138/29. No protocol enums or Desktop files changed this repair; Desktop806 and
DesktopstrictClippy were NOT rerun (IPC-internal owner repair); historical806is not fresh evidence.

Retained recoverable tooling issues: one inline PowerShell quote parse error; C# preflight
JSON-array nesting rejected BEFORE LIST/RUN/ZIP64; fixed flatten then127executed once; post-run
identity script initially assumedUTF8/logs areUTF16 and wrong4GiB-name expression, fixed against
existing logs only, identity verification passed without rerunning fixtures. All behavioral
reds above remain failures in their logs,not relabeled green. No failed/interrupted product
suite is hidden; postreview concrete red/fix/retest listed explicitly.

Python3.13.2 SHA2567D96A4ED35D6E596CD9DD8933FEAFC66349CC21F75BE3B15C89FD336E50140C1;
existing7-Zip26.00 SHA2564BCC005758C5E604C0AAA257B6097846A4A708E62A9C0C7471FA360FAD863F1F.
No download/install/PATH change. Scratch helper-only residue from early directed chain was
inspected (exact1helper/no source files/no live child),removed only that owned GUID root.
Fixture Drop now bounded3s image-release cleanup and explicit failure; repeated complete chain
and final full archive left0chain roots. Init guard precedes package/ZIP/host and server/child
registration is immediate. Final process/pipe/helper/static gate checks recorded below.

Native recycle stays native_protection_unproven/NotAttempted. No UI/FIFO/write functionality,
real product startup/package/install/sign/commit/push,user archive/recycle/permanent source
delete,registry/association/service/drive mapping. Broad100k/million/RSS/performance/UNC/longpath/
deployment/native-user acceptance remain unperformed. Stop this repair; no next phase/fullA/A-F.

## Final static and preservation gates

All four requested final gates exit0: spec-contracts106documents/14owners; GPUIcatalog79unique
pages/79rows/78locked; taskvalidate58implement/55check(no truncation); PlatformHost selection
sourcegatePASS. These are documentation/source checks,not UI/native/release acceptance.
Git diff --check exit0 (existing unrelated CRLF normalization warnings only). Scoped Rust/C#
whitespace/conflict/temporaryDEBUGchecksPASS; original Archive product.rs and existing consumer
regression byte-identical to pre-repair snapshot. No dependency/lock/protocol/Desktop/C#production/
source-delete/codec/output implementation edits. All initial dirty paths retained,no reset.
Final C#127exact names/order and postreview126exact set/order both independently reconciled
from existing logs;selected/run/pass/skip/fail/not_run summaries consistent.

Postfinal process observation0Cargo/rustc/archiveworker/testhost/7z; original installed desktop/
PlatformHost/indexer and unrelated Unitydotnet untouched.0observed nxtools-archive-product test
pipes,0scratch chain roots,0observed TEMP archive-/nxtools-archive- roots. Both fixed test/product
Debug helper copies absent. No further owned cleanup needed; unknown/historical roots not swept.
Early helper-only residue/manual cleanup and fixture bounded-cleanup fix remain recorded above,
not falsely described as automatically clean from the beginning. Scratch logs/scripts/markers/
baseline/scoped diff retained. Observed freeC27,720,343,552bytes/D45,070,077,952bytes.
Final report hashes match pre-repair baseline for both independent acceptance files. No genuine
blocker remains for these two bounded defects; native/release/full-phase limits above remain.
