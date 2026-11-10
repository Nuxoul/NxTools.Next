# OPT-05 — Reusable format and update plans

## Status and scope

**Completed and accepted at the authorized automated/owned-native boundary:** G-03/G-04/G-05/G-24 and touched G-19. Parent remains **Phase E — Format expansion and ZIP editing**. OPT-02/03/04 independent scoped acceptance and historical evidence remain intact. Overall task remains `in_progress`; G-00, OPT-00/01 overall closeout, complete G-04, Phase E and physical/Release acceptance remain open. OPT-06/07 are not started.

[Scoped acceptance](opt05-acceptance.md) · [Comparable measurements](opt05-measurements.md) · [All three-trial values and counters](opt05-measurements.json).

## Implementation and ownership

| Slice | Existing owner | Delivered boundary |
| --- | --- | --- |
| TAR/compound | `ArchiveEngine`, `formats/tar`, `formats/tgz`, `worker_job`, `worker_drag`, `tar_edit_outer` | One fully verified anonymous outer spool and original-session ordinal Vec per operation/attempt; catalog, qualification and payload consume the retained bytes; original source/outer/footer/inner metadata checks remain |
| ZIP records | `formats/zip/plan.rs::ValidatedZip`, original Test/extract/edit and retained worker | Exclusive actual-reader and immutable entry borrows; original ordinal/data/encoded extent/end plus optional exact local header; one local/descriptor/ZIP64/AES/overlap admission before streaming |
| ZIP readmission/edit intake | `worker::execute_attempt`, `ArchiveEngine::extract_impl`, `worker_zip_edit` | Readmission holds its reader-bound ZIP plan across native preflight into the same engine guards/finalization; actual editing uses catalog/native binding and intent-aware candidate admission, while public browse qualification and no-change Add remain freshly qualified |
| ZIP update integrity | `zip_edit::candidate` / original held-partial output chain | Retained/new payload verifies once in the complete final candidate; removed/replaced-away old payload verifies on source; additions remain structurally admitted; source-old and final-candidate actual-byte quotas are distinct |
| Selection | `planning_selection`, `ExtractionPlan`, ZIP/TAR/7z edit plans, original Desktop browser/model | Bind original IDs once, merge component-ended directory prefixes and expand catalog once in original order; existing descriptor `BTreeMap` presentation/collision order remains unchanged |
| Native observation | `ArchiveSourceLease`, `ArchiveOutputSession` | Share only one stage's complete observation after live full identity/version equality under uninterrupted protection; five fresh stages, reader-close/FILE-only handoff, flush/fingerprint/postpublication and real cleanup remain |

Plans and spools retire with their original operation/attempt. No live authority enters snapshots/wire/cache. Rust retains plans/control, C# native source/output authority, GPUI rendering. No new backend/dependency/vendor/global cache/runtime/scheduler/business-state owner or UI/layout/theme/control change.

### Budgets and diagnostics

ZIP reserves simultaneous raw directory records, source snapshot capacities, intent, copied output records, additions catalog/plan and final verification. The reader plan charges records/ranges/headers before growth and subtracts live working memory before codec minima and 64KiB buffer credit. TAR charges catalog, actual locator capacity, strict edit records, execution and codec work additively; fresh public Test/extract uses the same locator coexistence admission as retained execution. Selection scratch and caller-retained output use the smaller remaining metadata/working credit.

Original operation/attempt observations aggregate low-frequency work on success and failure. Native counters credit actual bytes and only complete EOF/digest observations; interrupted reads retain partial bytes without full-read credit. Source/target, output fingerprint/postpublication and held-partial readback remain separate roles. No paths, members, secrets, payloads or per-chunk events are logged.

## Reference

Saved Bandizip7.32.0.1 evidence was inspected as pseudocode **and assembly/callees**: retained input `0x180057cd0`, local admission `0x1800576f0`, streamed integrity `0x18007dca0`, old-index import `0x18001dd30`, encoded retention `0x180090e20`, compound selection `0x18003c960` and constructor `0x180023d70`. The recorded Next call chains are in `docs/reference-code/archive.md` and `archive-editing.md`. No reference executable was run, new reverse engineering performed or proprietary implementation copied.

## Real-boundary RED / GREEN

