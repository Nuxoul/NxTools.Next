# Archive workflow optimization — acceptance

OPT-00/OPT-01 remain in progress with G-00 incomplete. OPT-02/03/04/05/06 retain independent scoped acceptance; O5-AC01/02 remain closed. **OPT-07 is independently accepted scoped after directed repair; O7-AC01/02/03 are closed:** [current verdict](opt07-ac01-ac03-independent-acceptance.md). [Repair verification](opt07-ac01-ac03-repair.md), [original partial verdict](opt07-independent-acceptance.md), [workflow](opt07-workflows.md), [implementation verification](opt07-verification.md) and [45 rows](opt07-coverage.md) retain attribution. G-00, OPT-00/01 overall, complete G-04, Phase E, physical/Release, real G-18/G-28 and G-29 remain open. Requirements: [brief](prd.md); order: [implementation](implement.md); contracts: [design](design.md).

## Current Phase E preview budgets — source/owned-fixture verification

[Configurable preview budgets](preview-budgets.md) are implemented under the original Settings and preview owners. [Results](preview-budget-results.md) verify six actual number inputs/saves, leafwise concurrent persistence, immutable selection/native/read/decode/render budgets, ordinary text/photos, CR-only limits, distinct unknown-size selected-output exhaustion, actual timeout, stale suppression and lease/output cleanup. Core95/IPC61/Protocol114, Desktop preview111, shared document2 and exact C# preview2/2 pass with no skips; workspace test compilation, strict library lint and scoped format pass. Full Desktop1777/9/1 and package-wide strict test lint remain non-green with the documented fixture/creation/download/Shell-test blockers. This is not installed, physical/DPI/human or overall Phase E acceptance.

## Current Phase E ordinary extraction output — supplied-ZIP gate passed

The approved [ordinary extraction output contract](ordinary-extraction-output.md) is stabilization inside **Phase E — Format expansion and ZIP editing**, not a new OPT package. No historical OPT-05/06 result, Release45 measurement or old partial-publication fixture is current acceptance for this contract.

| Gate | Required evidence |
| --- | --- |
| OEO-01 final-name ownership | Original Job/worker/C# routes for toolbar/dialog/context, Smart/Here/Name/Desktop/custom and destination-directory drag across admitted ZIP/7z/RAR/TAR/compound/stream formats use held-relative `CREATE_NEW` final names. Observe no `.nxpart`, rename, `FlushFileBuffers`, per-write/content/file-ID fingerprint, per-file `VerifyDirectory` or published reopen. |
| OEO-02 conflicts | Initial exists returns `target_exists` plus `existing_bytes`; Rust Ask/Overwrite/Skip/AutoRename/ApplyAll remains current-attempt scoped; no `observe_target`; overwrite directly truncates the same path without signature/CAS; AutoRename uses bounded `CREATE_NEW`, advances on `target_exists` and never overwrites. |
| OEO-03 bounded output pipe | `extraction_output_v1` remains required inside bulk v1. Rust emits <=32KiB coalesced records through the original anonymous pipe with zero write ACKs. The original independent C# pumps use unchanged synchronous `CreatePipe` handles: input buffer-1 `FileStream` plus cancellation-prechecked synchronous `Write`; stdout `ReadFrames`/`ReadBulkWrite` plus synchronous `Read`/`ReadExactly`. No new pipe/channel/runtime, payload authority or native-flag change. Only privately negotiated ordinary parallel extraction uses bounded keyed controls and native tasks as described below; other directions retain their original rendezvous semantics. |
| OEO-04 integrity, metadata and cleanup | Commit carries Rust-retained mtime/attributes/MOTW and logically remains drain then close without durability flush. CRC/codec/size/source/cancel/native failure deletes only incomplete owned output(s) pending required integrity; completed files/directories and pre-existing skipped targets remain. Journal paths/counts/created/completion and actual cleanup are exact without ordinary file hash/ID retention. |
| OEO-05 diagnostics | Rust exports exact `output_{begin,close,directory,directory_metadata}_{calls,wait_us}` semantics, including returned failure/cancel calls and wait start after previous-tail flush. Native exports `native_begin_ms`, inclusive `native_parent_resolution_ms`, `native_final_open_ms`, `native_create_ms`, `native_open_inspection_ms`, `native_directory_ms` and `native_directory_metadata_ms`. `native_commit_ms` measures only `session.Commit` metadata/handle close after stdout payload drain; `output_close_wait_us` measures the full Rust await. Fingerprint/postpublication and write-ACK counters remain zero. All durations are inclusive/overlapping, non-additive and not exclusive IPC. |
| OEO-06 preservation | Archive Create/Edit candidate verification/readback/replacement, split publication, HZ5 reclamation, source lease/path/reparse safety and `ArchiveEntryCache` Seal/verified-cursor behavior remain unchanged. |

The historical synchronous-pump follow-up completed byte-correct button/Root extraction at4241/4214ms medians versus Bandizip1476.011ms:2.873×/2.855×, failing its required<=1.5× gate. Its [report](ordinary-extraction-pump-results.md) and [per-trial evidence](evidence/ordinary-extraction-pump-2026-11-07.json) retain native/control timings, actual OLE lifecycles, cleanup and environment restoration for that serial runtime.

### Current bounded parallel ZIP acceptance

[Parallel extraction](parallel-extraction.md) passes its scoped compiled lifecycle and genuine supplied-ZIP acceptance. Deterministic actual Rust/native overlap, bounded lane/control high water, ApplyAll/stale tickets, multiple-incomplete cancellation/completed retention, sticky real CRC/native error, monotonic aggregate progress and original-format/candidate/cache preservation are verified. Full isolated-codecs archive392passes/0failures/1existing ignored; C# parallel10/10, preservation15/15 and default Job4/4 pass with no skips. The matching scratch Release completed genuine button3+Root external drag3+Bandizip parallel=1 controls3: medians2370/2283/1827.211ms,1.297×/1.249×, both<=1.5×. All nine match4492files /313,474,195bytes; Root OLE chains, cleanup and original environment restoration pass. [Results](parallel-extraction-results.md), [evidence](evidence/parallel-extraction-2026-11-09.json). Installed-package, human acceptance and overall Phase E remain separate.

Retained pre-follow-up [implementation and verification results](ordinary-extraction-output-results.md): final directed C# LIST14/RUN14/PASS14/SKIP0/FAIL0 and all3 RAR/7z/TAR-stream readmission targets (40internal scenarios) pass; Rust boundary/affected contracts and strict archive Clippy pass. Genuine Release button/Root trials each complete4492 files and independently match313,474,195bytes. Their medians4474/4378ms are2.957×/2.894× Bandizip1512.826ms, so the required<=1.5× performance gate did not pass. [Historical per-trial evidence](evidence/ordinary-extraction-output-2026-11-07.json). Those immutable results remain the baseline for the authorized follow-up, not its acceptance. Source/spec, compiled, real GUI, installed Release and user acceptance retain distinct boundaries. The retained2780.91/411.86/7325.86ms sample and208-file Seal/cursor figures remain historical different-sample context.

