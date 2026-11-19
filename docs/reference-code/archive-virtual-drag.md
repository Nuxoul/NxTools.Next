# Phase F virtual-file OLE spike reference — 2026-10-22

Current2026-10-23 scope includes authorized bounded production ArchiveBrowser rows; historical spike
acceptance/real target gaps remain. Earlier archive/container/cache decisions are not reopened.

Reference review order: ABDM CompletedDownloadPage.kt103–135 uses Compose dragAndDropSource on the
visible pressed action; DownloadItemListDataFlavor.kt14–39 exposes javaFileListFlavor. Legacy
NxTools DownloadCompletedWindow.axaml.cs78–146 enters WindowsFileDragService29–61 from pressed
pointer; that service publishes real-file CF_HDROP, not virtual contents. Its fallback is historical,
NOT adopted. Next NativeFileDragCoordinator BeginAsync installs one admission before dedicated STA;
NativeFileDragService validates HWND, attaches input queues, sends WM_CANCELMODE, verifies left
GetAsyncKeyState highbit, then one Ole32 DoDragDrop. Shared serial reader detaches native work via
PlatformCommandSequenceDispatcher. No GPUI drag owner/clipboard/SendInput/after-extract launch.

Windows SDK10.0.26100.0 inspected: um/ShlObj_core.h1773–1774 CFSTR_FILEDESCRIPTORW/FileContents;
1833–1849 FD flags;1865–1879 FILEDESCRIPTORW (592bytes, UTF16[MAX_PATH], filename offset72),
1897 FILEGROUPDESCRIPTORW count+array. um/objidl.h IDataObject/FORMATETC/STGMEDIUM and IStream:
query exact DVASPECT_CONTENT/ptdNULL; descriptor lindex-1/TYMED_HGLOBAL, indexed contents
0..count-1/TYMED_ISTREAM. IDataObject GetData transfers medium ownership, consumer calls
Ole32 ReleaseStgMedium; pUnkForReleaseNULL means HGLOBAL/stream released by standard medium rule.
IStream uses SDK read/seek/stat/clone native contract; SHCreateStreamOnFileEx can return an ordinary
read-only native stream with STGM_SHARE_DENY_WRITE, avoiding managed COM stream final-release guesses.
No IDataObjectAsyncCapability advertised: synchronous GetData blocks ONLY owning dedicated OLE STA,
not GUI/IPC reader/event thread; async workers service the request. Already returned native streams
retain their own file handle after drag ends; new content requests stop after retirement.

Existing materialization chain: Rust ArchiveEntryCommand immutable browser source/snapshot/ordinal ->
ArchiveEntrySession.Dispatch bounded admission -> Task.Run Start -> original ArchiveWorkerRunner Job/
source/codec/output -> actual cleanup -> Cache.Lease.Seal -> result. The spike must request only on
FileContents, preserve each original ID and512MiB entry/aggregate bound, never pre-materialize in the
pointer callback. Source/close/disconnect cancellation uses original cancel and lease release, no
path-delete substitute. A tiny native safe-read accessor may validate sealed lease identity/version/
hash under held parents and open read-only native IStream while locked, then release only transient
handle. Existing cache Release retains busy target-owned data safely for original bounded cleanup.

Diagnostics before acceptance: correlation accepted/start/content_requested/content_ready/terminal/
cleanup, meaningful count/index/nativecode/elapsed only; no names/paths/source bytes/passwords. No
per-read/frame/pointer logs. Unsupported format/index/tymed/native target outcomes remain truthful.
Live source HWND/physical external target remains NOT RUN under existing GUI safety denial; this
is not Windows capability rejection. Production archive-row hookup waits for independent spike gate.

## Verified production-row reference review — 2026-10-23

User now authorizes production source hookup; earlier disconnected/spike wording is historical.
Re-read ABDM `CompletedDownloadPage.kt:106-139`: `dragAndDropSource` is a gesture detector, its
onClick is empty; `DownloadItemListDataFlavor.kt:14-39` returns completed paths only. It does not
justify triggering OLE on every ordinary archive-row down. Legacy `DownloadCompletedWindow.axaml.cs`
78-97/119-143 is a dedicated drag surface PointerPressed; `WindowsFileDragService.cs:29-61` calls
one real-file OLE loop after EnsureOleInitialized, with historical Avalonia fallback NOT adopted.
Current `NativeFileDragService.cs:64-121` validates HWND -> OLE -> attaches source input ->
WM_CANCELMODE capture release -> actual left highbit; coordinator BeginAsync admits then starts one
background STA, original dispatcher detaches unbounded work. Virtual session retains its identical
capture/button/final-cancel boundary and original content/cache owners. No competing GPUI drag layer.

