# Bandizip product reference

## Product model

The reference is a complete archive workflow: acquire a source, retain its catalog, select or transform members, plan outputs, execute through a worker, resolve questions on the owning window, verify configured results and release resources. Compression formats are one part of that workflow, not its organizing boundary.

The inspected installation is `D:\Tool\Bandizip`: Bandizip/bz/ark.x64 and the independently identified x64 `ark.x64.lgpl.dll`/`data\lm.x64.dll` report version7.32.0.1; scanner helper `data\Amsiman.x64.exe` reports7.17.0.4, and `bdzsfx.x86.sfx` is a separate x86 stub. Native observations are reconstructed by Ghidra12.1.4, not original source or observed execution. The [feature inventory](../../../docs/reference-code/archive.md) retains shipped menus/dialogs/settings; the reports below connect actual call/data/state paths. Approved NxTools delivery remains owned by [the archive task](../10-16-archive-manager/prd.md), not this research map.

## Functional reference map

| Product area | Connected mechanism | Primary report |
| --- | --- | --- |
| Open and recognize | Header/content classification, extension-dependent candidates, retained input interfaces and format-specific catalog admission; recognition can still end in an unsupported operation | [Format families](bandizip-format-families.md) |
| Browse and Find | Find walks the loaded catalog, matches path/name or wildcard mode, retains original member indices, navigates back to the source browser and reuses extraction/edit actions | [Browser and integration](bandizip-browser-integration.md) |
| Extract selected/all | One indexed selection plan; ZIP independent members, RAR solid predecessor state,7z folder grouping, TAR offsets, compressed-stream dependencies and image extents have different work units | [ZIP core](bandizip-core-pipeline.md), [solid extraction](bandizip-solid-output.md), [other families](bandizip-format-families.md) |
| Extraction destinations | Target modes and owner-bound chooser; smart mode consults whole-catalog root shape and the single-file subfolder setting before choosing base or archive-derived directory | [Browser and integration](bandizip-browser-integration.md), [picker](bandizip-performance-picker.md) |
| New archive | Source-list and destination validation, format-dependent constraints, semantic level selection, deferred member planning and separate encode/finalization | [Create/update/test/repair](bandizip-create-edit-repair.md) |
| Each input to its own archive | Per-source output records, extension generation and collision checks against both disk and names already planned in the batch, then sequential per-output execution | [Create/update/test/repair](bandizip-create-edit-repair.md) |
| Split and SFX creation | Segment-aware writer transitions; prefix and GUI settings/language block embedded in ZIP; x86 stub parses configuration, extracts, gates RUN on success and returns cancellation to enabled controls; output close, deletion policy and process-launch result stay distinct | [Create/update/test/repair](bandizip-create-edit-repair.md), [advanced/SFX](bandizip-advanced-sfx.md), [SFX cancellation](bandizip-sfx-cancellation.md), [split reader](bandizip-format-families.md) |
| Advanced creation options | UI/settings→native options→ZIP/7z/TAR writer effects; force-ZIP64 reaches end/locator emission, while adaptive Deflate uses distinct single/parallel thresholds and level0 coding without changing to ZIP Stored | [Advanced options and SFX](bandizip-advanced-sfx.md), [ZIP writer policy](bandizip-zip-writer-policy.md) |
| Add, replace, delete, rename | Existing ZIP members become original-index-bearing update records; rename/delete change the plan, retained encoded payloads copy and changed/new data encode | [Create/update/test/repair](bandizip-create-edit-repair.md) |
| Test and repair | Optional post-create reopen/test is a distinct stage; ZIP repair scans recoverable local records, reconstructs missing extent/CRC facts when possible and writes a separate output | [Create/update/test/repair](bandizip-create-edit-repair.md) |
| Batch extraction and volumes | One batch parent, one active archive worker at a time, per-archive outcome and next-item advancement; recognized later volumes can be removed from the input list rather than extracted as independent archives | [Operation lifecycle](bandizip-operation-lifecycle.md) |
| Existing-file questions | UI-owned conflict event, explicit skip/overwrite/rename response, proposed rename display and a separate persistent apply-all choice | [Operation lifecycle](bandizip-operation-lifecycle.md) |
| Password questions | Prerequisite and per-archive prompts, wrong-password retry and invalidation of a remembered batch choice; cancellation feeds explicit stop state | [Operation lifecycle](bandizip-operation-lifecycle.md) |
| Progress and task control | Background worker/event handoff, synchronized snapshots rendered outside the lock, separate member/overall progress, pause/resume, confirmed stop and worker join, retained/closed finish surface and tray visibility | [Operation lifecycle](bandizip-operation-lifecycle.md), [output lifecycle](bandizip-solid-output.md) |
| Copy, paste and incoming drop | Clipboard materialization/text, legacy synchronous drop and queued OLE input have different owners. Explorer ZIP DropHandler binds a target path and launches operation9; the open-window OLE callback posts before later Open/New/Add selection and cleanup. Shell acceptance is not the EXE archive outcome | [Input/clipboard](bandizip-input-clipboard.md), [Shell and queued input](bandizip-shell-incoming.md) |
| Failures, cancellation and output setup | Confirmed task stop is separate from current-output conditional deletion; buffered close drains/joins, deletion is not independently confirmed; conditional preallocation, long-path output, all/selected capacity confirmation and first-volume reopen have distinct owners | [Failure and cleanup](bandizip-failure-cleanup.md) |
| Open/edit an entry and nested archive | Original-member temporary extraction, associated executable warning/choice, success-gated whole-archive extraction before selected launch, distinct editor/self-child modes, exit-time change check and per-parent verified replacement/backup cleanup | [Viewer/execution](bandizip-viewer-execution.md), [nested writeback/replacement](bandizip-nested-writeback.md) |
| Names and encoding | Menu command → four-field encoding state → reader reopen → catalog-derived refresh, with captured browser-location restoration and a distinct auto-detection branch | [Encoding details](bandizip-browser-scan-details.md) |
| Archive thumbnails and image viewers | Thumbnail member-index decode/cache/publication is separate from reusable embedded `/aschild` string/context IPC and standalone archive-aware viewers. Associated mode materializes the member; standalone launch has no fallback in the inspected browser owner | [Thumbnail lifecycle](bandizip-browser-scan-details.md), [viewer/execution](bandizip-viewer-execution.md) |
| Settings and OS integration | Settings load/save/export, browser/view preferences, Shell action intent, application capabilities registration and Explorer selection/verb command construction; Windows default selection remains a separate boundary | [Browser and integration](bandizip-browser-integration.md) |
| Password manager and recovery | Unlock/validation retry, OK persistence versus session-master changes and immediate reset; recovery pause/resume/stop and separate provider-supplied length→worker transfer. Exact edition numeric cutoff remains unestablished | [Password controls](bandizip-password-controls.md), [length source](bandizip-recovery-length.md), [tools](bandizip-tools-security.md) |
| Malware scan and download provenance | Candidate filter → bounded scan stream → in-process/helper provider → verdict/worker/UI; helper request, detection count and permission to extract are separate. Provenance setting → native context/retained source ADS → suffix-filtered direct/buffered copy, with a result separate from main-file success | [Helper results](bandizip-amsi-helper.md), [scan path](bandizip-browser-scan-details.md), [provenance propagation](bandizip-provenance.md) |
| ZIP comments and archive management | Comment read/edit/EOCD→validation/replacement/refresh; draft label editing→explicit ZIP directory member→publication, including distinct empty/cancel behavior; Shell-copy Save As, close-before-delete and separate normal/elevated launches | [Comments and management](bandizip-comments-management.md), [directory commit](bandizip-folder-commit.md) |
| Browser controls | Immediate column/view/toolbar changes; toolbar close saves already-applied clicks; catalog-size aggregates are reused within a tree and released on Close/Open/edit-reopen. Tree/path history, explicit Backspace child selection, F6 and recent-list owners remain distinct | [Navigation](bandizip-browser-navigation.md), [toolbar/cache lifecycle](bandizip-toolbar-cache.md), [browser/Shell controls](bandizip-browser-shell-controls.md) |
| Shell output naming and mail handoff | Rendered rule→retained component→create output; `-email`→ordinary archive→MAPI compose request. Missing create-success gate, unchecked ANSI conversion and discarded MAPI result are separate findings; the archive is retained | [Shell delivery](bandizip-shell-delivery.md) |
| Explorer archive preview | QueryContextMenu→native worker→Ark catalog→retained flat/tree records→timer-driven menu refresh; flat file-count limit, callback deadline, owner-cleanup cancellation, reader/module lifetime and natural-completion handle leak are distinct | [Shell preview lifecycle](bandizip-shell-preview.md) |

