# Archive workflow optimization — design contract

Requirements are owned by the [task brief](D:/Code/NxTools.Next-main/.trellis/tasks/11-02-archive-workflow-optimization/prd.md); scenarios by [acceptance](D:/Code/NxTools.Next-main/.trellis/tasks/11-02-archive-workflow-optimization/acceptance.md). This design selects changes inside the original runtime and does not introduce a new archive backend.

## 1. Change boundary and module owners

| Module / existing entry | Responsibility in this task |
| --- | --- |
| [Archive engine](D:/Code/NxTools.Next-main/rust/crates/nxtools-archive/src/engine.rs) | Current-attempt source/catalog/admission ownership; explicit catalog-only versus capability-bearing entry; ordinary unleased source-change semantics |
| [ZIP reader](D:/Code/NxTools.Next-main/rust/crates/nxtools-archive/src/formats/zip.rs), [ZIP editor](D:/Code/NxTools.Next-main/rust/crates/nxtools-archive/src/zip_edit.rs) | One validated record plan, streaming integrity, encoded retention and final candidate verification |
| [Compound TAR adapter](D:/Code/NxTools.Next-main/rust/crates/nxtools-archive/src/tar_edit_outer.rs), [extraction plan](D:/Code/NxTools.Next-main/rust/crates/nxtools-archive/src/planning.rs) | Operation-local spool and normalized component-ended selection expansion |
| [Worker](D:/Code/NxTools.Next-main/rust/crates/nxtools-archive/src/worker.rs), [private output](D:/Code/NxTools.Next-main/rust/crates/nxtools-archive/src/worker_output.rs), [source adapter](D:/Code/NxTools.Next-main/rust/crates/nxtools-archive/src/worker_sources.rs) | Original control/data protocol, original operation context, per-member readiness and bounded ordered bulk transfer |
| [Virtual content](D:/Code/NxTools.Next-main/csharp/NxTools.PlatformHost/Windows/Archive/ArchiveVirtualContent.cs), [entry session](D:/Code/NxTools.Next-main/csharp/NxTools.PlatformHost/Protocol/ArchiveEntrySession.cs) | Demand/index deduplication, retained drag context, original source authorization and per-member cache leases |
| [OLE session](D:/Code/NxTools.Next-main/csharp/NxTools.PlatformHost/Windows/Archive/ArchiveVirtualDragSession.cs), [data object](D:/Code/NxTools.Next-main/csharp/NxTools.PlatformHost/Windows/Archive/ArchiveVirtualFileDataObject.cs) | One STA/gesture, immutable validated descriptors, sync/async receiver lifetime and transferred streams |
| [Native runner](D:/Code/NxTools.Next-main/csharp/NxTools.PlatformHost/Windows/Archive/ArchiveWorkerRunner.cs), [output session](D:/Code/NxTools.Next-main/csharp/NxTools.PlatformHost/Windows/Archive/ArchiveOutputSession.cs), [cache](D:/Code/NxTools.Next-main/csharp/NxTools.PlatformHost/Windows/Archive/ArchiveEntryCache.cs) | Actual process/Job/pipe cleanup; ordinary held final-name output and close journal; separate Create/Edit candidate publication; sealed cache size and real cursor checks |
| [Source lease](D:/Code/NxTools.Next-main/csharp/NxTools.PlatformHost/Windows/Archive/ArchiveSourceLease.cs) | Ordinary protected-object evidence and equivalent within-stage edit observations; not HZ5 mutation authority |
| [Creation planner](D:/Code/NxTools.Next-main/rust/crates/nxtools-archive/src/creation.rs), existing ZIP/7z/TAR worker edit modules | Add-specific incoming namespace planning, original source-ID binding, common conflict semantics and one candidate |
| [Application](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/application.rs), [forms](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/forms.rs), [browser](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/browser.rs) | Frozen destination, shared intake, local source removal and new-snapshot result selection |
| [Jobs](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/jobs.rs), [batch projection](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/batch_jobs.rs), [progress](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/progress.rs) | Original queue/attempt/prompt/terminal authority, actionable background prompts, honest controls and one batch presentation |

Changes are made only where these behaviors live. Refactoring is limited to source/catalog/plan/data or presentation lifetimes that remove demonstrated duplicate work; unrelated module splitting is not a deliverable.

## 2. One opened attempt

A private opened-attempt owner retains the actual source reader, immutable catalog and format admission state. ZIP adds an ordinal-indexed validated local/data/descriptor/extent plan. Compressed TAR adds its already verified anonymous decoded spool. Both are charged to existing finite metadata/work/spool budgets and retired with the operation.

