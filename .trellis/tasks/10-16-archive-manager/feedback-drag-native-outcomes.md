# Feedback item 2 — bounded native archive drag responsiveness/progress

Scope: C# original virtual drag/session/content/protocol/pipe and isolated tests only. Rust JobProgressView/protocol integration belongs to the parent lane. No checkpoint edits, Cargo, package/install/product launch, Explorer automation, Registry/ACL/service changes, dependencies/vendor edits or commits. Installed Release32 does **not** contain this source change. Read-only process query found no running NxTools.exe; user runtime identity therefore was not observable, not presumed current.

## Root cause / genuine behavioral RED before production change

`ArchiveVirtualFileDataObject.GetData` calls `ArchiveVirtualContent.Open`; Open waits synchronously for the original EntrySession→worker→native output→sealed cache result. A target without async negotiation must do that work inside its Drop/UI phase. Moving extraction to Task.Run already existed and did not unblock this target phase.

Added `ArchiveVirtualAsyncTests.ActualSlowConsumerReturnsUiBeforeContent` **before** production changes. It uses actual native IDataObject QueryInterface and slot3 GetData, original4MiB ZIP/EntrySession/worker/Job/cache, with only the original native output checkpoint held for600ms. Baseline builds successfully and executes the synchronous fallback because QI lacks the async IID. RED: target UI phase still pending at600ms, async_negotiated=False, Drop phase1823ms; RUN1/PASS0/FAIL1. This is neither a compile failure nor merely an interface-presence assertion.

Final GREEN on same boundary: target UI phase returned before the600ms hold released, async_negotiated=True, Drop phase3ms; content still materializes through the original owners. Consumer uses standard CoMarshalInterThreadInterfaceInStream/CoGetInterfaceAndReleaseStream for background IDataObject acquisition and calls native EndOperation. This is a controlled consumer, **not proof that actual Explorer never stalls**.

## SDK / references checked before implementation

- Installed SDK10.0.26100.0 `um/ShlDisp.h:8482-8501,8527-8552`; `um/shldisp.idl:1043-1100`.
- IID `3D8B0590-F691-11d2-8EA9-006097DF5BD4`, binary-identical IAsyncOperation/IDataObjectAsyncCapability. IUnknown then SetAsyncMode(BOOL), GetAsyncMode(BOOL*), StartOperation(IBindCtx*), InOperation(BOOL*), EndOperation(HRESULT,IBindCtx*,DWORD). Actual BOOL is32bit despite historical VARIANT_TRUE prose. Reserved pointers NULL. Historical paocb comments do not add a nonexistent released callback parameter.
- Microsoft Shell datascenarios saved HTML lines1036-1073: target Start before Drop returns, background extraction, End after extraction; source retains async cleanup ownership after DoDragDrop returns.
- Legacy `D:\Code\NxTools\Services\WindowsFileDragService.cs:29-61` and ABDM `DownloadItemListDataFlavor.kt:14-39`: real-file lists, not alternate virtual-content extraction owners.
- Original cache `ArchiveEntryCache.cs:222-267`: identity/hash/size-bound native read-only stream with its own handle; Release may retain busy exact owned output for original sweep. Cache was not modified.
- `docs/reference-code/archive-virtual-drag.md:148-183` records verified SDK delta/invariants before the production patch. Historical synchronous-only notes are retained, superseded by this explicit delta.

## Implemented lifecycle / ownership