## Format coverage is not an extension count

The [reader-family report](bandizip-format-families.md) records the complete inspected manager dispatch, real decoder interface pointers and explicit unsupported routes. ZIP-derived containers such as JAR/APK are not separate extraction engines merely because they have separate extensions. Single-file streams, compressed TAR, record-indexed images, solid containers, split byte streams and legacy method-dispatched formats are different data plans.

Connected representative paths cover ZIP Stored/Deflate, RAR/RAR5 solid processing,7z folders/LZMA/LZMA2, sequential/sparse TAR, gzip/bzip2 TAR, raw gzip/Zstd, ISO/UDF, selected/skip CAB and EGG, numbered split input and ZPAQ. [Compressed WIM](bandizip-wim-payload.md) follows a distinct classifier route into the shipped LGPL helper, joining resource/chunk tables, selected solid slices, XPRESS/LZX/LZMS, resource integrity and native result/cleanup. LZH/ALZ/ARJ and other method bindings remain at their stated evidence level; none of these traces certifies complete method, volume, encryption or runtime acceptance.

The nine writer dispatch branches and format-specific creation constraints are recorded separately. Readable, writable, editable, testable, recoverable and previewable must not collapse into one `supported` flag.

## Reusable behavior and efficiency

### Plan once within an attempt

