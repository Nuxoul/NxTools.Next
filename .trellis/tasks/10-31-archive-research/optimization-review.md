# Archive workflow and performance optimization review

## Decision

The primary optimization workstreams are end-to-end drag extraction, ordinary/selected extraction throughput, and adding external files into the current archive directory. They share engine/native mechanisms but require separate observable workflow outcomes. Retain the existing Rust domain/scheduler, GPUI rendering and C# Windows ownership rather than replace the archive module.

Start with extraction/drag first-content costs and Add conflict correctness, then reuse one materialization attempt across a drag's demanded members and optimize the update/bulk-data path. Background prompt and queued-control repairs are small independent fixes alongside those workstreams. The owning task's canonical phases and accepted format/native scope remain unchanged.

Behavioral references: [product map](bandizip-product-reference.md), [shared extraction measurements](overall-extraction.md), [retained ZIP pipeline](bandizip-core-pipeline.md), [operation lifecycle](bandizip-operation-lifecycle.md), [viewer/execution](bandizip-viewer-execution.md), [navigation](bandizip-browser-navigation.md) and [incoming files](bandizip-shell-incoming.md).

## Primary workflow: drag archive entries into Explorer

### Actual chain and distinct-member cold starts

```text
held row/Tree movement -> frozen descriptor index/original ordinal + source HWND
-> original platform admission -> dedicated STA -> one OLE DoDragDrop
-> target FILECONTENTS(index) -> ArchiveVirtualContent.Open
-> per-index ArchiveEntrySession -> fresh worker/source/catalog/extraction
-> native cache publication -> actual worker cleanup -> cache Seal
-> checked native IStream -> target reads -> negotiated EndOperation -> lease cleanup
```

Descriptor requests do not extract content. Same-index requests share one materialization and receive independent stream cursors. Different indices currently create separate full worker attempts, serialized through one actual-byte accounting semaphore. The dedicated STA and `IDataObjectAsyncCapability` already exist; targets can decline async mode, and synchronous fallback still waits for contents.

Sources: [held gesture](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/browser_drag.rs:171), [OLE owner](D:/Code/NxTools.Next-main/csharp/NxTools.PlatformHost/Windows/Archive/ArchiveVirtualDragSession.cs:62), [indexed GetData](D:/Code/NxTools.Next-main/csharp/NxTools.PlatformHost/Windows/Archive/ArchiveVirtualFileDataObject.cs:82), [per-index attempt](D:/Code/NxTools.Next-main/csharp/NxTools.PlatformHost/Windows/Archive/ArchiveVirtualContent.cs:41), [entry cleanup/Seal](D:/Code/NxTools.Next-main/csharp/NxTools.PlatformHost/Protocol/ArchiveEntrySession.cs:169).

N distinct dragged files therefore cause N source acquisitions, worker starts, catalog/revision work, codec setup and cleanup, rather than one selected-set extraction. ZIP repeats admission and record work; 7z/RAR can repeat dependency decoding. Descriptor order is presentation-path order, not archive ordinal order, so receiver order cannot be assumed suitable for a solid decoder.

Use a demand-started, drag-scoped materialization attempt through the original EntrySession/runner/output/cache owners. Retain source/catalog and admit each demanded original ordinal into that attempt. Keep independent member leases/cursors and repeat-index reuse. Reuse only format-valid decoder state supported by the exact backend; out-of-order solid demands may require a reset, but not inherently a new process/catalog. No worker/cache starts before content demand, and undemanded entries receive no filesystem output. Block/folder integrity and dependency completion cannot be bypassed by treating solid members as independent files.

Acceptance: one/two/sixteen ZIP demands; one drag-scoped source/catalog lifetime; repeated/out-of-order indices; same-block 7z and solid RAR work accounting; descriptor-only zero worker/cache; byte-correct independent cursors; cancel, receiver abort, async EndOperation and transferred-stream survival. Real Explorer acceptance remains distinct from controlled consumers.

### First stream waits for redundant complete cache reads

The current first-stream success path includes full reads for native precommit fingerprint, postpublication verification, cache Seal, an `OpenVirtualStream` used only to obtain actual byte accounting, and the real target cursor's `OpenVirtualStream`. The size-only cursor adds a complete hash and native stream open without transferring content.

`Seal` already captures verified actual size. Return it through the existing internal materialization result and charge remaining output bytes without opening an accounting cursor. Preserve every actual target cursor's identity/hash validation. Consolidating other publication/cache observations is a later exact-held-object handoff change, not permission to trust a path or mtime.

