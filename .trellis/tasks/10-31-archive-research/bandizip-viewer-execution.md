# Bandizip 7.32.0.1 — viewer, executable warning and extract-all-then-run

**Boundary.** Read-only static inspection of shipped `D:\Tool\Bandizip\Bandizip.exe` and, where the actual route crosses the process boundary, `D:\Tool\Bandizip\Arkview.x64.exe` (both x64 file version 7.32.0.1). Their addresses and evidence identities are independent. Ghidra 12.1.4 reconstructions and supplementary Capstone 5.0.7 PE instructions are not original source or observed execution. Only compiled artifacts and shipped language resources were read. Bandizip EULA §3/applicable-law limits recorded in `docs/reference-code/archive.md` apply; no proprietary implementation is proposed for reuse.

## Joined route map

| User intent | Original selection/source | Materialization | Launch boundary | Lifetime/result |
| --- | --- | --- | --- | --- |
| Image preview enabled in the archive browser | Current selected/focused row is resolved back to its original catalog member index and member path; current archive path/password context remain separate | None in the main process | Persistent `Arkview.x64.exe /aschild <parent HWND>` child; main sends archive/member/context through bounded `WM_COPYDATA` calls | Child HWND is reused. Parent disappearance or child close ends the Arkview message loop; the browser retries creation through timer 5 when the child is unavailable. |
| Double-click/Open with the internal image-viewer setting | `FUN_140156ce0` resolves one regular member and passes archive path, member path and original index to `FUN_140108950` | None | Standalone `Arkview.x64.exe` ordinary command mode, not `/aschild` | Launch result is returned by the helper but discarded by the void browser owner; this body has no fallback to selected-member temporary extraction. |
| Double-click/Open with BandiView/Honeyview | Same original member and archive source | None when the configured viewer admits archive/member arguments | Configured viewer receives `/open -p:<archive> -select:<member> ...`; an older compatibility form uses `"archive|member"` | Process is external and untracked here. Failure does not fall through to the associated-app temporary route in this owner. |
| Double-click/Open with associated image program | Same original member | Existing selected-member temporary extraction | Existing `ShellExecuteExW` associated-app route | Existing child/result/temp tracking applies. This is distinct from both Arkview archive-aware routes. |
| Open a member matching `extOfExtractAll` | Same original member path/index, plus current whole archive source | Whole archive is extracted to an application temporary root; selected path is reconstructed under that root | Existing mode-0 associated launch using the reconstructed selected output | Non-success/cancel suppresses launch. Warning cancel and no-process paths stage the temporary root for cleanup; actual recursive removal is not established here. |

## Full image viewer: `/aschild` preview versus file-viewer launch

### Embedded archive preview

`FUN_14015e190` is the browser-side producer for the `/aschild` route. It first requires the preview setting at application state `+0x308`, a usable opened reader and a selected/focused list item. It resolves the row through the browser map to the original member index, rejects a missing/directory/non-image member, and retains archive path, member path, password context and the original index as separate arguments. If the Arkview manager is not ready, it arms owner timer 5 for 400 ms. The timer owner `FUN_14013ae70` kills that timer, calls `FUN_140109be0`, then runs `FUN_14015e190` again.

`FUN_140109be0` starts `Arkview.x64.exe /aschild %08x` with the archive-window HWND. It waits up to 3 seconds for input idle, closes the returned process/thread handles, then searches for an `ArkviewClass` child under that parent with at most 30 additional 10 ms waits. Only the PID, child HWND and ready flag remain in the manager; subsequent requests reuse that window.

`FUN_14010a0a0` starts the helper if needed, constructs the archive/member/context payloads and sends them to the retained child with `SendMessageTimeoutW(WM_COPYDATA, ..., 200 ms)`. Its final path request is retried only while the child HWND remains valid, with the observed loop bounded at 20 attempts. The manager records the new current request only after a successful send. This channel transfers strings/context, not decoded pixels or a temporary extracted image.