Current official Component0.8.0 `list/list_item.rs:115-138,179-223` exposes Click/MouseDown and original
InteractiveElement move/up listeners; presentation remains selected ListItem32DIP, existing uniform_list
and sole BrowserModel selection. Parent root only arranges rows; no Toolbar/action variant change.
Approved ordinary-row seam: candidate on down, >=4DIP held movement, current complete selection freeze,
direct platform enqueue from that move callback with actual native Window HWND. Source/view projection
and generation invalidation cancel exact native operation; cross-entity notifications remain deferred.
This source/reference contract is NOT physical gesture/Explorer/foreign COM target acceptance.

## Formal production chain now wired

`ArchiveApplication.consume` installs the completed original `ArchiveBrowserContext` + platform bridge
into the view on a deferred turn. `ArchiveBrowser.render_row` official ListItem leftDown records a
candidate; root move checks stillpressed/4DIP and freezes complete BrowserModel selected rows, direct
`PlatformCommand::ArchiveVirtualDrag` enqueue before mouseup. Real build resolves current Window HWND;
only synthetic TestWindow uses cfg(test) HWND override. New formal wire/capability plus legacy alias
share original `ArchiveVirtualDragSession.Dispatch` before detached receipt wait, same one STA/OLE.
No changes to materialization/cache/stream retirement owners described above. Formal start results
match requestreceipt, not dragID; exact cancel has separate receipt and cannot clear original pending.
Current view local state retires on projection/refresh/source/close; app routes disconnect/late results
on deferred turns. Unsupported mixed selection rejects wholly with official Alert, never drops items.
Controlled formal tinyZIP/stream/cancel fixtures are not actual Explorer/physical/foreignCOM proof.

## F-P208-AC01 connection-origin repair boundary — 2026-10-23

Independent acceptance traced a missing identity, not a new gesture/native capability. Original
`nxtools-archive/product.rs::ArchiveReadClient::start` captures IPC connection before
`PlatformBridge::send_archive_read` atomically verifies it under CommandRouter; Pending already owns
that value. Preserve it on ArchiveReadOperation after terminal removes Pending, through original
`ArchiveApplication::consume` deferred install to `ArchiveBrowser::configure_drag`, then compare
that ORIGINAL connection at formal start in the original virtual-drag router lock, before any
pending/enqueue/reservation. Browser generation is NOT connection identity; current is_connected or
sampling generation during configure/move is insufficient. Tests leave old Disconnected queued and
replace connection after read completion/before install OR after row Down/before Move, then assert
new receiver empty/no admission leak and fresh read succeeds. Layout/focus/threshold/native show/
capture/oneSTA/content/worker/cache/cleanup owners above are unchanged. No wire/C# change.

## Directory / unknown-size native reference — 2026-10-23

Re-read installed SDK `C:\Program Files (x86)\Windows Kits\10\Include\10.0.26100.0`:
um/ShlObj_core.h1839 FD_ATTRIBUTES4,1843 FD_FILESIZE0x40,1847 FD_UNICODE0x80000000,
1865-1879 FILEDESCRIPTORW fields (592bytes,attributes36,sizeHigh64/Low68,name72),1897groupcount.
um/winnt.h FILE_ATTRIBUTE_DIRECTORY0x10. Directory descriptors carry attributes and Unicode but
NO FD_FILESIZE; unknown-size files likewise omit size validity, never promise zero. FORMATETC
objidl.h12246-12253 lindex indexes the FULL frozen descriptor array; directories reject FileContents
without invoking materialization. IStream STATSTG objidl.h2682-2695 cbSize reports ACTUAL sealed file
size, independent of absent descriptor estimate. Existing SHCreateStreamOnFileEx independent native
cursors/held identity/hash/medium lifetime remain. Controlled consumers do not prove Explorer behavior.
Original BrowserModel.entries owns immutable source metadata; selected roots expand without IO.
32roots/128descriptors/32files (cache32leases)/256requests/256KiBcommand bounded; descriptorHGLOBAL
<=75780bytes. Existing1MiBpipe unchanged. Optional estimate plus actual remaining512MiB output bound
must be enforced at original EntrySession/worker/output, not a second extractor or post-write-only gate.
Receivers require every ancestor as an actual same-case directory descriptor and independently cap
32distincttop-levelroots; no impliedancestor count bypass. BrowserEntry source-family admission keeps
its original512MiB ceiling while ONLY entry/outputbudget narrows to actualremaining, so a small selected
file from a larger encoded archive remains eligible. Original non-BrowserEntry limits are unchanged.

