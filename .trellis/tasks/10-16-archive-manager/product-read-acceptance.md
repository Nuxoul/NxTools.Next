# Independent acceptance — bounded product read-only chain

Date:2026-10-19. Canonical **Phase A — Engine contracts and safety foundation**.
Scope: headless product open/test, typed IPC, C# session/helper/original runner and bounded credit.
No product or project-test source was edited during acceptance; unrelated dirty state preserved.

## Verdict

**Not fully accepted: two P2 lifecycle/backpressure defects remain.** Normal real cross-process
open/test and existing regressions pass. P2-1 is independently reproduced at the real bounded
command router API; P2-2 is a source-confirmed interleaving, not a runtime reproduction.
Neither finding establishes native output corruption, privilege escalation or actual recycling.
Actual signed/protected deployment is explicitly unfinished, not counted as a new defect here.

## P2-1 — destruction loses cancellation when the shared command queue is full

Locations: `rust/crates/nxtools-archive/src/product.rs:76–79,262–271`; shared send/control at
`product.rs:300,329–331` and `rust/crates/nxtools-ipc/src/archive_read.rs:187–196`.
Operation Drop calls cancel once and ignores Busy. Client Drop likewise ignores send_control
failure, then removes/locally terminates the operation and destroys the sole receiver. There is
no reserved cancellation lane, retained cancellation intent, retry owner or connection teardown
fallback. The same best-effort pattern appears on local error/overflow retirement.

Independent scratch probe used public product owner and the SAME production bounded router with
platform_bridge(1), not a replacement cancellation implementation:
1. Connect original generation; start an open, consume its start command (models remote admission).
2. Fill shared command queue with a second bounded command on same connection.
3. Explicit cancel returns Busy, proving queue pressure.
4. Drop the operation, or separately drop the whole client.
5. Drain filler; no cancellation command is queued, connection is still live.

Actual output for both cases: DRAINED_FILLER=true,CANCEL_QUEUED=false,CONNECTION_STILL_LIVE=true.
The probe uses no worker/filesystem operation. Therefore it proves command cancellation is lost,
not an empirically measured worker cancellation latency. On a real outstanding test/open the
native work can continue until normal completion or timeout; existing unpressured Drop test does
not cover this path. Drop has no caller left to retry. Client Drop already reports local cancelled
with cleanup_complete=false; it does not falsely certify native cleanup, but fails the promised
consumer-destruction cancellation contract.

Required repair: bounded reliable cancellation intent/priority/control capacity or an equivalent
owner that survives Drop until cancellation is delivered or the ORIGINAL connection is retired.
Do not block a destructor/UI thread, spawn unbounded retries, expand normal queues, discard intent
on Busy or send old cancellation to a replacement generation. Cover both operation/client Drop,
local overflow/error retirement and generation/disconnect behavior. Add deterministic queue-full
regressions plus actual worker test showing source lease/Job release under command pressure.

## P2-2 — overflow reset and draining are not one atomic recovery boundary

Location: `rust/crates/nxtools-ipc/src/archive_read.rs:34–37`, emit at84–86.
The receiver clears overflow with swap(false), then independently drains events and wake.
Producer emit is unsynchronized with that whole recovery sequence. Concrete interleaving:
- existing overflow=true with full old queue;
- recv swaps it tofalse;
- producer attempts emit before draining frees a slot, fails, sets overflow=true and wake;
- recv drains old queue and wake, returns first Overflow but leaves new flag=true;
- consumer applies failure, starts fresh operation after recovery;
- next recv sees stale flag, returns second Overflow and cancels that fresh operation.
Existing recovery test injects all events BEFORE poll, so it cannot exercise this interval.

Evidence level: independently source-confirmed from actual atomic/drain ordering after one bounded
read-only review proposed it; no instrumented/real concurrent runtime reproduction was performed.
Do not describe it as an observed test failure. It is a concrete same-generation recovery race,
not speculation about arbitrary transport failures.

Required repair: define one race-safe overflow epoch/drain/reset boundary, or intentionally retire
the connection/consumer and disallow reuse until a fresh one. Merely adding a trailing store(false)
can lose a genuinely later overflow and is not sufficient without the corresponding ownership
argument. Add deterministic barrier/hook regression for overflow during reset/drain, then fresh
request admission; preserve finite8event/1wake budgets and cancellation/disconnect wakeup.
Do not rely on random stress/sleeps or suppress duplicate overflow without retaining real failures.

## Source assessment and evidence limits