Capability-bearing browser open lists and qualifies the same source before one final source recheck. Catalog-only Test/extract/materialize avoid unused edit qualification. Standalone public qualification still performs a fresh independent admission. Snapshot serialization remains metadata/source identity, not live handles, authority flags or a disk content cache.

Ordinary native held-source evidence is installed only after the original negotiated source acquisition. Rust cannot manufacture that evidence from an arbitrary request boolean. Under that evidence, archive-sized checks move out of member loops; original before/final source checks stay at their real stages. Unleased calls preserve their current safety interface. HZ5's changing source does not enter this immutable-source path.

Selection expansion validates session/ordinal directly, normalizes each member path once and merges directory prefixes before a single catalog pass. Prefix matching ends on a path component. Output order remains original ordinal order; original record names/bytes are not rewritten just to normalize display.

ZIP edit payload validation follows the final plan: retained/new content is decoded and CRC/MAC/size-verified in the final candidate; old content removed from the final candidate receives source verification required by the current corrupt-source policy. All source/local/central/extra/extent rules remain. Native held-partial readback and final publication are unchanged.

At one native edit validation stage, a digest can be shared only after source/target full identity and version establish one continuously protected object. The fresh prepublication validation and reader-close/FILE-only release cannot consume an earlier unprotected-stage observation. Long digest loops use the original cancellation token at bounded read boundaries.

### OPT-05 executable seams

`RetainedCatalog { input, tar_plan, gzip_members }` owns the first fully verified TAR spool and original-session ordinal Vec. Catalog/locators/strict qualification/execution coexistence is admitted before allocation; fresh public execution uses the same locator budget. `formats/zip/plan.rs::ValidatedZip` exclusively borrows the actual reader and immutable entries; record/range/header memory is charged before growth and streaming subtracts live plan memory before codec minima. Readmission retains that plan across native preflight through `extract_with_zip_plan` into the original `extract_impl` guards and metadata/progress finalization.

Actual `modify_zip` catalog intake binds native/browser source facts and leaves complete structural/kept-metadata/payload admission to the intent-aware candidate; public browse qualification remains separate and fresh. No-change Add independently qualifies because no candidate runs. Removed old payloads remain source-verified, retained/new verify in the final candidate, and exact encoded held-partial equality reuses that integrity without replacing native publication checks. Directory Rename uses normalized slash-ended component boundaries, including attribute-only directories without a stored slash.

`planning_selection::{bind_ordinals,expand_selection}` binds selected IDs once, merges roots and emits one catalog-order expansion under remaining metadata/working credit. Desktop noncontiguous IDs and original descriptor `BTreeMap` order remain separate concerns. Work aggregates once even on failure. Native `CaptureEditSource` checks complete version before/after each hash and compares a concurrently held target; only that call can reuse the observation. Five fresh stages plus output fingerprint/postpublication and FILE-only/real cleanup remain.

Concrete signatures, error matrix and regression assertions: [format plans](../../spec/nxtools-archive/backend/reusable-plans.md) and [native observation](../../spec/platformhost/archive-read-observation.md). [Delivered result and measured limits](opt05-reusable-plans.md).

## 3. Demand-started drag materialization

### Session and member facts

The existing native drag session owns one immutable descriptor-index → original-ordinal mapping and one source activity reservation. Parsing produces the validated plan consumed by DataObject and Content; a consumer-owned HGLOBAL is still allocated for each COM medium transfer.

The first uncached FileContents demand starts one original runner operation containing a drag-scoped source/catalog context. Later demands use that context through the original private control interface. Same-index requests deduplicate; each returned IStream is an independent cursor. No demand means no worker or member output. Existing metadata, actual-byte, native representation and transport limits bound demand state; active-worker limits are not logical member-count caps.

A retained process changes the meaning of readiness, so the interface separates:

- **Member ready:** required codec/block integrity completed, exact member output natively published, member writer/partial handles closed, cache sealed and actual size known. This is not overall process cleanup or a business terminal.
- **Session closing:** new demands refused, original cancel/EOF/end applied, any active codec/data transfer drains through the original owner.
- **Session terminal:** actual worker exit, pipe/Job/source cleanup and original session retirement observed. Only this stage releases its active-run reservation.

A member cannot become readable by falsely marking the whole Run `cleanup_complete=true`. A member failure stops new work and uses original cancellation/failure precedence; already transferred streams and published member facts survive. No timeout replacement process or per-member hidden cold worker is introduced.

ZIP seeks independent admitted records. 7z/RAR reuse only state valid for their actual folder/predecessor sequence. Non-monotonic requests may require a decoder reset; retaining allocations is not proof of valid history. Complete required folder/stream integrity precedes member readiness. Backend public capability must be verified at implementation entry; a limitation is recorded without silently removing required formats or claiming a one-decode result not achieved.

