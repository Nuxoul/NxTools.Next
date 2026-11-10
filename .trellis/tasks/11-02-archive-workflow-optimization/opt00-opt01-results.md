# OPT-00/01 — implementation and measured results

Date: 2026-11-02. Parent: **Phase E — Format expansion and ZIP editing**. OPT-02 through OPT-07 remain not started.

## Scope and evidence

Authorized effects are product changes, compilation, automated tests and isolated owned native fixtures. No packaging, signing, installation, service/registry changes, real Explorer/product GUI interaction, user archive mutation, source recycling, HZ5 mutation, commit or push was performed.

All seven OPT-01 changes have compiled regression evidence; shared reads and drag also have controlled-native evidence. G-00 remains incomplete at the internal read/decode measurement boundary. Physical UI and release acceptance remain separate.

## Task controls and editing progress

- Background Password/Conflict/Risk/Destination/CreateOverwrite prompts restore the unique original progress window on the next application turn. The callback rechecks job/attempt/generation, connection and the original ticket. Duplicate, terminal and retired events cannot revive a task or submit a reply.
- Queued jobs cannot Pause. Queued Stop locally cancels only that queue member. Running controls use admitted worker state, pending receipts, pause, verification/commit, cancellation and terminal facts.
- Stop confirmation remains in the original JobRecord/JobProgressView and Root Dialog. Matching successful Pause receipt **and** actual Paused state confer temporary-pause ownership, regardless of arrival order. Decline or closing the window resumes only that acquired pause. A previously paused task stays paused; confirm sends one Cancel and waits for original cleanup. A new prompt, commit transition, retired attempt or terminal invalidates the question without answering another ticket.
- The released Kit0.8.0 Root/DialogFooter/DialogAction/DialogClose slots and Medium Button are retained. The continue action is outlined and is not routed through Toolbar's ghost/compact adaptation. Entity writes from button callbacks are deferred. Keyboard confirmation uses the original armed key-release/Confirm path.
- `running`, `edit_planning`, `edit_reading`, `edit_reencoding`, `edit_verifying` and `edit_committing` are the explicit C#/Rust progress vocabulary. The original C# Operation/OnFrame channels serialize six real events; Rust tests consume those exported payloads through `validate → job_notification → JobProgressView`, checking actual values and Chinese phase labels. Invalid phases/member facts remain rejected.
- Low-noise `prompt_restored`, `stop_question`, `stop_answered` and phase-transition diagnostics retain job/attempt/generation and original control receipts. No path, password, payload or progress-tick logging is added.

## Shared reads

- Internal catalog-only admission is used by Test, extraction, materialization and password-format discovery. Public browser open retains edit capability discovery.
- Ordinary ZIP browser admission shares its open source and ephemeral central-directory records with the existing structural/record-preservation checks. Initial and final whole-source revisions enclose both operations. Public independent qualification retains fresh source-change detection.
- A private `NativeReadLease` is constructed only after successful native `source_acquire`, borrowed through password attempts, and restricted to ordinary single-source reads. It is not a request field. Public extraction receives no proof. Reclamation, modification and volume sets cannot receive this optimization.
- Protected ZIP extraction omits loop-level whole-source scans. Original session/ordinal, local/central/descriptor, codec end, CRC/MAC, actual size, cancellation, metadata, native commit and initial/final source checks remain.
- Optional captured edit records retain their metadata charge. If that optional capture exceeds the budget, browser admission retries the original catalog-only listing and reports `UnverifiedMetadata` for editing. A readable catalog remains readable; a genuine catalog budget failure still refuses. The before/final revisions remain, and the normal eligible path still loads one catalog.

### Pairing and fixture

Both sides use Debug `isolated-codecs` workers and the original C# Read/Job/Entry → worker → native source/output/cache routes. Before worker: 11,992,064 bytes, UTC 2026-11-02T03:35:49.8681502Z. After worker: 11,999,232 bytes, UTC 2026-11-02T03:50:44.597617Z. The host/test assembly is built from the current workspace; the after measurement runs a frozen copy of that Debug test package to avoid concurrent build replacement.

The same retained `sixteen.zip` is used before/after: 20,973,276 archive bytes, 17 members, 16 selectable 262,144-byte members and one unselected 16-MiB member. Total payload is 20,971,520 bytes. Every trial checks exact fixture/output bytes. The source is validated before measurement: trial 1 means first measured use, not a cache-flushed cold run. Trials 2/3 reuse the source with fresh owned outputs. No system cache flush occurs.

### Wall time

Milliseconds; each cell retains all three completed trials. Operations are not summed.