## Incoming Add — independent scoped acceptance

The explicit “添加到当前压缩包” action directly submits one frozen original Add request without a form. Original callback and queued dispatch refuse stale targets; normal backslash-directory ZIPs and isolated read-only sample copies succeed at root and in an existing folder. Existing NotificationCenter owns operation-specific failure feedback and hidden-main preparation/frame/reveal. Native publication, original source/record preservation and cleanup remain authoritative.

[Independent acceptance](incoming-add-independent-acceptance.md): fresh546 distinct Rust passes and exact C# LIST/RUN11 passes, skip0/fail0/not_run0. Five-format worker145scenarios, ZIP preservation17scenarios and14incoming native operations pass; four actual Desktop bridges emit one request/forms0, twelve successful incoming operations publish once and Skip/Cancel publish zero. Strict Clippy, scoped formatting and relevant source/spec gates pass. Release46 predates and excludes this batch; physical/installed/overall Phase E acceptance remains separate.

## OPT-07 directed independent acceptance — accepted scoped

O7-AC01 warning Find actions refuse before Dialog/model mutation, O7-AC02 Retry dispatch/receipt/old Started remain Admitting until current Started, and O7-AC03 expired checked consent preserves original input/ticket/choice until explicit uncheck. [Independent directed execution](opt07-ac01-ac03-independent-acceptance.md) passes exact LIST61/RUN61/PASS61/FAIL0/ignored0, strict Desktop Clippy, six-file formatting, Debug link and relevant source/spec gates. Original-callback behavioral REDs, exact six-file delta and142repair freeze agree. G-16, touched Retry G-17 and G-20/touched G-26 are accepted at the authorized automated/owned-native boundary; not physical or overall acceptance.

Independent execution passes Desktop57unique names, Protocol105/IPC3/matcher1/CLI15, five-package strict Clippy, current worker build and C#0warnings/0errors. Exact controlled LIST14/RUN14/PASS14/SKIP0 and actual-native LIST5/RUN5/PASS5/SKIP0 have fail0/not_run0. All24historical failed names occur among the fresh57passes; the historical448run is not claimed rerun. Source/evidence/freeze and45row identity audits pass; no new performance ratio or physical acceptance is issued. [Exact findings and minimal regression repairs](opt07-independent-acceptance.md).

Real G-18/G-28 and original-object recycle G-29 remain separate unaccepted boundaries. Native adapter/STA/vector coverage does not certify an actual system dialog, and refusal-only recycling cannot close G-29. Original implementation verification and historical results stay intact; overall remains in_progress.

## OPT-06 independent acceptance — accepted scoped

G-14 and touched G-01/G-02/G-04/G-07/G-19/G-23/G-25 pass. Standards0/Spec0findings; actual worker28units/95distinct integration cases, exact C#wire/legacy13 and affected actual-native13 pass with0skip/not_run. Current-only LIST1/RUN1 completes9Create/Extract/AddJobs with exact payload/native counters, one publication, independent bytes and actual cleanup. Strict archive Clippy, C#0warnings/0errors and related source/spec gates pass.

The independent audit reconciles18retained raw samples/48summaries,332frozen source files and matching current/delivery artifacts. Three Debug workloads show lower retained wire/wall; equal native/payload work isolates transport savings without a source-I/O claim. Current9samples are separate controls, not a new historical before. [Exact verification, data and remaining limits](opt06-independent-acceptance.md). Prior reports/counts retain their attribution; overlapping groups are not summed. That verdict remains scoped to OPT-06; current OPT-07 implementation verification is separate.

## OPT-05 independent acceptance — accepted scoped after directed repair

G-03/G-05/G-24 and touched G-04/G-19 functional boundaries pass: one operation-local verified TAR spool/ordinal plan, reader-bound ZIP records, component-ended selection, final-candidate/removed-source payload responsibility, same-stage protected observations and actual publication/cleanup. No reachable integrity or source-protection defect was confirmed. **O5-AC01** production role-separated diagnostics and **O5-AC02** actual-worker all-Skip fresh qualification coverage are now independently closed. The original partial report and repair verification remain historical records.

Independent runs: archive48passes, Desktop browser58/model21/forms41/preview4 plus Add-selection6, protocol104 plus integration2/IPC55; native core LIST12/RUN12/PASS12/SKIP0 plus current-only setup LIST1/RUN1/PASS1/SKIP0. Worker/C# builds, strict archive/Desktop all-target Clippy and relevant source/Kit/editor gates pass. The measurement audit reconciles240 retained raw samples/118 summaries and58 current samples. Native old-object14→7 excludes the two output reads. Slower and overlapping routes remain explicit; no uniform end-to-end speedup is accepted.

Directed independent verification: actual-worker4/4, C#0warnings/0errors and exact native LIST9/RUN9/PASS9/SKIP0, not_run0. Production success/cancel terminals agree with resolved results and exact source/target/output/partial counts; interrupted65536bytes receive no full-read credit, and a rejecting sink preserves successful publication/cleanup. Current implicit/explicit all-Skip controls pass and the retained qualification-only mutant independently fails both fresh-qualification assertions while completing with zero publication and unchanged source. [Current independent decision and exact evidence](opt05-ac01-ac02-independent-acceptance.md). The delivery's48/71/124 and prior native groups retain their attribution; overlapping groups and rounds are not summed. No new performance, overall Phase E or physical/Release acceptance is added.

## OPT-04 scoped automated/owned-native acceptance

| Gate | Result |
| --- | --- |
| G-07 | Descriptor-only zero worker/output; same-path ZIP1/2/16 each three completed trials, one retained worker/source/catalog, occupied Run at member ready and one actual terminal after receiver end |
| G-08 | Solid7z full-block/reset, duplicate/no-block/multiblock ordinals, genuine solid RAR4/RAR5 replay, one TGZ/TXZ outer decode and retained split ZIP source pass; unrequested predecessors have no output |
| G-09 | Async/sync/slow/Start-End/abort/Escape/source-close/disconnect,31s receiver idle, write/Seal30s work timeout, verification cancel, worker exit/EOF and transferred-stream survival pass |
| G-10 | One admitted immutable plan/activity; exact ordinary/encrypted preview adoption, distinct consumer share, independent grant/revoke, no GetData password prompt and actual-byte budgets pass |
| G-27 | Four new real Desktop prepare/visible-Ready tests and105-test overlapping preview selection pass; A→B→A, refresh/codepage/tab/close/password/stale decode retirement preserved |
| G-19 touched | Original Seal accounting/no size cursor, full first/repeat tamper checks, complete33/256-file/257-descriptor/64-root representation, native publication/source preservation and actual cleanup pass |

