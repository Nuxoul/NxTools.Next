# OPT-06 — independent acceptance

## Decision

**Accept OPT-06 — Bounded binary bulk transfer at the authorized automated/owned-native stop point.** G-14 and touched G-01/G-02/G-04/G-07/G-19/G-23/G-25 pass the independently reviewed and executed boundaries. No in-scope production defect or blocking regression-evidence gap was confirmed. Standards:0findings. Spec:0findings.

Scope is OPT-06 current working-tree changes against HEAD `0940eec23286c1a750b3aff4284f32e76da68164`, distinguished from earlier work using the frozen OPT-06 before/delivery source. The332frozen Rust/C#/Python source files match current source exactly; worker, Host and test binaries match the frozen delivery artifacts. Prior OPT-02/03/04/05 independent acceptance and closed O5-AC01/02 remain intact. This review changed acceptance documents only, not product/test source or unrelated user changes.

The [delivery verification](D:/Code/NxTools.Next-main/.trellis/tasks/11-02-archive-workflow-optimization/opt06-acceptance.md), [private frame contract](D:/Code/NxTools.Next-main/.trellis/tasks/11-02-archive-workflow-optimization/opt06-binary-transfer.md) and [measurement tables](D:/Code/NxTools.Next-main/.trellis/tasks/11-02-archive-workflow-optimization/opt06-measurements.md) retain their original attribution. The [parent checkpoint](D:/Code/NxTools.Next-main/.trellis/tasks/10-16-archive-manager/phase-a-checkpoint.md) remains status authority under **Phase E — Format expansion and ZIP editing**. Overall task stays `in_progress`; OPT-07 is unstarted.

## Standards

**Pass;0findings.** No reachable documented-standard violation or actionable baseline smell was confirmed. The change remains in the original private pipes/Run/source/output owners, without a parallel Job/IPC runtime. Rust retains policy and the pending-call sequence; C# retains native source/token/write/publication authority and actual resource drain.

[BeginBulk/SendBulkResult](D:/Code/NxTools.Next-main/csharp/NxTools.PlatformHost/Windows/Archive/ArchiveWorkerRunner.Bulk.cs:44) retain one pending binding and actual input-send completion. Each payload is at most64KiB. Rust output borrows its codec block; host read replies briefly retain native bytes plus one envelope, retired/zeroed by the original owners. The fixed1MiB reservation is inside existing64MiB runtime overhead; process, codec, metadata, output, concurrency and deadline ceilings are not increased. Runtime counters are bounded numeric terminal observations, with no payload/path/token/chunk logging. Sink failure does not determine operation results.

Original Run cleanup, member-ready versus session retirement, preview consumer shares and transferred streams remain distinct. Actual boundary fixtures assert bytes, ordering, failure causes and resource retirement rather than treating compilation, imports, readiness or queued writes as acceptance.

## Spec

**Pass;0findings.** No confirmed missing/partial requirement, scope expansion or implemented-but-wrong requirement was found for the [G-14 contract](D:/Code/NxTools.Next-main/.trellis/spec/nxtools-archive/backend/binary-transfer.md:19).

- Modern schema2 data operations require `bulk_transfer:1` and unique exact `binary_bulk_v1` before source payload reads/output namespace effects. Schema1 ZIP, read-only operations and explicit HZ5 keep their separate contracts; there is no modern JSON-data fallback or public wire expansion.
- The high-bit discriminator and76-byte header preserve RFC4122 operation/call/token bytes, positive operation-wide sequence, exact offset/requested/actual lengths and direction. Original [Rust rendezvous](D:/Code/NxTools.Next-main/rust/crates/nxtools-archive/src/worker_output.rs:123) checks the exact pending binding; global sequence does not restart for members/tokens/attempts.
- Create/Add source bytes, Extract/retained output and held-partial readback all use raw transfer through their original native owners. The public-Job readmission adapter declares its modern private version without changing public schema or finite retry policy.
- The producer waits for ACK/read reply and the host waits for actual send completion. Metadata/source-close/finish/commit/member-ready cannot overtake data. Pause/cancel/EOF wake original waiters; first accepted native failure and committed/unknown/late-cancel publication truth remain authoritative.
- Full source freshness, codec end/CRC/MAC/actual size, metadata/MOTW, final-candidate versus removed-source payload verification, five fresh native stages, single-use reader completion, FILE-only handoff, postpublication verification and actual cleanup remain in their original owners.

