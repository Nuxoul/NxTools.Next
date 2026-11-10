# Implementation and verification

## Reference and boundary

- [x] Read request, AGENTS, workflow and applicable Indexer/Protocol/IPC/Desktop contracts.
- [x] Preserve the139-entry dirty baseline and relevant source copies; retain unrelated work.
- [x] Verify Listary managed session/status first, SwiftList source second and absence of a matching legacy NxTools indexer owner; record chains and invariants.
- [x] Keep testing synthetic/temp-file/temp-pipe/TestApp only. No commit/archive/package/product/service/real-volume operation.

## Correctness and service ownership

- [x] H2 behavioral red and green:5000direct leaves/invisible deletes, mostly-tombstoned or moved-away directory children and truly large bounded visible subtree.
- [x] H3 production enumeration seam red and green: HighUsn=i64::MAX, saved journal start cursor, modified existing file/directory/unchanged descendant, idempotent create+modify and unknown-metadata nonresurrection.
- [x] M3 publication red and green: successive temporary durable pairs, compaction publication, unpublished-pair protection, valid search mapping and deferred native deletion failure cleanup. Actual compaction worker success/failure regressions require mapped input release before either terminal event is received.
- [x] M4 admitted-runtime red and green: service-lifetime original coordinator, current endpoint identity/lease, bounded nonreading output, retained single scan/offline publication, stale generation/results and unchanged admission.
- [x] L11 allocation red and green: reject infeasible counts before allocation, safe maximal headers and original base recovery.

## Maintenance and resource evidence

- [x] Add optional defaulted maintenance DTO under outer protocol1; verify legacy/new/old-DTO compatibility.
- [x] Service per-volume accepted/start/terminal state, source-origin typed reason/native code and low-noise correlation. Empty snapshots Ready; queued cancellation/rediscovery and true failure/last-good owners retained.
- [x] Production Settings red and green: official Small Tag/Label composition,920×480 Light/Dark row bounds/text, original scroll and single saved selection command. No popup strip or geometry change.
- [x] Durable base-publication failure and native error source regressions:2intended red assertions, then green. Failed new persistence preserves last-good and does not report success.
- [x] Default-ignored1Mbase/1k+100koverlay baseline and final apply/classify allocation/time measurements.
- [x] Confirm native unchanged cursor returns before clone; optimize only the measured costly path with shared immutable Arc rows and actual-mutation isolation.
- [x] Measure owned/mapped heap, file size, working set and private commit separately. Preserve product residency.

## Final checks

- [x] Safe Indexer library133passed, including12journal cache recovery tests and two worker-handoff ownership tests; two admission tests passed. Five native/system-discovery/production-scan cases and real-process integration tests not run.
- [x] Protocol100unit+2integration, IPC48 and affected Desktop200passed. Counts are nonoverlapping; explicit measurements2passed separately.
- [x] Strict Indexer/Protocol/Desktop test Clippy passed with no warning suppression.
- [x] Changed-file rustfmt check23files and git diff --check passed; unrelated Archive format debt retained.
- [x] Spec/component catalog/GPUI release/Markdown editor and Indexer/Quick Launch lifecycle static gates passed.
- [x] One bounded integration review completed; both concrete publication/diagnostic findings fixed and regression-tested.
- [x] Owner specs, complete outcomes, measurements, exact commands, known limits and completion metadata synchronized.

All Cargo invocations are serialized/offline and set CARGO_BUILD_JOBS=1, CARGO_INCREMENTAL=0, RUSTFLAGS=-C codegen-units=256 in the same process. Desktop uses test debug0 and one test thread; Cargo timeout21600seconds. Exact commands/results/logs are in outcomes.md.

## Pending authorized release acceptance

Packaged LocalSystem idle CPU/I/O, explicit RestartService/AssertRegression, persisted base+delta restart, real service search and protected atomic binary tuple gates require a separately authorized current package. Exact commands are recorded in outcomes.md and were not executed. Synthetic/source/TestApp evidence does not certify native/DPI/accessibility/user visual acceptance.
