# ZIP risk confirmation — independent scoped acceptance

2026-10-20. **PASS for ZIP confirmation within the original admitted budget only.** Reviewed
[evidence](zip-risk-evidence.md), [results](zip-risk-results.json), [source boundary](research/zip-risk-sources.md),
zip_risk, ZIP extraction/stream counters, InteractionPort matching, original worker/PrivateOutput,
C#typed response/test seam and actual risk fixture. No concrete blocker found in this delivered
portion. Larger finite-budget readmission, other formats, full B3/SAFE04/Phase A/A–F remain incomplete.
This does not accept the entire original B3 request as finished.

## Confirmed behavior

- Production predicate remains strictly >64MiB AND >1000x known positive packed size, using u128
  multiplication. Unknown/zero packed size has no invented ratio. Domain boundary tests pass.
- Original path/collision/header/declared-budget checks precede current-member risk confirmation.
  Confirmation precedes current decode/partial; directories and earlier committed members are not
  globally rolled back. No-output adapter returns ResourceLimit rather than silently approving.
- Existing InteractionPort installs member ticket before notification. Exact admitted entry limit
  must cover declaration and match approved_limit; larger answers and stale tickets do not approve.
  No cached/member-to-member/cross-task approval or admitted-request/Job limit mutation.
- Actual entry and total output counters remain in stream_entry; approval does not bypass CRC,
  exact size, source validation, unsafe paths or internal collisions. Decline/cancel stops later
  members, preserves old targets/foreign partials/earlier commits, cleans owned unpublished partials.
- DEBUG small threshold seam only lowers warning thresholds. Rust cfg(debug_assertions) and C#DEBUG
  exclude the field/setting in Release; strict Release parsing rejection is source-inspected only,
  NOT compiled/runtime-verified here. No release build/package was needed for this scoped acceptance.
- Original runner/Job/worker/typed interaction reused; no new approval/runtime framework/dependency.

## Independently executed checks

Waited for unrelated Desktop Cargo run to release, never terminated it or overlapped builds.
C#isolated apphost first, matchingisolatedworker last before C#execution. Every Cargo inherited
CARGO_BUILD_JOBS=1,CARGO_INCREMENTAL=0,RUSTFLAGS=-C codegen-units=256, offline/serial.

| Check | Actual result |
| --- | --- |
| C#Debug build |exit0,0warnings/errors |
| Six affected Rust integration targets |31pass/0fail/ignored,exit0 |
| zip_risk::tests |2pass/14filtered,exit0 |
| Strict Clippy isolated/default tests/examples |both exit0,-Dwarnings |
| Archive scoped fmt /matchingisolatedworker build |both exit0 |
| Exact C#LIST/RUN/PASS |same4names;selected4/run4/pass4/skip0/fail0/not_run0,exits0 |
| Spec/context/fixtureselector/diff |all exit0 |
| Sourcehash/identity assertions |PASS;64reviewedfiles unchanged during checks |

New actual runner/Job/worker/native fixture executes13small scenarios: ordinary no-prompt,
production declared threshold pending/cancel, confirmed publication, decline, cancel after prior
commit, fresh member tickets/stale answer, larger-answer rejection, actual entry/total overflow,
declared hard-limit rejection, CRCdamage, unsafe member/internal collision. Max actual expansion
8448bytes per ZIP. Positive small publication uses stricter DEBUG thresholds; production-trigger
case only forges declaration then cancels. No large expansion or bomb was generated. Assertions
verify exact bytes/commitcounts, no current partial/write before approval, prior commits retained,
later member stopped, source protection/release, one worker, owned cleanup and fixed diagnostics.
Existing original read-only product privacy/native ZIP/boundary regressions pass in same collection.

Specs106docs/14owners,context82implement/79checkentries; existing native-contracts35497byte injection
warning retained. No global/workspace suite or unrelated format matrix claimed. No optional review
or additional timing/resource probes; no implementation changes by acceptance.

## Evidence and stop

Scratch C:\Users\Administrator\.pi-desktop\scratch\ceebe351-d540-4ca3-a460-dadc71f3a7df:
`risk-independent-verify.ps1`, per-step logs, ledger, baseline and `risk-independent-assertions.json`
retain actual commands/exits/counts/exactnames. Submitted reports/results and checkpoint snapshot
were not modified. Current64file baseline verified; no fresh claim to reproduce implementer's
entire111historicalfile opening comparison. Closing0archiveprocesses/0testpipes/helperabsent.

No100k/longpathmatrix/1GiB/largeZIP64/sampling,UI/FIFO,formalapp,UNC/systemconfiguration,
sourceRecycle/delete,package/install/sign/commit/push. **Delivered current-budget ZIP confirmation
accepted; stop. Budget escalation/new admission and other formats remain explicitly open.**