Sources: [size-only cursor](D:/Code/NxTools.Next-main/csharp/NxTools.PlatformHost/Windows/Archive/ArchiveVirtualContent.cs:82), [sealed size/digest](D:/Code/NxTools.Next-main/csharp/NxTools.PlatformHost/Windows/Archive/ArchiveEntryCache.cs:174), [real cursor validation](D:/Code/NxTools.Next-main/csharp/NxTools.PlatformHost/Windows/Archive/ArchiveEntryCache.cs:227).

Acceptance: one fewer complete cache read before first stream, including unknown descriptor size; exact actual-byte accounting; same-size substitution between commit/Seal/stream still rejects; independent repeated cursors still revalidate.

### Start latency, preview reuse and final completion

Native parsing and data-object construction both validate the tree; drag admission and content construction also independently reserve/probe source activity. Consume one immutable validated native plan and one session-owned activity reservation, while retaining the actual worker's source lease. Merge selected roots before one Rust catalog scan. Keep the final HWND/capture/pressed-button check at OLE entry; never extract in the pointer callback. [Tree validation](D:/Code/NxTools.Next-main/csharp/NxTools.PlatformHost/Windows/Archive/ArchiveVirtualFileDataObject.cs:51), [activity probe](D:/Code/NxTools.Next-main/csharp/NxTools.PlatformHost/Windows/Archive/ArchiveVirtualContent.cs:32), [selection expansion](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/model_drag.rs:161).

Automatic preview materializes and releases a member before later drag independently materializes it again. A later narrow improvement can retain the visible preview's existing sealed lease and reuse it only under exact source/session/generation/snapshot/ordinal match. Existing cache ownership, actual-byte accounting, per-cursor checks and retirement on refresh/selection/close remain. This covers qualified preview files, not a new global cache. [Preview materialization](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/entry.rs:697), [preview release](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/entry.rs:909).

Measure four intervals separately: gesture to OLE entry; drag-over responsiveness; first content request to verified stream; all demanded content plus receiver/lease cleanup to terminal. Cache-ready, OLE-loop return and receiver-reported copy completion are different facts. Progress stays on the original owner and never labels decoded/cache bytes as measured Explorer destination writes.

Bandizip's ordinary outgoing route uses temporary real-file paths and `CF_HDROP`. Its fast-success branch checks `DRAGDROP_S_DROP`, reads a mapped destination through `FUN_1401aedd0`, then calls common selected extraction `FUN_1401481d0` with that destination. This destination-handoff branch is not NxTools' virtual-stream protocol. The retained branch does not establish the destination producer, live target compatibility or that the measured CLI sample used fast GUI drag. [Outgoing branch](D:/Code/NxTools.Next-main/.trellis/tasks/10-31-archive-research/evidence/raw/bandizip-function-0x140147200.json), [mapped destination](D:/Code/NxTools.Next-main/.trellis/tasks/10-31-archive-research/evidence/raw/bandizip-fast-code-0x1401aedd0.json), [current OLE contract](D:/Code/NxTools.Next-main/docs/reference-code/archive-virtual-drag.md:148). Optimize the existing demand protocol rather than introduce a competing Shell gesture/destination framework.

## Primary workflow: why selected extraction is fast in the reference

The supplied ZIP is 137,737,166 bytes; the selected member is 7,255,143 compressed bytes and 8,324,412 decoded bytes. Historical Bandizip CLI runs took 110.731 ms and 72.7695 ms including process launch. The latter reported 8,355,741 ordinary process-read bytes and one byte-correct output; counters do not attribute every possible mapped read.

The repaired NxTools Debug native sample still measured browser open 8,625.7682 ms, member materialization 18,967.6833 ms and ordinary selected extraction 27,101.1222 ms, all with successful cleanup and the intended output. These are separate operations, not one summed UI delay. A separate core probe measured open 9,203.1564 ms before output IPC. [Selected reference](bandizip-performance-picker.md), [post-repair sample](outcomes.md), [core measurement](overall-extraction.md).

The reference has a concrete work-volume advantage:

- It retains source handle, catalog, decoder and validated member data offset.
- One selected-index set enters the engine instead of reopening for every member.
- Ordinary ZIP seeks to selected payloads, streams codec/CRC/size checks and binary output; the inspected loop has no per-member whole-source SHA-256 pass.
- 7z groups selected files by folder/block; solid RAR advances one ordered dependency range without unselected filesystem outputs.
- Eligible large outputs can overlap producer/consumer writes. The supplied small JAR instead takes direct output, and single ordinary ZIP extraction is sequential: giant buffers or blanket multithreading do not explain this sample.