The engine retains source interfaces and the loaded catalog. GUI selection, Find results and batch items preserve real member indices instead of inventing another content identity. Format-specific plans account for required predecessor or block work without creating unselected filesystem outputs.

NxTools already has its own source/ordinal/native-authority owners. The useful adaptation is reuse of their validated state, not a new caller flag claiming immutability. [Shared-engine measurements](overall-extraction.md) establish repeated whole-source revision work before payload decode; this remains the first measured common optimization boundary.

### Keep task and question lifetimes explicit

The batch parent is separate from the active archive worker and the progress window. A conflict response and apply-all policy have different lifetimes. A wrong password invalidates the retained answer rather than poisoning the rest of a batch. Declining a stop confirmation restores the prior pause state. A window may remain visible after work completes; hiding/tray presentation is not cancellation.

Those behaviors map to the original NxTools scheduler, operation/attempt/generation and notification owners. They do not require a parallel interaction framework or synchronous GPUI/native callbacks.

### Count work and output completion separately

A solid predecessor may consume decode work without producing a selected output. A codec may finish while the native output queue still needs drain/close. A create job with testing enabled has a real verification phase after encode. Archive success and optional source-deletion success are different outcomes. Per-member and aggregate progress must describe the right work unit without treating processed bytes as published bytes.

### Reuse encoded data where the format permits

The inspected ZIP update path copies unchanged compressed extents and rebuilds relevant headers/directory records. TAR and7z have different rewrite/dependency costs. Editing capability must include archive state and metadata preservation, not only the existence of an encoder. The existing [open-source editing reference](../../../docs/reference-code/archive-editing.md) remains the source for reusable implementation/API choices and licenses.

### Preserve stronger native guarantees

Bandizip's source-delete workflow uses an optional test warning and Shell path-list operations. Browser replacement uses two `MoveFileW` calls and an unchecked best-effort move-back on second-move failure; each nested parent has its own admission/confirmation. Those are not substitutes for NxTools' native original-object binding, output verification, publication and cleanup. A cheap/fast reference path is not authority to lower the product's data guarantees. Likewise, a ZIP repair result needs verified payload recovery rather than a newly written central directory alone.

## Evidence and limits

Complete local observations, query parameters, target/provider identities, independent snapshots and replay recipes are retained in [the evidence archive](evidence/README.md) and [index](evidence/index.json). Raw native reconstructions remain local/ignored, not product code or distributable artifacts. Bandizip EULA§3 and applicable-law limitations remain recorded in the reference note; proprietary code reuse is not proposed.

The measured runtime reference remains the supplied ordinary ZIP member and its byte comparison. Broader paths are static implementation evidence. Remaining static edges include history-capacity/enable-state details; Shell drop-vector final destruction and operation-9 delete-intent field transfer; popup-close/COM-release/publication-timer retirement; SFX delete-on-cancel policy enablement; recovery's exact Standard/paid numeric length and per-candidate boundary; WIM multi-volume/solid-admission details; and missing-volume/less-common method branches. Reports preserve known result/lifetime defects separately from those gaps. Viewer/Explorer acceptance, concurrent queued inputs, all extract-all temporary siblings' removal after cancel/crash, large-catalog aggregate latency, live focus/DPI/accessibility, mail-client transmission, exhaustive codec/edition and installed-release acceptance remain unverified. No product feature, dependency, package, service, settings or installed binary was changed by this research.