Final same-pair lifecycle LIST1/RUN1/PASS1/SKIP0 and integrated LIST6/RUN6/PASS6/SKIP0. Scoped old-chain preservation passed six async/password selections and a ten-selection tree/split/accounting/tamper/budget/gesture/cancel group; overlap is not added twice. Archive59/59, protocol50/50, IPC17/17, scoped format, archive/Desktop all-target strict Clippy, C#0warnings/0errors and four spec/Kit/editor gates pass. Detailed format work, real RED/GREEN, paired source/binaries, three-trial latency ranges and limitations are in the [OPT-04 result](opt04-retained-materialization.md). No real Explorer/GUI/clipboard/picker, user-source/HZ5/recycle, service/registry, package/sign/install or commit/push acceptance is implied.

**Independent OPT-04 verification:** six retained native fixtures and twelve async/password/wire/cursor/budget preservation fixtures pass exact LIST/RUN with zero skips. Two current-only lifecycle/control modes each pass descriptor-only plus9 ZIP1/2/16 trials, with one live Run at first ready and actual terminal after receiver end; these use the same current worker, not historical before/after versions. Archive59, default Desktop newpreview4/currentpreviewfilter122 (overlapping), protocol50/IPC17 and archive/Desktop strict all-target Clippy pass. Current report reconciliation covers12 format orders,2 split orders and3 RAR4 grant scenarios. Split two-demand runs retain40 per-volume revision passes and TAR still builds an ordinal index per demand; these disclosed costs do not grant source-check removal or close OPT-05/G-00. Historical27.68s→2.45s remains delivery-attributed rather than independently recomputed here.

---

**OPT-00/01 independent acceptance: overall closeout not accepted.** [Independent report](opt00-opt01-independent-acceptance.md) accepts the tested G-01/G-02/G-06 slice, leaves G-15/G-25 partial for AC-01/02/03, and retains G-00 incomplete under AC-04. Its verdict and historical execution matrix remain intact; subsequent AC repair and independent OPT-02 evidence do not close G-00 or the overall task. Task status remains `in_progress`.

**AC-01/02/03 repair evidence:** [Focused repair record](opt01-ac01-ac03-repair.md) adds passing original background/native-close callbacks and once-only restored decisions, actual two-channel reordering with operation-local sequence, and matched SevenZ/TAR-family phase presentation. C#3/3, desktop jobs83/83, progress15/15, protocol101/101 and strict Clippy passed. This supplements the independent verdict; G-00 and physical/Release acceptance remain open.

## OPT-03 execution matrix

| Gate | Scoped evidence | Status |
| --- | --- | --- |
| G-12 | Original Remove/New-all/Add/Paste controls; frozen picker/clipboard/submission context; native owned HDROP ANSI/Unicode/empty/malformed/overflow and balanced borrowed resources | Passed automated/owned-native; physical input remains separate |
| G-13 | Actual worker35 scenarios and native Add21; new-snapshot result mapping,301-member business output/256-member display, no Skip selection, stale/unpublished/unknown/unclean isolation, published late-cancel refresh without success selection | Passed automated/owned-native |
| G-21 | Original forms41/drafts12,7-format9-case actual writer matrix and invalid options,8 native independent interoperability selections/43 outputs; source/root/exclusion/secret/frozen settings retained | Passed automated/owned-native |
| G-22 | Complete disk/requested/generated output plan before any admission;15 planner cases including real form, timeout/malformed/close/disconnect, full/near-full atomic refusal, compound/split/case names and bounded large destination; native late-target confirmation/cancel | Passed automated/owned-native; split publication remains per-volume |
| G-23 | Actual ordinary/split ZIP/split7z C# location exports through original Desktop terminal; post-Test success/failure/cancel facts, missing location refusal, cleanup-held disconnect, native precommit failure and published late cancel | Passed automated/owned-native; physical Locate remains separate |
| G-18 touched | Original form/prompt/progress lifecycle and exact Kit components; stable Inputs, deferred entity callbacks, pending/focus/retirement | Passed automated boundary; visual/DPI/IME/accessibility acceptance remains open |
| G-19 touched | Original Job107, protocol50, IPC17, CLI7; native OPT02 preservation79 scenarios, actual single publication/cleanup and independent contents | Passed touched preservation; no source recycling or broader phase closure |

Native OPT03 LIST5/RUN5 passes with zero skips; final C# build has zero warnings/errors. Strict five-package all-target Clippy and spec/catalog/release/editor source gates pass. [Evidence, RED/GREEN distinctions, worker identities and limitations](opt03-intake-results.md). Stop at OPT-03; G-00, OPT-00/01 overall, complete G-04 and Phase E/Release acceptance remain open.

The original independent run passed worker Add35/seven-format Create contracts, native/production exports, Desktop Job107/forms41/draft12/progress13/OPT03 entry17/planner15, protocol50/IPC17/CLI7, native interoperability8 and OPT02 preservation3/79, plus strict Clippy. Its O3-AC01 connection finding and O3-AC02/03 evidence gaps are retained in the historical report and closed by the directed independent acceptance below; those counts are not new repair runs.

### O3-AC01/02/03 focused repair

| Finding | Current scoped result |
| --- | --- |
| O3-AC01 / G-12 | Original `EditFormOrigin` captures the creation connection; real drop/Add/form controls reject submit/Paste/both pickers after actual router replacement while the old disconnect remains queued. Same-connection actions pass; intake8/8. |
| O3-AC02 / G-13 | Each legitimate no-refresh event passes protocol validation and its terminal is installed exactly before unchanged-browser assertions. Six valid terminal shapes and separately named malformed rejection are covered; Add result group6/6. |
| O3-AC03 / G-23 | Actual Create exports for ordinary ZIP/split ZIP/split7z feed original Test Start and valid Test cleanup=false terminal. Nine combinations retain Create facts/Test outcome and hold the running slot; Locate/Retry/recycle remain refused. Result group4/4. |

Original Job111/111, OPT0322/22 and Step411/11 pass; groups overlap. Desktop all-target strict Clippy and scoped formatting pass. No C#/worker/writer/protocol/queue or native publication change; prior independent native results retain their original attribution. Stop at OPT-03.

**Directed independent acceptance:** fresh native Create-location LIST1/RUN1/PASS1/SKIP0 supplies the required current C# exports. Intake8/8, Add terminal6/6, post-Test4/4 including9 cleanup-false combinations, original Job111/111, Step411/11 and Desktop all-target strict Clippy pass. O3-AC01 original connection binding, O3-AC02 valid terminal installation/no-refresh and O3-AC03 Test-side cleanup conjunction/held slot are closed. Groups overlap and are not additive. G-12/G-13/G-23 now join the retained G-21/G-22 scoped acceptance; no physical interaction or new full native-matrix/performance acceptance is implied.