NxTools adds repeated archive-sized revision/qualification, per-member drag cold starts, decimal-JSON conversion/synchronous output replies and cache observation passes. Perform each required admission/decode/output task once at its proper lifetime, retaining CRC/authentication, original ordinal, protected source and verified publication. Build modes differ and stage shares are not isolated; these measurements do not establish a speed ratio or guaranteed millisecond target.

Acceptance requires lower source-read volume, fewer catalog/outer/block decode passes, lower verified first-stream latency and lower selected-set total time on comparable builds, plus byte equality and native cleanup. A smoother progress window alone is not a performance result.

## Primary workflow: add files into the current archive directory

### Existing end-to-end behavior

Toolbar Add freezes the current internal folder; window drop also freezes and rechecks browser generation/snapshot/folder. The form accepts multi-file selection and repeated file/folder additions, then submits original archive plus external roots through the original serialized queue. C# holds original archive and external source objects. The worker plans conflicts, generates/verifies one candidate and uses the reader-close/native replacement owner. Actual commit and cleanup gate browser refresh. ZIP, qualified ordinary single-volume 7z and TAR/TGZ/TXZ use real adapters; read-only formats/states remain capability-gated.

Sources: [internal prefix](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/browser.rs:1033), [frozen drop target](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/application.rs:1271), [Add form](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/forms.rs:1543), [submission revalidation](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/jobs.rs:205), [native dispatch](D:/Code/NxTools.Next-main/csharp/NxTools.PlatformHost/Protocol/ArchiveJobSession.cs:83).

The target workflow is choose/drop/paste -> adjustable source list and fixed internal destination -> complete candidate namespace/conflicts -> encode accepted new content -> retain unchanged records where possible -> verify one final candidate -> one native publication -> refresh original directory and select actual results. ZIP unchanged compressed-record copying already exists and stays. Current ZIP new entries are explicitly Deflate/unencrypted while old compression/encryption records remain unchanged; automatic inheritance of old encryption is not an existing capability. Non-ZIP rewrites retain their distinct recompression costs.

### Same-batch incoming conflicts bypass Ask

The form deduplicates complete filesystem paths, so `C:\A\report.txt` and `D:\B\report.txt` are both valid sources. Shared `CreationPlan::build` maps roots to basename and rejects the second destination with `OutputCollision` before any ZIP/7z/TAR Add conflict prompt. Ask/Overwrite/Skip/AutoRename cannot resolve this supported input shape. [Source intake](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/forms.rs:1238), [root collision](D:/Code/NxTools.Next-main/rust/crates/nxtools-archive/src/creation.rs:126), [Add planning before conflicts](D:/Code/NxTools.Next-main/rust/crates/nxtools-archive/src/worker_zip_edit.rs:29).

Use an Add-specific complete incoming plan retaining protected source IDs and requested names before namespace resolution. Same-type source/source and source/archive conflicts use original tickets and attempt policy. Overwrite changes only the candidate plan, never source files or an intermediate published archive. Directory decisions bind descendants coherently; different file/directory types keep their explicit refusal. Ordinary new-archive collision policy is not globally relaxed.

Acceptance: same-basename files/directories from different roots, existing members, nested target, case conflicts and every current conflict choice across qualified adapters. Different-type conflicts still refuse; cancel/all-skip publishes nothing; success publishes once.

### 7z/TAR AutoRename can consume a later incoming name

ZIP reserves existing and all incoming names before handling conflicts. 7z/TAR inspect only their current dynamic plan. With existing `same.txt` and incoming `same.txt` plus `same (2).txt`, the first AutoRename can occupy the second source's original destination, causing cascading renames and extra prompts. [ZIP reservation](D:/Code/NxTools.Next-main/rust/crates/nxtools-archive/src/worker_zip_edit.rs:69), [7z loop](D:/Code/NxTools.Next-main/rust/crates/nxtools-archive/src/worker_sevenz_edit.rs:346), [TAR loop](D:/Code/NxTools.Next-main/rust/crates/nxtools-archive/src/worker_tar_edit.rs:190).

Reserve the complete incoming name/type set before resolving any qualified 7z/TAR-family Add conflict. Generated names cannot consume later original requests. Verify content-to-final-name binding under input order changes, nested directories and case conflicts.

### Finish source intake and result selection