Arkview's `/aschild` command branch constructs `CChildWnd` under the parsed parent HWND and enters a message loop. The retained static CChildWnd window body handles `WM_COPYDATA`: type 0 copies the supplied path and arms timer 2 for 50 ms before opening; the other observed types update auxiliary context/state. `WM_CLOSE`/window destruction exits the loop. Independently, the outer `/aschild` loop waits in 200 ms message-aware intervals and posts quit when the parent HWND is no longer valid. Destruction releases the retained string/window base. This is parent/window lifetime coupling, not process-handle supervision from Bandizip.

### Full viewer on Open

The ordinary Open route is separate. `FUN_140156ce0` resolves one selected row to its original archive member and, for eligible image members, calls `FUN_140108950` rather than extracting the member first.

- Image-viewer setting 0 (the UI label is “Bandizip internal image viewer”) launches the shipped `Arkview.x64.exe` in its ordinary archive/image command mode. Arkview opens the archive itself and owns a normal top-level viewer/message loop. It is not the reusable `/aschild` child window.
- Image-viewer setting 2 launches the separately installed BandiView/Honeyview path. Supported versions receive archive-aware `/open -p:<archive> -select:<member> ...` arguments; the inspected compatibility branch can use `"archive|member"` for an older viewer.
- Image-viewer setting 1 (“associated program”) bypasses this archive-aware viewer helper and follows the already connected selected-member temporary extraction plus associated `ShellExecuteExW` route.

The standalone viewer launches do not create a Bandizip-owned temporary member and do not enter the child-exit/writeback tracker. Their launch return is not converted into a browser error or fallback action by `FUN_140156ce0`.

## Executable-open warning and choice

The warning is owned by the associated `ShellExecuteExW` branch in `FUN_140161eb0`, after the selected output exists. The extracted file path is classified by `FUN_140123230`; when it is executable and application setting `+0x614` is clear, Bandizip creates owner-modal dialog template `0xd3` (`CAskExecutableDlg`). Shipped copy identifies the risk, asks whether to continue, and offers “Do not ask again.”

The dialog's actual event handler `FUN_14010e060` has two terminal choices:

- **OK (`ID=1`)** reads checkbox `0x47e` into dialog state and ends with result 1. If checked, `FUN_140161eb0` sets application field `+0x614=1`, then continues to `ShellExecuteExW`.
- **Cancel (`ID=2`)** ends with result 2. The launch helper stages the temporary root through `FUN_140029bf0`, destroys the dialog, releases the request strings and returns handled-success to its caller without calling `ShellExecuteExW`; no secondary launch-error message is shown.

The setting key is `bDoNotAskExecutable`. The settings reader binds it to `+0x614`, and the settings writer persists that field. The dialog changes in-memory state; persistence follows the normal settings-save path rather than a direct write inside the dialog handler.

The Shell request uses `SEE_MASK_NOCLOSEPROCESS`. A successful call with a process handle enters the existing tracked-child record. Success without a process handle stages the temporary root in the global cleanup queue. Native `ERROR_CANCELLED (0x4c7)` also stages the temporary root and returns without a child. Other association/direct-execution failures remain launch failures and can produce `TEXT_ERR_AT_EXECUTE_DIRECT`; they are not reported as successful execution.

This warning is not inserted into the configured-editor or Open-with-Bandizip `CreateProcessW` branches. It protects the associated executable-open boundary.

## Extract all, then run the original member

Settings key `extOfExtractAll` is stored at application state `+0x2b8`. `FUN_1401213b0` copies the semicolon-separated pattern list, normalizes each pattern and applies `PathMatchSpecW` to the selected member path. Shipped settings copy describes this as running configured file types only after all files are extracted.

In `FUN_1401499f0`, a match is tested only for Open modes below 2. A match diverts before the normal one-member temporary extraction and enters `FUN_14014a6b0`:

1. The original selected member path is retained independently from the archive source.
2. `FUN_1400ce7c0(...,0)` supplies the application temporary root.
3. A common operation descriptor is built with operation 5, the current archive source and no selected-index vector, so the ordinary extraction owner processes the whole archive into that root.
4. `FUN_14014efc0` runs the common progress/worker boundary. Only result 1 with operation error 0 continues.
5. The selected launch target is reconstructed as temporary root + original member path. `FUN_14014a3c0` receives the archive source, temporary root, reconstructed output and original member context, then calls the existing mode-0 associated launch manager.
6. The executable classifier and warning therefore run after successful whole-archive extraction and before the selected executable is handed to Shell.

