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
