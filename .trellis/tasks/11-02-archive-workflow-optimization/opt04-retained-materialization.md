# OPT-04 — Drag-scoped retained materialization

## Status and authority

**Implemented; automated and owned-native acceptance passed for G-07/G-08/G-09/G-10/G-27 and touched G-19.** Parent remains **Phase E — Format expansion and ZIP editing**. The independent OPT-04 authorization permits product changes, necessary builds/automation and effects-audited owned native fixtures/controlled COM consumers. Physical G-18 remains separately authorized.

OPT-02 and OPT-03 scoped independent acceptance are retained, including O3-AC01/02/03 closure. OPT-01's protected shared-read and Seal actual-size changes are the foundation. G-00, OPT-00/01 overall closeout, complete G-04 and Phase E/Release remain open. OPT-05 through OPT-07 are not started.

## Behavioral reference

The saved Bandizip7.32.0.1 ZIP and solid/output records establish retained input/catalog plus original-index selection, not a new Shell drag protocol. `ark-core-code-0x180057cd0` retains input interfaces; `0x1800576f0` admits a local header when its cached offset is absent; `0x18007dca0` streams decode/CRC/output. `ark-7z-code-0x180139760` groups file indices by folder before `0x1801611c0`; solid RAR's selected range advances ordered predecessors without unselected filesystem output. `ark-families-code-0x18003c960` binds selected indices to a retained outer-stream adapter. `ark-output-consumer-close-0x180023620` joins output before EOF/metadata/handle closure.

The outgoing dossier `0x140147200` has matching pseudocode/assembly/callees: OLE call at `0x140147af8 → 0x140053460`, success compare at `0x140147d2f`, mapped-destination call at `0x140147d4a → 0x1401aedd0`, then extraction at `0x140147e60 → 0x1401481d0`. That destination-handoff branch is not the existing NxTools FileContents contract. Only saved records are read; no reference execution, new reverse engineering, proprietary code reuse or backend/vendor change.

## Original owner boundary

`browser_drag`/`model_drag` freeze the still-pressed row/Tree selection, original ordinals, source HWND/read connection/snapshot. Original native admission precedes detached receipt waiting and one dedicated STA/DoDragDrop loop. `ArchiveVirtualFileDataObject` owns indexed COM media; `ArchiveVirtualContent` deduplicates each index; `ArchiveEntrySession` owns protected materialization and cache leases; `ArchiveWorkerRunner` owns actual process/Job/pipe/source/output cleanup; `ArchiveEntryCache` seals actual size and validates every transferred native cursor.

Before retained implementation, distinct misses dispatched separate EntrySession operations and worker/source/catalog attempts. Same-index requests shared materialization but each consumer received its own verified native cursor. Member readiness waited for the whole worker's cleanup before Seal. Native parsing and DataObject both validated the tree, and session/content/per-member paths repeated activity reservation.

The retained change must keep one session-owned validated descriptor plan and top-level activity reservation, while the worker independently acquires true source protection. First uncached demand starts the retained operation. Member readiness requires full required format integrity, committed native output with closed member writer/partial and completed Seal, but cannot imply overall Run cleanup or release its active reservation. Closing refuses new demands and follows original end/cancel/EOF/drain. Session terminal remains actual process/pipe/Job/source cleanup. Returned IStreams remain target-owned through source retirement.

Before retained implementation, visible preview released its lease after decoding and before Ready. Current visible Ready retains that exact lease until original retirement, while the original cache holds a distinct drag-consumer share. Preview success never grants encrypted drag authority. Exact locked codec APIs constrain reuse/reset; decoder allocation reuse is not proof of valid solid history.

## Baseline and regression boundary

Extend the existing effects-audited controlled COM consumer path: DragSession → indexed GetData → EntrySession → actual runner/worker/native output → cache Seal → native IStream → receiver EndOperation → actual session cleanup. Descriptor-only cancellation and1/2/16 distinct ZIP demands use the same generated input/build/path before and after. Main latency groups retain three completed trials with no system-cache flush. Repeated/out-of-order indices, independent cursor positions and source exclusive reopen are asserted.

Record actual worker starts, available source/catalog observations, cache verification reads, sealed/output bytes and the distinct gesture/OLE/descriptor/demand/member/receiver/cleanup clocks. A field absent from the current observation seam stays unavailable rather than being inferred from bytes or elapsed time. Existing WorkLog/ResourceProbe semantics distinguish transport JSON bytes, revision-only reads and observed notification intervals from total source I/O and exclusive codec time.