Cancellation or extraction failure never reaches step 5. The request and operation objects are destroyed after the coordinator returns. The common progress owner retains its existing behavior: stop joins the worker before modal `EndDialog` or modeless `DestroyWindow`; terminal cancellation remains distinct from extraction success.

The launch record retains both the temporary root and the selected extracted path. On tracked child exit, the existing poller closes process/thread handles and checks the selected file's last-write time. Unchanged and warning-canceled paths stage the temporary root through the existing cleanup queues according to `deleteTempFileWhenChildProcessExit`; changed-file handling remains the already documented explicit parent update path and is not repeated here.

## Evidence and subjects

Canonical filenames in the table are relative to `evidence/raw/`.

| Boundary | Canonical evidence | Evidence ID | Subject |
| --- | --- | --- | --- |
| Selected row/mode and viewer diversion | `bandizip-create-p-0x140156ea1.json` | `ev_a3f8fedddb98a2e20bda53693cd1b096f84ae2e94ab791b291e92c1b20a7acb1` | `Bandizip.exe` SHA-256 `b6c0eefb56b6d08e627ecea84dbd09bfa265ff7b3a08d35c742b60e747f95324` |
| Standalone internal/external viewer command selection | `bandizip-browser-code-0x1401098e9.json` | `ev_51d2a953ab8371099445269af99715f914c18d95f31c85c253088c4c510e90bb` | `Bandizip.exe` |
| `/aschild` process/child-HWND creation | `bandizip-browser-code-0x140109be0.json` | `ev_b1e84cd1bbd942e0403a3a7af79b56bf4e493f3bb195d8509b5e3a5f752f382c` | `Bandizip.exe` |
| Browser preview selection producer | `bandizip-execview-p-0x14015e190.json` | `ev_a13d76132158acdb3cf230ae2c5bfbf82bb55535c4ee01947e73a3fa2d71dd2c` | `Bandizip.exe` |
| Main-side `WM_COPYDATA` request | `bandizip-browser-cont-code-0x14010a0a0.json` | `ev_ee26f269bef59842e008e6ef847496221b8d05d548730e558da3daf45fcb7aa6` | `Bandizip.exe` |
| Extract-all extension matching | `bandizip-execview-p-0x1401213b0.json` | `ev_38c239088c24c727df1b62586a256f38e012209e8b163af2e63a6093de945c24` | `Bandizip.exe` |
| Whole-archive temporary operation and selected run | `bandizip-execview-analysis-0x14014a6b0.json` | `ev_063e8678e02926a017b61a027a7d0e2b83855f5dd3bea130a130b0ac8726ee78` | `Bandizip.exe` |
| Associated launch, warning gate and result queues | `bandizip-browser-code-from-0x140161f3e.json` | `ev_19bb8f824c88232b31e22426d59a58980ca6c3df4ccbb1a04f4253b3dfdf4864` | `Bandizip.exe` |
| Warning dialog event/choice | `bandizip-execview-p-0x14010e060.json` | `ev_0d65431af83fcf302f4b606ba985d1a8e0c414355b94927dafd5e8a2830ed7b0` | `Bandizip.exe` |
| Warning/settings string inventory | `bandizip-execview-strings-exec.json` | `ev_b5fcbbfe1f43efb75232dde8d24a28f2636d3c4350ad8c975ef1c48e607afe2f` | `Bandizip.exe` |
| Extract-all / warning-setting xrefs | `bandizip-execview-xrefs-extOfExtractAll.json`; `bandizip-execview-xrefs-bDoNotAskExecutable.json` | `ev_e274b225c2cb32c467b18b8e16c65942fe32173e3002bea6768b148b01c4c748`; `ev_dc6f8c0fb9e8895692cfe05e226977c7d1a60e9fd3cc88239197337d8affa507` | `Bandizip.exe` |
| Settings read/write owners | `bandizip-browser-code-from-0x140165b52.json`; `bandizip-browser-code-from-0x14016ae74.json` | `ev_b337340b1014fec23f907058bc13566c3ca7f897618dd5ea619147d55fb9f634`; `ev_f9fb926fd126cdfa26cdb64777d5f3c89aa2efa8a5ca0940a26c332774e531d2` | `Bandizip.exe` |
| Arkview command-mode switch and `/aschild` loop | `bandizip-browser-arkview-code-0x14009d915.json` | `ev_a37c83e05e499a3c8aa04314cee28917f600fe2d7e6cb5f3999dd96e0b052c10` | `Arkview.x64.exe` SHA-256 `8b9eb807e90f2db104112f06886f366bf7aa1039df8f05a14243692457b28418` |
| Arkview child-window initialization | `bandizip-browser-arkview-code-0x14009ab10.json` | `ev_d458160b4639297908328973d1330b316b13de95a544b026308793542d149003` | `Arkview.x64.exe` |
| CChildWnd vtable identity/bytes | `arkview-execview-x-CChildWnd-vftable.json`; `arkview-execview-bytes-CChildWnd-vtable.json` | `ev_41cd1083f3eb998915922a217a682cf18c8ea4f3493ff8b6beda0f32f2ea0f39`; `ev_d6bbedf40163cc26c573b344392ae6e3cffcbedcacd32b06f1b8e94175a58e0e` | `Arkview.x64.exe` |
| Arkview child destruction | `arkview-execview-p-0x14009aaa0.json` | `ev_9ae1d8b66bcffc4f082b57f8782b7ad5ef2d90da4890146729fa1323f100ce16` | `Arkview.x64.exe` |