## OPT-02 execution matrix — 2026-11-02

| Gate | Scoped evidence | Status |
| --- | --- | --- |
| G-11 | Five-format worker matrix145 operations; native ZIP17/7z20/TAR14/TGZ14/TXZ14 scenarios; distinct source IDs, original decisions, complete future/case/reverse reservation, directory merge/type/no-op/new-empty | Passed |
| G-04 preservation | Decisions before incoming payload opens; zero candidate for merge-only/allSkip; one verified candidate/commit on changes; original verification remains | Passed for touched preservation only; OPT-05 optimization remains open |
| G-19 preservation | Native79 scenarios:43 publications/36 zero-publication outcomes, source bytes/cleanup/independent content; original native source/cancel/unknown/latecancel7/7; format preservation35/35 | Passed for touched boundary only |

Ordinary Create5/5, ZIP interaction3/3, ZIP64 local-header1/1, strict archive Clippy and C# build passed. Native LIST3/RUN3 and preservation LIST7/RUN7 had zero skips. [Full implementation, RED/GREEN, identity, replay and limitations](opt02-add-namespace.md).

**OPT-02 independent acceptance passed at its scoped stop point:** [Independent report](D:/Code/NxTools.Next-main/.trellis/tasks/11-02-archive-workflow-optimization/opt02-independent-acceptance.md). Fresh verification passed49 Rust test cases including145 worker operations; native LIST3/RUN3 produced79 scenarios with43 single publications and36 zero publications; original native preservation LIST7/RUN7 passed without skips. A separate actual-worker7z Add/Overwrite probe preserved Created/Accessed/Modified/readonly Attributes and exact content. G-11 and touched G-04/G-19 are accepted; G-00, OPT-00/01 overall closeout, complete G-04 and physical/Release gates remain open. OPT-03 was not started at that historical stop point.

## OPT-00/01 execution matrix — 2026-11-02

This matrix records the authorized slice, not the completion of all scenarios in later packages. Full results and raw trials are in [OPT-00/01 results](opt00-opt01-results.md) and [native metrics](opt00-opt01-native-metrics.jsonl).

| Gate | Current scoped evidence | Status |
|---|---|---|
| G-00 | Three completed before/after trials for ZIP read/drag, creation/each/split/post-Test/Add, solid7z/solidRAR5 and compressed TAR; 216 supplemental native jobs include actual identities, notification times, transport bytes and bounded memory samples | Incomplete: total source reads, internal format phase costs and block-decode counts remain missing |
| G-01 | Normal ZIP open one catalog/two revisions; read-only paths no edit qualification; standalone source-change refusal; optional edit-record budget fallback and real catalog budget refusal | Behavior regression passed; complete I/O accounting remains under G-00 |
| G-02 | Native selected1/2/16 revision6/6/6; exact output; source/session/ordinal, CRC/MAC, record/end/size and original cancellation/publication/cleanup preservation | Scoped compiled/native checks passed |
| G-06 | First accounting-only cursor removed; first consumer hash524288→262144, native opens2→1; original Seal, first/repeat tamper and actual byte-budget checks retained | Scoped native checks passed; timing overlap is not a latency speedup |
| G-15 | Five prompt kinds through original deferred window/ticket owner; duplicate/stale/terminal/disconnect; queued Pause refusal and single-member Stop | Production TestWindow/control regressions passed; physical UI not run |
| G-25 | Actual C# six-frame export through validate/job_notification/view; ACK-owned Stop, both ACK/Paused orders, prior pause, decline/close, new prompt, commit/terminal and cleanup-held slot | Touched production protocol/control regressions passed; unrelated later workflow scenarios unchanged |
| G-19 | Original native ZIP create/each/split and independent payload checks, capability/record/crypto preservation | Touched preservation checks passed; no new format or generic create-policy acceptance |
| G-24 | ZIP and four-format all/selected exact bytes, no solid-predecessor output; old browser source refusal, native conflict cancellation and late publication truth | Touched preservation checks passed; no destination-policy redesign |
| G-26 | Original source-bound browse, immutable ordinal/selection checks and readable-but-not-editable budget boundary | Touched preservation checks passed; no new browser/navigation acceptance |

OPT-00/01 remain in progress until the G-00 gap is resolved. Later independent OPT-02 and authorized OPT-03 evidence is recorded above. No physical Explorer/UI, HZ5/source recycling, package/install or Release acceptance is added.

## Performance evidence rules

Use the same fixture, build mode, host/worker pairing and operation route before/after. Record three completed trials and median for primary latency cases, retaining all results and cleanup states. Separate initial-run from already-used source/cache conditions; do not claim a cache-flushed cold run. Historical Bandizip/Debug measurements remain context, not the new baseline.

Report wall time, phase times, source/transport/cache read bytes, revision/catalog/outer/block passes, worker starts, actual outputs and bounded sampled memory. Instrument counters at real owners; do not infer passes from source regex or log per chunk. Generated outputs use fresh owned roots and direct byte/member/metadata comparison with independent fixture expectations. No new per-file fingerprint artifact system is required.

Work-reduction assertions below are mandatory. Primary before/after latency must also improve beyond observed run variation; unchanged/noisy timing is not a speedup result even when a counter improves. A regression or workload tradeoff stays visible in the same table. Functional correctness never trades away for timing. No fixed claim to match the reference's CLI milliseconds is made before comparable product measurements.

## Gates

### G-00 — Comparable baseline

**Coverage:** PERF-01, PERF-02, DRAG-06.

Measure browser open, Test, one-member materialize, ordinary selected/all Extract, one/multiple demanded drag members and representative Add. Retain source/build/operation identity, full counts and actual cleanup. A missing worker, zero selected tests, incomplete measurement or forced cleanup cannot produce baseline PASS. The original ZIP remains read-only and extracted code is not executed.

### G-01 — Open and qualification reuse

**Coverage:** PERF-01.

On ordinary eligible single-volume ZIP, capability discovery adds no independent pair of whole-source scans to browser open. The combined guarded open/list/qualification performs at most the original before/final pair of complete revisions, rather than four. Test/materialize/extract do not invoke unused edit qualification. Standalone qualification/unleased callers still reject changed source; capability refusals and immutable catalog metadata remain correct. Count real source reads and qualification invocations.

### G-02 — Selected-member revision does not multiply

**Coverage:** PERF-02.

Extract 1, 2 and 16 small members from the same large ZIP under actual ordinary native held-source authority. Full-source revision pass count is constant across the three selections. All output bytes, ordinal binding, CRC/MAC/end/size and source-before/final checks remain. Current ordinary output truth is final-name begin plus successful metadata/drain/close journal; controls cover unleased source change, stale browser source, wrong ordinal, corruption, cancellation before close and late cancellation after successfully closed output.

### G-03 — One format plan/spool

**Coverage:** PERF-03.