| Operation | Before trials | Before median | After trials | After median |
|---|---|---:|---|---:|
| Browser open | 1664.9688, 1607.9311, 1573.9699 | 1607.9311 | 886.7548, 799.3294, 878.7776 | 878.7776 |
| Test | 3786.9140, 3850.2606, 3866.8218 | 3850.2606 | 2239.0768, 2143.7526, 2329.4806 | 2239.0768 |
| Materialize one | 3153.4347, 3031.1308, 3150.5412 | 3150.5412 | 1659.6527, 1538.7672, 1628.9407 | 1628.9407 |
| Extract all | 14994.7011, 15222.0045, 14853.7274 | 14994.7011 | 6426.4303, 6510.6588, 6839.9471 | 6510.6588 |
| Extract selected 1 | 4576.3179, 4627.1075, 4651.9008 | 4627.1075 | 2207.6178, 2269.5498, 2236.7031 | 2236.7031 |
| Extract selected 2 | 4966.6133, 5093.4705, 5089.1143 | 5089.1143 | 2260.7599, 2308.6391, 2235.6336 | 2260.7599 |
| Extract selected 16 | 10936.1909, 11161.2450, 11195.2127 | 11161.2450 | 3665.1157, 3232.7522, 3262.0011 | 3262.0011 |

All seven observed before/after timing ranges are disjoint on this fixture. This is a Debug controlled-native workload result, not general format, physical Explorer or Release throughput acceptance.

### Actual work

Each operation starts one worker. Values are the same in each side's three trials.

| Operation | Revision before → after | Catalog before → after | Eligibility before → after |
|---|---:|---:|---:|
| Browser open | 4 → 2 | 2 → 1 | 1 → 1 |
| Test | 10 → 6 | 4 → 2 | 2 → 0 |
| Materialize one | 8 → 4 | 2 → 1 | 1 → 0 |
| Extract all | 28 → 6 | 4 → 2 | 2 → 0 |
| Extract selected 1 | 12 → 6 | 4 → 2 | 2 → 0 |
| Extract selected 2 | 13 → 6 | 4 → 2 | 2 → 0 |
| Extract selected 16 | 27 → 6 | 4 → 2 | 2 → 0 |

Revision bytes are measured at actual revision reads and equal the count above × 20,973,276. They are **revision-only**, not total source I/O. ZIP outer decode observations are zero. Terminal observations are bounded aggregate numeric fields correlated by the original operation ID; no per-chunk logs or payload data are emitted.

Each successful extraction checks native committed counts, exact independently generated payload bytes and absence of partials. Test verifies all 17 members without output. Materialization releases its original cache lease. After every trial, the source reopens exclusively for write, proving the native lease has ended. Helpers and owned outputs are removed; only the explicitly retained baseline ZIP remains.

## Drag actual-size accounting

`Lease.Seal` returns the actual stamped size after full verification and manifest persistence. The original completion owner carries this size in internal `MaterializationCompletion`; the public DTO is unchanged. `ArchiveVirtualContent` charges sealed bytes once per materialization and no longer opens an accounting-only native stream. Every real consumer cursor retains independent position and complete object/content validation.

Three trials use two 256-KiB members plus an empty member, unknown/known/zero descriptors and a repeated first-member request. RED and GREEN use the same frozen before worker, isolating this C# change.

| Observation | Before | After |
|---|---:|---:|
| First demand Seal hash bytes | 262144 | 262144 |
| First demand consumer hash bytes | 524288 | 262144 |
| First demand native cursor opens | 2 | 1 |
| All demands Seal hash bytes | 524288 | 524288 |
| All demands consumer hash bytes | 1310720 | 786432 |
| All demands cursor checks/opens | 7/7 | 4/4 |

| Trial | Before first ms | After first ms | Before all ms | After all ms |
|---|---:|---:|---:|---:|
| 1 | 303.790 | 299.087 | 680.890 | 650.413 |
| 2 | 233.534 | 201.538 | 604.806 | 557.937 |
| 3 | 200.897 | 232.017 | 510.656 | 572.720 |
| Median | 233.534 | 232.017 | 604.806 | 572.720 |

Timing ranges overlap: this establishes work reduction, not a drag latency speedup. The integrated after-worker run also passes accounting, first/repeated same-size same-time tamper rejection, false-zero/unknown-size actual budgets, independent cursor lifetime and retirement cleanup. Modified cache files are retained by the original policy; unchanged owned fixtures are removed.

## Creation, explicit post-Test, Add and split baseline