## Independent verification

Every Rust invocation used offline mode, one Cargo lane, `CARGO_BUILD_JOBS=1`, `CARGO_INCREMENTAL=0`, `RUSTFLAGS=-C codegen-units=256` in the same invocation and a full-operation timeout. TEMP/TMP and generated fixture files were confined to this session scratch. The existing TAR/stream preparation script produced independent owned fixtures through CPython/libarchive and the documented stock decoding oracle.

| Check | Observed result |
| --- | --- |
| Current isolated worker build |PASS; current bytes match the delivery freeze. |
| Worker-related library filter |28/28PASS,0ignored. |
| Scoped integration contracts |95distinct executed passes across26complete targets plus the5-case TAR worker subset; no passing case is counted twice. |
| Strict archive isolated-codecs all-target Clippy, `-D warnings` |PASS. |
| C# Debug test-host build, `--no-restore -m:1 -p:UseSharedCompilation=false` |0warnings/0errors. |
| C# wire/legacy group |Exact LIST13/RUN13/PASS13/SKIP0/FAIL0/not_run0. |
| Affected actual native group |Exact LIST13/RUN13/PASS13/SKIP0/FAIL0/not_run0. |
| Actual current Create/Extract/Add control |Exact LIST1/RUN1/PASS1/SKIP0;9completed original Job/Run/native operations, three per route. |
| Source/document gates |Tree-root drag3, Phase F ownership, CLI16, Kit catalog/release, pinned editor and spec123documents/14owners:PASS. Source/provenance scope only. |
| Retained measurement/source audit |18raw samples,48summary blocks,332source files and9current native samples reconcile. |

Integration scope covers actual bulk worker8groups/35process scenarios, worker framing/cancellation/extraction, ZIP/7z/TAR/stream creation, edits and held readback, Add namespace/results/all-Skip qualification, metadata, original finite readmission, seven-format options and TAR raw-header/retained plans. Some are typed/in-process contracts;95tests are not represented as95native publications. Native work is separately established by the actual apphost selections and independent output expectations.

The C# wire group includes the two legacy worker fixtures plus11bulk selections: literal header/short IO, negotiation before effects, malformed/stale binding refusal, source/partial short reads/EOF, first failure/late publication/sink isolation, control/backpressure cancellation, read-request refusal before source bytes, ACK-before-commit, full-channel cancellation/zero drain and binary-versus-JSON budgets.

The actual native group preserves Gzip creation, empty Gzip/XZ interoperability, candidate-readback safety, O5production terminal counters, metadata/MOTW, genuine split ZIP, encrypted solid RAR4 preview versus separate drag authorization, corrupt required-block refusal,12solid7z/RAR5/TGZ/TXZ demand orders, six retained lifetime modes, preview adoption and TAR/7z native failure/unknown/no-retry/late-cancel boundaries. Exact output/refusal, source protection/exclusive reopen and actual process/pipe/Job/cache cleanup remain required. Controlled COM is not physical Explorer acceptance.

## Comparable measurements

The [measurement JSON](D:/Code/NxTools.Next-main/.trellis/tasks/11-02-archive-workflow-optimization/opt06-measurements.json) contains the exact retained before/after reports, each complete with9operations. Raw report files and frozen before/delivery binaries remain available. Build mode, deterministic1MiB inputs, disk, route and stable execution paths agree across sides. All18samples and48statistical summaries were independently recomputed, including rounded wall/wire tables, CPU/memory scope and exact native/payload work equality.