Before Debug worker and host/test files are frozen separately from the stable comparison path. The GUID-owned 20,973,276-byte ZIP contains sixteen256KiB demanded members and one16MiB unrequested member. `archive OPT04 drag completed baseline` LIST1/RUN1/PASS1/SKIP0 completed descriptor-only plus three trials for each1/2/16 group. Actual worker/source/catalog counts are0/0/absent,1/1/1,2/2/2 and16/16/16. All requested native outputs and cursor bytes matched; all workers and drag owners reported actual cleanup, outputs retired and source reopened exclusively.

`archive OPT04 drag retained context lifecycle` LIST1/RUN1/FAIL1/SKIP0 completed the same scenarios then failed the production-boundary assertion: first member readiness already had one worker terminal and no active Run. Its report is retained separately as RED, not overwritten by after measurements. Before first-ready ranges were1633–1863ms; all-ready ranges were1696–1867ms(1),3424–3623ms(2),27101–29393ms(16). No system-cache flush, physical gesture/OLE/DragOver/target-disk measurement or G-00 closure is claimed.

## Private protocol and ownership

The existing schema2 `extract` start gains optional `drag_materialization:{version:1,members:[{index:u32,ordinal:u64}]}`. It requires browser_entry/source binding, no prompt/job/edit/reclamation/read-contract authority, and an actual `drag_materialization_v1` accepted capability before source work. Absent fields preserve one-shot callers. The frozen descriptor plan admits unique file indices and original ordinals; directories have no demands. Private1MiB framing and original metadata/native capacity remain authoritative.

Existing version1 control frames gain `op:"drag_demand",demand:{request_id:UUID,index:u32,ordinal:u64,output_limit:u64}` and `op:"drag_end"`. Operation ID must match the one original Run; demand must exactly match its immutable map, carry a nonempty previously unused request ID and a positive narrowed remaining output allowance. Only one member may be outstanding; duplicates, mismatches, end-after-end and demands after closing fail the original operation. Cancel and EOF wake the existing control/codec paths; no replacement worker.

For each admitted demand, Rust calls existing private native rendezvous action `member_acquire` with token=request_id. The C# runner binds that exact pending demand to its original cache lease/output root; filesystem roots never cross to Rust. Each member uses an original ArchiveWorkerOutputBridge/ArchiveOutputSession under the same Run, source, Job and pipe ownership. Published member bridges retain their facts until actual drain. New member bridges inherit the original captured MOTW policy. Unrequested members never acquire an output bridge/token.

After full required codec/block/stream integrity, actual bytes, original-source check and native commit/closed member writer, Rust emits `op:"drag_member_ready",member_ready:{request_id,index,ordinal,bytes}`. Runner requires the matching live demand and exactly one committed output. EntrySession seals that member's original cache lease and checks Seal size against bytes before resolving a private ready result. This result contains member readiness, not a public ArchiveEntryResult with invented cleanup=true. Every actual cursor still validates the sealed object/content independently.

The drag operation lives in the existing EntrySession and runner. First uncached demand starts it; same-index dedup and actual-size debit remain in ArchiveVirtualContent. A member-ready transition stops its work clock, not the Run. Runner remains in local/global active reservations with actual source/Job/pipes alive. The next demand starts a fresh original30-second work domain; startup/catalog belong to the first domain. Receiver idle is not member work. The original DragSession120-second stop remains unchanged; asynchronous receiver lifetime/COM pumping remains independent and an uncooperative receiver is not fabricated as ended.

Normal content retirement stops new demand, sends drag_end and waits for actual worker terminal/process/pumps/Job/source/output drain. Cancellation, disconnect, protocol error and per-member timeout use original cancellation/own-Job cleanup. Terminal cleanup=false keeps the original reservation and output evidence. Completed members and transferred native IStreams survive later failure/cancellation; release occurs only through original cache ownership after actual worker drain, and target-owned cursors are never force-closed.

ZIP retains protected reader/catalog and one structural preflight, then seeks original members with fresh bounded member decoders. Compound TAR retains the existing fully verified anonymous spool locally for this operation and seeks validated records.7z retains its admitted catalog/reader and decodes only the demanded public BlockDecoder block through full integrity; its consuming API requires another block invocation for a later demand, which is counted rather than called resumable state. RAR retains ordered TEST state, advances unrequested predecessors without outputs and reopens/replays only for backward demand. Observable format decode/replay counters describe actual invocations; unavailable native internal reads/block counters remain unavailable.

