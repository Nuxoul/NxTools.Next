# Source recycle: bounded Windows API investigation

Phase A foundation, source/API evidence only. No Shell recycle/delete call was executed.
Full collected SDK/Microsoft source report is retained in session scratch
`phase-a-source-deletion-native-capability-report.md`; its intermediate working-tree line numbers
are historical observations, not a stable current implementation index.

## Reference findings

Read archive.md first. Legacy tracked NxTools search for IFileOperation/SHFileOperation/
FOF_ALLOWUNDO/FOFX_RECYCLEONDELETE/VisualBasic RecycleOption found no filesystem recycling owner.
`D:\Code\NxTools\Views\ClipboardHistoryPopupWindow.axaml.cs:1136–1142` raises an in-app
history-item deletion callback, not filesystem recycling. Archiver core/src/lib.rs791–824 and
986–1073 delete rewrite/failed-output artifacts; they are not delete-after-compress workflows.
SupaZip source search likewise found no reusable recycle/source-delete chain. These are bounded
source findings, not exhaustive dependency/runtime claims. Bandizip evidence remains public
resource text only; no proprietary decompilation/disassembly was done.

## Installed SDK / public APIs actually inspected

Windows SDK10.0.26100.0 under `C:\Program Files (x86)\Windows Kits\10\Include\10.0.26100.0\um`:
- ShObjIdl_core.h17126–17236: FOFX_RECYCLEONDELETE0x00080000, EARLYFAILURE0x00100000,
  IFileOperation.DeleteItem(IShellItem*,sink),PerformOperations,GetAnyOperationsAborted.
- ShObjIdl_core.h10714–10722: PostDeleteItem(flags,item,hrDelete,psiNewlyCreated).
- ShObjIdl_core.h9853–9900: **ITransferSource::RecycleItem** is a real public candidate,
  taking source/destination IShellItem and returning recycled destination; it is not HANDLE-bound.
- ShlGuid.h555–557: BHID_Transfer binds folder transfer handler.
- ShObjIdl_core.h26813–26940: IFileSystemBindData2.SetFileID is a64bit parse optimization,
  not full128bit identity/version/hash-conditioned recycle authority.
- shellapi.h249–266: FOF_ALLOWUNDO0x40, FOF_NORECURSION0x1000 (not FOFX_NORECURSION),
  WANTNUKEWARNING is warning UI, not fail-closed prevention.
- sherrors.h100–105: COPYENGINE_E_RECYCLE_FORCE_NUKE/SIZE_TOO_BIG/PATH_TOO_LONG/BIN_NOT_FOUND.

Microsoft public contracts (each inspected in the bounded report):
- https://learn.microsoft.com/en-us/windows/win32/api/shobjidl_core/nn-shobjidl_core-ifileoperation
- https://learn.microsoft.com/en-us/windows/win32/api/shobjidl_core/nf-shobjidl_core-ifileoperation-deleteitem
- https://learn.microsoft.com/en-us/windows/win32/api/shobjidl_core/nf-shobjidl_core-ifileoperation-performoperations
- https://learn.microsoft.com/en-us/windows/win32/api/shobjidl_core/nf-shobjidl_core-ifileoperation-getanyoperationsaborted
- https://learn.microsoft.com/en-us/windows/win32/api/shobjidl_core/nf-shobjidl_core-ifileoperationprogresssink-postdeleteitem
- https://learn.microsoft.com/en-us/windows/win32/api/shobjidl_core/nf-shobjidl_core-ifileoperation-setoperationflags
- https://learn.microsoft.com/en-us/windows/win32/api/shobjidl_core/nf-shobjidl_core-itransfersource-recycleitem
- https://learn.microsoft.com/en-us/windows/win32/api/shobjidl_core/nf-shobjidl_core-shcreateitemfromparsingname
- https://learn.microsoft.com/en-us/windows/win32/api/shobjidl_core/nf-shobjidl_core-ifilesystembinddata2-setfileid
- https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfilew
- https://learn.microsoft.com/en-us/uwp/api/windows.storage.storagedeleteoption?view=winrt-26100

IFileOperation is STA-only. DeleteItem only queues. PerformOperations success is not per-item
success and can accompany cancellation: inspect GetAnyOperationsAborted regardless of HRESULT.
PostDeleteItem.hrDelete is the actual item result; nonnull psiNewlyCreated is recycled destination,
null on successful full deletion. Missing callback/failed COM transport is unknown, not retry.
PreDeleteItem can veto before start; no general Cancel method guarantees interrupting an in-flight
item. Keep bounded STA admission and original source ownership until actual completion, not timeout.

FOFX_RECYCLEONDELETE expressly requests recycling; ALLOWUNDO is only 'if possible'. No inspected
public contract guarantees every provider/location/configuration cannot permanently delete with
that flag. Public ITransferSource.RecycleItem is a stronger candidate for recycle-specific semantics,
but still not an expected fullID/version compare-and-recycle API. WinRT Default explicitly permits
permanent deletion depending on location, so is unsuitable. Post-hoc detecting permanent deletion
cannot meet 'never permanently delete'. No fallback adapter is implemented.

## Identity / sharing blocker

Current C# path: Start -> Task.Run ExecuteAsync -> private source_acquire -> ArchiveSourceLease
or ArchiveCreationSources -> held relative/no-reparse opens -> worker full integrity/source checks
-> native output publication -> actual process/pump drainage -> qualification while source remains
held -> independent recycle projection -> source disposal. No serial reader/shared EventWindow work.
Source files share READ only; parents omit SHARE_DELETE. Current sources compare volume/full128ID,
size/mtime/ChangeTime; single-source qualification additionally captures same-handle basic version.
CreateFileW documents that share modes live until close and delete access includes rename.
Holding these leases blocks Shell's required delete/move access. Closing them permits replacement.

SHCreateItemFromParsingName/RelativeName/WithParent take names/PIDL/IShellItem, not kernel HANDLE
plus expected fullID/version/hash. Precreating a Shell item does not have a documented persistent
file-object binding through later recycle. RecycleItem also uses IShellItem. No inspected installed
SDK/public API exposes handle-bound recycle with the required version precondition. This is NOT a
claim about all undocumented/provider-specific/future APIs. Approved output close->publish TOCTOU
is unrelated authority and is NOT reused for source recycling.

## Minimal no-Shell reproduction

Owned fixture only: acquire production source lease; request DELETE access with CreateFileW but
perform no mutation/disposition, observe sharing32; verify/close lease; rename original to an owned
backup and write replacement at the same name; name-based resolver observes new ID/bytes while
original survives. This reproduces the protection gap, not Shell recycling behavior. Fixture cleanup
is ordinary cleanup of only this GUID-owned tree; no user sources/recycle bin touched.

## Exact pending decisions (none automatically selected)

1. Preserve strict identity and never-permanent contracts: retain sources; return archive success
   plus native_protection_unproven/NotAttempted. **Current fail-closed implementation.**
2. Separately approve source final-recheck->close->Shell-bind TOCTOU; then investigate/test a
   recycle-only adapter (prefer explicit RecycleItem candidate) under separate real native authority.
   Authoritative per-item recycled destination required, no fallback/retry after unknown.
3. Handle-based permanent disposition is not recycling and contradicts approval. Not implemented
   or proposed as a fallback; any product change would require explicit new authorization.

Actual source safety, eligibility/isolation test evidence and current checkpoint remain separate
from real native recycle support, which is unapproved/unverified.