The source payload is 150,000 independently generated bytes (`Random(7123)`), reused across three trials with fresh output directories. Each-input is two original independent jobs, not a new batch runtime. The post-Test row is a separate actual Test job after native creation cleanup; it is not inferred from `TestAfterCreate=true`.

Split admission compares the activity reservation against the **requested held volume**, not decoder `Primary`. A three-volume ZIP's decoder order begins at `.z01`, while a request for `.zip` reserves the last volume. The original regression failed `source_acquire` before any revision/catalog work. `ArchiveVolumeSources.Requested` retains the matching already-held object; the runner preserves the complete family validation and compares its identity without reopening a path. Three-volume Test and Extract now complete, and the existing missing-volume, write/parent protection and case-variant `.z01` gates pass 2/2.

Both comparison sides use this prerequisite-corrected C# host; before/after selects the frozen workers listed above. Each side completes 27 jobs (9 operations × 3 trials), with zero skips and exact independent output checks. The three published ZIP volumes are 65,536 / 65,536 / 27,329 bytes. Split Test verifies one 150,000-byte member without publication; split Extract publishes exactly that payload. Add replaces only the owned created archive, with two independently verified members. All helpers, partials and successful output trees are removed; source leases are exclusively reopenable afterward.

| Operation | Before trials ms | Before median | After trials ms | After median |
|---|---|---:|---|---:|
| Create | 640.1806, 240.7572, 228.7359 | 240.7572 | 342.3813, 243.4640, 212.1728 | 243.4640 |
| Each-input 0 | 232.1208, 249.4734, 230.2816 | 232.1208 | 223.8867, 218.1229, 215.5821 | 218.1229 |
| Each-input 1 | 231.3009, 231.4984, 230.5062 | 231.3009 | 230.7957, 231.8730, 185.2083 | 230.7957 |
| Create with post-Test flag | 246.0985, 198.8888, 198.1846 | 198.8888 | 231.0427, 216.1858, 232.4947 | 231.0427 |
| Explicit post-create Test | 183.9577, 168.2780, 155.2701 | 168.2780 | 137.6199, 138.7530, 168.3177 | 138.7530 |
| Add | 353.7040, 278.8496, 294.6432 | 294.6432 | 338.5874, 312.3059, 279.0056 | 312.3059 |
| Create split | 355.2447, 275.4919, 264.3944 | 275.4919 | 323.2437, 277.9753, 247.8564 | 277.9753 |
| Split Test | 149.4015, 164.1332, 184.1437 | 164.1332 | 167.0261, 153.2894, 134.1708 | 153.2894 |
| Split Extract | 222.4369, 215.4359, 184.5151 | 215.4359 | 249.0677, 234.0754, 200.3487 | 234.0754 |

These small-workload ranges overlap except each-input 0; creation has no intended implementation speed change, and no general creation/Add speedup is claimed. All operations start one worker. Ordinary/each/post-flag create retain revision/catalog/eligibility 0/1/0; split create retains 0/5/0. Explicit Test changes 10/4/2 → 6/2/0; Add changes 10/10/2 → 6/8/1 through shared admission. Split Test retains 24/5 revisions/catalog and split Extract 30/5; both remove two unused eligibility calls. No protected per-member shortcut applies to volume sets.

## Instrumented baseline supplement

[Native measurement data](opt00-opt01-native-metrics.jsonl) preserves 216 completed operations: read 21 and creation 27 per side, plus four formats × five operations × three trials per side. Each side uses the same current Debug C# host and the frozen before/after worker identities above. This is a separate measured batch, not a replacement for the earlier raw trials.

The original runner accumulates actual stdin/stdout JSON frame bytes and publishes them once at terminal. These include encoding overhead and are not source file bytes. Existing resource-probe collectors sample the host and the worker obtained from the runner's retained native handle, verifying creation time and image identity. The requested interval is 20 ms; actual gaps, failures and possible missed peaks are retained. Every measured worker has valid samples, with minima of 38/21 for read before/after, 3/2 for creation, and 2/1 for the small format workloads. Notification timestamps and adjacent intervals describe the actual runner lifecycle, not exclusive codec/native phase costs.

| Operation | Supplement before median ms | Supplement after median ms |
|---|---:|---:|
| Browser open | 1777.9156 | 913.1176 |
| Test | 3983.3032 | 2465.7023 |
| Materialize one | 3183.5465 | 1717.4910 |
| Extract all | 14765.5772 | 7257.9006 |
| Extract selected 1 | 4490.5385 | 2476.8149 |
| Extract selected 2 | 4979.9115 | 2524.7045 |
| Extract selected 16 | 11841.5587 | 3473.8228 |
| Create | 231.1755 | 243.0869 |
| Each-input 0 | 199.9004 | 243.8094 |
| Each-input 1 | 233.0617 | 247.1830 |
| Create with post-Test flag | 225.4239 | 244.4624 |
| Explicit post-create Test | 171.4975 | 183.8754 |
| Add | 325.5729 | 324.6634 |
| Create split | 294.6711 | 304.7996 |
| Split Test | 176.3383 | 180.8216 |
| Split Extract | 230.0200 | 221.6708 |