The original cache gains only an operation-local preview-consumer share. Matching includes source/session, generation/snapshot and original ordinal; the current visible preview keeps its sealed lease until its existing retirement paths. Drag adoption must independently validate encrypted authorization and charge actual bytes once. Preview retirement cannot retire a borrowed drag consumer; all cache hits retain per-cursor checks and deferred GPUI application.

## Scope limits

No second public business channel, worker pool, scheduler, cache framework, task manager or competing Shell gesture. Ordinary shared extraction remains the engine. Keep complete admitted subtrees, actual budgets, immutable ordinals, full codec end/CRC/MAC/size, native publication, independent cursor checks, password grants, deferred GPUI callbacks and current prompt/progress/connection repairs.

OPT-05 general ZIP record-plan/TAR spool/edit verification, OPT-06 binary bulk transfer and OPT-07 batch/password/Find/picker work remain later packages. No real Explorer/product GUI, system clipboard/picker, user archive/source mutation, recycling/HZ5, service/registry, package/sign/install, commit or push. Stop after OPT-04.

## Initial retained ZIP/native checkpoint

The same Debug fixture/path now passes `archive OPT04 drag retained context lifecycle`: LIST1/RUN1/PASS1/SKIP0, descriptor-only plus three completed1/2/16 trials. Each demanded group has one actual worker/source/catalog, no worker terminal and an occupied Run at first member readiness, then one real worker terminal after receiver end. Independent repeated/out-of-order native cursors, no unrequested outputs, target-stream survival and source exclusive reopen pass. This is the ZIP vertical slice, not complete format/G-07–G-10/G-27 acceptance; the final worker will be remeasured after format completion.

C# retains one typed descriptor plan and top-level drag activity. EntrySession's private VirtualMaterialization/VirtualMember distinguish Seal-ready from overall cleanup. Runner's original stdout pump binds member_acquire to one pending member/output root, validates ready against actual native publication and holds the original Run/global slot until real drain. Work clocks cover first startup, each admitted member and Seal; idle is excluded, normal end has bounded work and original120s cancellation stays registered through retirement. Ordinary one-shot Entry operations remain unchanged.

`archive OPT04 preview drag lease adoption` covers exact hit, mixed hit/miss, different source/session/generation/snapshot/ordinal and retired preview. Actual preview output is sealed through the original worker; a matching drag creates no new worker, while each miss creates one retained worker. Preview release preserves the drag consumer and independent cursors through source retirement. Mixed preview A → preview close → uncached B → repeated A reproduced a native GetData failure; the original cache sweep now honors its consumer share as Release already does. LIST2/RUN2/PASS2/SKIP0 jointly verifies all eight preview scenarios and original receiver cancel/abort/source-close/disconnect.

Receiver cleanup assertions now independently require original runner slots0 and exclusive source reopen while the already transferred output stream remains readable. Disconnect has actual cancelled/cleanup=true after internal retained drain; receiver failure/native HRESULT is retained. There is no forced cleanup or invented EndOperation. Original async multi-directory/unknown-size/End ordering and OPT-01 Seal accounting/first-and-repeat tamper/cache-cursor lifetime pass a separate LIST4/RUN4/PASS4/SKIP0 selection.

The initial ZIP checkpoint passed Rust wire/admission3/3 and demand-port identity/cancel2/2, with isolated worker and C# compilation. Its frozen before package and first ZIP-green package remain separate from the final all-format pairing below.

## Final implementation and boundary verification

The original private operation now spans demand-started source/catalog admission, demanded member publication and actual final drain. `ArchiveVirtualPlan` is the once-admitted immutable COM index→ordinal map. EntrySession's `VirtualMaterialization` owns retained members and preview shares; `VirtualMember` represents full codec/native/Seal readiness rather than a public cleanup terminal. Runner and Rust retain their original Job, pipes, source authority and reservations. Ordinary one-shot callers remain compatible.

Seal completion uses `ArchiveWorkerRunner.ConfirmVirtualMember`: the post-Seal ready transition and work-deadline check share the original runner gate. Hashing stays outside that gate; a failed/expired Run cannot resolve a member successfully. Actual native RED returned a medium after a31s Seal hold; the same fixture now refuses it, preserves timeout and waits for actual drain. Original120s drag and30s work limits are unchanged;31s receiver idle accepts the next demand without another worker.