- **Per-source Remove:** Add currently renders source text only. Provide official Remove actions on the local draft, disabled while submitted/picker-pending. Correct one mistaken source without restarting the form. [Source list](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/forms.rs:1552).
- **Paste files:** the browser has no file Paste/CF_HDROP intake. Add bounded native file-list reading through the original frozen snapshot/folder incoming path. Accept standard HDROP-only producers; COPY/CUT intent grants no source-deletion permission. Input-field text paste retains its focus owner. [Commands](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/browser.rs:142), [reference Paste](bandizip-input-clipboard.md).
- **Select actual added results:** refresh currently lacks final affected names after Skip/Overwrite/AutoRename. Return a bounded display-only outcome from the original worker, then select actual members in the new snapshot after matching job/attempt/generation and real publication/cleanup. Never reuse old ordinals across snapshots or log private member names. [Refresh](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/browser.rs:431).
- **Multi-folder picker:** repeated folder addition already works; system multi-folder selection is a later convenience within the original STA/owner lifecycle and current source-root budget, not an engine gap. [Picker](D:/Code/NxTools.Next-main/csharp/NxTools.PlatformHost/Windows/ArchiveJobPicker.cs:22).

### Update throughput: encode new data, retain old encoded records

ZIP Add encodes accepted new content into an additions ZIP, fully decodes that temporary ZIP for verification, copies it into the final candidate and decodes the new payload again in final verification. Preserve additions structure/local/central/extent checks and let complete final-candidate verification discharge the new payload check once. Bad CRC, truncation, local mismatch and corrupt new content still fail before native staging/publication. Keep removed-source payload checks wherever today's corrupt-source refusal requires them. [Additions encoding](D:/Code/NxTools.Next-main/rust/crates/nxtools-archive/src/worker_zip_edit.rs:212), [repeated verification](D:/Code/NxTools.Next-main/rust/crates/nxtools-archive/src/zip_edit.rs:508), [reference encoded-copy update](bandizip-create-edit-repair.md).

Measure new bytes encoded, unchanged encoded bytes copied, payload decode passes, native transfer/readback and final publication separately. ZIP retained-record fidelity and stated encryption behavior remain exact; 7z and outer-compressed TAR costs remain format-specific and visible.

## Supporting task and engine changes

The mechanisms and secondary interactions below support the same source, task and native owners.

## Task decision and control reachability

### Background tasks need a durable decision entry

The Prompt event handler stores `record.prompt` and a waiting label. Backgrounding removes the progress window; `refresh_job` returns when that window is absent. The current recovery action is attached to the earlier transient background notification, not to arrival of the new prompt. A single background task can therefore require a password, conflict or destination decision without an accessible decision surface after that notification expires.

Sources: [Prompt handling](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/jobs.rs:910), [refresh without a window](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/jobs.rs:1455), [background/notification ownership](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/jobs.rs:1616).

Restore the unique original progress surface when a current prompt arrives, using the original job/attempt/generation and deferred window delivery. Present the existing Dialog from its retained ticket; do not create another interaction owner or depend only on an expiring toast. The task remains background-capable while it requires no decision.

Acceptance: background then Password/Conflict/Risk/Destination; repeated or stale tickets; close, cancel, disconnect and terminal interleavings; exactly one window and one response; no synchronous cross-entity reentry. Waiting and eventual timeout remain distinct outcomes.

### Pause must reflect actual worker state

A new queued job has no connection. The Pause button is nevertheless enabled until a prompt, terminal, pending control or cancellation disables it. Clicking it reaches the control handler's connection-unavailable message instead of pausing anything.

Sources: [queued record](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/jobs.rs:380), [connection guard](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/jobs.rs:636), [Pause eligibility](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/progress.rs:1109).

Derive Pause/Resume availability from the actual admitted worker/control state. Keep local queued Cancel available; it must not require a connection. Verification, commit, pending pause and cancellation each retain their existing state rules.

Acceptance: two running jobs and one queued job; queued Pause disabled, queued Stop removes only that member; no duplicate control during admission, pause, verification, commit or terminal transitions.

## Highest-return engine work: one validated attempt

### Separate catalog open from edit qualification

`ArchiveEngine::open` always computes modification eligibility for ZIP/TAR/TGZ/TXZ. Ordinary ZIP open hashes the complete source before and after listing; ZIP edit eligibility opens it again and performs two more whole-source revision checks. Test, ordinary extract and materialize also use this open path unless the specific extract output-readmission branch selects its separate entry.