- `ArchiveVirtualFileDataObject.cs:9-43`: real public ComVisible SDK interface (public visibility is required for CCW QI); complete five methods, BOOL32/HRESULT vtable, negotiated mode, exactly one Start/End, invalid reserved/duplicate/late transitions rejected. End records receiver HRESULT/effect and rejects new content calls; cancellation also rejects new requests.
- `ArchiveVirtualDragSession.cs:69-144`: same admission/source HWND/capture/pressed/dedicated STA/one COPY OLE loop. Explicit source COM reference retained through **loop end AND EndOperation AND original cleanup**. Input-queue attachment ends immediately with the original loop. Post-loop CoWaitForMultipleHandles dispatches COM/window messages on the original apartment, not a second OLE loop.
- Cancellation/Escape/source disappearance/disconnect/120s deadline cancel original materialization; they do **not** impersonate receiver End. An uncooperative receiver retains the sole native slot/STA, even after cancellation; no timeout replacement or second worker/loop. Dispose may consequently wait for that receiver: truthful ownership, not an interrupt guarantee.
- `ArchiveVirtualContent.cs:30-94`: original demand/serialized per-index materialization, original authorization/source/worker/output/cache and shrinking actual-output allowance remain. Repeats reuse one materialization, with independent identity-checked native stream cursors. Returned native streams remain readable across cancel, End and retirement.
- Original32roots/128descriptors/32files/256requests/512MiB/256KiB command/120s cancellation remain. Directories are descriptor-only; no pre-extraction/password prompt/GetData retry. Original entry budgets remain64MiB codec workspace +128MiB metadata +64MiB overhead =256MiB process reservation at ArchiveWorkerBudget.Approve; original runner2GiB aggregate unchanged. No measured peak-memory claim.
- A full original EntrySession.Stop before deferred lease release can reject release; the disconnect fixture explicitly expects `virtual_cleanup_incomplete`, cleanup_complete=false and retained slot rather than invented cleanup success. Original bounded cache recovery owns retained output; no force-delete/parallel recovery added.

## Exact progress DTO / parent integration contract

`Protocol/ArchiveVirtualDragContract.cs:14-20` adds:

```csharp
record ArchiveVirtualDragProgress(uint Version, Guid OperationId, Guid SessionId,
    ulong Generation, string Phase, ulong BytesDone, ulong? BytesTotal,
    ulong FilesDone, ulong FilesTotal, int? CurrentIndex = null,
    string? CurrentMember = null, bool? CleanupComplete = null,
    string? Code = null, int? NativeErrorCode = null);
```

Envelope type `platform_event`, payload event `archive_virtual_drag_progress`, data=this, correlation=OperationId. Original session `Action<ProtocolEnvelope>? Progress`; no new task manager/queue/extractor. No EntrySession production changes were needed: reliable sealed data supplies truthful coarse byte progress.

Phases:
- **materializing** at actual original content-materialization demand (not gesture/descriptor/acceptance).
- **content_ready** records actual sealed cache bytes once per distinct descriptor; this milestone may coalesce into a later snapshot.
- **target_copying** after original content stream becomes ready for receiver; waiting for target consumption/completion, **not measured target IO**.
- **finished** only after native owner conditions and actual original cleanup. Code `receiver_reported_complete` for successful EndOperation COPY; synchronous fallback code `ole_only` explicitly has no target completion confirmation.
- **cancelled / failed** after actual cleanup/retained-cleanup decision; fixed code/native HRESULT preserved. Cleanup failure always failed/cleanup_complete=false.

BytesDone is actual sealed output, never source/archive length or invented ticks; FilesDone is distinct materialized files. FilesTotal counts actual non-directory descriptors. BytesTotal is null if any file estimate unknown. CurrentIndex is full frozen descriptor index, CurrentMember UI-only and never logged. No target path is supplied or inferred; cache is not the receiver's output location.

`ArchiveVirtualDragSession.cs:146-172`: first demand immediate; subsequent latest snapshot coalesced to100ms, one one-shot per-run timer/no idle timer; terminal flush/callback immediate. State transitions may replace intermediate snapshots. `PlatformPipeClient.cs:199-211`: original outgoing bounded256 TryWrite for expendable running snapshots; bounded2 terminal forwarder awaits outgoing transport asynchronously. A full terminal staging channel cancels transport, rather than silently losing success or blocking OLE. No per-byte IPC/logging or modal/focus actions. Parent must route this generation-scoped event into existing nonactivating JobProgressView presentation.

## Tests / exact final accounting

Four new reviewed Fixture registrations at `csharp/NxTools.PlatformHost.Tests/Program.cs:573-580`:
1. actual slow native consumer UI (real RED→GREEN,4MiB, standard COM marshal);
2. native ABI/state/references (QI/AddRef/Release, all five vtable slots, BOOL32/reserved/default/duplicate/late/cancel, native stream after End);
3. receiver lease cancel/abort/source-close/disconnect (loop returned but lease/slot held, later serial dispatcher command responsive, new gets reject, held stream survives, native HRESULT/cleanup truth);
4. actual multi/directory/unknown progress and both End-before-loop / loop-before-End orderings, no eager output, repeated request no second decode, exact6 sealed bytes/2files, privacy and bounded snapshots.