## P208 password preauthorization seam — 2026-10-23

Read PhaseCcloseout/extractpassword accepted contracts and indexed password-operation/extract-password
sources FIRST. Verified `archive/password.rs` original Dialog/maskedInput/deferredreply + currentticket,
`ArchiveBrowserPassword.Respond` -> `ArchiveEntrySession.RespondPassword` -> original runner private
`password_reply` -> worker_password::execute/InteractionPort max3typedattempts. ZIPopen is not key
validation; extractpassword sources require fullnooutputTEST before oneextract. Reuse that TEST loop
for explicitbrowserunlock,not an OLE-modalprompt or copiedSupaZip respawn/Test->Open intentloss.
`ArchiveWorkerRunner` sourceacquire/nativeidentity precedes originalworker execute; worker_browser::bind
must compare expectedsource during authorizationTEST as during extraction. Capture last supplied secret
ONLY for this pendingauthorization; protectedmemory at rest, original privatezeroedbuffers on use.
Successfulactualcleanup authorizes ephemeral perpipe sourcebound opaquehandle,not a cache/diskpassword.
Refresh/close/sourceinvalid/connectionloss/expiry revoke; boundedoriginalrouter controls neverreplay across
connections. Originalmetadataread/entryopen doesNOTsilentlyauthorizefutureOLE. Unlockhas onefulltest
read/decodecost,zero selectedoutputs; afterdialog usermakes freshheldgesture. NoGetDataUI,original
file-index->EntrySession/worker/Job/source/output/cache/nativeIStream continues withoutpasswordretry.
Requestedlayout unchangedexcept originalToolbar.content officialButtonoutlineMedium32DIP unlockaction;
sameDialog/Input/selection/focus/nativewindowowner; no secondpasswordsurface or frameworkpatch.

## Tree/root source continuation (2026-10-27)

Exact current Tree publicrenderItem/ListItem child down plus BaseTree click-onlyselection was
rechecked; same liveheld4DIP callback/freezer/connection-locked IPC/capture/STA/content/cache path.
Icon/labeldown arms Tree metadata identity without navigating rightselection, disclosure never
arms, crossingthreshold suppresses normalTreeclick. Root freezes complete immutable entries with
one safe compoundstemwrapper; descriptorname gains wrapper, original entryordinal/worker/cache
path does not. Rootwrapper counts toward128descriptors, original32innerroots/32files/512MiB remain.
Empty/unsafe/overbudget rejects whole, no sanitization/truncation. Rust/C# device/control/UTF8name
checks agree without wireversion/field changes. Original NativeSession/Content/Cache untouched.
Seven actualGPUI Treegesture and threemodel tests plus protocol/IPC and matchingworker native
HGLOBAL/IStream/path fixtures are recorded in
[Tree outcomes](../../.trellis/tasks/10-16-archive-manager/tree-root-drag-outcomes.md).
Main verified all7gesturetests and fullrelatedarchive205; exactC#LIST5/RUN5/PASS5/SKIP0 with
freshisolated-codecsworker. Not realpressedHWND/capture/OLEforeignTarget/Explorer/DPI acceptance.
Hosttabs suspend originalcandidate/marquee/pendingdrag only; source-invalidating close/refresh
still revokes originalpasswordgrant. No alternate drag/selection/configuration owner.

## Native asynchronous demand delta — 2026-10-27 (before implementation)

Re-read actual SDK10.0.26100.0 `um/ShlDisp.h:8482-8501,8527-8552` and
`um/shldisp.idl:1043-1100`: IAsyncOperation was renamed IDataObjectAsyncCapability without
ABI/IID change (`3D8B0590-F691-11d2-8EA9-006097DF5BD4`); IUnknown then SetAsyncMode(BOOL),
GetAsyncMode(BOOL*), StartOperation(IBindCtx* reservedNULL), InOperation(BOOL*),
EndOperation(HRESULT,IBindCtx* reservedNULL,DWORD). BOOL is32bit, not VARIANT_BOOL despite the
older prose saying VARIANT_TRUE. The IDL paocb comments are historical: released SetAsyncMode
has no callback argument. Microsoft `shell/datascenarios`, section 'Dragging and Dropping Shell
Objects Asynchronously' (saved reference lines1036-1073) explicitly requires source enable,
target Start BEFORE Drop returns, background extraction, End after extraction; source must not
retire on DoDragDrop return while InOperation. Targets may decline and keep synchronous fallback.

