# Independent acceptance — product read P2 cancellation / overflow repair

Date:2026-10-19. Canonical **Phase A — Engine contracts and safety foundation**.
Scope: reliable original-connection cancellation, overflow recovery epoch and related ordinary
receipt saturation fallback. Original [independent findings](product-read-acceptance.md) are
unchanged; [implementation record](product-read-p2-remediation.md) preserves its own reds/history.

## Verdict

**PASS for both original P2 findings and the related saturated-notification correction.**
No concrete new in-scope blocker was found. This is bounded source/isolated cross-process
acceptance, not deployment/native-user/Phase A/A–F/release completion. Product/project-test
source was not changed during this acceptance; only this report is added. No real Desktop,
installed PlatformHost, native recycle, source deletion or user archive operation was run.

## P2-1 — cancellation survives Drop and full queues

Read actual router/command receiver/read reservation and unchanged product Drop callers.
- Original connection owns at most2reservations with exact operation/session/generation and
  start command identity. Cancel coalesces once independent of ordinary queue/receipt capacity.
- Same consumer marks start dequeued before it can send cancel, alternates cancellation with
  normal commands and wakes through finite capacity1signal. No destructor wait/unbounded task/
  retry loop or queue growth. Original generation replacement does not replay old intent.
- Local false-cleanup retirement does not release reservation; matching remote cleanup or
  original connection retirement does. Process/native cleanup remains the original C#owner.

Independent scratch probe uses public ArchiveReadClient and production platform_bridge(1),
project Cargo.lock copied outside workspace. It starts/read-dequeues original, fills normal queue
with an ordinary ActivateWindow command, verifies another admission returns Busy, then drops ONLY
operation or whole client WITHOUT explicit cancel first. It drains both filler and exact original
cancel and asserts no duplicates and same live connection. This adapts expected behavior rather
than keeping the old probe's now-invalid 'cancel must return Busy' assertion; original probe is
preserved unchanged as historical failing code.

Actual independent output:
```text
DROP_CLIENT=false QUEUE_FULL_CONFIRMED=true CANCEL_DELIVERED=true EXACT_ORIGINAL_IDENTITY=true FILLER_DELIVERED=true ORIGINAL_CONNECTION_LIVE=true
DROP_CLIENT=true QUEUE_FULL_CONFIRMED=true CANCEL_DELIVERED=true EXACT_ORIGINAL_IDENTITY=true FILLER_DELIVERED=true ORIGINAL_CONNECTION_LIVE=true
```
Probe: scratch product-read-drop-fixed-probe/{Cargo.toml,src/main.rs,probe.log}.
No worker/filesystem action in this router probe. Runtime native evidence comes separately from
actual product chain tests: controlled production dequeue pause + full32commands + Drop, no
consumer polling/retry, original live connection Cancelled/cleanuptrue, source writable and
helper removable before10s credit timeout. Both operation/client scenarios actually passed.

## P2-2 — emit and recovery share one epoch

Read archive_read.rs recover_overflow/recv/emit and all three deterministic regressions.
Overflow observation/swap, old event/wake drain and producer emit serialize on one short mutex;
no lock spans await. Both fast-path and waiting-select re-enter recovery. Concurrent producer
cannot cross locked drain; after unlock its new notification survives. A genuinely later full
queue still reports overflow. No trailing unsynchronized store or duplicate-error suppression.

This acceptance actually ran all3epoch tests:
- production emit scheduled in reset/drain recovery ordering;
- already-pending select wake path with same recovery;
- cross-thread barrier attempts producer while epoch lock held, then verifies new notification
  and later real overflow. These are deterministic tests, not probabilistic stress.
The historical independent P2-2 evidence was source-only. This report does not retroactively
claim a prior runtime red; implementer red is recorded separately in remediation evidence.

## Ordinary receipt saturation — actual original pipe EOF

Read lib.rs nonblocking ordinary send_notification/error propagation and handle_connection
retirement/return. A full notification receiver cannot block that pipe task from processing
retained cancels indefinitely: failure retires/closes the original connection, not local-only
Cancelled. This is explicitly connection-wide and interrupts its other pending commands.

Actual chain test first fills normal notifications, sends ordinary command and observes real
complete-command boundary before Drop. With no draining/retry, original owned C#host exits,
source lease and helper image release before credit timeout, bridge disconnects. It passed on
current final code. Normal command-pressure tests still stay connected and deliver Cancelled,
so EOF is not substituted for every cancellation. No unrelated process/session is terminated.