| Retained original route | Before median ms [min–max] | After median ms [min–max] | Bidirectional wire bytes, median | Reduction |
| --- | ---: | ---: | ---: | ---: |
| ZIP Store Create |691.335 [601.668–972.028]|183.681 [183.278–335.040]|6,789,553→3,172,163|53.28%|
| ZIP Extract |542.544 [517.313–571.784]|246.609 [244.524–279.812]|3,755,971→1,059,473|71.79%|
| ZIP Add |1260.297 [1209.646–1315.571]|495.964 [494.115–498.746]|12,054,485→5,280,000|56.20%|

These retained Debug ranges support lower wall time for the three measured workloads. They are not an all-format/Release speed claim. Source/output/partial payload calls/bytes and native source/target/output attempts/full/actual/completed reads agree for every paired trial. Add retains source5/target2/output2; output verification reads are not counted as old-source reads. Each raw binary record adds80wire bytes. Nine delivered samples have zero modern conversion input, bounded read-envelope copies, and observed host payload high-water2/131072bytes for Create/Add or1/65536bytes for Extract. Every sample retains one native publication, committed journal and actual cleanup.

The independent current control used a new GUID-owned root and the current implementation only. Its fixture initialization mode is explicitly annotated as **not historical before**. All9operations independently decode expected bytes, preserve inputs, publish once, drain actual Runs and retire owned partials; role/payload counts match the delivered expectations. Current wall medians/ranges were:

| Current route | Median ms [min–max] |
| --- | ---: |
| ZIP Store Create |243.788 [174.802–589.966]|
| ZIP Extract |263.222 [244.556–309.276]|
| ZIP Add |593.776 [590.591–685.123]|

Current control times are separate observations, not another before/after ratio or a reason to select a preferred trial. No historical before binary was executed by this review, and no system-cache flush was performed.

## Evidence

Session root: `D:\.pi-desktop\data\scratch\aab2f188-352d-4f1c-b7a3-5375fe0df2d8`. Logs use `opt06-independent-*`. Scoped Rust integration results are in `worker-integration`, `worker-remaining` and `tar-worker` logs; the actual distinct passing case set is95. Native wire/preservation/current-control selections retain separate LIST and RUN logs. The source/data reconciliation is `opt06-independent-measurement-audit.json`, produced by `audit_opt06_acceptance.py`.

Current worker freeze: `opt06-independent/Debug/NxTools.ArchiveWorker.exe`, SHA256 `85892352B07449FD92D95712CBCC8A67FB361D59CC9F597134F0CF21DAA5BD4A`. Its bytes, current Host DLL and current test DLL match the frozen `opt06-delivery` artifacts. The audit compares332source files directly; artifact identities are existing execution-provenance checks, not additional archive-content fingerprints.

Current9-operation report: `archive-product-opt06-4f747251-327f-45e7-8bfd-ff6d1448b173/reports/independent-current.json`, with `independent-acceptance-scope.json` recording its current-only purpose. Original retained reports remain under the delivery evidence root named in the measurement JSON and were not overwritten.

## Limitations and stop point

No OPT-06 repair blocker remains. Standards and Spec each report0findings. Existing logical/runtime resource limits, split-volume freshness and locked codec replay/block costs retain their original contracts. Wire-byte reduction is not source-file/disk I/O reduction. CPU values are sampled worker-process intervals, including coarse zeros; memory is a maximum among successful samples and can miss peaks. Exclusive codec/transport CPU, total/internal source I/O, exact process peaks and separate cleanup duration remain unavailable; cleanup is asserted and included in wall time, not assigned zero cost.

G-00, OPT-00/01 overall closeout, complete G-04, overall Phase E and physical/Release acceptance remain open. The prior accepted OPT-02/03/04/05 boundaries are preserved, not counted as new complete phase acceptance. No product GUI/Explorer/system clipboard/picker, user archive mutation/recycle/HZ5, service/registry, package/sign/install, reference-program execution/new reverse engineering, commit or push occurred. Stop at OPT-06; OPT-07 remains unstarted.