TGZ/TXZ list plus qualification and payload work reuse one verified outer spool in one operation; count outer decodes and check outer end/CRC. ZIP preflight/stream/edit reuse admitted ordinal data ranges instead of rediscovering local/descriptor facts. Wrong local names, descriptors, ZIP64/AES extras, overlap, uncertain encoding, metadata budget and cancellation still refuse/clean up. No spool/plan survives operation retirement as an authority-bearing disk/global cache.

### G-04 — Update validation and native observations

**Coverage:** PERF-04, PERF-06, ADD-05.

For ZIP rename/comment/Add/replace-as-currently-supported/delete, count payload validation: retained/new content receives one complete final-candidate decode; removed old content retains the current source-integrity refusal. Corrupt retained and corrupt removed members both reject. Additions with bad CRC/length, truncated payload or mismatched local records fail before native staging/publication. Original ZIP record/extra/encryption preservation and held readback remain.

Within one native edit stage, source/target digest reuse requires matching full identity/version of the protected object. Changed target, wrong lease/read-completion, cancellation, same-size content substitution, failed/unknown native commit and actual late success retain their original outcomes. Fresh final handoff validation is still executed; output postpublication checks are not removed to meet a pass count.

### G-05 — Directory plan complexity and semantics

**Coverage:** PERF-05.

A pure in-memory 65,536-entry catalog with up to 4,096 selected IDs exercises overlapping selected roots. Count visits/normalizations to demonstrate one catalog expansion pass plus bounded prefix/index construction, not selected-root × catalog scans. Verify stale session/ordinal, `a/` versus `ab/`, explicit/implicit/empty directories, slash/backslash names, case/collision policy, unchanged output membership and original ordinal order for extraction, directory drag and edit.

### G-06 — Verified size without a temporary cursor

**Coverage:** DRAG-03.

First content readiness opens no native cursor solely for actual-size charging and performs one fewer complete cache read than baseline. Known/unknown/zero descriptor size uses exact sealed bytes; no overspend across distinct/repeated requests. Actual target cursors still detect same-size tampering between commit/Seal/stream and each repeated open. Native publish/Seal/readback and unchanged-owned cleanup remain.

### G-07 — One demand-started retained context

**Coverage:** DRAG-01, DRAG-02.

Request only descriptors and cancel: zero workers/member output. Demand 1, 2 and 16 distinct ZIP members: at most one drag-scoped worker/source/catalog context, fewer if qualified preview output satisfies all requests. Repeated indices materialize once and return independent cursors. Out-of-order descriptor indices map to original ordinals. Undemanded members have no output, directories are descriptor-only and actual-byte accounting is once per distinct demanded member.

A member-ready result requires full relevant integrity/native close/Seal but is not Run cleanup. The active reservation stays occupied while the retained worker is alive; overall terminal follows actual process/pipe/Job/source cleanup. Idle receiver time, per-member work deadlines and the original overall drag deadline remain separate. Failure does not create hidden replacement workers.

Retain complete-subtree regressions including 33/256 files, 257 descriptors and 64 roots where existing metadata/transport/native representation admits them. No old logical count/512MiB drag cap is reintroduced.

### G-08 — Dependency-aware demanded content

**Coverage:** DRAG-02, PERF-03.

Use same-folder 7z, multi-block/empty-stream entries and solid RAR4/RAR5 fixtures. Descriptor order differs from archive and decoder callback order; include repeated and non-monotonic demands. Correct output is associated by original ordinal, not duplicate names or callback count. Count actual catalog, block/predecessor work and permitted resets. Never publish before required block/member integrity. Unselected predecessor decoding creates no member output. A declared backend limitation does not silently remove required format support or become an unmeasured one-decode claim.

### G-09 — Native receiver and stream lifetime

**Coverage:** DRAG-01, DRAG-05, DRAG-06.

Controlled consumers cover async accepted/declined, StartOperation before Drop returns, EndOperation success/failure, slow read, repeated cursor, target abort, Escape/source close/disconnect and cancellation during wait/data/verification. Source HWND/capture/button/connection gates remain; one STA/OLE loop and serial command responsiveness are observed. Returned streams remain usable until their target releases them even after session cancellation/retirement. No early overall success, freed live slot, orphan process or cache deletion of live target data.

### G-10 — Admission and preview reuse

**Coverage:** DRAG-04.

Count one validated native descriptor plan and top-level activity probe per admitted drag, while each actual worker still acquires its source authority. Large valid subtree/long-name freeze stays metadata-only. Preview-complete then same-member drag adopts the existing sealed lease without a second materialization; different source/snapshot/generation/ordinal or retired output cannot reuse it. Encrypted preview alone cannot authorize drag. Preview replacement/close releases only its consumer ownership; borrowed drag cursors and actual-byte limits remain correct.

### G-11 — Complete incoming namespace

**Coverage:** ADD-03, ADD-04, ADD-05.

For qualified ZIP/7z/TAR/TGZ/TXZ, add two different-root `report.txt` sources to a non-root internal directory. Ask must occur before payload source reads/encoding, not a generic early OutputCollision. Exercise Overwrite/Skip/AutoRename/Cancel/ApplyAll, incoming/original and incoming/incoming cases. Same-target directories merge; child conflicts remain coherent; different file/directory types refuse without deleting anything.

With old `same.txt` and incoming `same.txt` plus `same (2).txt`, keep the second source's requested name; generated names use the next free destination. Verify final content-to-name binding under reordered inputs, nested folders and case conflicts. Merge-only plus all children skipped has zero payload opens/candidate/publication and unchanged original bytes. A new accepted empty directory publishes exactly once. Original logical-directory evidence is not fabricated from future reserved incoming names.

Include encrypted ZIP and qualified encrypted/header-encrypted7z Add. ZIP old encryption/metadata remains exact and new members remain explicitly unencrypted Deflate; 7z keeps its qualified password/encryption rewrite rules. Wrong password/cancel is zero publication. Unsupported edit actions/formats retain explicit capability refusal; encrypted-read capability does not enable arbitrary editing.

### G-12 — Intake, frozen target and source editing

**Coverage:** ADD-01, ADD-02.

Toolbar Add, multi-file picker, repeated folder additions, window drop and Paste converge correctly. Drop offers explicitly named Open-first, New using the full source list and capability-gated Add. Test readonly archive, mixed files/folders, Cancel and stale current tab/snapshot/internal folder. Invalidated input never retargets or queues a stale Add.

Remove first/middle/last source, re-add it, duplicate selection, long names and the current external-root limit; removal changes only the draft and causes no I/O. Paste covers standard Explorer Copy/Cut-shaped data, HDROP-only, empty/invalid/bounded overflow, parser/native failure, disconnect and late response. Text-field Ctrl+V remains text paste. Cut leaves sources intact. Real system clipboard interaction is separate from synthetic data/adapter tests.