`evidence/raw/arkview-execview-static-cchild.txt` supplies address-labelled `CChildWnd` message instructions, and `evidence/raw/bandizip-execview-static-language.txt` supplies shipped menu/settings/warning copy and exact versions. They are supplemental static slices and intentionally have no REA Evidence ID.

## Replay

Bandizip records seed from `evidence/raw/bandizip-create-session-snapshot.json` and are retained in `evidence/raw/bandizip-execview-session-snapshot.json`. Replay recipes are:

- `evidence/raw/recipes/rea-bandizip-execview-core.json`
- `evidence/raw/recipes/rea-bandizip-execview-follow.json`
- `evidence/raw/recipes/rea-bandizip-execview-vtable.json`
- `evidence/raw/recipes/rea-bandizip-execview-dialog.json`

Arkview records seed from `evidence/raw/arkview-reference-session-snapshot.json` and are retained in `evidence/raw/arkview-execview-session-snapshot.json`. Replay recipes are:

- `evidence/raw/recipes/rea-arkview-execview-class.json`
- `evidence/raw/recipes/rea-arkview-execview-vtable.json`
- `evidence/raw/recipes/rea-arkview-execview-handler.json`
- `evidence/raw/recipes/rea-arkview-execview-dispatch.json`

Supplemental CChildWnd instructions replay with:

```powershell
python evidence/raw/tools/inspect-pe-static.py Arkview.x64.exe arkview-execview-static-cchild.txt function:0x14009a430 function:0x1400a4a10
```

## Limits

- No target executable, Arkview process, GUI, Explorer/helper, archive, open/view/run/extract action, registry, live configuration, credential, user log, service, build, package, installation or product code was executed or changed.
- The joined static path proves selection/source/context transfer, operation and launch gating. It does not establish live focus, visible error presentation, native application acceptance, process exit timing or accessibility behavior.
- Standalone viewer launch failures are not given a fallback by the inspected browser owner; exact user-visible failure behavior below the launch helper remains unobserved.
- Extract-all non-success suppresses launch, but `FUN_14014a6b0` does not itself recursively delete the temporary destination. Cleanup is queued/staged through existing root/path lists; actual removal of every sibling, partial output and directory after warning cancel, worker cancel, child crash or application crash is not established.
- `extOfExtractAll` is user-configurable pattern data. The shipped default value and every pattern/case interaction were not read from live settings.
- `/aschild` uses bounded sends and parent-window polling, not a generation token or process-handle join. A timed-out request, helper crash during a send and rapid archive replacement remain runtime interleavings.