Sources: [open and eligibility](D:/Code/NxTools.Next-main/rust/crates/nxtools-archive/src/engine.rs:66), [ZIP open](D:/Code/NxTools.Next-main/rust/crates/nxtools-archive/src/engine.rs:214), [worker entry selection](D:/Code/NxTools.Next-main/rust/crates/nxtools-archive/src/worker.rs:1189), [ZIP eligibility](D:/Code/NxTools.Next-main/rust/crates/nxtools-archive/src/zip_edit.rs:78).

Use explicit internal interfaces for catalog-only operations and browser capability admission, rather than another caller-supplied boolean claiming source immutability. Test/extract/materialize do not need to independently discover editing capability. A browser that does need it should qualify the already opened source/catalog in the same attempt. Public unleased callers retain their before/after source-change guarantees.

Historical Debug evidence for the supplied 137,737,166-byte ZIP is 280.225 ms for one sequential read, 9,203.1564 ms for open and 4,555.7176 ms for a separate eligibility call. Current source still contains the corresponding four-pass eligible-ZIP open. These numbers establish priority, not a Release speedup target.

### Move archive-sized revision work out of the member loop

ZIP extraction does an initial full structural preflight and source recheck, then a complete source recheck after each output member is decoded and before its commit. Engine-level entry/return checks add more passes. On the ordinary eligible single-volume ZIP path without output readmission, N actually decoded/committed members produce approximately N+7 full-source revision passes across open and extraction.

Sources: [per-member recheck](D:/Code/NxTools.Next-main/rust/crates/nxtools-archive/src/formats/zip.rs:633), [engine extraction](D:/Code/NxTools.Next-main/rust/crates/nxtools-archive/src/engine.rs:535), [revision implementation](D:/Code/NxTools.Next-main/rust/crates/nxtools-archive/src/engine.rs:725).

Within a worker attempt whose original native source acquisition actually holds the protected object, use that authority to avoid rereading the whole archive for each member. Keep source binding and final checks at their owning stages. The public engine path without that native authority remains independently validating. Each member still requires original ordinal, local/central/descriptor consistency, complete codec end, CRC/MAC, actual size, cancellation and native output commit/result.

Acceptance: the same large ZIP with one, two and eight small selected members; source-read volume must not grow by another archive size for every selected member. Retain stale-before-decode/commit, CRC, cancellation and truthful already-published-output regressions. Opening and extraction timings must be measured separately.

### Consolidate duplicate native edit-source observations within a check

`OpenForEdit` separately captures the held source's full digest and the target signature. `VerifyEditSource` again hashes the source and obtains a target signature, whose implementation hashes the target from the beginning. Completion and commit invoke these checks at additional stages.

Sources: [lease capture and verification](D:/Code/NxTools.Next-main/csharp/NxTools.PlatformHost/Windows/Archive/ArchiveSourceLease.cs:89), [target signature hashing](D:/Code/NxTools.Next-main/csharp/NxTools.PlatformHost/Windows/Archive/ArchiveOutputSession.cs:289), [edit commit handoff](D:/Code/NxTools.Next-main/csharp/NxTools.PlatformHost/Windows/Archive/ArchiveOutputSession.cs:407).

At one native validation stage, observe full identity/version once and reuse the digest only after confirming that source and target refer to that same continuously protected object. Keep the final fresh source/target verification, reader-close evidence and immediate prepublication FILE-handle handoff. Reuse does not extend across an unprotected interval. Long digest loops should observe the original cancellation token between bounded reads.

Acceptance: count archive-sized native reads per validation stage; same-object and changed-target controls; wrong lease/read-completion evidence; cancel before handoff; commit success followed by late cancel; unknown publication remains non-retryable. The output fingerprint and postpublication verification are not removed by this change.

## Reuse format work instead of repeating decode and parsing

### Retain the verified TAR spool for the operation

TGZ/TXZ listing already fully decodes the outer stream into an anonymous spool. Automatic edit eligibility opens and decodes the outer stream again. A subsequent selected materialize or Test decodes it for the operation, making three full outer decodes reachable on the ordinary capability-bearing path.

Sources: [compressed TAR discovery](D:/Code/NxTools.Next-main/rust/crates/nxtools-archive/src/engine.rs:239), [qualification](D:/Code/NxTools.Next-main/rust/crates/nxtools-archive/src/engine.rs:106), [outer decoder](D:/Code/NxTools.Next-main/rust/crates/nxtools-archive/src/tar_edit_outer.rs:44), [compound extraction](D:/Code/NxTools.Next-main/rust/crates/nxtools-archive/src/worker_job.rs:121).