All seven ZIP read timing ranges remain disjoint in this batch. All creation/Add ranges overlap; their medians and tradeoffs are retained without a general speedup claim.

### Solid and compressed-TAR coverage

The same-block 7z fixture is the attributed sevenz-rust2 0.23.0 `solid.7z` (2419 bytes; two files of 3447/2654 bytes). Solid RAR5 is libarchive commit `79c571f845dbdd250d69bff9d93ae034f674135e`, `test_read_format_rar5_multiple_files_solid.rar.uu` (677 bytes; four 4096-byte files), with its original encoded data and license retained. Python's standard library creates two deterministic 65536-byte USTAR members in TAR.GZ/TAR.XZ (1030/472 archive bytes). Windows bsdtar/libarchive 3.7.7 independently decodes all expected payloads before measurement. Preparation is reproducible through `tests/performance/archive-resources/prepare-opt-formats.py`.

| Format / operation | Before median ms | After median ms |
|---|---:|---:|
| Solid 7z / open | 150.1981 | 100.9416 |
| Solid 7z / Test | 139.6455 | 125.1185 |
| Solid 7z / materialize last | 157.2069 | 121.0287 |
| Solid 7z / extract all | 165.0062 | 132.0267 |
| Solid 7z / extract selected last | 166.4596 | 121.5488 |
| Solid RAR5 / open | 118.2304 | 122.5632 |
| Solid RAR5 / Test | 103.8374 | 139.3161 |
| Solid RAR5 / materialize last | 156.2102 | 125.1818 |
| Solid RAR5 / extract all | 135.0968 | 152.8272 |
| Solid RAR5 / extract selected last | 137.8242 | 132.1033 |
| TAR.GZ / open | 135.7641 | 104.4023 |
| TAR.GZ / Test | 139.8118 | 104.0076 |
| TAR.GZ / materialize last | 140.4531 | 157.9386 |
| TAR.GZ / extract all | 151.4479 | 134.6596 |
| TAR.GZ / extract selected last | 155.4920 | 137.5620 |
| TAR.XZ / open | 179.7957 | 183.5194 |
| TAR.XZ / Test | 199.1549 | 188.0090 |
| TAR.XZ / materialize last | 235.3549 | 190.4677 |
| TAR.XZ / extract all | 244.0963 | 211.6223 |
| TAR.XZ / extract selected last | 245.7656 | 194.9480 |

Each side completes all 60 operations, zero skips. Every output is compared directly with independent expected bytes. Last-ordinal materialization and selection do not publish unselected solid predecessors. Test produces no output; native counts, cache release, partial/helper/output cleanup and source exclusive reopen all pass. Most timing ranges overlap; isolated disjoint ranges on these tiny workloads do not establish a general format acceleration. Block and total source-read work are not inferred from these timings or from format metadata.

The owned pre-fix creation diagnostic output tree (23 files) was removed after the corrected split baseline completed; its source fixture, ownership marker and measurement logs remain. Successful baseline output trees and helpers are removed. Retained inputs/independent expectations and policy-retained modified cache fixtures remain explicitly owned.

## Executed checks

All Cargo commands set `CARGO_BUILD_JOBS=1`, `CARGO_INCREMENTAL=0`, `RUSTFLAGS=-C codegen-units=256` in the same invocation, using one Cargo lane. C# selections use full unique names, exact LIST followed by the identical RUN selection; the harness does not expose a `--exact` flag. Owned native runs set `TEMP` and `TMP` to the session scratch root and explicitly select the frozen worker.