The required system multi-folder path additionally passes G-28. Repeated single-folder additions alone do not satisfy ADD-07.

### G-13 — Publish and select actual Add results

**Coverage:** ADD-05, ADD-06.

Independent parsing/content/metadata comparison verifies new files/directories, Overwrite, Skip, AutoRename, all-skip and partial-skip updates. Final staging/readback/reader-close/native publication occur once or zero for no-op/cancel. Precommit failures preserve original bytes; unknown/postcommit facts forbid blind retry.

Only matching committed/cleaned-up terminal refreshes the original browser. Restore the original internal directory or nearest surviving ancestor, select actual final members under NEW snapshot IDs and leave unrelated tabs untouched. Bounded outcome overflow shows truthful total/subset, not truncated business work. Member paths remain private display data and absent from diagnostics.

### G-14 — Private binary data and ordered drain

**Coverage:** PERF-07.

Symmetric Rust/C# fixtures keep the accepted acknowledged bulk contract for source reads, Create/Edit candidates/readback and retained cache output. Ordinary extraction separately covers negotiated/missing/malformed `extraction_output_v1`, <=32KiB record bounds, original anonymous-pipe backpressure, single-pump ordering, first native write failure, cancel/EOF and no mutation before agreement. No per-record ACK exists; begin/commit cannot be overtaken by payload, and commit reports drain+metadata+close truth. Waiting producers wake and original actual cleanup owns the reservation. Existing public framing/resources and other private operations remain compatible with their explicit contracts.

Measure transfer bytes/frames, CPU/wall contribution and memory on source reads, ordinary pipe sends, native close and candidate readback after source/plan savings are isolated. For N uncontended ordinary files require two synchronous JSON calls each plus operation overhead, `output_write_ack_frames=0`, `fingerprint_ms=0` and `postpublication_ms=0`. Reducing calls or JSON size alone is not an unmeasured total-performance claim.

**OPT-06 implementation verification:** negotiated raw source/output/held-partial transfer passes actual Rust worker8groups/35process scenarios, worker-related unit28 and95directed integration contracts. C# exact wire/legacy LIST13/RUN13/PASS13/SKIP0 and affected actual-native selections5/5+6/6+2/2 have zero skips/not_run. Before/after each complete9Jobs with equal native/payload work, one publication and actual cleanup; raw18samples show lower wire and complete-operation wall for these three Debug routes. Two-buffer bounds/zero modern conversion are observed; total/internal I/O, exclusive CPU/exact memory peaks and separate cleanup time remain unavailable. [Executed evidence](opt06-acceptance.md) supports G-14 and touched G-01/G-02/G-04/G-07/G-19/G-23/G-25 at the owned boundary, not independent/physical/Release acceptance.

**OPT-06 independent acceptance:** [Independent report](opt06-independent-acceptance.md) confirms the negotiated/bounded/ordered original-owner boundary and relevant preservation with95distinct integration passes,28worker units, C#13wire/legacy plus13actual-native selections and9current complete Jobs; all accepted native selections have0skip/not_run. The18retained samples/48statistical summaries and332source-file/current-delivery identity audit pass. G-14 is accepted scoped; overall/native-user/Release gates remain open.

The accepted OPT-06 reports remain historical for their measured implementation and continue to cover Create/Edit/readback/retained-cache directions. Current ordinary extraction OEO-01 through OEO-06 uses its separate [compiled and genuine parallel results](parallel-extraction-results.md), not the old acknowledged-write/held-partial result.

### G-15 — Background decisions and truthful controls

**Coverage:** FLOW-01, FLOW-02.

Background a single task, allow the original toast to expire, then deliver Password/Conflict/Risk/Destination/CreateOverwrite. The original prompt remains actionable through one recovered view and original ticket. Duplicate/stale/wrong-attempt, close, cancel, disconnect and terminal cannot duplicate responses or reopen retired work.

With two running and one queued member, queued Pause is disabled and queued Stop cancels only that local member. Started/paused/pause-pending/verifying/committing/cancelling/terminal states expose only actual controls. No synchronous cross-entity writeback or foreground-stealing loop.

### G-16 — Batch surface and memory-only password

**Coverage:** FLOW-03, FLOW-04.

A multi-archive submission creates one batch progress surface, not one window per queued member. Preserve original two active slots/FIFO and exact job/attempt identities; simultaneous member prompts can each be selected/responded to once. Details/background/close never cancel implicitly. Partial admission, failed/cancelled/skipped/cleanup-unknown members and repeated terminals remain truthful.

Batch password reuse requires explicit choice, isolates unrelated batches and invalidates wrong answers before a fresh prompt. Test different passwords, cancellation, disconnect, retry generation and terminal retirement. No secrets in persistence/Debug/logs; no conversion of a preview password into a broader OLE grant.

### G-17 — Safe retry, warning result and parent row

**Coverage:** FLOW-05, FLOW-06, FLOW-07.

Qualified single Test/full Extract receives the initial native baseline and a terminal Retry action. Exact zero successfully closed output, completed cleanup of any incomplete final-name output, eligible code and fresh native source/target revalidation are required. Repeated click creates one new attempt. Changed source/target, any completed/unknown output, modify/create/reclamation shapes cannot acquire generic retry.

A success-with-warnings result with trustworthy location/cleanup has manual Locate and warning counts, but no automatic foreground action. Failed/cancelled/unknown outcomes cannot guess a target. Backspace root is a no-op; parent navigation selects/focuses/reveals only the folder just left, with stale refresh guarded.

### G-18 — Production UI and real Explorer

**Coverage:** all user-facing requirements, including DRAG-01/05/06, ADD-01/02/06/07, CREATE-01/02/03/04/06, EXTRACT-01, FLOW-01 through FLOW-11 and BROWSE-01/02. CREATE-05 native effects have their separate G-29 authorization.

Production GPUI default/minimum, Light/Dark, long source/member names, pointer/keyboard, disabled/pending/error and fixed-footer behavior use current official components and one callback owner. Exercise batch decisions, Retry, warning-result location, parent selection and Find query/result actions as required workflows. Verify true picker/clipboard cancellation and focus under relevant authorization. Screenshots/imports/headless semantics do not replace interaction acceptance.

Exercise the complete Create form, each-input output review, overwrite cancellation, post-create Test status and actual-result Locate. Running Stop confirmation must restore prior paused/running state on refusal without stealing focus from a newer prompt/tab. Multi-folder system selection and required MD/TXT/image preview retain original native/window ownership.

After explicit permission, use fresh disposable targets for actual Explorer single-file, multi-file and directory drag; include negotiated receiver observation, slow materialization, target abort and ordinary application commands while drag is active. Record all four drag intervals, target responsiveness, byte-correct destination and resource cleanup. Add into root/non-root through picker/drop/paste and verify visible final selection. Supported DPI/mixed-monitor and user visual/focus verdict remain separately recorded; unavailable rows stay pending. Controlled async consumers are not substituted for the reported Explorer stall.