Give the current opened attempt ownership of the original source, immutable catalog and verified anonymous spool. Listing, qualification and the requested payload operation reuse that spool. It remains finite, cancelable and delete-on-close, is not serialized into browser snapshots, and does not become a cross-task or persistent cache.

Acceptance: count outer decode invocations for list/Test/one-member materialize, preserve outer CRC/end validation and metadata/work/output budgets, and verify cancellation and spool cleanup. Release payoff remains to be measured.

### Build one private validated ZIP record plan

ZIP preflight validates local records/descriptors for the catalog; streaming then validates local information again. ZIP editing calls that preflight and immediately repeats local/descriptor discovery to build its own record list. `ZipLocation` retains offsets/flags but not the resulting validated data ranges.

Sources: [ZIP preflight](D:/Code/NxTools.Next-main/rust/crates/nxtools-archive/src/formats/zip.rs:432), [streaming](D:/Code/NxTools.Next-main/rust/crates/nxtools-archive/src/formats/zip.rs:518), [edit structure](D:/Code/NxTools.Next-main/rust/crates/nxtools-archive/src/zip_edit.rs:248), [ZIP metadata](D:/Code/NxTools.Next-main/rust/crates/nxtools-archive/src/model.rs:98).

Construct an attempt-local validated plan indexed by original ordinal, containing admitted data offsets, record ends and relevant local/descriptor conclusions. Test/extract/edit use that plan instead of rediscovering it. Charge it to existing metadata limits and invalidate it with its source attempt. This is format implementation behind the existing engine interface, not a general cache framework.

Acceptance: metadata-heavy ZIP open, Test, one-member extraction and rename; count local/descriptor inspections and seeks. Keep mismatched names, descriptor/ZIP64/AES extras, overlapping extents, encoding and cancellation regressions.

### Validate copied ZIP payload once in its final candidate

ZIP edit currently verifies every source payload, copies unchanged encoded extents, then verifies every final candidate payload. Rename/comment therefore decode retained content twice even though it is not recompressed. Additions receive their own verification and are decoded again in the final candidate.

Source: [candidate construction](D:/Code/NxTools.Next-main/rust/crates/nxtools-archive/src/zip_edit.rs:466).

Keep complete source structure admission, encoded copying, final candidate decode/CRC/MAC/size verification and held-partial byte-exact readback. Assign payload integrity checks by the final edit plan: retained/new content is verified in the final candidate; deleted/replaced-away source content is still verified at source so a corrupt archive cannot bypass today's refusal merely by deleting its bad member.

Acceptance: rename/comment/Add/replace/delete, plain/ZipCrypto/AES, corrupted retained and removed members, unknown extras, failed native readback and no publication on failure. Both corrupted-source cases must still reject. Actual decode reduction and elapsed savings require measurement.

### Replace per-selection full catalog scans

Extraction selection builds a complete EntryId map, then rescans every member for each selected directory while allocating normalized path strings. ZIP Delete similarly scans the catalog for each selected input. Supported large catalogs and thousands of selected members make this O(selection × catalog) work reachable.

Sources: [extraction plan](D:/Code/NxTools.Next-main/rust/crates/nxtools-archive/src/planning.rs:103), [ZIP deletion plan](D:/Code/NxTools.Next-main/rust/crates/nxtools-archive/src/zip_edit.rs:188).

Validate session/ordinal directly, normalize member paths once, merge overlapping directory prefixes and collect descendants in one catalog pass using component-ended subtree matching. Reuse that domain logic for extraction and edit planning; browser presentation is not a second naming authority. Retain original ordinal order, collision checks and exact format-record preservation.

Acceptance: overlapping/nested directory selection, `a/` versus `ab/`, slash/backslash names, stale IDs and unchanged extraction/delete membership. A pure in-memory large-catalog test should count visits, not rely solely on a wall-time threshold.

## Optimize the existing private bulk-data interface

Rust output builds a new `Vec<u8>` per call, serializes it as a JSON number array and synchronously waits for one native reply. C# parses the JSON, allocates another byte array and converts each array element before writing. Creation source reads and candidate readback return Base64 and likewise share the synchronous rendezvous. This affects extraction, materialization, creation and editing.