Implemented OPT-04 uses optional private `drag_materialization` start, exact `drag_demand`/`drag_member_ready` identities and `drag_end` on the original runner pipe. No output path or public business channel is added. Split ZIP retains its admitted normalized source; compound TAR retains the first verified spool. Public7z BlockDecoder is consuming, so each later distinct member uses a counted complete demanded-block pass. RAR retains ordered TEST history with counted backward reopen/replay. Exact signatures, validation/error matrix and tests: [retained executable contract](../../spec/nxtools-archive/backend/product-read.md#scenario-drag-scoped-retained-materialization); completed evidence: [OPT-04](opt04-retained-materialization.md).

### Lifetime, cancellation and deadlines

Keep one pressed-HWND/capture/connection-bound OLE start. Async receivers retain COM/data/content/STA pumping through StartOperation/EndOperation; synchronous receivers keep truthful fallback status. DoDragDrop return alone cannot retire an active async receiver. Returned native streams are target-owned and are never force-closed by source cancel/session retirement.

Retain the current overall drag deadline and original member-work budgets. Member-work timing starts at actual member admission, not descriptor creation or receiver idle time. Do not accidentally reuse the old one-member process deadline as an aggregate deadline that truncates a valid multi-member session. Original timeout/cancel/cleanup ownership, finite overall lifetime and admission limits remain; any deadline-policy change needs an explicit decision rather than an automatic increase.
`ConfirmVirtualMember` is the post-Seal readiness handshake under the same original runner gate as timeout checks. Seal hashing stays outside that gate; a failed or expired Run cannot publish a medium after hashing returns. First startup/catalog and each member/Seal use the original30s work domain, receiver idle is excluded, and original120s drag cancellation stays registered through actual drain.

Encrypted drag retains its separate no-output authorization Test and fresh gesture. No password modal occurs in GetData. Authorization expiry/revocation refuses new demands without invalidating already transferred native streams.

### Cache size and preview adoption

The cache's sealed output stamp supplies actual size to private materialization accounting. Remove the size-only OpenVirtualStream and its full hash. Real target cursors continue exact object/content validation; descriptor estimates never replace actual charging. Any other output-to-cache observation consolidation requires an exact-held-object handoff with unchanged tamper detection.

Visible preview reuse adopts only its existing sealed lease under the full source/session/generation/snapshot/ordinal match. Memory-only consumer retention stays in the original cache owner; close/refresh/selection changes retire preview ownership without deleting a still-borrowed drag stream. Encrypted preview success is not drag authorization. Reused outputs are still charged once per distinct demanded member and checked for each real cursor.

### Configurable archive preview budgets

[Preview budgets](preview-budgets.md) and the canonical [cross-layer contract](../../spec/nxtools-desktop/backend/archive-preview.md) define six validated Core fields, official Settings NumberField controls and changed-leaf saves under the original section lock. Selection freezes the complete profile; the preview-only optional wire changes only the original native purpose quota/runner timeout. Actual member length, text lines, image pixels/decode allocation and the original readonly document use that snapshot. Missing/null wire options retain legacy behavior; non-preview non-null options reject and the private worker schema stays unchanged. Source/spool/solid work, two-operation/decode admission, full integrity, stale suppression, true joins and visible-preview lease retirement remain original.

[Verification](preview-budget-results.md) covers real official number-input saves, frozen request/rendering budgets, CR/LF/CRLF line accounting and distinct SelectedOutputLimit versus ReadLimit through the actual worker/native chain. The same original two-slot admission, full integrity and lease cleanup boundaries remain.

## 4. Add-specific input and namespace plan

All chosen Add entry points converge on the existing Modify draft, keeping archive/source/snapshot/internal directory immutable. Incoming Open-first and New-from-all reuse their original open/create owners; they do not retarget or mutate the current archive. A clipboard read is a bounded native observation, not permission to modify. Install correlated pending context before dispatch; late, closed-tab or changed-folder results do not retarget. Editable text controls keep text paste. Clipboard COPY/CUT never supplies delete-source authority.

Per-source removal changes only the local draft, clears dependent validation and performs no source I/O. Current external-root limits and overlap/reparse checks remain; source roots are not the number of resulting archive entries.

The generic new-archive collision refusal remains intact. Add introduces a private incoming plan before destructive name collapse, retaining each protected source ID, type, requested relative name and descendant relationship. Resolve the union of original archive namespace and all incoming requested names before encoding accepted sources:

1. Same canonical directory destinations merge logically. Existing archive directory metadata is retained; without an old directory, new-directory metadata follows the current adapter's normalized-new-entry policy. Merging a directory is not an extra directory publication.
2. Same-type file collisions use the original Ask/Overwrite/Skip/AutoRename/Cancel ticket. Incoming/incoming Overwrite selects the accepted payload in the one candidate; it never overwrites filesystem sources.
3. Different file/directory types retain explicit refusal. No recursive delete-old-first substitute is added.
4. AutoRename consults all original and incoming requested names/types plus accepted generated names. A generated `same (2).txt` cannot occupy a later source's original request. Directory remaps apply coherently to descendants.
5. ApplyAll lives only for the current attempt. Decisions precede payload source opens/encoding. Merge-only plus all skipped children is zero candidate/publication; a genuinely new accepted empty directory remains a real change.

ZIP copies unchanged encoded records and encodes accepted additions once. Final complete candidate integrity verification replaces redundant additions-payload verification, not structural checks or old removed-member verification. Qualified 7z/TAR/outer-compressed writers retain their distinct full rewrite costs and disclosed encryption/metadata rules.

The result route carries a bounded, Debug-redacted display-only affected-member outcome with actual final names and total count. It grants no native authority. After actual publication/cleanup and matching job/attempt/generation, original browser refresh maps those names to new IDs. If display retention is exhausted, show a truthful selected subset/count rather than truncate the archive change or reuse old ordinals.

## 5. Private data transport and ordinary extraction output

The original worker connection keeps `bulk_transfer:1` / unique `binary_bulk_v1` for modern schema2 source reads, Create/Edit candidate writes/readback and retained cache output. Controls, prompts and terminal facts remain typed JSON. The [fixed private contract](opt06-binary-transfer.md) continues to govern those acknowledged directions. Schema1 ZIP, read-only, virtual cache and explicit HZ5 retain their separate contracts; no public IPC field or second runtime is introduced.

Ordinary known-destination extraction additionally requires private `extraction_output_v1` agreement inside bulk v1. Capability refusal precedes destination mutation. C# opens the validated final relative component beneath the held no-reparse parent with `CREATE_NEW`; no `.nxpart`, rename, output hash/file-ID fingerprint, per-file `VerifyDirectory`, published reopen or `FlushFileBuffers` occurs. The failed begin result `target_exists` plus `existing_bytes` feeds the existing Rust conflict owner. `observe_target` is removed; `begin_overwrite` directly truncates the same observed relative path without signature/CAS, while AutoRename repeats bounded `CREATE_NEW` attempts.

Rust coalesces ordinary payload into <=32KiB raw records and writes the original anonymous stdout pipe without waiting for per-record ACK. The measured 2026-11-07 optimization keeps the original independent C# pumps and synchronous `CreatePipe` handles: the input `FileStream` uses buffer size 1 and synchronous `Write` after a cancellation precheck; stdout `ReadFrames`/`ReadBulkWrite` use synchronous `Read`/`ReadExactly`. The OS pipe supplies bounded backpressure. There is no new pipe, channel, runtime, thread pool, request map, protocol/capability, handle authority or native-flag change.

For an uncontended new ordinary file, `begin` and `commit` remain the only synchronous per-file JSON round trips. Logical commit remains payload drain -> sticky-error check -> Rust-retained metadata/MOTW -> handle close -> completion journal. The stdout pump has already consumed preceding payload before it invokes `session.Commit`; `native_commit_ms` therefore times only that native metadata/handle-close call. Rust `output_close_wait_us` covers the full commit await, including control serialization/send/reply and queued payload the Host must consume before close. Solid RAR/7z and other delayed-integrity units retain the bounded set of final-name handles until their required codec result; failure/cancel deletes only those incomplete owned outputs and keeps successfully closed files/directories.

Archive Create/Edit and split publication remain unchanged: exclusive candidates, complete candidate validation/readback, source/target checks and verified replacement. HZ5 retains append/flush/readback; `ArchiveEntryCache` retains Seal/cursor verification. The current contract is [ordinary extraction output alignment](ordinary-extraction-output.md).

The bounded synchronous-pump follow-up measured begin, close, parent/path-open, directory and directory-metadata cost. Its [historical genuine Release trials](ordinary-extraction-pump-results.md) completed exact-byte button/Root extraction at4241/4214ms medians versus Bandizip1476.011ms,2.873×/2.855×, failing the required<=1.5× gate for that serial runtime.

### Bounded parallel ZIP extraction

The subsequent user request selects [bounded cross-file ZIP parallelism](parallel-extraction.md). Ordinary unencrypted Stored/Deflate ZIP uses independent source cursors and bounded member lanes within the original Job/worker; native begin/close for independent files may overlap. Private `parallel_extraction_v1` extends only negotiated ordinary extraction to bounded call-ID rendezvous. Complete original-pipe frames remain atomic, same-file payload precedes close, decisions serialize, first real errors stick, aggregate progress is monotonic, and every Rust/native task joins before cleanup and lease release. Other owners retain their scope. Compiled lifecycle and genuine scratch-Release acceptance pass: button2370ms/1.297× and Root2283ms/1.249× versus Bandizip1827.211ms; all nine outputs match4492files /313,474,195bytes, with cleanup and environment restoration. [Results and limits](parallel-extraction-results.md).

## 6. Required task, batch and browser interaction

Current prompt arrival can reopen the unique original progress view on a deferred turn. The current ticket opens the existing Dialog, without a second prompt manager or foreground-stealing loop. Pause eligibility follows actual worker state; queued Cancel remains local. One original batch projection selects member details and pending decisions while both active jobs retain their original tickets.

Batch password reuse is explicit, memory-only and batch-scoped inside original secret/prompt ownership. It neither broadens source-bound OLE grants nor stores a password in task history. Wrong-password/cancel/disconnect/retirement clears or invalidates it.

Single-job Retry needs initial native baseline collection as well as a terminal button. It reuses original retry predicates/revalidation/queue and cannot retry published/unknown output. Success-with-warnings permits manual confirmed-result location, not automatic focus. Up navigation restores only the child row just left.

Archive-wide Find is a required catalog projection. Keep current-folder Ctrl+F and provide the separate archive menu/Ctrl+Shift+F command. The original browser Root hosts the Find Dialog, stable query Input and bounded virtual result rows. Rust searches the immutable loaded catalog by name/path and the existing project wildcard rules; there is no archive reread, decoder Job or second archive runtime. Invalid query, empty/no-match and current results remain distinct.

Each result retains the original source/session/snapshot/ordinal, with explicit directories or the existing real-descendant-bound implicit-folder semantics. One current query generation per view prevents an old query/snapshot from installing or acting after refresh/close. Enter/double-click closes the Find surface and navigates/selects the actual member in its original browser; selected extraction/edit reuse original capability, ordinal and final-submit guards. Search/filter changes do not erase the browser's unrelated selection, and display names do not become authority.

### Component selection ledger

| Requested shape | Released public composition | Variant/size and parent adaptation |
| --- | --- | --- |
| Add form, fixed internal target and scrollable source rows | Existing `Form::vertical`, `Field`, original source-row layout/`ListItem` presentation, `Button::new` | Original form tier; per-row Remove Small outline; submit/footer retain existing Medium actions; no new ListState/controller |
| Incoming Open-first/New/Add choice | Existing Root Dialog with `DialogFooter`, `DialogAction`, `DialogClose`, actual Buttons | One primary decision, other Medium outline; Add capability-disabled with reason; bounded body/reachable footer |
| Task and batch progress | Existing official `Progress`, current batch summary and Buttons | Preserve current default/minimum bounds, known/unknown semantics and fixed footer; body scrolls instead of multiplying windows |
| Current conflict/password decision | Existing Dialog/Form/Field, masked Input, Checkbox and original ticket callbacks | Medium32 outlined decisions; controlled checkbox; original focus/cancel/secret clearing |
| Paste and result actions | Existing PopupMenu/keyboard route and Button | Same command owner for pointer/keyboard; source fields retain text paste; icons receive semantic names |
| Toolbar commands | Existing `Toolbar.content` with real Buttons | Medium outline; `Toolbar.child` must not force ghost/compact appearance |
| Archive-wide Find | Existing Root Dialog/Form/Field/InputGroup and presentational ListItem/public virtual-list; original member actions | Current input tier and Medium outlined actions; one query-generation owner; bounded scrolling, keyboard result navigation and explicit empty/error states |
| Create source/options and per-input output review | Existing `Form::vertical`/`Field`, `Input`, `Select`, `Checkbox`, source-row Buttons and bounded scroll body | Preserve current form tiers; Small outlined Remove, Medium outlined secondary/footer actions and one primary submit; no Toolbar.child adaptation |
| Running Stop confirmation | Original window Root `Dialog`/`DialogFooter`/`DialogAction`/`DialogClose`, original controlled Buttons | Medium actions; explicit Continue/Stop, close means decline; one modal owner, no second pause/control dispatcher |
| System multi-folder choice | C# owner-bound native picker; GPUI existing form Button | Same Medium outlined trigger, disabled while pending; preserve native focus/cancel and result correlation, not an in-window imitation |

The ledger uses exact current Kit0.8.0 public families; inspect the active released compound implementation before each UI edit. Default/minimum, long source names, Light/Dark, keyboard, disabled/pending/error and real reader/DPI evidence remain separate. No custom ordinary-control painting or unrelated Quick Launch UI change is included.

## 7. Diagnostics and performance evidence

Reuse operation/job/attempt/call correlation. Record accepted/start, meaningful phase transitions and actual session terminal/cleanup with elapsed/native errors. Aggregate source bytes, revision/catalog/outer/block passes, worker starts, native-call/data bytes, sealed output and cache verification reads in bounded operation-level counters. Production diagnostics contain no archive/member paths, clipboard contents, passwords, raw payload or capability tokens and no per-chunk/pointer events.

Rust `ArchiveWorkObservation` records `output_{begin,close,directory,directory_metadata}_{calls,wait_us}`. Begin includes `begin` and `begin_overwrite`; close is `commit`; directory includes `directory` and `directory_new`. Each wait begins after the previous Rust coalescing tail is flushed and includes control serialization, send and reply wait; close wait also includes queued payload the Host must consume before close. Counts include returned failure and cancellation calls.

Native aggregates are `native_begin_ms` for the whole bridge begin action; inclusive `native_parent_resolution_ms` for validation/cache parent resolution; `native_final_open_ms` for ordinary final-open allocation, syscall and inspection; `native_create_ms` for `NtCreateFile` plus P/Invoke marshalling only; `native_open_inspection_ms` for post-open inspection; and `native_directory_ms` / `native_directory_metadata_ms` for their complete native actions. `native_commit_ms` starts inside the stdout pump after payload drain and measures only `session.Commit` metadata/handle close. `output_close_wait_us` is the full Rust await. `fingerprint_ms` and `postpublication_ms` remain zero; `output_write_ack_frames` is zero; JSON frames remain `2N + operation overhead`; `output_ipc_ack_us` remains pipe send/backpressure time. All durations are inclusive/overlapping, not additive and not exclusive IPC attribution.

Four drag clocks remain distinct: gesture-to-OLE; OLE/descriptor responsiveness; demand-to-verified-stream; receiver end plus cleanup-to-terminal. Ordinary extraction separates source admission, listing/qualification, plan/preflight, decode, Rust pipe send/backpressure, Rust begin/close/directory waits, native begin/open/directory actions, native metadata/close and final cleanup. Update additionally separates new encoding, unchanged encoded copy, candidate verification and replacement.

Source/static checks, compiled fixtures, controlled native consumers, real Explorer/physical UI and release acceptance are reported independently. The original sample is read-only; generated fixtures/outputs use fresh owned roots. Existing required identity/hash mechanisms remain only for their archive Create/Edit, HZ5 and cache scopes; ordinary destination output adds no fingerprint files or direct-file hash/ID retention.

## 8. Creation plan, output and post-create Test

Owners are [CreateDraft](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/draft.rs), [form layout](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/forms_layout.rs), the original [creation planner](D:/Code/NxTools.Next-main/rust/crates/nxtools-archive/src/creation.rs), [jobs](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/jobs.rs), existing format writers and native [creation sources](D:/Code/NxTools.Next-main/csharp/NxTools.PlatformHost/Windows/Archive/ArchiveCreationSources.cs)/[output session](D:/Code/NxTools.Next-main/csharp/NxTools.PlatformHost/Windows/Archive/ArchiveOutputSession.cs). No parallel create runtime is introduced.

Create already supports source Remove. Share only intake/draft presentation with Add, preserving distinct member namespaces. Native observation supplies count/known-size facts where available; unknown totals remain labelled and no source traversal occurs during render. Format changes and submission validate the same admitted method/level/password/solid/header/split combinations. Secrets are private transient state, not persisted options.

There are three separate collision domains:

1. **Ordinary new archive members:** retain generic CreationPlan collision refusal and protected source IDs.
2. **Each-input output archives:** build the complete per-source output-name plan before submitting any member. Reserve disk names, all original requested names and accepted generated names; choose `name (2).ext` and subsequent free names without consuming later original requests. Native final admission remains authoritative when a target changes. No automatic overwrite or unbounded retry loop follows a late collision.
3. **Add members:** use the complete incoming/original namespace and existing tickets from section4; one candidate/publication for each archive modification.

Ordinary single-output overwrite still requires the native observation-bound confirmation. Split output families keep existing no-partial-overwrite admission, per-volume publication journal and failure semantics; do not claim whole-family atomic replacement. Each-input requests reuse one original batch, admission budget and FIFO queue. A partial queue admission reports every accepted/rejected member instead of returning apparent total success.

Current lifecycle is encode → candidate integrity/native readback → publication → actual creation cleanup → optional post-create Test attempt → logical terminal. Test failure preserves already published output and denies generic retry/recycling. Preserve commit state, counts, journal, output-published facts and native-confirmed location across the Test attempt; Test completion cannot rewrite them as zero output. Native result location is captured before output disposal and identifies the real created archive/family. Presentation/location failure never changes archive success and never authorizes guessing a path. Manual actions retain success/warnings/cleanup gates; a failed post-Test retains output facts but does not gain an unsafe action.

## 9. Source recycling remains a blocked required capability

The parent R4 requirement remains visible as CREATE-05. Its original [Rust eligibility](D:/Code/NxTools.Next-main/.trellis/spec/nxtools-archive/backend/source-deletion.md) and [native authorization](D:/Code/NxTools.Next-main/.trellis/spec/platformhost/archive-source-deletion.md) contracts are prerequisites, not replaced by Bandizip's Shell path list.

Production has no proved original-object recycle backend and rejects JobExecution+DeleteSource. Keep that explicit unavailable/refusal state. Resolving the prerequisite requires the owning native capability decision before wiring an enabled product action, with separate source-mutation authorization. A disabled explanation does not complete source recycling; G-29 stays pending while the native prerequisite is unresolved.

After that decision, explicit per-job consent and successful required Test must remain joined to the original eligible regular-file source objects through the original owner. Reopening paths after create cleanup/post-Test cannot mint equivalent source authority. No sharing relaxation, directory-root deletion, excluded-member loss, temporary-source deletion, permanent fallback or retry of unknown recycle outcome is permitted. Archive success and per-source Recycled/Cancelled/Unsupported/Failed/OutcomeUnknown/NotAttempted remain separate facts. This lifecycle work does not create a global lease registry or new worker pool.

## 10. Progress event and confirmed-stop contracts

The C# progress publisher and Rust `ArchiveJobEvent::validate` must agree on the explicit supported phase vocabulary, including the five current edit phases. Preserve old `running` frames and reject unknown/malformed phases. Tests enter through serialized public events and `job_notification`, not directly constructed view snapshots. Current attempt/generation controls application; no new phase string grants control or publication authority.

`ArchiveJobProgress.sequence` is optional, omitted when absent, and positive `u64`/`ulong` when present. The original C# Operation increments it under the session gate only when publishing progress into either existing channel. It continues across password replies/internal phase restarts and starts at1 for a new Operation. Rust applies it only after the original connection/session/attempt/generation gates: reject equal/older or missing sequence after sequenced progress, allow a newer phase/counter restart, and preserve same-phase counter checks for legacy unsequenced observations. `send_job_attempt` clears prior progress. This field orders presentation only; it grants no native/control authority.

Closing/backgrounding rechecks the retained prompt after clearing the original window, through the same deferred ticket/attempt/generation/connection-checked restoration. Terminal/disconnect/replacement cannot revive that decision. The original snapshot projects frozen `modify.browser.format` for SevenZ versus TAR-family preparation/rewrite labels. [AC-01/02/03 regressions](opt01-ac01-ac03-repair.md) enter the original callbacks and C# publication/Rust consumption boundaries.

Running Stop confirmation belongs to the original progress view and job control owner. Capture actual pause/control state and install confirmation pending state before dispatch. If pause is legal, request it through the existing acknowledgement path; only an acknowledged pause acquired for this confirmation may be resumed on decline. An already paused task stays paused. Non-pausable verification/commit stages keep their current rules; do not pretend the worker stopped while showing a question. Confirm sends one Cancel; terminal/retired attempts invalidate the question and any delayed resume. Queued Stop is immediate/local. Existing password/conflict dialogs retain their tickets; stop confirmation cannot submit or replace their answers.

Progress snapshots keep member/overall totals distinct from solid dependency work, actual commits and cleanup. Speed/ETA restarts with the existing phase/attempt metric owner; no percentage for unknown totals, artificial completion or per-byte logging. One batch surface exposes both running workers' decisions and preserves cleanup-bound reservations.

## 11. Browser, extraction and picker preservation

Catalog/source authority and display projection remain separate. Here/Name/Smart/custom/Desktop and all/selected forms freeze the current selection and original source; Smart keeps the current selected-plan root-shape contract. Empty/implicit directories, full subtrees and slash/backslash presentation preserve original member metadata. Capacity/missing-target/readmission prompts keep existing finite budgets and native facts. Their ordinary output follows the shared final-name `CREATE_NEW`/direct-overwrite/close contract, including destination-directory drag; virtual descriptor/IStream drag remains separate.

Encoding reread, refresh and close retire old snapshot/preview/Find actions. Details/name-list/Tree/sort/filter/history and large-catalog virtual rendering retain current owners. Backspace restores only the departed child; Find locates through the same original folder/member action. Automatic preview uses the existing browser selection fingerprint/generation and lease retirement; no synchronous entity writeback, external execution or automatic edit writeback is added.

Multi-folder selection is required in OPT-07 through [ArchiveJobPicker](D:/Code/NxTools.Next-main/csharp/NxTools.PlatformHost/Windows/ArchiveJobPicker.cs). Extend its existing typed result and original STA/owner lifetime to a bounded folder vector. Native cancellation, parent closure, disconnection and late results retire the original pending request. Validate the whole returned set against the same current root/overlap limits before installing it; never truncate silently. New/Add keep repeated folder addition and text-input paste ownership.

## OPT-03 implemented contracts

- Original form intake uses `forms_intake.rs` plus `clipboard_requests.rs`; C# EventWindow owns borrowed CF_HDROP access. `Jobs.edit_forms` retains `EditFormOrigin { browser:(window_id,generation,snapshot), connection }` captured before form construction. Input and final admission require that creation connection even before a replaced connection's Disconnected notification is consumed. Both receipt and deferred application retain per-request connection/form/generation and Add's frozen source/snapshot/folder checks. Small outlined Remove stays local; text-focused Ctrl+V stays text paste.
- `create_output_plan.rs` extends ArchiveApplication, not a scheduler. Native `ArchiveCreateDestination.Observe` supplies a bounded held-root namespace without mutation. Rust reserves disk, every requested name and generated names before original batch admission. Preindexed normalized occupancy prevents per-candidate directory rescans; ZIP/7z family reservation does not imply family-atomic publication. Native final confirmation remains mandatory for a late target.
- Actual Add display facts originate after full selected-plan resolution and confirmed commit; the optional public result is bounded to256 members/65536 UTF-8 path bytes with an independent total. Native successful clean publication carries those facts through the original Job. Matching browser refresh installs fresh IDs and original folder/ancestor; stale contexts never retarget. Confirmed published late cancellation still refreshes but has no success selection/summary.
- Native `ArchiveWorkerRunner.ExecuteAsync` captures held Create result location before disposal. `finish_attempt` starts optional Test only from that confirmed primary; `job_terminal` preserves Create counts/commit/journal/publication/location and combines cleanup while retaining Test outcome. Unknown cleanup keeps the original slot. No display fact grants write/retry/recycle authority.

Executable fields, limits, errors and regression assertions are in [archive batch/input/result contracts](../../spec/nxtools-desktop/backend/archive-batches.md). [OPT-03 evidence](opt03-intake-results.md) records the authorized automated/owned-native stop point; later OPT packages and physical/Release acceptance remain separate.

## OPT-07 implemented contracts

Original Jobs separates defer_presentation from defer_pump; batch membership precedes one-window projection. Summary Stop carries concrete attempt/generation into request_job_stop. Original live Job/Batch facts gate default-off password consent at render/confirmation/reply; successful explicit Retry retires reuse for the whole batch before generation2. One common terminal finalizer covers the last cancelled confirmation with zero Jobs. Pending Locate/auto-location receipts protect the original Job from retirement.

Original C# pipe-session retains only SAME_PROCESS protected candidates; first missing-password attempt1 can reuse once for eligible generation1 Test/full Extract. Exact current ticket/batch/session and wrong/ambiguous/retirement semantics remain separate from preview/OLE grants. Native folder selection uses original bounded STA and owner-bound IFileOpenDialog with FOS_ALLOWMULTISELECT for source folders only. Rust rejects combined existing/incoming root/UTF-8/overlap violations atomically before draft mutation and again at asynchronous publication.

Find uses Dialog::child with official fixed DialogFooter, stable InputState, Checkbox::on_click and virtual ListItem. Query-generation plus original RowId binds both element identity and pointer callback; deferred actions and final selected-Extract/Modify submit check original snapshot/WindowSlot/read connection. Up selects/focuses/reveals the departed child. Both deferred Tree publications read latest same-snapshot folder/focus. Model-local folder cache and transition-owned edit/preview readiness keep full-catalog work out of repeated render; final commands retain authoritative validation.

Exact fields/errors/assertions: [batch](../../spec/nxtools-desktop/backend/archive-batches.md#scenario-opt-07-unified-surface-and-protected-consent), [browser/intake](../../spec/nxtools-desktop/backend/archive-browser-workflows.md), [native owner](../../spec/platformhost/archive-workflows.md), [protocol](../../spec/nxtools-protocol/backend/directory-structure.md#scenario-batch-password-wire), [connection capability](../../spec/nxtools-ipc/backend/directory-structure.md#scenario-connection-scoped-multi-folder-capability). [Implementation verification](opt07-verification.md) and [45-row reconciliation](opt07-coverage.md) do not issue independent/physical/Release acceptance; G-29 remains prerequisite-blocked.