Behavioral RED at original native consumer→IDataObject slot3→ArchiveVirtualContent.Open→original
EntrySession/worker/output checkpoint:4MiB member,600ms controlled writer hold, target UI phase
still blocked, async negotiation false; total Drop phase1823ms. `drag-native-red-run.log` in
round scratch records RUN1/PASS0/FAIL1, not a compile/interface-presence gate. Legacy
`D:\Code\NxTools\Services\WindowsFileDragService.cs:29-61` and ABDM
`DownloadItemListDataFlavor.kt:14-39` rechecked: real-file lists, no virtual async substitute.

Required surviving invariants: original held4DIP/source HWND/capture/pressed admission, one native
STA/one COPY OLE loop, original immutable ordinals/password preauthorization/worker/Job/source/cache,
no extraction before FileContents. Keep original32roots/128descriptors/32files/256requests/512MiB/
120second cancellation. After loop return keep dataobject COM reference, content, cancellation,
leases and STA COM/window pumping until EndOperation AND original cleanup; uncooperative target
keeps the sole slot, never timeout replacement threads. Cancellation/End/retirement reject new
requests but never close transferred native IStreams. Input-queue attachment ends with the OLE
loop, not background extraction; source HWND validity remains checked without foreground changes.

Progress is original session callback, demand-only, bounded100ms coalesced; bytes are only actual
sealed output (not source/archive length), file totals exclude directories, any unknown size means
unknown aggregate. Materializing/content_ready are cache facts; target_copying is awaiting receiver
completion, not measured target disk IO. Finished receiver_reported_complete requires successful
EndOperation COPY; synchronous fallback reports ole_only/no_target_completion, never cache=output
or a discovered target path. Diagnostics carry IDs/count/index/native/elapsed, never member names.
Isolated native consumers and apartment marshalling remain distinct from actual Explorer acceptance.

## Release38 selection admission and collision boundary (2026-10-30)

Re-read legacy DownloadCompletedWindow.PointerPressed -> WindowsFileDragService.TryDragFile,
ABDM HomeComponent stable-ID option selection, current Browser ListItem/Tree held4DIP ->
model_drag.freeze_drag_roots -> connection-locked IPC -> ArchiveVirtualDragSession oneSTA ->
ArchiveVirtualFileDataObject indexed demand -> ArchiveVirtualContent -> EntrySession ->
worker_browser.selected -> worker_job.plan -> ExtractionPlan -> ArchiveEngine.extract ->
ordinal-addressed ZIP local-record streaming -> owned cache/nativeIStream. Directories are
descriptor-only; nullable encryption/size is metadata, not output authority. Original aggregate
actual-byte accounting, source lease, async target/cancel/stream retirement remain owners.

The freezer admits missing optional encryption/size metadata and resolves representable descriptor
collisions without changing original file ordinals. Original directory keys are case-folded so a
renamed ancestor remains shared. Single-member browser extraction has a private fresh lease:
its output-name namespace is not the whole archive's namespace. The narrow plan still validates
the selected original path/kind, source and
original record extent/CRC; general extraction/backend collision gates stay intact. See
[bounded repair contract](../../.trellis/tasks/10-16-archive-manager/release38-drag-preview-repair.md).

Locked `sevenz-rust2-0.23.0` reader.rs::ArchiveReader::for_each_entries traverses blocks first,
then original file indices whose stream_map.file_block_index is None. BlockDecoder::for_each_entries
uses block_first_file_index .. start+num_unpack_sub_streams, including empty entries within that
range. worker_codecs metadata ordinals are archive.files indices, not names or callback counts.
Selected-member content must associate callbacks through that exact traversal; identical names
cannot be HashMap keys. Keep complete decode/CRC, first failure, source recheck and staged commit.
The native two-Copy-block fixture includes identical names and interleaved zero-stream files;
descriptor lindex order deliberately differs from both original index and decoder order.