Supported split ZIP uses the existing all-volume acquisition and normalization adapter. Catalog admission retains that normalized anonymous source for preflight and every demand; external source-set identity/revision remain authoritative. Actual native RED failed its first demand; forward/backward demands now return exact independent bytes from one retained Run/source. Admission reports two real directory passes: original volume directory and normalized ZIP catalog. No per-demand normalization/catalog reload is hidden, and no general OPT-05 record-plan work is included.

Visible preview has a real Desktop RED/GREEN: actual background text prepare reached visible Ready while releasing its original lease. `BrowserSlot.preview_lease` now receives the current Ready lease; original selection, refresh/codepage, tab, close and disconnect retirement releases it once. Failed/stale/owner-lost completion releases its exact original lease. The four new tests pass separately and within the105-test preview-filter selection, including decode completion after close and password-overlay cancellation. This does not alter layout, focus, Kit or passive rendering.

The original wire rejection type is retained at Parse; invalid plans map to InvalidDataException. The touched existing UTF8 component contract now rejects258-byte components while accepting255-byte components. Complete33/256-file,257-descriptor and64-root representations remain covered by original wire/native descriptor and protocol regressions. Budget fixtures exercise known/unknown/zero estimates, concurrent/repeated demand, actual debit and complete cleanup. An intentionally failed retained member ends further uncached work; zero-file/repeat accounting is verified before that failure rather than by starting a replacement worker.

## Format results

| Format/boundary | Observed retained behavior |
| --- | --- |
| ZIP1/2/16 | One actual worker/source/catalog per demanded group; actual member decodes1/2/16; each index materializes once |
| Split ZIP | Two150000-byte members across65536-byte volumes, forward/backward orders; one worker/source, two admission directory passes, no per-demand reload |
| Solid7z | Two distinct demanded files in one block require two complete public BlockDecoder invocations; catalog1, no cold worker restart |
| 7z duplicate/empty/multiple blocks | Original ordinals preserved under reversed descriptor order, identical names and no-block empty entries |
| Corrupt required7z block | Independent7-Zip reports second-member CRC/Data Error; demanding the earlier member returns no medium/ready or committed/partial output; actual cleanup succeeds |
| Solid RAR5 | Forward/backward/nonmonotonic demands: TEST opens1/4/2 and replayed predecessors0/3/2; catalog1; unrequested predecessors have no output |
| Solid encrypted RAR4 | Independently decoded pinned libarchive fixture, D→B→C plus repeats: one retained worker/source/catalog, TEST opens2, replayed predecessor1; full ProcessFile integrity before commit |
| TGZ/TXZ | First fully verified outer spool retained, outer decode1 and catalog1 per drag; original validated ordinal reads and exact output bytes |
| Standalone unknown Zstd | Original unknown metadata, actual sealed size and independent native stream/cursor lifetime pass |

RAR4 uses `prepare-opt04-rar4.py`, fixed libarchive commit79c571f845dbdd250d69bff9d93ae034f674135e and an independent7-Zip decoder. Three controlled drag scenarios verify encrypted preview alone cannot grant drag, independent whole-TEST authorization enables exact adoption, and revoke refuses cached/new GetData without a prompt or replacement worker. Matching preview A uses zero new worker, survives preview close and cache sweep, and shares no authorization with GetData. Eight ordinary preview hit/mismatch/retired/mixed-hit-miss scenarios also pass.

## Comparable completed measurements

Same20,973,276-byte owned input, same Debug mode and stable `opt04-comparison/Debug` worker/host/test paths; three completed trials for each demand group, no system-cache flush. Frozen before, RED, initial ZIP-green and final all-format pairs are distinct. The final worker is12,218,880bytes; host2,156,032bytes. `opt04-final-source` retains74 relevant changed/new source files and the associated HEAD/diff/status evidence.

| Distinct ZIP demands | Before→after worker/source/catalog | First verified stream ms, before→after range | All demanded ready ms, before→after median [range] | Receiver end→actual cleanup ms, before→after range |
| --- | --- | --- | --- | --- |
| 1 | 1/1/1→1/1/1 | 1695–1863→1488–1589 | 1736[1696–1867]→1492[1490–1592] | 1.7–5.0→78.8–88.2 |
| 2 | 2/2/2→1/1/1 | 1726–1736→1451–1486 | 3428[3424–3623]→1525[1516–1552] | 5.9–18.2→90.3–93.6 |
| 16 | 16/16/16→1/1/1 | 1633–1719→1463–1635 | 27682[27101–29393]→2448[2438–2621] | 62.5–67.4→92.8–123.8 |

