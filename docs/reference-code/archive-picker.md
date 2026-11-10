# Archive owned picker

## Reference boundary

[Archive reference](archive.md#11-selected-extraction-and-picker-follow-up) and the saved [picker investigation](../../.trellis/tasks/10-31-archive-research/bandizip-performance-picker.md#native-directory-browse) identify Bandizip 7.32.0.1 CArchiveDlg, not an extraction-specific form. Legacy NxTools' indexed tracked-source scan found no general archive owner; its package/FFmpeg ZIP services are not archive-window evidence.

Saved artifacts under `.trellis/tasks/10-31-archive-research/evidence/raw/` were inspected without executing a reference program or collecting new reverse-engineering evidence:

- `bandizip-picker-code-0x1400f4da2.json`: reconstructed constructor `FUN_1400f4d60` zeroes the HWND, initializes the base/control owner through `FUN_14002f1e0`, installs CArchiveDlg vtables and retains options at +0x198. This constructor does not itself establish the native form creation API.
- `bandizip-picker-code-0x1400f3e20.json` and `bandizip-picker-function-0x1400f3e20.json`: dispatcher compares WM_COMMAND control 0x54a and calls `FUN_1400fa3d0` at 0x1400f4834. IDCANCEL calls the imported EndDialog at 0x1400f40fd with result 2; WM_DESTROY destroys its owned child options window. Callee and assembly evidence agree with the pseudocode.
- `bandizip-function-0x1400fa3d0.json`: call at 0x1400fa4db passes the original +8 HWND, current folder and translated TEXT_SELECT_DEST_FOLDER to `FUN_140024f10`. Only a nonempty result updates destination control 0x53a through SetDlgItemTextW at 0x1400fa637.
- `bandizip-function-0x140024f10.json`: BROWSEINFO owner is stored at 0x140024fb9, flags 0x45 at 0x140024fe3; the real synchronous native show boundary is SHBrowseForFolderW at 0x140024ff0. Null PIDL returns empty; success resolves its filesystem path and releases the PIDL. `bandizip-function-0x140024ee0.json` confirms initial selection via SendMessageW(BFFM_SETSELECTIONW=0x467,1,initial) during initialization.
- `bandizip-picker-code-0x1400f520f.json` is the indexed destructor evidence. The saved lifecycle snapshot does not establish the complete CArchiveDlg base-to-DialogBox creation chain. No unrelated DialogBox caller is substituted for that missing evidence.

These establish owner-bound synchronous selection and empty-on-cancel, not live focus, disabled-owner, physical keyboard or DPI acceptance. Multi-folder selection is the explicitly approved OPT-07 change; the reference Browse path was single-destination.

## Next production call chain and invariants

`PlatformPipeClient.ReadCommandsAsync` archive_picker branch parses `ArchivePickerCommand`, verifies envelope correlation, then dispatches `pick_archive_path` through the original detached `PlatformCommandSequenceDispatcher`. `ArchiveJobPicker.Pick` validates IsWindow/GetWindowThreadProcessId against the pipe peer, enters the original `BoundedStaOperationRunner.TryRun` reservation (one archive picker, `Timeout.InfiniteTimeSpan` for human selection), and calls `StaDialogRunner.RunInline`. Its cancellation registration enumerates only that STA's native dialog windows and posts WM_CLOSE. A modal-thread timer requests the same close when the original HWND/process owner disappears. The bounded reservation remains held through actual modal return and dispatcher cleanup, including after logical cancellation. There is no replacement runtime or extra dialog thread.

The original folder and destination branches used FolderBrowserDialog.ShowDialog(IWin32Window); files use OpenFileDialog.Multiselect and save uses SaveFileDialog. InitialDirectory is supplied only when the requested directory exists. The typed version-1 result already supports a complete vector of at most 32 roots / 131072 UTF-8 bytes, with each path subject to ArchiveJobWire.PathValid. Native picker selection grants no source execution, overlap approval or write authority. Rust retains draft/source/snapshot/folder/connection validity and once-only installation.

Required implementation invariants:

- Folder-source selection uses one true Windows multi-folder dialog. Destination remains single, files remain multiple, save remains single.
- The exact original HWND/peer is validated before presentation and after successful return; retirement/cancellation yields no selection and cannot activate or install into a replacement owner.
- Owner/modal placement and focus remain native-owned. No explicit foreground steal, independent chooser controller, additional thread or elapsed deadline for human selection. `PickerOwner` temporarily suspends visible Topmost windows of the authenticated Desktop process and restores their recorded states after presentation; unrelated applications are unchanged.
- Validate the entire returned vector before serialization; overflow or an invalid path returns `picker_limit` and zero paths, never a prefix or generic failure. Native failures retain native error codes without exception messages or paths.
- Existing operation/session/generation/envelope correlation and detached pipe route remain unchanged. Capability `archive_multi_folder_picker_v1` describes adapter availability; negotiated absence refuses source-folder selection locally before pending/send. Destination/files/save retain their original semantics and the wire remains version1.
- Diagnostics use the original IPlatformLog and operation correlation: accepted/start/terminal, elapsed milliseconds, fixed category/native code and count only. No path, window title or payload.

## Native adapter and verification boundary

The dependency-free native adapter uses Windows SDK `IFileOpenDialog` (IID D57C7288-D4AD-4768-BE02-9D969532D960 / CLSID DC1C5A9C-E88A-4DDE-A5A1-60F82A20AEF7). It preserves the inherited IModalWindow/IFileDialog vtable, sets `FOS_PICKFOLDERS | FOS_FORCEFILESYSTEM | FOS_PATHMUSTEXIST | FOS_NOCHANGEDIR`, and adds `FOS_ALLOWMULTISELECT` only for source folders. `SHCreateItemFromParsingName` plus `SetFolder` seeds the existing initial directory; `Show(original HWND)` is the native modal boundary. `GetResults` / `IShellItemArray.GetCount/GetItemAt` / `IShellItem.GetDisplayName(SIGDN_FILESYSPATH)` returns the complete filesystem vector with balanced COM references. ERROR_CANCELLED HRESULT returns no selection. An instance-scoped presentation delegate controls only modal return in fixtures; owner validation, adapter options, result bounds and actual reservation are production code.

Controlled tests may use self-owned hidden HWNDs and the presentation delegate; they never invoke a system picker. Real system multiselect, focus/keyboard/DPI and foreign-process owner behavior remain separately authorized G-18/G-28 acceptance.

## Shared human-selection contract

Directory/program, clipboard archive/storage, extension manifest/runtime/settings-path and browser-export destination commands carry the initiating `source_window_handle` and `source_process_id`. `PlatformPipeClient.ValidatePickerPeer` binds the process to the authenticated pipe peer; `PickerOwner.Run` validates the live native HWND before and after presentation. The existing detached route and bounded STA own admission and cleanup. Human waits have no IPC/native/feature elapsed deadline; ordinary machine commands keep their existing budgets.

Browser export first uses `pick_browser_extension_export_path { format, source_window_handle, source_process_id }`. Selected nonempty data starts `export_browser_extension { format, destination }` with a distinct command ID and the same logical correlation; normal Cancel returns successful empty data and starts no exporter. Rust installs pending state before either send, validates the original window/draft, and rejects retired or duplicate replies. A started exporter retains its bounded machine policy and may finish after settings close.

Desktop directory and FFmpeg results apply only to the original draft and unchanged input generation, with another check inside deferred application. FFmpeg's managed-install completion remains separate from a manual picker result. Closing settings retires its waiting selection, allowing a replacement draft to start a new operation. Controlled protocol/router, original-STA/owned-HWND and GPUI regressions are source acceptance; real focus, keyboard and DPI remain native acceptance.