Sources: [native call construction/wait](D:/Code/NxTools.Next-main/rust/crates/nxtools-archive/src/worker_output.rs:371), [output write](D:/Code/NxTools.Next-main/rust/crates/nxtools-archive/src/worker_output.rs:513), [frame serialization](D:/Code/NxTools.Next-main/rust/crates/nxtools-archive/src/worker.rs:698), [C# byte conversion](D:/Code/NxTools.Next-main/csharp/NxTools.PlatformHost/Windows/Archive/ArchiveWorkerOutputBridge.cs:118), [creation source reads](D:/Code/NxTools.Next-main/csharp/NxTools.PlatformHost/Windows/Archive/ArchiveCreationSources.cs:378), [source adapter](D:/Code/NxTools.Next-main/rust/crates/nxtools-archive/src/worker_sources.rs:87).

Keep the original worker pipes, process/Job lifetime, operation/token/call correlation and native ownership. Introduce an explicitly negotiated bounded binary bulk frame on that private interface, shared by source read, output write and partial readback. Control, prompts and terminal facts keep their typed control representation. Use a fixed finite buffer budget and ordered backpressure; finish/commit must wait for all preceding writes, and cancel/EOF must wake pending producers and drain through original cleanup. No plaintext source spool, extra public business channel or replacement worker pool is needed.

C# already buffers normal writes and does not flush every chunk: [native Write](D:/Code/NxTools.Next-main/csharp/NxTools.PlatformHost/Windows/Archive/ArchiveOutputSession.cs:340). Its [commit fingerprint/flush/postcheck](D:/Code/NxTools.Next-main/csharp/NxTools.PlatformHost/Windows/Archive/ArchiveOutputSession.cs:439) serves publication and cleanup guarantees and remains separate from transport optimization.

Acceptance: isolate decode, transfer, native write, verification and commit time on the same fixtures; record transferred bytes, frame count and bounded memory. Test short read/write, backpressure, stale token/call, cancel, pipe loss and no commit before drain. Do not claim this channel explains all historical elapsed time before measuring its share.

## Consolidate batch and input presentation

### One batch surface over the existing queue

`submit_batch` calls `submit_job` for each member, and each submission opens a progress window before the batch is bound. A multi-archive action can consequently open a window for every running and queued member.

Source: [submission and presentation](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/jobs.rs:415).

Retain the existing two-running/FIFO/batch/job/attempt owners. Use one batch progress surface with member details and explicit pending-decision selection. The two active workers remain independent; one presentation must not discard either worker's prompt ticket. Background/close remains presentation-only.

Acceptance: two active plus queued members, simultaneous prompts, member failure/cancel, detail switching, summary and close without implicit cancellation or window flood.

### Complete Open/New/Add for dropped paths

The existing dialog retains all paths for Add but only `paths[0]` for Open, and provides no New action. The editable case does not tell the user that Open uses only the first input.

Source: [drop action dialog](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/application.rs:1193).

Offer explicitly named Open-first, New archive from all paths, and capability-gated Add to the frozen original target. Reuse the current create form and Add owner; cancellation changes nothing. Do not silently reinterpret all inputs or lose the frozen target/generation guard.

Acceptance: multiple archives, ordinary files/directories, mixed input, read-only current archive, cancel and target snapshot changes.

### Batch password reuse needs batch scope, not persistence

Password prompts have no apply-to-batch control, and the protocol explicitly disallows `apply_to_all` on Password replies. Matching-password archives in a batch are therefore prompted independently despite the required batch reuse behavior.

Sources: [password prompt](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/progress.rs:461), [reply validation](D:/Code/NxTools.Next-main/rust/crates/nxtools-protocol/src/archive_job.rs:639).

Add an explicit current-batch memory-only reuse choice through the original prompt/secret owners. Wrong password invalidates the retained answer and prompts again; batch/attempt retirement clears it. This is not a password manager or a saved preference.

Acceptance: same and different passwords, wrong-password invalidation, independent batches, cancellation, disconnect and terminal secret retirement; no plaintext diagnostics or persisted secrets.

## Finish existing result and navigation actions

### Single-task safe retry

The retry predicate already restricts retries to qualified Test/full Extract, actual cleanup, affirmative zero publication, approved failure categories and native source/target revalidation. The action is rendered only in batch member summaries. Initial retry-baseline observation also rejects records without batch membership, so adding a button alone is insufficient.

Sources: [retry predicate](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/batch_jobs.rs:12), [baseline qualification](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/batch_jobs.rs:333), [batch Retry action](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/batch.rs:150).

Acquire the same bounded initial read-only baseline for qualified single tasks and expose the original retry action on their terminal surface. Preserve fresh native revalidation, one new attempt, and refusal after any published/unknown output. Create/modify/HZ5 retries are not generalized by this change.

### Manual result location after success with warnings

Both automatic open and the manual Locate button require exact `Succeeded`. A terminal with warnings, trustworthy `result_location` and completed cleanup therefore has no manual result action.

Sources: [completion policy](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/jobs.rs:1168), [manual Locate](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/progress.rs:1090).

Keep automatic focus limited to complete success. Allow manual location of a confirmed warning result, showing skipped/warning counts. Reuse the native result-location authority; failed, canceled or cleanup-unknown outcomes do not gain a guessed path action.

### Parent navigation should reveal the folder just left

`up` delegates to `enter`, which clears selection, focus and anchor. Backspace therefore does not select/reveal the child directory just left in the rebuilt parent projection.

Source: [parent navigation](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/model.rs:255).

Retain the old directory RowId for upward navigation and select/focus/reveal that one child after projection. Keep root a no-op; do not add a general multi-selection history framework.

### Archive-wide Find is a later enhancement

Ctrl+F currently filters current-folder projection rows and removes invisible selection; it is not archive-wide Find. This matches the existing local-filter shape, so it is an enhancement rather than a blocker for the first optimization slice.

Sources: [filter entry](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/browser.rs:249), [projection and selection](D:/Code/NxTools.Next-main/rust/apps/nxtools-desktop/src/archive/model.rs:109).

Keep the local filter. A separate Find projection can search the already loaded catalog without archive I/O, retain original ordinals and navigate to/reuse existing member actions. Its scope and command should be approved before replacing Ctrl+F behavior.

## Delivery order

1. **Extraction and drag first-content:** measure comparable builds; consolidate ZIP open/qualification and per-member revision under actual source authority, retain one operation spool/validated record plan where applicable, and remove the size-only virtual stream/hash. Preserve correct source, CRC, cache seal, cursor and native result semantics.
2. **Add correctness and usability:** same-batch incoming collision planning and full incoming-name reservation first, then removable sources, Paste intake and actual-result selection. Preserve the current internal destination/snapshot freeze, per-format capability and single candidate/publication.
3. **Multi-member drag reuse:** one demand-started retained materialization attempt, shared source/catalog and legal format-dependency work, separate member leases/cursors and original cancellation/async receiver cleanup. Initial metadata/identity validation is performed once per admitted plan, not removed. Preview lease reuse is a later narrow optimization.
4. **Shared update and bulk throughput:** avoid repeated new/retained payload decode, consolidate same-stage native observations, make selection expansion one pass, and replace decimal JSON bulk data on the original private worker interface with bounded ordered transfer. Measure these costs separately rather than raising timeouts or worker counts.
5. **Independent lifecycle repairs:** background prompt reachability and queued Pause eligibility can proceed alongside the primary workstreams. Single batch presentation/password scope, single-task retry and manual warning-result location follow through the original owners; parent navigation and archive-wide Find remain smaller later items.

All slices stay in the current Rust domain/plan/scheduler, GPUI rendering and C# Windows ownership. Existing format support, source recycling and HZ5 qualification do not expand merely because performance or presentation changes.

## Retained strengths

- Original session/ordinal member identity and immutable catalog projection.
- Source-bound contextual extraction and correlated directory picker with pending retirement.
- Slash/backslash normalization in browser selection and drag, without rewriting original member metadata.
- One original queue/attempt/terminal authority; published and unknown native commit facts outrank late cancellation.
- Exclusive owned partials, complete codec/CRC/MAC/size validation, held candidate readback and native publication.
- Dual extraction progress with processed bytes distinct from committed output and no fabricated final 100%.
- Deferred cross-entity callbacks, original preview generations and lease cleanup.
- Official Kit0.8.0 controls and existing layout ownership; no visual framework or palette replacement.
- Default-off object-bound HZ5 continuation with fresh Start, separate irreversible consent and original native validation.

## Evidence limits

This review confirms current source paths and compares them with retained reference evidence. The supplied ZIP measurements are historical Debug measurements; they are not current Release benchmarks or a measured speed ratio against Bandizip. Format decode counts and algorithmic costs above are static findings; their individual elapsed contributions remain unmeasured.

No current product build, runtime performance experiment, GUI/Explorer interaction, source recycling, HZ5 mutation, package or installed-release acceptance is part of this review. Real focus/DPI/accessibility, native target interaction and crash/power-loss guarantees retain their independent acceptance requirements.