Read product-read-contract/wire/evidence, Rust product/IPC routing and tests, C# ArchiveReadSession,
strict DTO parser, helper resolver, runner global ProductReads admission/cleanup, production pipe
forwarding and actual chain fixtures. One bounded read-only Rust reviewer, no edits/tests from it;
parent independently inspected findings and reproduced P2-1. No repeat broad review.
Helper uses fixed source-layout Debug mapping/deployed sibling; no command/env/PATH override.
Current positive tests use shared production dispatcher/session + original runner/Job/worker,
not runner-only fake metadata. Source/helper leases are retained through native cleanup and
process-wide read reservations retain original Runs across session replacement. Real signature/
installer/protected-copy worker inclusion remains explicitly unverified. Source-delete P1/P2
owner acceptance SHA256 stayed47089376D106F59C267021157C630D577EB3729A4F69DF387D05DD966C3CC6EB.

## Independently executed verification

Foreground serial batch15:36:06->15:48:11(+08:00), all steps exit0/no timeout/interruption.
Exact scratch script/ledger/log prefix: product-read-acceptance-. All Cargo inherited
CARGO_BUILD_JOBS=1,CARGO_INCREMENTAL=0,RUSTFLAGS=-C codegen-units=256; machine-wide lane checks.
C# was built BEFORE archive integration because actual Rust pipe tests launch the test apphost;
actual same-feature worker rebuilt last before C#fixture execution.

| Gate | Independent actual result |
| --- | --- |
| archive full isolated-codecs,offline,no-fail-fast,serial |124pass/0fail/0ignored across25result groups |
| actual product_read_chain included above |7/7,including open/test/CRC/slowcredit/cancel/EOF/drop/source errors |
| product_read_consumer included above |5/5 |
| protocol --lib |50pass/0fail/0ignored |
| IPC --lib |25pass/0fail/0ignored |
| Desktop full serial tests |806pass/0fail/0ignored |
| strict Clippy Archive/Protocol/IPC isolated+default including tests |all exit0 |
| strict Desktop Clippy including tests |exit0 |
| workspace fmt --all --check |exit0 |
| workspace check --tests --offline |exit0; NOT workspace tests |
| extensions --test archive_import |7pass/0fail/0ignored |
| clipboard --lib archive_ |5pass/0fail/0ignored |
| actual isolated worker build |exit0 |
| C# product/test build |0warnings/0errors |
| original119+new7 exact fixture list |selected126/run0/pass0/skip0/fail0/not_run126 |
| same exact fixture execution |selected126/run126/pass126/skip0/fail0/not_run0 |
| LIST/RUN/PASS names |0differences |
| actual4GiB+17FastDeflateZIP64 |RUN1/PASS1,once only this acceptance |
| spec gate |PASS106documents/14owners |
| catalog gate |PASS79pages/79rows/78locked |
| task context |PASS56implement/53check |
| PlatformHost selector source gate |PASS |
| git diff --check |exit0;existing unrelated CRLF warnings only |
| scoped source/test whitespace/conflict/DEBUG hygiene |PASS |

These counts are not increased by the independent probe. Scratch product-read-drop-probe uses
project Cargo.lock copied outside workspace, public API and contract endpoint; its probe.log
records both Drop cases. No production file/test was patched for a deterministic result.
Existing explicit7-Zip26.00 SHA256 verified
4bcc005758c5e604c0aaa257b6097846a4a708e62a9c0c7471fa360fad863f1f.
No download/install/PATH change. Real product never launched; Desktop unit tests are not a real UI.

## Cleanup and remaining scope

Posttests no Cargo/rustc/archiveworker/testhost/7z/probe processes;fixed test helper absent;
0inspected TEMP archive fixture roots. One scratch archive-product-chain GUID root created during
this acceptance contained only package/NxTools.ArchiveWorker.exe (no source/archive/userdata).
After verifying creation timestamp, exact contents and no live processes, parent removed this
owned residue. Final scratch chain roots0. This is not a claim that tests left zero residue before
manual cleanup. Scratch probe/scripts/logs/build outputs retained. C25.9GiB/D42.1GiB observed.

No actual native recycling/permanent source deletion,user archives,UI/FIFO/product write operations,
package/install/signing/commit/push,registry/association/service/drive-map changes. Native recycle
contract remains unapproved and fail-closed. Full workspace tests/unrestrictedC#/actual deployment/
100k-million metadata RSS/performance/longpath/UNC/native-user/release acceptance not performed.
Fix the two bounded lifecycle/backpressure defects before closing this read-only slice; no Phase B
or Phase A/A–F completion claim.