| Check | Result |
|---|---|
| `read_work_contract` | Original RED: 2 failed / 1 preservation passed; optional-record budget RED: 1 failed; final 4/4 passed, including real catalog limit preservation |
| Native `archive OPT01 native shared read revision work` | Original RED at shared browser catalog; after 1/1 passed with 1/2/16 revision constant |
| Native `archive OPT00 completed read baseline` | Before 1/1 and after 1/1 passed; 21 completed measured operations per side |
| `cargo test -p nxtools-archive --features isolated-codecs --test zip_edit_contract --test zip_boundary_contract --test zip_crypto_contract --test worker_extraction_contract --test output_readmission_chain --test zip_volume_contract -- --test-threads=1` | 32/32 passed; structure, crypto, source, output readmission, volume routing and record preservation |
| C# `archive OPT01 CSharp production editing progress wire` | Original RED rejected unknown-phase assertion; explicit phase whitelist GREEN 1/1 |
| C# `archive extraction dual progress fixture wire` | 1/1 passed |
| `dotnet build csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-restore` | Passed, 0 warnings/errors |
| Drag accounting / actual first repeated cursor tamper / actual remaining budget cleanup / cache cursor integrity lifetime | 4/4 passed, zero skips, same before worker |
| Integrated native conflict global admission cancel / cancel after publication native facts / four drag gates above | 6/6 passed, zero skips, after worker |
| Native `archive OPT00 completed creation baseline` | Prerequisite-corrected host: before 1/1 and after 1/1 passed, 27 jobs per side |
| Native all-volume protection/missing + single ZIP exact disk count | 2/2 passed, zero skips |
| Final Rust engine / extraction-plan / worker-extraction / ZIP boundary / crypto / edit contracts | 46/46 passed after the budget/context changes |
| Desktop `archive::application::jobs::tests` | 76/76 passed, including 15 OPT01 cases and actual exported C# progress payloads |
| Desktop `archive::progress::` | 15/15 passed |
| Protocol library tests | 101/101 passed |
| `cargo clippy -p nxtools-archive --all-targets --features isolated-codecs -- -D warnings` | Passed |
| `cargo clippy -p nxtools-desktop -p nxtools-protocol --tests -- -D warnings` | Passed, including dependencies |
| Instrumented read / creation baseline | Per side: 21/21 and 27/27 completed native operations, zero skips |
| Native `archive OPT00 completed format baseline` | Before 1/1 and after 1/1; 60/60 operations per side, zero skips |
| Final current-worker shared-read / browser binding / drag accounting / first-repeat tamper | 4/4 passed, zero skips |
| `python tests/architecture/spec-contracts.py` | Passed: 120 documents / 14 owners; documents only |
| `python tests/architecture/gpui-release-contracts.py` | Passed: coherent locked graph, two licensed vendor deltas |
| `python tests/architecture/markdown-editor-contracts.py` | Passed: pinned provenance and production seams |

C# progress export is produced by the original `Operation` → `OnFrame` → lossless edit `Events` or coalesced `running` `Progress` channel → production JSON serializer. Supported phases are `running`, `edit_planning`, `edit_reading`, `edit_reencoding`, `edit_verifying`, `edit_committing`. Unknown phases are rejected without weakening member progress validation.

## Remaining acceptance

- G-00 remains incomplete: exact total source reads, internal format phase costs and block-decode counts are not available in the retained observations. Revision bytes, runner notification intervals and sampled memory must not be relabeled as those missing measurements.
- Read, creation/split/post-Test/Add, same-block 7z, solid RAR5 and compressed-TAR native measurements are complete for the listed fixtures. Explicit native post-create Test and Rust scheduler follow-up remain distinct evidence routes.
- G-01/G-02/G-06 and G-15/G-25 have the compiled/controlled-native evidence listed above. Their overall batch acceptance remains conditional on G-00 and the scoped gate matrix; physical UI acceptance is not implied.
- Physical GUI/Explorer/DPI/user acceptance and Release acceptance were not authorized and remain unexecuted.

### Internal measurement boundary

The current revision counters do not include ZIP directory/local/payload reads, compound-stream input reads or isolated-codec fingerprint work. Completing those counters requires measuring actual source `Read` results and separating temporary spool reads; the extra ZIP/TAR/7z observations and internal phase timing have not been implemented. Frozen before-worker observations cannot supply those missing fields retrospectively.

Locked `sevenz-rust2=0.23.0` accepts `Read+Seek` and exposes blocks/stream maps; complete successful payload traversals can be measured at the existing reader owner. They do not reveal each internal reset or a failed partial block's exact work. Locked `unrar_sys=0.5.8` exposes `RARReadHeaderEx`, `RARProcessFileW` and decoded `UCM_PROCESSDATA`, but no compressed-volume read callback or solid-block decode/reset event. SourceSet fingerprint bytes and RAR call/decoded-byte counts are observable; the decoder's internal source bytes/block events are not. Header calls, decoded bytes and OS process I/O cannot substitute for them. A complete G-00 result therefore needs an explicitly supported measurement decision for that native boundary as well as the remaining Rust instrumentation; this record leaves G-00 incomplete.