Compatibility selection includes original16 virtual drag/tree/password fixtures: changed source, slow materialization cancellation/Escape/source-close/disconnect, directory/unknown/budget/request bounds, password TEST/preauthorization/revoke/expiry and already-returned native handles. Existing tests were not weakened or disabled.

Final commands (serial dotnet lane):

```powershell
dotnet build csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-restore -m:1 -nr:false -p:UseSharedCompilation=false
dotnet run --project csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-build --no-restore -- --fixtures --list 'archive Phase F virtual'
dotnet run --project csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-build --no-restore -- --fixtures 'archive Phase F virtual'
```

**LIST20 = RUN20 = PASS20 / SKIP0 / FAIL0 / not_run0**, build0warnings/0errors. Final integrated run followed bounded diagnosis: first implementation kept interface internal, actual QI test still correctly RED; public SDK interface fixes CCW exposure. Lifecycle fixture initially incorrectly assumed stopped EntrySession could acknowledge release; corrected to require explicit retained-cleanup failure, not production fake success.

Validation: scoped `git diff --check` PASS; `python tests/architecture/spec-contracts.py` PASS114documents/14owners; `tests/architecture/platformhost-test-selection.ps1` PASS (source-only safety). Cargo/Rust/UI gates not run in this lane by explicit instruction. No actual external Explorer Drop, pressed product HWND, mixed-DPI/user visual/foreground/native acceptance or full named-pipe saturation run is claimed.

## Evidence / worker provenance / lane release

All logs/backups under:
`C:\Users\Administrator\.pi-desktop\scratch\9d8169e5-656b-47ae-9113-af5789f5091c`

- `drag-native-round-start/`: round-start copies/hashes/git-status for changed existing production/reference/registration files.
- `drag-native-red-{build,list,run}.log`: successful baseline build and genuine RUN1/FAIL1.
- `drag-native-green-second-run.log`: first actual native QI/background consumer GREEN.
- `drag-native-lifecycle-{list,run}.log`, `drag-native-diagnose-run.log`, `drag-native-lifecycle-fixed-run.log`: exact intermediate bounded failures/diagnosis.
- `drag-native-final-{build,list,run}.log`, `drag-native-accounting.json`: authoritative final20/20.
- `drag-native-worker-provenance.json`: reused `target/debug/nxtools-archive-worker.exe`,10994688bytes, UTC2026-10-27 09:39:29; SHA256 `04C6863DBA4B46ACEF8E736DAFAC904A12E2155044F1F788A6B11D25E091BFE0`, identical before/after. Existing Cargo fingerprint has isolated-codecs and codegen-units256, archive sources have no later timestamps; real ZIP/RAR/Zstd/password compatibility passes. This is reused matching private-worker evidence, **not a fresh build or Release-approved worker**; parent's concurrent Rust protocol edits are not compiled into it.
- NXTOOLS_ARCHIVE_WORKER_FIXTURE points to that exact binary; ZIP fixtures from `rust/crates/nxtools-archive/tests/fixtures`, codec fixtures from its `upstream` child.
- `drag-native-preservation.json`: original EntrySession.cs/Password.cs unchanged. No Rust/ArchiveJobSession/ArchiveJobContract/ArchiveOutputSession/worker/cache edits.
- `drag-native-final-processes.json`: no task-owned dotnet/test/worker remains. Only unrelated Unity dotnet processes were observed and left untouched. Dotnet lane is released to parent.

## Parent convergence (2026-10-28)

Parent connected original native event to one Jobs UI-only slot and shared JobProgressView, with hidden/focus:false then success-ACKed NOACTIVATE/finalbounds/show; no extractor/queue/OLE owner added. Actual original held-gesture app test verifies demand creates that presentation and closing does not cancel drag. Parent fresh-built isolated-codecs worker SHA256 `EED2F89901670AA9BE4EFA4D03BE5BC65BE37885CFA854B565D0CB4991666770` and current C# rerun selected39=RUN39=PASS39/skip0/fail0, including original20drag + result-location/Shell/HZ1 fixtures. New logs `feedback-native-all-final-{list,run}.log`; latest target UI phase1ms under600mscontrolled worker gate. This supersedes reused-worker limitation only for this parent run, not Release approval. Overall handoff remainsPARTIAL: [outcomes](feedback-hz2-outcomes.md). ActualExplorer/physical/installed stillNOTRUN.