### G-19 — Retained product and data guarantees

**Coverage:** PERF-01, PERF-02, PERF-03, PERF-04, PERF-05, PERF-06, PERF-07, DRAG-01, DRAG-02, DRAG-03, DRAG-04, DRAG-05, DRAG-06, ADD-01, ADD-02, ADD-03, ADD-04, ADD-05, ADD-06, FLOW-01, FLOW-02, FLOW-03, FLOW-04, FLOW-05, FLOW-06, FLOW-07, FLOW-08.
The same data/ownership gate applies to ADD-07, CREATE-01 through CREATE-06, EXTRACT-01/02, FLOW-09/10/11 and BROWSE-01/02. Required source recycling cannot pass until G-29's original native capability is resolved; source/static refusal evidence is not production execution.

Retain affected original archive/codec/ZIP-record/encoding/path/source/output/metadata/MOTW/queue/interaction/picker/preview/drag/readmission/cleanup tests plus scoped strict quality gates. Ordinary destination output now proves final-name creation, close completion and incomplete-output deletion rather than partial rename/fingerprint publication; Create/Edit publication, HZ5 and cache tests retain their original semantics. Preserve already repaired backslash/UTF-16/root/picker correlation/contextual Extract/double progress/recovery-object behavior. No wider format support, source sharing/recycle, destructive HZ5 authority, dependency/vendor/theme or unrelated feature change.

Ordinary extraction/create/edit source/output checks remain at their real owners. HZ5 remains default-off with independent consent, qualified scope and fresh continuation; shared ordinary-source proof never applies after mutation. Actual native mutation/recycle/crash actions retain separate effects authorization. Known unrelated failures are not erased by a scoped pass.

### G-20 — Required archive-wide Find

**Coverage:** FLOW-08.

Search names and full internal paths across several loaded folders using substring and supported wildcard rules. Opening/changing a query performs zero archive source reads/decoder starts. Keep current-folder Ctrl+F behavior and the separate archive-wide menu/Ctrl+Shift+F entry. Test empty, no-match, invalid query, duplicate names, slash/backslash paths, explicit/implicit directories and a large virtual result projection.

Results retain original source/snapshot/ordinal. Enter/double-click navigates to the owning folder and selects the actual member; selected extraction/edit reuse original commands/capability and final current-source guards. Changing query does not clear unrelated browser selection. Query replacement, refresh, source change, tab retirement and close invalidate old results without cross-entity reentry. Verify bounded metadata/rows, focus/keyboard, result actions and no payload/query logging. Find remains required even when scheduled after the core performance packages.

### G-21 — Create intake and real option mapping

**Coverage:** CREATE-01, CREATE-02, ADD-02.

Start New through module/browser, chosen New-from-all drop and supported CLI/Shell intent routing. Use files, folders, mixed sources and the current root limit. Existing Remove first/middle/last/re-add works locally; source count and total/unknown size are truthful; duplicate/overlap, output self-inclusion, excluded files, root stripping and links retain current policy. No recursive I/O occurs in render. Cancellation/late picker/Paste responses leave no submitted or retargeted draft. G-28 covers one multi-folder system selection.

Round-trip each admitted create family with independent member/byte expectations: ZIP Store/Deflate and AES-256,7z LZMA2 solid/non-solid with admitted encryption/header-encryption, TAR/TGZ/TXZ, regular-file GZ/XZ. Test semantic levels, disabled combinations, password mismatch/show-hide/retirement, UTF-8 names, exclusions and permitted split options. Format changes and final submit use the same validation. GZ/XZ folders refuse; several regular files require each-input mode. No RAR/SFX/reference-only option is enabled by this gate.

### G-22 — Ordinary, each-input and split output names

**Coverage:** CREATE-03, FLOW-03.

Ordinary single-target creation confirms overwrite with original native observation; refusal is zero output and unchanged old bytes. Target replacement while waiting is freshly rejected/reconfirmed under the existing owner. Same-basename source roots still cannot collide inside one ordinary new archive.

Each-input mode with different-root `report.txt` sources plus `report (2).txt` plans all output names before submitting workers. Reserve all original names and disk names, then allocate free suffixes without consuming later requested names. Reordered inputs retain correct source-to-output mapping; show resulting names. Include folders, GZ/XZ files, existing destination outputs, a newly occupied target before native admission and full queue/partial admission. Never overwrite a prior batch member, silently drop a source or create a second scheduler. Each member records its own attempt/result; final unified surface additionally passes G-16.

Split ZIP/7z only use admitted configurations; family name collisions/partial existing families refuse under current policy. Independent open/test/extract checks the created family and actual member naming. Cancel or native failure records each volume's actual published/unknown state, with no invented whole-family rollback or atomicity.

### G-23 — Create publication, post-Test and result location

**Coverage:** CREATE-04, CREATE-06, PERF-07, FLOW-06.

Inject source change, short source read, encode failure, invalid candidate/readback, disk-full/native error, cancellation before commit, unknown commit and late cancel after actual publication at the original owners. Precommit failure preserves prior target bytes and removes only owned partials; committed output stays reported. Direct output/member/metadata comparison is independent of encoder success.

With post-create Test off/on, assert candidate verification → publication → actual creation cleanup → optional Test → logical terminal. Post-Test failure or cancellation retains the created commit journal/counts and `output_published=true` when publication was confirmed, rather than adopting Test's no-output false. No generic Retry/recycle grant appears. The original native-created location survives successful Test; missing location remains missing, never reconstructed from a request. Confirmed success/warnings with completed cleanup supports manual Locate; failed/cancelled/unknown remains gated. Test disabled is not permission to skip required candidate integrity/readback.

Observe the real runner→JobSession→Rust terminal merge→Locate handler. A UI-only fabricated location does not pass. Include ordinary and split output families, each-input members, changed/missing result at action time and native Shell failure. Location presentation failure does not turn successful creation into failed creation. Measure source-transfer/encode/candidate-check/post-Test/native-publish/cleanup separately for the same before/after fixtures.

### G-24 — All/selected extraction and destination preservation

**Coverage:** EXTRACT-01, EXTRACT-02, PERF-02, PERF-03, PERF-05.

Use all and selected members, explicit/implicit/empty folders, overlapping roots, single root file/directory and multiple roots with Here/Name/Smart/custom/Desktop. Preserve the current selected-plan Smart shape, single-file-folder setting and non-colliding archive-directory suffix. Contextual Extract retains its source and selection; changed source/snapshot refuses. Existing source chooser and destination picker failures/cancel/late results clear original pending state and never strand or retarget the form.