| Regression | Effective RED | Delivered GREEN |
| --- | --- | --- |
| Ordinary TAR execution | 2 ordinary scans /38 header seeks | 1 scan /19 header seeks |
| Qualified compound open | 2 outer decodes | 1 complete outer decode, strict qualification retained |
| Three retained TAR demands | 3 ordinal HashMaps | 1 ordinal Vec |
| ZIP Test on16 records | 32 local checks | 16 local checks; original integrity paths retained |
| ZIP update responsibility | Rename/comment source128 plus final128; Add source128/addition1/final129 | Rename/comment source0/final128; Add0/0/129; overwrite1/0/128; Delete1/0/127 |
| Selection at65536/4096 | Extraction/7z and ZIP/TAR Delete each268,435,456 visits; Desktop Extract/Delete same repeated expansion | 65,536 visits, exact original member set/order before complexity assertions |
| Attribute-defined ZIP directory Rename | Supported directory `a` fails with `UnsafePath` | `b/`, `b/file`, untouched `ab/file`; exact payloads and source preserved |
| Original ZIP edit worker | 3 local/descriptor checks for one source plus final candidate | 2; both normal and corrupt-readback branches execute |
| ZIP output-readmission worker | Four records admitted twice:8 checks | 4 checks; successful original terminal, three actual commits and output_ready precede count assertions |
| Coexistence budgets | ZIP plan/encrypted codec, source/additions snapshot, public TAR locator/execution and extraction/7z working-credit limits wrongly admitted | Original `ResourceLimit` refusals and positive controls pass; public TAR Extract refuses before output |
| Equivalent native observation | Old-object full reads14 /117,442,220actual bytes | 7 /58,721,110; output reads2 and held-partial8,388,728bytes unchanged |

RED source/test/worker evidence remains separate in `opt05-review-red-source`, `opt05-review-red-pair`, original observed before snapshots and supplemental budget/selection RED snapshots. Accepted results require actual nonzero execution, original business/cleanup facts and matching artifacts. All three ZIP integration regressions pass in the delivery pair.

## Measured results

All principal boundaries retain three completed comparable trials. [Full tables](opt05-measurements.md) include ZIP8routes×3, TAR15routes×3, native compound10routes×3, six selection boundaries×3 and native owner3trials. Same retained input/build mode/disk/route/stable execution path; no system-cache flush or best-run selection. Separate owner latencies are not added into a product-operation estimate.

| Work / boundary | Before | After |
| --- | --- | --- |
| Native TGZ/TXZ per Run | Outer2; ordinary TAR scan browser1/payload2 | Outer1 /scan1 /index1 on all30 completed jobs |
| Retained TAR locator backing across3demands | 3HashMaps /1,344cumulative bytes | 1Vec /256cumulative bytes |
| ZIP128-member rename/comment | Local768 /descriptor512 /record-admission seeks1024 | 256 /256 /384 |
| ZIP1024-record open /Test local checks | 2048 /2048 | 1024 /1024 |
| Extraction planning median [range], s | 164.915 [163.061–179.398] | 5.919 [5.875–6.094] |
| 7z selection planning median [range], ms | 19461.484 [19431.353–19519.247] | 74.969 [73.323–79.047] |
| Desktop Extract /Delete /final edit admission medians, ms | 7088.868 /7660.492 /1947.310 | 80.663 /96.079 /19.252 |
| Drag model median [range], s | 102.370 [102.124–102.604] | 37.991 [35.899–39.643] |
| Protected native owner median [range], ms | 130.377 [126.809–170.213] | 87.545 [87.252–91.267] |
| Same native owner actual cleanup median [range], ms | 0.446 [0.410–2.144] | 0.559 [0.538–0.588] |

Selection expansion visits fall268,435,456→65,536. Drag prefix probes/allocations fall268,431,360→557,055probes/4,096prefix allocations; original ordinal probes133,236,736→69,632. Merged drag roots4096→128; all65,536descriptors retain original presentation/collision semantics. Remaining descriptor projection work is included in its37.991s median.

ZIP metadata open/Test medians57.825/33.328→44.788/23.080ms. ZIP update timings do not show a uniform end-to-end improvement; most ranges overlap. Small native compound ranges also overlap. Those cases establish reduced work, not a blanket speedup. Kernel TXZ open/Test/materialize show separate fixture-local reductions; plain TAR and some other worker routes are slower in the retained measurements. Every result remains in the tables.

## Verification