## Independently executed final integration

Foreground serial batch2026-10-19 17:19:05->17:29:55(+08:00), every step exit0/no interruption.
Scratch exact script/ledger/logs: product-read-p2-acceptance-verify.ps1,
product-read-p2-acceptance-ledger.txt,product-read-p2-acceptance-*.log.
Every Cargo inherited CARGO_BUILD_JOBS=1,CARGO_INCREMENTAL=0,RUSTFLAGS=-C codegen-units=256;
machine-wide Cargo/rustc inspected before each invocation. C#apphost built first; actual isolated
worker rebuilt after feature-unifying commands before final C#collection. No duplicate build.

| Gate | Actual independent result |
| --- | --- |
| Archive complete isolated-codecs,offline,no-fail-fast,serial |138pass/0fail/0ignored across26result groups(24nonempty) |
| cancellation / consumer / actual typed chain INCLUDED above |10/10,5/5,11/11 |
| Protocol --lib |50pass/0fail/0ignored |
| IPC --lib |29pass/0fail/0ignored,all3epoch tests passed |
| Archive/Protocol/IPC strict Clippy including tests,isolated/default |all exit0 |
| workspace fmt --all --check |exit0 |
| workspace check --tests --offline |exit0;NOT workspace tests |
| extensions --test archive_import |7pass/0fail/0ignored |
| clipboard --lib archive_ |5pass/0fail/0ignored |
| same-feature actual worker build |exit0 |
| C#product/test build |0warnings/0errors |
| exact original126+new1list |selected127/run0/pass0/skip0/fail0/not_run127 |
| SAME exact execution on FINAL source |selected127/run127/pass127/skip0/fail0/not_run0 |
| LIST/RUN/PASS names |0differences |
| actual4GiB+17FastDeflateZIP64 |RUN1/PASS1,once only THIS acceptance,on final code |
| independent full-command-queue Drop probe |bothcases original cancel+filler delivered,same connection |
| spec gate |PASS106documents/14owners |
| catalog gate |PASS79pages/79rows/78locked |
| task context |PASS58implement/55check |
| PlatformHost selector source gate |PASS |
| git diff --check |exit0;existing unrelated CRLF normalization warnings only |
| scoped production/test whitespace/conflict/DEBUG hygiene |PASS |

Unlike implementer historical127-before-finalIPCfix +126after, this acceptance reran the entire
127after all fixes, including one actualZIP64. No extra big fixture loop was run. Counts within
scenario loops are not extra harness tests. Desktop not changed in this repair; its previous806
and strict lint were NOT rerun here and are historical only. No full workspace/unrestrictedC#tests.
One bounded read-only reviewer inspected reservations/fairness/generation/pipe-close/epoch and
returned no concrete finding; no tests by reviewer. Parent did source review and all checks above.
No second broad review. Stock local7-Zip26.00 SHA256 matched
4bcc005758c5e604c0aaa257b6097846a4a708e62a9c0c7471fa360fad863f1f.
No download/install/PATH change; existing explicit Python/codec fixture configuration used.

## Preservation / cleanup / remaining limits

Original product-read-acceptance SHA256 unchanged:
3af6051903a868226d84f2ab1c82b48374b0dbfeeecaf34944f5d7a6deafdf59.
Prior source-delete remediation acceptance SHA256 unchanged:
47089376d106f59c267021157c630d577eb3729a4f69df387d05dd966c3cc6eb.
Unrelated dirty changes preserved; no product repair performed by acceptance.

Postprobe0Cargo/rustc/archiveworker/testhost/7z/probe processes;0archive test pipes;both fixed
product/test helper copies absent;0inspected TEMP archive roots and0scratch archive-product-chain
roots. No manual fixture cleanup was needed THIS acceptance. Scratch probe/scripts/logs/build
intermediates retained outside project. FreeC25.8GiB/D41.8GiB observed.

No actual native recycle/permanent source deletion,userarchives,UI/FIFO/write feature,real product
startup,packaging/install/sign/commit/push,registry/association/service/drive-map changes. Native
recycle remains native_protection_unproven/NotAttempted. Deployment/signed/protected worker mapping,
100k/million-entry timing/RSS/performance/longpaths/UNC/native-user/full PhaseA exits remain open.
Close these two P2s without claiming Phase A/B/A–F or release completion.