## Ordinary directory drag admission correction (2026-10-30)
User rejects product-made32roots/128descriptors/32files/256requests/512MiB drag caps. Rechecked
SDK10.0.26100.0 ShlObj_core.h1833–1900/objidl.h12246–12253/ShlDisp.h8482–8501: FILEGROUPDESCRIPTORW
stores count, FILEDESCRIPTORW is592bytes with260WCHAR name and64bit size; FileContents lindex
addresses the full array and repeated independent IStreams are valid. Those business caps are not
native OLE requirements. Legacy pressed DownloadCompletedWindow -> WindowsFileDragService still
enters one OLE loop; it supplies no rationale for virtual-member caps.
The production freezer/protocol/native descriptor/parser and original content/cache pipeline must
admit a normal complete subtree, not cap logical members by active worker or old cache slot counts.
One frozen source context belongs to one command; ordinary per-member descriptors must not repeat
archive comments/source bindings until a valid290-entry ZIP exceeds the existing transport frame.
Preserve immutable original ordinals, all selected roots, directories and64bit metadata, original
held gesture/connection/source HWND, one OLE async demand loop, source/CRC/output integrity and
transferred stream lifetime. Existing admitted metadata/transport/native representability and
active-worker constraints are separate from logical file count; no silent omission or fake target.
Expanded real-boundary regressions cover33/256files,257descriptors,64roots,known64bit/unknown sizes,
4KiBsource comment sharing, full progress counts and34 distinct demanded leases plus repeat cursors.
Collision numbering reserves the existing component-byte and full FILEDESCRIPTORW UTF-16 budgets
before appending its suffix. A valid colliding presentation may shorten only its stem at character
boundaries; original member name/path/ordinal and content are unchanged. Regression covers255-byte
ASCII, multibyte stems and a nested path near259UTF-16 units; necessary native constraints remain.

## Retained materialization owner chain

`ArchiveVirtualDragWire.Parse` now freezes `ArchiveVirtualPlan` once; DataObject/Content consume the same admitted descriptor-index→original-ordinal map while each COM medium still owns fresh memory. `ArchiveVirtualDragSession` retains the original top-level source activity, dedicated STA/OLE and async receiver. `ArchiveVirtualContent` deduplicates indices and charges actual sealed bytes; `ArchiveEntrySession.VirtualMaterialization` starts one original runner on first uncached demand. `ArchiveWorkerRunner.Virtual` binds each `member_acquire` to that pending member's original native output/cache root. Genuine worker source acquisition and local/global Run reservations remain independent of the plan/activity hold.

`worker_drag` retains source/catalog, ZIP structural preflight, the original split-ZIP adapter's normalized anonymous source and compound TAR's first fully verified anonymous spool. External volume-set revision remains the original identity. `worker_codecs/sevenz_drag` uses locked0.23.0 public `BlockDecoder::entry_count`/consuming `for_each_entries` for a complete demanded block before commit; later demands require counted block invocations. `worker_codecs/rar::retained` keeps ordered native TEST state, processes predecessors without output and reopens/replays on backward demands. Full required integrity/native writer close precedes private `drag_member_ready`; EntrySession then Seal-checks actual bytes and `ConfirmVirtualMember` atomically rejects failed/expired Run readiness under the original timeout gate. Member-ready retains the Run; actual process/pipes/Job/source drain alone authorizes session cleanup. Original StartOperation/EndOperation and target-owned IStreams survive OLE/source retirement as appropriate. Original120s drag lifetime,30s member work/startup/Seal and receiver idle remain distinct; normal end is bounded.