| Boundary | Executed result | Evidence log in session scratch |
| --- | --- | --- |
| Delivery ZIP7, TAR12+explicit baseline1, shared selection17, extraction worker10, edit worker1 | 48PASS /0FAIL /0ignored; three ZIP integration regressions included | `opt05-delivery-format-after.log` |
| Touched ZIP/encoding/engine/readmission/cancellation/edit-worker preservation | 71PASS; crypto/ZIP64/extras/codec-end/source/failure checks retained | `opt05-final-zip-worker-preservation.log` |
| Desktop browser/model/forms/visible preview lifecycle | 58+21+41+4=124PASS,0ignored | `opt05-final-desktop-preservation-selected.log` |
| Protocol /IPC | 104 /55PASS | `opt05-final-protocol-ipc.log` |
| Original Job/Add namespaces/TAR edit/ZIP native outputs | LIST15/RUN15/PASS15/SKIP0/not_run0 | `opt05-final-native-edit-output-{list,run}.log` |
| Retained split/solid/encrypted/block/format/lifetime/preview | Six actual fixture passes plus separate LIST1/RUN1/PASS1 context lifecycle; zero accepted skips | `opt05-final-native-retained-run.log`, `opt05-final-native-context-owned-{list,run}.log` |
| Native source/output observation and original ZIP handoff | LIST8/RUN8/PASS8/SKIP0, including three owner trials and30compound jobs | `opt05-final-native-measure-{list,run}.log` |
| Delivery paired native selected/readmission/split/preview/AddZIP/ZIPallactions | LIST6/RUN6/PASS6/SKIP0/not_run0 | `opt05-delivery-native-controls-{list,run}.log` |
| Delivery paired native compound three-trial measurement | LIST1/RUN1/PASS1/SKIP0; all30jobs outer1/scan1/index1, exact bytes and cleanup | `opt05-delivery-native-measure-{list,run}.log` |
| TAR stream/native readmission preservation | LIST6/RUN6/PASS6/SKIP0, actual drain/exclusive reopen | `opt05-native-stream-owned-{list,run}.log` |
| Quality | Archive isolated and default Desktop all-target Clippy `-D warnings`; scoped rustfmt32paths; C#0warnings/0errors | `opt05-delivery-clippy-compile.log`, `opt05-final-rustfmt-selected-check.log`, `opt05-final-csharp-build.log` |
| Source/document gates | Spec contracts, Tree-root drag3, PhaseFownership, CLI16, Kit catalog/release and pinned editor pass | `opt05-delivery-static-gates.log`, final spec-contract log |

Groups overlap and are not summed into a unique-test count. Worker/private output peers are distinguished from actual Windows source/output acceptance. Native checks preserve same-size replacement/version refusal, wrong/single-use read-completion, FILE-only handoff, failed/unknown/no-retry, late publication and independently responsive cancellation. Corrupt retained/removed/additions never publish invalid output. Actual published/unknown facts survive late cancellation; owned partial and Run/pipe/Job cleanup remain truthful.

Original Add namespace includes79scenarios across the three format groups; no-change/merged-only decisions stay zero-publication. Metadata/MOTW/directory finalization, All/selected/contextual Extract, Here/Name/Smart/custom/Desktop rules, picker pending/late cancellation, current result selection and OPT-04 member-ready versus actual cleanup remain preserved. Encrypted preview still cannot authorize GetData.

## Evidence and executable contracts

Session evidence root: `D:\.pi-desktop\data\scratch\a2072588-1f47-4232-be7a-3951a25df615`.

- Observed before: `opt05-before-pair/Debug`, `opt05-observed-before-source`, original ZIP/TAR/native/selection reports and frozen test executables.
- Delivery: `opt05-delivery-pair/Debug` and `opt05-delivery-source`; worker12,341,248bytes, UTC2026-11-03T03:44:17. Matching C#Host2,164,224bytes. Frozen integration/unit executables retain their original embedded worker-path attribution.
- Actual principal execution uses `opt05-comparison/Debug` for native pairs and original engine/worker test paths. The old context preservation control uses its original `opt04-comparison/Debug` and a new owned copy of the old fixture; it is not a new OPT-04 speed claim.
- ZIP `measurements-before.json` versus `measurements-delivery.json`; TAR `before-baseline.json` versus `delivery-baseline.json`; native compound equivalent report names. The native owner after is `measurements-regression-037badee65f6428bb24280b5071cf94f.json`, using the unchanged matching C#pair. Exact roots and all raw values are in `opt05-measurements.json`.
- Contracts: [format/selection plans](../../spec/nxtools-archive/backend/reusable-plans.md), [native observation](../../spec/platformhost/archive-read-observation.md), original Desktop batch/selection, TAR/stream/edit and retained materialization contracts. Design, implementation, task metadata, acceptance and parent checkpoint share this scoped result.

## Limitations and stop point

- Total source/internal codec I/O, exclusive codec time and phase peak/RSS remain unavailable. Revision bytes are not total I/O; record-admission seeks are not total seeks; TAR ordinary scan excludes strict edit parsing. Locator backing capacity and normalization-owned results are not allocator-call or peak-memory measurements. Native process sampling may miss short-lived peaks; native compound separate cleanup duration is unavailable.
- Split ZIP retains complete all-volume revision work; the disclosed two-demand40revision-pass cost is not removed. Ordinary `NativeReadLease` is not a volume/edit/reclamation shortcut. Locked7z consuming-block limitations and RAR backward replay remain.
- Debug synthetic/owned-native evidence does not establish Release, all-format/Bandizip-equivalent speed or real Explorer/GUI/DPI/user acceptance. G-00, OPT-00/01 overall, complete G-04 and Phase E remain open.
- Stop at OPT-05. No real Explorer/product GUI, system clipboard/picker, user-source mutation/recycle/HZ5, services/registry, packaging/signing/install, commit or push was performed. OPT-06/07 remain unstarted.
