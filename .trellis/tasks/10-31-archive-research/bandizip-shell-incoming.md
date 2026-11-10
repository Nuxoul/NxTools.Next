# Bandizip 7.32.0.1 — Shell ZIP drop and queued incoming files (static reference)

## Scope and identity

Read-only shipped-artifact reconstruction of `D:\Tool\Bandizip\bdzshl.x64.dll` (x64, SHA-256 `2f6f959726db2460725d8c4e0027e2c5e844a978ce3a2ddf1fd2ec1c44a04ae7`) and `D:\Tool\Bandizip\Bandizip.exe` 7.32.0.1 (x64, SHA-256 `b6c0eefb56b6d08e627ecea84dbd09bfa265ff7b3a08d35c742b60e747f95324`). Ghidra 12.1.4 pseudocode is reconstruction, not original source or observed execution. PE/Capstone/RTTI slices are separately marked static evidence without REA Evidence IDs. No target, Explorer, GUI, registry, live setting, archive, helper, credential, log, service or network operation was executed or changed. Bandizip EULA §3/applicable-law limits apply; no proprietary implementation is proposed for reuse.

## Mechanisms

### Four different drag/drop boundaries

| User shape | Actual static boundary | Operation timing |
| --- | --- | --- |
| Explorer files dropped **onto a ZIP icon** | `Bandizip.zip\shellex\DropHandler` → `bdzshl.x64.dll` `IPersistFile` + `IDropTarget` → a new `Bandizip.exe` command | Shell DLL serializes and launches; the archive operation happens in the EXE, not in Explorer or the open-window drop callback. |
| Files dropped into an open archive window, legacy mode | `WM_DROPFILES (0x233) → FUN_140148740` | The window message body extracts paths, runs `FUN_140148b20` synchronously, then clears the list. This already-established path is not re-described here. |
| Files dropped into an open archive window, OLE/fast mode | EXE `XDropTarget::Drop` → `CMainWnd` callback `0x140148a10` → retained list → posted `WM_COMMAND/0x8263` → `FUN_140148ae0` | Producer and consumer are separate turns. Returning from the OLE callback does not mean Open/New/Add has run. |
| Archive members dragged outward | `FUN_140147200 → FUN_140053460 → Ole32 DoDragDrop` | Outgoing data-object path; unrelated to either incoming receiver. |

### Explorer drop onto ZIP: association is an implemented COM target