Desktop `entry.rs::decode_preview` now transfers its exact original lease to `BrowserSlot.preview_lease` for current visible Ready; `cancel_preview`, refresh/codepage/tab/close/disconnect retire it, while stale/failed/owner-lost decode completion releases it once. `ArchiveEntryCache.Consumers` supplies a separate operation-local drag share for exact source/session/generation/snapshot/codepage/depth/MOTW/ordinal. Release/Dispose/Sweep all honor that share; every native cursor still validates sealed object/content. Encrypted preview never supplies independent source-bound drag authorization. Executable contracts and current automated/owned-native evidence: [retained runtime](../../.trellis/spec/nxtools-archive/backend/product-read.md#scenario-drag-scoped-retained-materialization), [OPT-04 record](../../.trellis/tasks/11-02-archive-workflow-optimization/opt04-retained-materialization.md).

## Release 45 presentation and Root destination boundary — 2026-11-04

Verified Desktop presentation chain: `archive/drag_progress.rs:86-96` sets the native title and constructs `JobProgressView`; `archive/progress.rs:194-205` initializes Test kind, `apply_drag:462-472` leaves that kind unchanged, and the visible custom titlebar at `1412` selects the Test label. Dual current/overall Progress at `1047` explicitly excludes drag; the fallback at `1092` is single-progress. The drag body at `1100` supplies the reported implementation-boundary prose.

Original progress ingress exists below the missing presentation seam: `worker_drag.rs:352` configures per-member extraction progress; `PrivateOutput::write_file:712` publishes decoded current bytes. `ArchiveEntrySession.Virtual.cs::OnVirtualRunner:97` consumes original worker frames without forwarding those snapshots to drag presentation. `ArchiveVirtualContent.cs:68-83` and `ArchiveVirtualDragSession.cs:93-96` publish start/sealed-ready milestones; protocol `ArchiveVirtualDragProgress:341` has no current-member byte pair. Any repair keeps the original demand identity, 100ms coalescing, deferred entity updates and cache-versus-receiver completion distinction.

Root placement chain rechecked through the native boundary: `model_drag.rs:408-439` prefixes every descriptor and inserts the wrapper before IPC; `ArchiveVirtualPlan.cs:18` retains names; `ArchiveVirtualFileDataObject.cs:98-108` writes the prefixed names into FILEDESCRIPTORW. FileContents maps index to original ordinal and private cache, not destination paths. The controlled user-performed Directory Opus13 run recorded wrapper-first creation, 208 files, 255 Created events, zero outside-wrapper/rename/delete/error events, and successful actual cleanup. The user-visible scatter/collect transition was reported during that same run; it is not physical collect/move evidence.

The ZIP Deflated reader at `formats/zip.rs:881-919` returns output from an 8KiB encoded buffer. `PrivateOutput::write_file:712-723` forwards each produced chunk; `call_payload:559-583` sends then waits for the original acknowledgement. The same live run used one worker/catalog and 16,980 binary output records for 206,572,163 payload bytes. Small-write coalescing is a bounded experiment within the current transport/reservations, not authority to weaken checks or add a competing destination-handoff protocol. [Current diagnosis, stage measurements and repair boundary](../../.trellis/tasks/11-02-archive-workflow-optimization/release45-drag-diagnosis.md).

## Release 45 repair contracts — 2026-11-04

The preceding diagnosis records the Release45 baseline. Current `worker.rs` installs `ProgressWriter::with_drag`; `worker_drag::progress_demand` exposes the initialized request/index/original ordinal and `worker_job` attaches it to existing bounded progress frames. `ArchiveEntrySession.Virtual::OnVirtualRunner` filters live demand identity and monotonic current bytes, forwards through `ArchiveVirtualContent` and `ArchiveVirtualDragSession`, and publishes ordered current/processed snapshots. Desktop `browser_drag` freezes the descriptor/request map; `drag_progress` rejects mismatched/stale snapshots and defers the original `JobProgressView` update. `new_drag` and the native title share `拖出解压`; original Test identity is unchanged. Current/overall official Progress remain distinct, unknown totals stay unknown, and sealed cache does not imply receiver completion.

Root component admission uses255 UTF-16 units in both Rust and C#, with full FILEDESCRIPTORW relative path<260 units including room for NUL. UTF-8 remains the whole-envelope transport unit, not a255-byte component veto. The real Rust serialized86-CJK wrapper passes native admission and HGLOBAL descriptor checks without shortening or ordinal changes. Original collision suffix policy remains separate.

`worker_output::PrivateOutput` stages extraction-only32KiB writes in one reusable buffer within existing reservations. Non-write calls drain before metadata/source_precommit/commit; abort/Drop discard; failed flush remains sticky. Direct Create/Edit writes retain their path. Aggregate ZIP/IPC/native/cache timings preserve overlapping interval definitions and all integrity work. Same-source ordinary Extract and controlled Root measurements validate208 files/206572163 bytes independently; [performance record](../../.trellis/tasks/11-02-archive-workflow-optimization/release45-performance-repair.md).

Original DataObject first-descriptor/async Start/End and Session OLE-return events now share the drag correlation and elapsed clock, alongside demand/ready/cleanup. They enable an authorized visible-transition trace without exposing payloads or introducing destination moves. DOpus visible scatter/collect remains unresolved; the historical wrapper-first/no-move topology result stands. [Repair and verification](../../.trellis/tasks/11-02-archive-workflow-optimization/release45-repair.md).

### Multi-output failure cleanup boundary

`worker_codecs.rs::run_sevenz` stages each selected file token in `pending` before decode, keeps first callback/CRC/size failure, and aborts remaining tokens in creation order after full decode fails. Thus the first abort can name an older output while `PrivateOutput` buffers the current member. `worker_output.rs::call_payload` must discard buffered extraction bytes for every abort, without flushing a different token or clearing the first write failure. Successful begin/metadata/source_precommit/commit remain drain barriers. `ArchiveOutputSession.Dispose` retains the native fallback for all unpublished entries when cancellation or transport failure prevents explicit abort. This is Next's original failed-output contract, not Bandizip's buffered close-and-drain lifecycle. [R45-AC01](../../.trellis/tasks/11-02-archive-workflow-optimization/release45-ac01-repair.md).

## Cache retirement durability boundary

`ArchiveEntrySession.Virtual::CloseVirtual` waits for the original worker terminal before releasing each original cache lease. `ArchiveEntryCache.Lease.Release` durably retires the lease, verifies the original output identity/metadata/MOTW/content and deletes only that object. `RetireEmpty` moves its manifest to the fixed parent before deleting verified empty directories and finally the manifest. The durable retired record and original output stamp support recovery when deletion precedes a crash; `OutputRemoved` need not receive an additional durable write immediately before manifest deletion. A fixed-parent retired record with that flag still false must reopen through the same identity checks, verify or observe the original output absent, and retire even when the original root is already gone. Replaced roots/ancestors, edited output, live consumer holds, malformed records and unknown directory contents remain preserved. `ArchiveCacheRetirementTests.AncestryAndMalformedRecovery` exercises obstructed retirement, its persisted state and restart on both existing and removed roots; original cursor/tamper/stream lifetime fixtures remain binding.

## Root drop destination location — 2026-11-05

Bandizip 7.32 reference (`FUN_140147200`, `FUN_1401aedd0`, `FUN_140145ac1`): with Fast DnD enabled, a Tree-root drag, any folder selection or more than 300 MiB sets `rm_bIsFastDragDropMode`. Nothing is extracted during the gesture. After `DRAGDROP_S_DROP` the EXE reads the drop folder from the named mapping `BandizipDragDropExtMapping` (guarded by `BandizipDragDropExtMutex`), written by `bdzshl.x64.dll`'s `Directory`/`Drive` `DragDropHandlers\AABdzCtx` handler, and calls ordinary selected extraction `FUN_1401481d0` into it. Receivers that never invoke that handler produce `TEXT_ERR_AT_DRAGANDDROP_OFF_FAST_DND`.

Next reaches the same product behavior without a Shell extension or registration. Root drags send `action: "locate"` carrying exactly one descriptor-only wrapper named by the archive stem (`model_drag::freeze_drag_locate`); no member is frozen, materialized or unlocked. `ArchiveVirtualDragSession.Locate` keeps the original HWND/peer validation, input-queue attachment, capture release, pressed-button check, one STA and one COPY `DoDragDrop`, and the single native drag slot and exact cancel. The data object is the existing real-file `CF_HDROP` + `Preferred DropEffect` (`NativeFileDragService.CreateFileDropDataObject`) for an owned staging folder `%TEMP%\NxTools\drop-<operation>\<stem>\` holding one hidden zero-byte marker `.nxtools-drop-<operation>`.

`ArchiveDropLocator` watches fixed/removable volume roots for that marker name before the loop starts. The receiver copies the folder like any other folder (Explorer, Directory Opus, desktop); the folder that now holds the copied marker is the destination, including receiver-chosen collision renames. After one second it also probes the desktops and every open Explorer window folder for `<stem>*\<marker>`. A dropped effect waits up to 60 s for the copy (receiver confirmations can hold it); a drop without an effect waits 3 s. The copied marker is removed before the result is returned; the staging folder is always removed. Results: `located` with `destination`, `cancelled`, or `drop_location_unknown`/`drop_marker_retained`/`virtual_target_not_accepted`.

Desktop `drag_result` returns `LocatedDrop` only for the current generation/epoch; the application starts `extract_dropped` on the next turn: an ordinary whole-archive `Extract` job with strategy `Here` into the receiver-created folder, the user's conflict/MOTW settings and the browser code page. Encryption, size limits, prompts, progress, cancellation and cleanup are the ordinary job's. Member and sub-folder drags keep the virtual `FileGroupDescriptorW`/`FileContents` route above.

Diagnostics: `archive.virtual_drag.{accepted,locate_started,ole_return,located,terminal}` with the drag correlation and elapsed time, and `archive.drop_extract.submitted`; no paths or names are logged.

Regressions: protocol `locate_admits_exactly_one_unauthorized_wrapper_directory`; C# `archive Root drop locate wire shape`, `receiver copy` (created, receiver-renamed, probe fallback) and `uncopied cancelled busy`; Desktop `production_tree_root_drag_locates_receiver_folder_named_by_archive_stem`, `production_tree_root_drag_rejects_empty_or_unsafe_and_locates_any_size_or_encryption`, `icon_layout_tree_icon_and_text_still_enter_original_held_native_drag`, and `root_drop_extracts_whole_archive_into_receiver_created_folder`. Controlled receivers are not physical Explorer/Directory Opus acceptance.

### Superseded — Tree Root uses the virtual descriptor route (2026-11-12)

The two-stage Root locate route produced a different result from dragging the same folder out of the list: the receiver first copied an empty archive-named folder, then a separate ordinary `Here` extraction filled it after the gesture ended. Its Desktop entry is removed. `browser_drag::drag_command` now freezes every Tree path, including Root, through `model_drag::freeze_drag_tree` and sends `action: "start"`; Root yields the archive-stem wrapper descriptor first and every member beneath it, exactly the list's `FileGroupDescriptorW`/`FileContents` route. Encrypted Root follows the list contract (unlock, then a fresh gesture). Desktop regressions: `production_tree_root_drag_uses_virtual_wrapper_named_by_archive_stem`, `production_tree_root_drag_rejects_empty_unsafe_or_locked_and_wraps_any_size`, and `icon_layout_tree_icon_and_text_still_enter_original_held_native_drag`. The native `locate` action, `ArchiveDropLocator`, `LocatedDrop` handling and `extract_dropped` have no Desktop sender after this change.

## Browser name-only row gesture boundary

`archive/browser.rs::render_row` makes only the name cell a ListItem when `ArchiveSettings.full_row_select` is false; metadata cells remain outside that handler. The original `browser_marquee.rs::marquee_down` owns those non-name cells as rectangle-selection background and retires the native drag candidate. Preserve that distinction: name gestures retain the original `RowId`/snapshot candidate, while enabling full-row selection expands the ListItem hit region. Non-name gestures in name-only mode are not evidence of unsupported member extraction.

The native chain remains `browser_drag.rs::drag_move` -> `drag_command` -> the existing connected bridge -> `ArchiveVirtualDragSession.cs::Run`, with the original source HWND, one STA/OLE loop and cancellation. Legacy `D:\Code\NxTools\Views\DownloadCompletedWindow.axaml.cs::DragFileSurface_PointerPressed` -> `Services\WindowsFileDragService.cs::TryDragFile` provides the held-pointer/native-loop precedent; old NxTools has no general archive browser. Failures beginning on an eligible name require the source/member, receiver and corresponding operation result to distinguish admission, content extraction and target acceptance.

## Unified destination-located extraction

The approved drag behavior uses the Root destination-location mechanism for every source. `browser_drag::drag_move` freezes an operation-local source/selection before sending one `locate` directory; `ArchiveVirtualDragSession::Dispatch/Locate/Prepare` retains HWND/peer validation, capture release, pressed-button admission, one STA/COPY loop, exact cancellation and marker disposal. The receiver-created folder is the ordinary extraction destination, not a virtual-file cache. `ArchiveApplication` applies only the current receipt and defers `jobs::extract_dropped` to a later application turn.

Root uses the archive stem and ordinary whole-archive extraction. A single directory uses its basename and removes that complete selected directory prefix from ordinary output paths; a file or multiple selection uses an archive-stem wrapper and removes only the current list folder prefix. Original snapshot-bound ordinals/source paths still own decoding and integrity. Password prompts, conflicts, progress and cancellation belong to the ordinary job after location, not OLE demand or its 120-second lifetime. No pre-extraction, virtual content requests or member-sized descriptor list is sent during the gesture.

Legacy `D:\Code\NxTools\Views\DownloadCompletedWindow.axaml.cs::DragFileSurface_PointerPressed/StartFileDragAsync` -> `Services\WindowsFileDragService.cs::TryDragFile` supplies held-pointer CF_HDROP and one native OLE loop, without a general archive-browser implementation. Native Next `ArchiveVirtualDragSession::Locate` disposes its input-queue lease, removes the marker and disposes the locator before terminal completion. The ordinary job retains the drag operation ID; Desktop defers the handoff through the original application weak owner. Existing Bandizip selected-extraction reference above remains the behavioral precedent. Exact scope and component/lifecycle invariants: [ordinary drop extraction](../../.trellis/tasks/10-16-archive-manager/drop-selection-repair.md).