Check missing-target creation, insufficient/unknown capacity and finite risk/readmission decisions at their native/worker owners. Across admitted ZIP/7z/RAR/TAR/compound/stream paths, outputs match exactly; unselected predecessors produce no output, unknown totals stay unknown, links/special/unsafe paths retain refusal/skip policy. Verify commit-carried timestamps/attributes/MOTW and directory finalization. Mid-operation error/cancel preserves successfully closed files/directories and deletes only incomplete owned final-name output(s) still pending required integrity; no fingerprint rollback and no selected/skipped/failed task recycling. Record ordinary all/selected performance independently from preview/drag latency.

### G-25 — Real phase events, decisions and confirmed Stop

**Coverage:** FLOW-09, FLOW-10, FLOW-11, FLOW-01, FLOW-02.

Serialize actual C# progress for `running`, `edit_planning`, `edit_reading`, `edit_reencoding`, `edit_verifying`, `edit_committing`; pass it through Rust `ArchiveJobEvent::validate`, `job_notification` and the production progress view. All supported phases arrive; malformed/unknown phase, wrong attempt/generation and retired events are rejected. This test must fail while Rust accepts only `running`. Direct view snapshot injection is insufficient.

Current member and overall progress can differ. Exercise unknown/zero sizes, solid dependency work, member switches, encode→verify→commit→cleanup and post-create Test. Metrics reset/pause through the existing phase/attempt owner, prompts/terminals stay lossless and bounded, latest progress remains coalesced. No fake100%, destination-write claims or per-chunk diagnostics. Finish-window retention/background behavior does not alter terminal facts.

Running Stop acquires at most one confirmation. If pause is legal, use the actual acknowledgement: running→temporary paused→decline returns running; previously paused→decline stays paused. Close/Escape means decline, not Cancel. Confirm sends one Cancel and awaits original cleanup. Pause failure, non-pausable commit, simultaneous password/conflict, terminal during confirmation, background/close/disconnect and stale resume cannot change a newer attempt. Queued Stop remains direct/local and affects one queued member only.

Password retry preserves all task options, distinguishes confirmed WrongPassword from ambiguous authentication/data failure and clears secrets on retirement. Conflict display shows current native/member facts and proposed AutoRename. ApplyAll is current-attempt policy; old/double tickets and two simultaneous jobs cannot cross-answer. Include show/hide, cancel, timeout and original focus restoration.

### G-26 — Open, browse, encoding and catalog projection

**Coverage:** BROWSE-01, FLOW-07, FLOW-08, PERF-01.

Open admitted content through original module/browser/launch routes; unsupported content, corrupt metadata, missing password, cancel and source change have distinct states. Open and edit capability remain separate: a readable archive never gains an unsupported write action. After refresh/encoding reread, new snapshot IDs replace old actions and the original location or nearest surviving ancestor is restored according to current navigation policy.

Exercise tabs, Tree, details/name-list, sort, current-folder filter, multi-selection, status and existing history; no new history model is required. Use a large in-memory catalog to measure projection/rebuild work and bounded rendered rows separately from source admission; queries/navigation do not reread archive payload. Refresh/close/tab replacement invalidates old selections/Find actions without synchronous entity reentry. Actual default/minimum/keyboard/focus evidence is G-18.

### G-27 — Preview and explicit entry lifecycle

**Coverage:** BROWSE-02, DRAG-04.

Automatic MD/TXT/image preview follows both keyboard and pointer selection. Exercise loading/empty/unsupported/password/failure, A→B→A, rapid selection, refresh, encoding change, tab switch, close, disconnect and decode completion after view retirement. Only current generation/snapshot/projection/ordinal installs content; every original lease is released exactly once after its consumers retire. Password-dialog dismissal restores current tab focus without a late focus steal.

Internal preview never executes an entry; explicit associated Open/OpenWith/nested-open retain current confirmation, capability and temporary-lease policy. No automatic edit writeback is introduced. G-10 proves exact-match preview-to-drag adoption, independent encrypted-drag authorization, per-cursor validation and transferred-stream survival. Existing safe preview behavior must not be replaced with the reference's weaker index-only publication.

### G-28 — Required system multi-folder intake

**Coverage:** ADD-07, CREATE-01, ADD-01.

Select2,16 and the currently admitted maximum number of folders in one owner-bound system dialog for New and Add. Preserve ordinary file multiselect and repeated folder additions. Test cancel/owner close/native failure, duplicates/overlaps, mixed existing source draft, long paths, over-budget vector, late result, tab/source/folder changes and disconnect. The result installs once in the original draft or refuses completely with a reason; no silent truncation or retargeting. Remove/re-add any folder without source mutation. Real system focus/keyboard/DPI checks are separately authorized G-18; synthetic vectors alone do not certify native multiselect.

### G-29 — Required source recycling and its native prerequisite

**Coverage:** CREATE-05; original source-preservation contracts also apply to EXTRACT-02.

**Current prerequisite: unresolved original-object production recycle capability.** Read the owning Rust/native source-deletion contracts before implementation. No enabled action, actual source mutation or PASS is authorized by this document. Source/static tests may verify explicit unavailable/refused presentation and zero backend calls, but this does not close the required production workflow.

When the owning capability decision and effects authorization exist, require explicit consent, successful required Test and continuity of original eligible source objects/version through the original lifecycle. Turning on recycling with Test off asks to enable Test; declining cannot grant deletion. Reacquiring source paths after post-Test is not continuity. Directory roots/exclusions/temporary sources, selected/skipped/failed tasks, changed source, unknown/postcommit qualification failures, stale/replayed/wrong-operation evidence and already consumed grants all refuse.

Use exact audited native fixture selection for complete output success before testing separate recycle outcomes: Recycled/Cancelled/Unsupported/Failed/OutcomeUnknown/NotAttempted, mixed per-source results, bounded STA occupancy, cancellation before/in-flight and loss of acknowledgement. Never permanently delete, weaken sharing or retry an unknown recycle result. Archive output remains successful/intact when recycling fails; diagnostic correlation/native error comes from its real owner without names/paths/secrets. Real production acceptance remains pending until the original-object guarantee is proved, separately authorized and exercised; a recording backend is not a substitute.

## Closeout record

Each gate records exact fixture/build/command/effects, expected versus actual result, original failing boundary where relevant, before/after counts/time, output comparison and cleanup. Separate source, compiled, controlled-native, real Explorer/UI/DPI/user and release statuses. Pending/skip/unknown is never counted as PASS.

Acceptance requires all 45 rows in the [complete delivery coverage](D:/Code/NxTools.Next-main/.trellis/tasks/11-02-archive-workflow-optimization/requirements-map.md), G-00 through G-29 and measurable outcomes across all five workstreams. Batch, retry, result location, Backspace, Find, preview reuse and multi-folder intake are required even when scheduled later. Unavailable real-target evidence leaves G-18 pending; unresolved production recycling leaves G-29 pending. Independent completed packages may be reported, but neither pending gate can be silently omitted from overall acceptance.