`FUN_14018be20` handles the ZIP association branch. When the current format token is `ZIP`, it opens or creates `HKEY_CLASSES_ROOT\Bandizip.zip\shellex\DropHandler\` and writes default value `{5B69A6B4-393B-459C-8EBB-214237A9E7AC}` [E1]. The shipped DLL exposes the same `AABdzCtx` CLSID and its RTTI hierarchy contains both `IPersistFile` and `IDropTarget`; the instantiated object places those interfaces at object offsets `0x18` and `0x20`, respectively [S1]. This is not merely a shell `command` value and not an inference from context-menu support.

The association writer discards the `RegSetValueExW` result and supplies `lstrlenW(CLSID) * 2` bytes for a `REG_SZ`, excluding the documented terminating UTF-16 NUL [E1]. Static code therefore does not establish successful registration, and the stored value is not written with the documented `REG_SZ` byte count. Explorer tolerance and actual installed state were not tested.

### Target path, source admission and copy/move intent

The public `IPersistFile::Load` slot resolves to `FUN_18001e2e8`. After its internal enablement gate, it copies the supplied pathname into the COM object field that is object-relative `+0xf20`; failure of that gate returns `E_FAIL` [S1,E2]. The `IDropTarget` subobject reads the same field at its own `+0xf00`, so the target archive is the file whose Shell object was loaded, not the active path of an existing Bandizip window.

`IDropTarget::DragEnter` is `FUN_18001dfc8`. After its own gate, it resets the retained move flag and advertises `DROPEFFECT_COPY (1)` by default. Holding either Shift key changes the advertised effect to `DROPEFFECT_MOVE (2)` and sets the retained flag [S1,E3]. `DragOver`/`DragLeave` occupy the neighboring public slots; no outgoing `DoDragDrop` is involved.

`IDropTarget::Drop` is `FUN_18001e074` [S1,E4]:

1. It copies the `IPersistFile::Load` target into the common command-owner target field (`main object +0xae8`). Empty target logs an error and returns `S_OK` without launching.
2. It clears the Shell object's retained source vector (`main object +0x9a8`, count `+0x9b0`), requests `CF_HDROP`/`TYMED_HGLOBAL`, locks the returned medium, enumerates every filesystem pathname with `DragQueryFileW`, then unlocks and releases the `STGMEDIUM`.
3. A nonempty source vector calls `FUN_180016d1c` with command `4000` for copy or `4001` for move. The move bit is the Shift-derived state retained by `DragEnter`, not a later EXE/window decision.
4. The command builder quotes the target at `+0xae8`, emits `a -storeroot:yes <target> <sources...>` for copy and adds `-delsrc` for move, then uses the ordinary process/MMF launch path [E4,E5]. Commands above the existing length threshold use the established named-mapping handoff.

The source vector is cleared at the beginning of each `Drop`, populated with owned strings, and read synchronously while the command is serialized. `Drop` does not explicitly clear it after the launch call; the next `Drop` clears it. Final destructor release of this particular vector was not separately retained. The launched EXE owns its parsed copies, so later archive work does not borrow Explorer's `IDataObject` or the Shell object's vector.

### EXE target binding, eligible Add, completion and cancellation

The EXE parser maps command `a` to operation `9`; it retains the first positional pathname as descriptor `+0x08`, retains the remaining positional pathnames in the source list, recognizes `-storeroot:yes`, and retains `-delsrc` at descriptor `+0xe4` [E6]. Window bootstrap invokes `FUN_140159f90` before ordinary main-window setup, so this Shell command is not delivered through GUI `WM_COMMAND` and does not use the open-window queue [E7].

`FUN_140159f90` case `9` binds the exact Shell-supplied target [E7]:

- If the target path is not present, the descriptor is changed to operation `0x0b` and follows ordinary creation. This is a separate fallback, not Add to an existing archive.
- For an existing target, it opens that path with `FUN_1401579a0`, converts each retained filesystem source into update records, and calls `FUN_14014f8d0` with the same target pathname and source records.
- `FUN_14014f8d0` asks the active reader's update-capability boundary; refusal reports `TEXT_ERR_EDIT_NOT_SUPPORTED_ARCHIVE`. Admission constructs the update plan and calls `FUN_14013be50`. A nonzero operation/failure result deletes the temporary output and returns false; only the admitted zero-result branch calls `FUN_140154650` [E8].
- `FUN_140154650` performs configured reopen/test checks and the established original→backup, temporary→original replacement sequence before reopening the archive; failed admission or canceled/failed progress does not publish the temporary archive [E9].

The Shell DLL always returns `S_OK` after calling the launcher and ignores the launcher's return. Empty source data also returns `S_OK`. Explorer therefore receives no later archive result: EXE launch failure, unsupported update, progress cancellation, test failure and replacement failure are not propagated back through `IDropTarget::Drop` [E4,E5]. The EXE case maps an updater false result into its own result field `2`, but that is process-local and collapses cancellation with other Add failures at this boundary [E7,E8].

Move intent is represented by `-delsrc`; the Shell DLL does not delete sources. Its parser field `+0xe4` is established, while the exact copy into the operation-9 edit plan remains unretained. The separately inspected common post-item source-delete owner is cancellation/error-gated [E10], but that does not complete this Add-specific field transfer. `DROPEFFECT_MOVE` and Shell `S_OK` therefore establish requested intent, not completed source deletion.

### Queued open-window input: producer, posted handoff and drain owner

`CMainWnd` constructs an embedded `XDropTarget` at window `+0x250`, the retained incoming list at `+0x78/+0x80`, and its `IDropTargetCallback` subobject at `+0x58` [E11,S2]. Startup chooses between `DragAcceptFiles` and `CoLockObjectExternal`/`RegisterDragDrop`; the OLE branch registers the embedded target on the visible main HWND [E12]. Startup also initializes the named Shell drag/drop mapping, but that mapping initialization is not the queued-list producer.

The actual OLE producer is the second `CMainWnd` callback slot, `0x140148a10` [S2,S3]:

1. Admission callback `0x1401489f0` rejects either main-window busy byte `+0x2e8/+0x2e9`; the actual Drop callback `0x140148a10` rechecks `+0x2e8`.
2. It clears list `+0x78` and count `+0x80` before parsing the new object.
3. `FUN_140053b00` requests `CF_HDROP`/HGLOBAL, obtains the returned medium, enumerates each path with `DragQueryFileW`, and `FUN_140053ce0` appends an owned string node while incrementing the list count.
4. After successful parsing it adjusts the requested effect to COPY or MOVE according to `MK_SHIFT`, then calls `PostMessageW(main HWND, WM_COMMAND, 0x8263, 0)` and returns. The `PostMessageW` result is ignored [S3].

The posted command is the missing join. Main dispatch maps command `0x8263` to `FUN_140148ae0`; that function calls the already-established `FUN_140148b20`, then destroys all retained nodes and resets count `+0x80` [E13]. `FUN_140148b20` remains the Open/New/Add classifier/dialog owner already documented: cancel performs no source operation, accepted Open/New/Add take their distinct paths, and list cleanup occurs after the coordinator returns [E14].

The queue lifetime is explicit: the OLE callback fills the window's owned path list, returns after posting, and the later `WM_COMMAND` drain runs action selection before clearing. `FUN_140148b20` consults the then-current reader/location; this list is not the Shell COM target-path binding described above. A failed `PostMessageW` has no recovery branch: the populated list remains without a drain scheduled by that invocation, until another owner clears it [S3]. No failed post or concurrent-drop outcome was exercised.

`FUN_140053b00` has another concrete ownership defect. After successful `IDataObject::GetData`, it calls `GlobalLock` and enumerates the `HDROP`, but the inspected body has no `GlobalUnlock` and no `ReleaseStgMedium` on its success or lock-failure exits [S3]. Microsoft's `IDataObject::GetData` contract transfers the returned medium to the caller for `ReleaseStgMedium`; the OLE fast-drop path does not discharge that ownership. This differs from both the Shell DLL's ZIP `Drop`, which does unlock/release, and the legacy `WM_DROPFILES` receiver, which calls `DragFinish`.

## Evidence

Paths are relative to `.trellis/tasks/10-31-archive-research/evidence/raw/`. Every `ev_…` is the actual retained evidence ID for the named subject. Static rows have no REA Evidence ID.

| Ref | Canonical filename | Actual evidence_id | Subject / connected boundary |
| --- | --- | --- | --- |
| E1 | `bandizip-incoming-drop-association-procedure_pseudo_code-0x14018d139.json` | `ev_969745286c5fd772b6cb3a435d2b3dbce5d1286393cf466afdfae5e67051528b` | `D:\Tool\Bandizip\Bandizip.exe`; ZIP ProgID DropHandler key and CLSID write |
| S1 | `bdzshl-incoming-vtables-static.txt` | Static PE/RTTI; no REA ID | `D:\Tool\Bandizip\bdzshl.x64.dll`; `IPersistFile`/`IDropTarget` slots and object offsets |
| E2 | `bdzshl-incoming-ipersist-load-procedure_pseudo_code-0x18001e2e8.json` | `ev_ec939844ba97a6ec9b77a09007c170fe88a65d13a688149b754cd3702ff09ddf` | Shell DLL; target archive pathname retention |
| E3 | `bdzshl-incoming-dragenter-procedure_pseudo_code-0x18001dfc8.json` | `ev_dc21789b784fc7fec3bfa67bd9cd7359754195fc8c8381e2ac3a207e8fdd4fc0` | Shell DLL; COPY/MOVE admission and Shift intent |
| E4 | `bdzshl-incoming-drop-procedure_pseudo_code-0x18001e074.json` | `ev_7dff96618525afc06ae8dd1edcff05705f5601aeb42397701a662285bbd4ca81` | Shell DLL; target/source binding, CF_HDROP extraction, command choice, release and return |
| E5 | `bandizip-browser-bdzshl-code-0x1800172df.json` | `ev_b1c50768f6743f14816c9457e35dbe42f69299258bb8fd435884f4b32ae082a6` | Shell DLL; `4000/4001` → `a -storeroot:yes [-delsrc]`, source append and launch/MMF path |
| E6 | `bandizip-browser-code-0x1400d7a70.json` | `ev_17598dbe35fa71507923a1c4122869d3fd40db2c1b5fc4da735c4101d41942c1` | EXE; command `a` operation 9, options and retained descriptor fields |
| E7 | `bandizip-delivery-final-procedure_pseudo_code-0x140159f90.json` | `ev_d03f8fe6f1cff9c5f0ff6fa44549cf32842594bf240dd094e2afc4ef4cf4f2c7` | EXE; operation 9 target open, source-record conversion, Add/create fallback and process result field |
| E8 | `bandizip-browser-cont-analysis-0x14014f8d0.json` | `ev_7ecccc40046c4d6f05c2e1c9d5c497ebc63a8a1f4a2543612dac212f0d1925b1` | EXE; update eligibility, coordinator, temp deletion, completion gate and return |
| E9 | `bandizip-browser-code-from-0x140154b1f.json` | `ev_23defda9d8520169930e8fdcdf50b333d77edf140f471e6aa2c306ac3dd6e915` | EXE; admitted edit verification/replacement/reopen |
| E10 | `bandizip-operations-code-0x14011acd0.json` | `ev_aa275fd4da73e76d1c99ece289793ce0fc28ebc21a801cf80bd29fde6f087092` | EXE; cancellation/error-gated post-item source-delete owner |
| E11 | `bandizip-browser-code-from-0x1401390f3.json` | `ev_9c22b686aec98bd054861ad0c51c22994d75b885347a88c8063c386720bc671a` | EXE; `CMainWnd` queue, embedded `XDropTarget` and callback construction |
| E12 | `bandizip-incoming-queue-analyze_function-0x140145ac1.json` | `ev_7cf9b8ffa03bdc55a4bdb5cdea34d875d7209053d0bba11fd3d80b2e9806e239` | EXE; startup `DragAcceptFiles` versus `RegisterDragDrop`, visible HWND and mapping initialization |
| S2 | `bandizip-incoming-vtables-static.txt`; `bandizip-incoming-mainowner-static.txt` | Static PE/RTTI/Capstone; no REA ID | EXE; `XDropTarget::Drop`, `CMainWnd` callback vtable and constructor/startup placement |
| S3 | `bandizip-incoming-droptarget-static.txt`; `bandizip-incoming-queue-callback-static.txt` | Static PE x64 instructions; no REA ID | EXE; OLE Drop callback, CF_HDROP parser, owned-node append, `PostMessageW(0x111,0x8263)`, missing OLE medium release |
| E13 | `bandizip-input-core-7-b-0x140141638.json`; `bandizip-input-core-3-p-0x140148ae0.json` | `ev_b06d37c77e2d4f32d0625106ca439786f9a21f916cfdb944d3154693686cc1ee`; `ev_ceea2e8e663d9f645a9ea2d2eb635360853d004e6d8b02e7e3b2374760b0bd34` | EXE; `0x8263` dispatch and queued-list drain/clear |
| E14 | `bandizip-input-follow-3-p-0x140148b20.json` | `ev_73f45cec8047dd9efc031371de9538e7b0457f7d14a7713f4d99dd641f9bd376` | EXE; queued path classification, cancel, Open/New/Add and request cleanup |

Microsoft's public `IPersistFile`/`IDropTarget` method order identifies the slots. The `IDataObject::GetData`, `RegSetValueExW` and `PostMessageW` ownership, length and result contracts are retained in `evidence/raw/bandizip-reference-incoming-api.json`; those excerpts are API documentation, not target execution.

## Replay

REA replay uses `evidence/raw/tools/rea_follow_session.py` with the private target-matching snapshots `bdzshl-incoming-session-snapshot.json` and `bandizip-incoming-session-snapshot.json`. Run one recipe at a time with `GHIDRA_HEADLESS_MAXMEM=768M`, `REA_GHIDRA_STARTUP_TIMEOUT_MS=900000` and a distinct `REA_SESSION_LABEL`:

- `recipes/rea-bdzshl-incoming-methods.json`
- `recipes/rea-bandizip-incoming-core.json`

Both recipes close the binary and overwrite only their matching private snapshot. Existing canonical EXE parser/dispatcher/update records replay through their already-retained recipes.

Static instruction replay uses `evidence/raw/tools/inspect-pe-static.py`:

- `Bandizip.exe bandizip-incoming-mainowner-static.txt function:0x140131070 function:0x140138a70`
- `Bandizip.exe bandizip-incoming-droptarget-static.txt function:0x1400537a0 function:0x1400538f0 range:0x140053960..0x140053ae0 function:0x140053720`
- `Bandizip.exe bandizip-incoming-queue-callback-static.txt range:0x1401489e0..0x140148b20 function:0x140053b00 function:0x140053ce0`

The two `*-incoming-vtables-static.txt` files record exact shipped-image RTTI/vtable addresses and qwords; their slot names follow the public COM interface method order.

## Limitations

- Static code proves the implemented association/COM/CLI and queued producer-consumer boundaries, not that the current machine has the association enabled or that Explorer accepted a live drop.
- No archive operation was run. ZIP admission, progress UI, cancellation, source deletion, replacement and process exit behavior are connected through retained static owners, not runtime acceptance.
- The Shell COM source vector is observed clearing at each `Drop` and synchronous serialization; its final destructor body was not separately retained. The EXE queued list has a complete producer→posted message→drain→clear trace.
- The Shell DLL does not receive an EXE terminal result. Provider/process launch behavior, UAC, filesystem failures, long-command consumer timing and exact process exit-code presentation remain runtime boundaries.
- The parser's `-delsrc` slot and the common cancellation/error-gated source-delete owner are connected retained evidence; the exact field-copy instruction into the operation-9 edit plan was not separately retained.
- The association `REG_SZ` byte count/result handling and EXE OLE medium-release omissions are static defects. Actual registry/Shell tolerance, retained handle growth and user-visible failure rates were not measured.