Descriptor-only cancellation has zero worker/source/new output. Its descriptor GetData was1.955→1.098ms and QueryGetData0.0299→0.0163ms, one controlled observation each, not a general responsiveness claim. Output bytes remain262144/524288/4194304 for1/2/16 demands, with no unrequested output. Actual revision work changes4/8/64→4/4/4 passes and83,893,104/167,786,208/1,342,289,664→83,893,104bytes per trial. These are revision reads, not total source I/O. Before ZIP decoder invocation counters were unavailable; after observes1/2/16. Cache Seal and per-consumer validation remain intact, not counted as eliminated work.

The principal measured reduction is repeated-worker/catalog/revision work and multi-member readiness in this controlled fixture. First-ready ranges overlap across groups; no general first-stream or cold-cache speed claim is made. The retained operation performs its real drain after receiver end, whereas the before implementation had already drained each per-member worker before readiness. The table preserves that cleanup-placement tradeoff.

## Final checks and acceptance

| Gate | Evidence and result |
| --- | --- |
| G-07 | Final lifecycle LIST1/RUN1/PASS1/SKIP0: descriptor-only and nine completed demanded trials; one occupied Run/source/catalog at first ready, zero worker terminals until actual end |
| G-08 | Final integrated format/7z-integrity/RAR4/split selections pass;12 format-order cases, genuine solid RAR4/RAR5 and original duplicate/empty/multiblock regression |
| G-09 | Six lifetime modes: Seal timeout,31s receiver idle, write timeout, Seal cancel, owned worker exit and stdin EOF; original async/slow/abort/Escape/source-close/disconnect/Start-End ordering preservation passes |
| G-10 | Once-admitted typed plan/activity; eight exact/mismatched/retired preview scenarios and three encrypted RAR4 authorization/adoption/revoke scenarios; independent cursors and actual-byte budgets preserved |
| G-27 | Desktop4/4 new real prepare/visible Ready tests and105/105 preview-filter tests including those four; native preview shares survive close/mixed cache sweep and transferred-stream retirement |
| G-19 touched | Original OPT-01 Seal/no-size-cursor/full-content tamper, complete tree/representation, source-versus-output budgets, source preservation and actual resource cleanup pass; accepted OPT-02/03 behavior remains unchanged |

Final same-pair native integration is LIST6/RUN6/PASS6/SKIP0. Touched old-chain preservation passed six async/password selections, then LIST10/RUN10/PASS10/SKIP0 for tree/descriptor/split/accounting/tamper/budget/gesture/delayed/slow-cancel boundaries; split overlaps final integration and is not added twice. Rust archive59/59, protocol50/50 and IPC17/17 pass. Scoped Rust formatting and archive isolated-codecs/Desktop all-target strict Clippy pass. C# build has0warnings/0errors. Spec120documents/14owners, component catalog, coherent Kit0.8.0/pre0.3.9 and pinned Markdown-editor gates pass.

Evidence is retained under `PI_SCRATCH_DIR`: `opt04-final-measure-{list,run}.log`, `opt04-final-integrated-{list,run}.log`, `opt04-seal-timeout-red-run.log`, `opt04-split-red-run.log`, `opt04-seal-handshake-green-run.log`, `opt04-tree-contract-preservation-green-run.log`, `opt04-final-{archive-clippy,archive-lib,wire-ipc,desktop-clippy,worker-build}.log`, `opt04-comparison-summary.json`, the owned format reports and immutable before/after JSON. The original native lifecycle and Desktop Ready REDs remain separate evidence.

## Limitations and stop point

This accepts only the authorized automated/owned-native OPT-04 boundary. Controlled COM is not real Explorer/GPUI gesture/DragOver/target-disk acceptance; those clocks remain null. Internal native source I/O, exclusive codec time and UnRAR block counts remain unavailable. 7z's public decoder consumes a complete block per distinct demand, and RAR backward demands reopen/replay; no one-decode claim is made. Plain TAR/TBZ2/TZST and other standalone combinations are not new native measurement rows in this matrix; their original adapters and finite resources remain unchanged. This is not a broad format/resource or Release performance certification.

G-00, OPT-00/01 overall, complete G-04, Phase E and physical/Release gates remain open. No real GUI/Explorer/clipboard/picker, user archive/source mutation, recycling/HZ5, service/registry, packaging/signing/install or commit/push occurred. OPT-05/06/07 remain unstarted. Stop at completed OPT-04.
