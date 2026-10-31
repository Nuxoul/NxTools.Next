# Phase F P208 isolated virtual-file spike contract

User approval2026-10-22 is recorded in implement.md before code. Production archive rows remain
DISCONNECTED until independent spike acceptance. P206/P207 boundaries: phase-f-shell-contract.md.
Current0.8.0/pre0.3.9 and all earlier acceptance debts unchanged; no dependency/vendor/recycle work.

## Source-backed vertical boundary

Rust protocol ArchiveVirtualDragSpike start contains version1/dragID/session/generation/sourceHWND
and1..32frozen ordinary-file descriptors with original ArchiveEntryCommand source/snapshot/ordinal.
Same source set/codepage, unique original IDs/ordinals/names, safe relative Unicode filenames<260UTF16,
known size/aggregate<=512MiB. No directory, unknown-size or password interaction capability in spike;
these remain unsupported/not advertised, not a full-format claim. Separate exact cancel carries no
new materialization authority. Outer protocol1/ordinary commands remain compatible.

Isolated cfg(test) GPUI VirtualDragSpike owns one direct MouseDown→enqueue callback without await/
defer/GPUI on_drag. It is NOT mounted by ArchiveBrowser. Native production-handshake capability
archive_virtual_drag_spike_v1 exposes only explicit internal spike protocol, not a user row action.
C# admission validates sourceHWND against connected Desktop pipe peer, installs one active slot before
next cancel. Dedicated background STA owns OLE init/inputqueue lease/WM_CANCELMODE capture release/
button highbit check/final cancel recheck/ONE DoDragDrop COPY loop. Never serial pipe reader/shared
EventWindow. Source disappearance/Escape/stop/120s limit cancels; slot retained until actual cleanup.

DataObject advertises only FileGroupDescriptorW(HGLOBAL),FileContents(indexedISTREAM),Preferred
DropEffect(COPY). SDK10.0.26100.0 described in archive-virtual-drag.md. Descriptor does not materialize;
GetData target request→Task.Run→original ArchiveEntrySession/worker/WindowsJob/source binding/output
verification/Cache.Seal→safe lease-held native read-only IStream. SourceID/version/hash/size rechecked
under protected parents/exactFILE before stable path/native stream open. Repeated requests reuse
one materialization but get independent native stream cursors; total contentrequests<=256. No
clipboard/CF_HDROP/SendInput/Extract-then-drag or arbitrary output path wire authority.

Each successful GetData medium is transferred+1; consumer ReleaseStgMedium owns HGLOBAL/IStream
release. Target-held native streams remain readable after dataobject/session retirement. New calls
stop after retirement; existing cache Release retains busy owned file/manifest rather than deleting
by path. No new cleanup framework, cache review or force-process termination. No
IDataObjectAsyncCapability advertised; synchronous COM GetData may block ONLY dedicated OLE STA
while original async worker proceeds. Live external target/marshaled-apartment support unaccepted.

Lifecycle diagnostics correlation=dragID;accepted/start/firstcontentrequest+readyperindex/terminal,
count/index/elapsed/nativecode/success/cleanup only. No paths/names/passwords/rawcontent/ticks. Sink
failure cannot change result. Native Drop requires DRAGDROP_S_DROP+COPY, Cancel distinct, no target
acceptance/failure truthful. Sourceclose/disconnect cancels current worker reads and releases own
leases only after original actualcleanup; failedcleanup retains occupiedslot and explicit failure.

## Evidence and exact stop

Controlled consumer validates SDK descriptor layout/count/UTF16 names, COM IDataObject slot3 ABI,
format/index/medium rejects, enumNext/Skip/Reset/Clone, independent/repeated streams and retirement.
Injected capture/button/OLE boundary proves ordering/dedicatedSTA/one-loop/later-command responsiveness;
actualworker tinyZIP proves lazy/indexed/multi/sourcechange and slowmaterialization cancellation/
Escape/sourceclose/disconnect/partialcleanup. Isolated GPUI callback proves pressed callback timing/
frozenIDs; no physicalWindow/Explorer or actual foreign-target DoDragDrop acceptance claim.

An attempted direct Ole32 pre-cancel fixture TIMED OUT (native-abi-run.log); not fakePASS or Windows
unsupported. Production native wrapper now makes final cancellation check before entering Ole32.
Corrected fixture proves COM ABI plus that cancelled admission boundary only, NOT actualDoDragDrop.
Real source capture/physicalbutton/foreign target/fullOle32 lifecycle remain NOT RUN. No window/
launcher/tool switch may bypass self-app denial. Independent spike acceptance is still required;
production hookup/fullP208/PhaseF/firstrelease are not declared complete. No second optional review.
