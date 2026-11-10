# Bandizip 7.32.0.1 — Shell preview worker, limits and lifetime (static reference)

**Subject and boundary.** Only the shipped x64 `D:\Tool\Bandizip\bdzshl.x64.dll` was analyzed (SHA-256 `2f6f959726db2460725d8c4e0027e2c5e844a978ce3a2ddf1fd2ec1c44a04ae7`). Ghidra12.1.4/REA reconstructions and PE/Capstone instruction slices are static observations, not original source or runtime/Explorer acceptance. Adjacent `ark.x64.dll` appears as the loader/API boundary consumed by this DLL, not a separate analysis subject here. No target program, Explorer, GUI or archive operation was executed; no live registry, settings, credentials, user logs or services were read or changed. Bandizip EULA§3 and applicable-law limits apply; no proprietary implementation reuse is proposed.

## Joined mechanism: `QueryContextMenu` to published preview

1. **Admission and owner-draw row.** The existing `QueryContextMenu` query at `0x18001a2a0` resolves to actual `FUN_18001a278`. After the existing recognition path accepts one source, it inserts a separator and a blank `MF_OWNERDRAW | MF_BYPOSITION` preview item. The item command ID is the next Shell command ID. It copies the preview source path into owner `+0x2b0`, which is worker-state `+0x178`, then calls `FUN_180013474(owner+0x138, 0)` [P1,P2]. This is the real preview start edge; paint labels are downstream consumers.
2. **Worker creation.** `FUN_180013474` refuses a second active start, resets the worker events/state and creates a native thread for `FUN_180013278`; the CRT adapter reaches `CreateThread`. The thread initializes COM, calls the worker virtual at `FUN_18001f45c`, then uninitializes COM [P2,P3; S1]. The preview state was constructed with a direct pointer to the Shell settings block at owner `+0xf28`, retained at worker state `+0x180` [P4].
3. **Enumeration source.** `FUN_18001f45c` builds the adjacent `ark.x64.dll` path, loads it through `FUN_180014450`, resolves `CreateArk` and `CreateArkCompressor`, creates the Ark interface with ABI value `0x20230308`, installs the worker callback, applies reader options from the settings block and opens the retained source path. It obtains the catalog count through Ark method slot `+0x68` and each member through slot `+0x70`. For each retained member it stores a separate 24-byte preview record containing a display path, original member path and directory flag. The preview therefore comes from the archive reader catalog, not filesystem traversal [P3,P5].
4. **Result storage.** The worker-owned vector is at state `+0x130/+0x138/+0x140`, corresponding to owner `+0x268/+0x270/+0x278`, exactly the vector later consumed by `FUN_18001e5f0`. The flat worker projection excludes directory records; tree mode retains directory records and paint builds its tree from the same vector [P3,P6].
5. **Actual publication.** On a successful start, `FUN_18001a278` cancels any prior global preview timer, stores the menu handle, preview command ID and worker-state pointer in `DAT_18007f108/110/118`, and starts a 200-ms thread timer for `FUN_18001dde8`. The timer polls `FUN_180013510`; when the worker is no longer active it toggles that command `disabled → enabled`, forcing the live menu item to refresh, then kills the timer and clears the global state pointer [P1,P7,P8]. Until that transition, paint's state poll selects the opening presentation; afterwards the same owner-draw row consumes the retained vector or terminal reader fields. This connects menu construction to result publication rather than inferring behavior from preview strings.

## Preview-size unit and stopping rules

- **Effective setting range is `[1,20]`.** The settings reader maps every value `<2` to `1`, leaves `2..19` unchanged, and maps values `>19` to `20` [P9].
- `shell_PreviewSize` is consumed as a **count of non-directory archive-member callbacks**. The callback count is worker state `+0x170`; the configured value is settings `+0x44` (owner `+0xf6c`). In flat mode (`shell_PreviewWithTree == 0`) enumeration requests stop when `PreviewSize <= completed_non_directory_count` [P10]. It is not bytes, MiB, compressed size, elapsed seconds or pixel rows.
- Directories do not increment that count. In flat mode the worker also omits directory records from the published vector. In tree mode (`settings+0x48`, owner `+0xf70`) the configured preview-size comparison is bypassed and directories are retained [P3,P10].
- Both modes retain independent safeguards: the callback requests stop once the non-directory count has reached 700, and it checks the elapsed-time condition while below that cap [P10,S2]. The `300` passed by the painter into `FUN_180029428` is a separate tree-builder parameter; it is not the persisted preview-size value [P6].
- The callback performs its stop tests before incrementing for the current non-directory member. Whether the Ark reader includes the stop-triggering member in its returned catalog is an external callback-contract edge not established from this DLL alone.

## Deadline and cancellation

- The worker stores `GetTickCount()` immediately after the Ark DLL is successfully loaded, before reader setup/open. The per-member callback computes unsigned elapsed ticks and requests stop only when elapsed time is **strictly greater than 2000 ms** (`0x7d0`); it also sets the timeout field consumed by paint [P3,P10,S2].
- This is **not a wall-clock watchdog**. The check runs only when Ark invokes the member callback. A stall in `LoadLibraryW`, factory creation, archive open, or reader work before the next callback is not interrupted at two seconds.
- The cancellation flag tested by the callback is worker state `+0xf8`, the same storage as owner `+0x230`. The bounded full-image instruction scan found the only write to that owner field in `FUN_180015efc`, where owner cleanup sets it to `1` before waiting for the worker [P10,P11]. There is no separate menu-close write in the inspected DLL.
- Owner cleanup waits up to **30,000 ms** through `FUN_180013558`. If the wait times out it logs the condition and calls `TerminateThread` on owner `+0x148`; otherwise the callback's cancel flag can stop enumeration cooperatively at the next member callback [P11,P12]. Cancellation is therefore callback-cooperative first, with a 30-second forced termination fallback during owner cleanup; it is not immediate cancellation merely because the popup became inactive.

## Reader, DLL, result and handle lifetime

- At worker start, `FUN_18001f1d8` destroys the prior preview-record vector. After enumeration, the Ark archive is closed through reader method slot `+0x30`, but the worker's Ark interface and `ark.x64.dll` module remain resident in the embedded worker state [P3,P13].
- The retained preview vector remains available to owner-draw paint until the next worker start or worker-state destruction. There is no separate cross-owner archive-content cache in this path [P3,P13].
- Worker-state destruction frees the preview records, releases the retained Ark interface and calls `FreeLibrary` for its `ark.x64.dll` handle. Separately, the recognition loader at owner `+0x980` retains its own Ark object/module and releases/unloads them in owner cleanup. Recognition and preview therefore use separate loader/reader instances with the same owner lifetime [P11,P13,S3].
- **Natural-completion thread-handle leak.** When `FUN_18001f45c` returns status0, generic body `0x1800135ac` clears preview state `+0x10` before returning. The six-argument thread creator returns a real `CreateThread` handle to its caller without assigning it to CRT thread-parameter `+0x10`. The exit helper can close that separate optional CRT handle, but does not recover the handle lost from preview state. In the ordinary ordering where start stores the returned handle before enumeration finishes, later polling sees a null slot and neither `FUN_180013510` nor owner cleanup can close it. Cancel/teardown can close a handle it successfully takes and waits on; natural completion loses this reference. [P2,P8,S1,S4,S6]

## Evidence locator

All REA records below have the same exact subject shown in the subject table. Addresses name Ghidra procedures; a queried interior address may resolve to an earlier actual function entry.

| Ref | Canonical filename under `evidence/raw/` | Actual `evidence_id` | Boundary |
| --- | --- | --- | --- |
| P1 | `bdzshl-viewcontrols-follow1-procedure_pseudo_code-0x18001a2a0.json` | `ev_12571bdba6d62d8a467ebd07140e9be182245dd72d0c51dddb676cfd795a4e22` | Actual `FUN_18001a278`: recognition, owner-draw insertion, worker start and 200-ms timer registration |
| P2 | `bdzshl-preview-thread-start-procedure_pseudo_code-0x180013474.json` | `ev_45f9bb824a785079fc1a3bc1daa06af807014a405fd6c87179e3cfe6b75fdd50` | Worker-state reset and thread creation |
| P3 | `bdzshl-preview-worker-procedure_pseudo_code-0x18001f45c.json` | `ev_4a2a4577c33026044d2dd26005f87aa13e93dc58b5a4f7f96731f590a5f17a5b` | Ark load/open, catalog enumeration, member-vector publication fields |
| P4 | `bdzshl-preview-state-constructor-procedure_pseudo_code-0x18001f000.json` | `ev_4ea09fb82850770c0cbf4a9af2af0b29b8d9b5cf0cd3ff1404411630a77d9606` | Embedded worker/loader construction and retained settings pointer |
| P5 | `bdzshl-preview-reader-loader-procedure_pseudo_code-0x180014450.json` | `ev_fbc024cd504cb1a17a5e6176f4c03cac8c93e26375a3db8695f7d4505b6ef206` | `LoadLibraryW`, Ark exports, interface creation and loader errors |
| P6 | `bdzshl-viewcontrols-follow2-procedure_pseudo_code-0x18001e9f0.json` | `ev_76b013eaeb1b3a83011d2ee1b8694ecc441fc8114bddf18b944e977e7b839d76` | Actual `FUN_18001e5f0`: retained-vector consumption and flat/tree paint split |
| P7 | `bdzshl-preview-publication-procedure_pseudo_code-0x18001dde8.json` | `ev_c0eb3ef434e95a60e1acb06f8a2a2a3447d29dbc55211fb6841e21f5d57a9839` | Timer result publication by command enable-state toggle |
| P8 | `bdzshl-preview-thread-poll-procedure_pseudo_code-0x180013510.json` | `ev_8e79face115431f0f828aebbb7cc1f1412c757fefda707bd24e6af14227fa6b1` | Nonblocking worker-state poll and close-if-signaled path |
| P9 | `bdzshl-viewcontrols-follow2-procedure_pseudo_code-0x180028b21.json` | `ev_fe7a1cc454b5b8b1b3c9012130baa61ef32b743c1da06c06a2235048328a08bf` | Shell settings read; `<2 → 1`, `>19 → 20` normalization |
| P10 | `bdzshl-preview-callback-procedure_pseudo_code-0x18001f3c8.json` | `ev_5317505e15f8d319e1a05d0c17efafe6fb0ad844b6cc1b52fa9aa864d052b3d7` | Per-member count, tree bypass, 700 cap, timeout/cancel stop requests |
| P11 | `bdzshl-preview-owner-cleanup-procedure_pseudo_code-0x180015efc.json` | `ev_3945405f8e918b77b7f0a6474956944d212320e25a9c74248e21985660f7b179` | Owner cancel flag, 30-second wait, forced termination and loader teardown |
| P12 | `bdzshl-preview-thread-wait-procedure_pseudo_code-0x180013558.json` | `ev_8dbfeebf2c8980940b99457e5bd6cfd81ccc9e0c1799cbb1e702cba759c130ce` | Atomic handle take, timed wait, close or restore-on-timeout |
| P13 | `bdzshl-preview-state-cleanup-procedure_pseudo_code-0x18001f1d8.json` | `ev_a8e14f5933ef54a70e0fed92ad2d8454167c5bd3c68d6782bbab67681d0ae108` | Retained preview-record/vector cleanup |
| P14 | `bdzshl-preview-worker-loop-procedure_pseudo_code-0x18001335c.json` | `ev_1b9228b63a1309efa9718f63c106c0fdd4d756becdc63cd1a82c5304d033c86a` | Generic worker event/cancel loop |

| Subject | SHA-256 | Format / architecture | Provider/profile |
| --- | --- | --- | --- |
| `D:\Tool\Bandizip\bdzshl.x64.dll` | `2f6f959726db2460725d8c4e0027e2c5e844a978ce3a2ddf1fd2ec1c44a04ae7` | PE shared library / x86-64 | Ghidra 12.1.4; profile digest `90e13888e18b7d587414bb259f5e3dc44a7c9293bdf290ad8c459be524747d9f` |

Static instruction records have no REA `evidence_id`; each identifies the same subject, file size, image base, Capstone/pefile versions and `.pdata` limitation in its header.

| Ref | Static record | Coverage |
| --- | --- | --- |
| S1 | `bdzshl-preview-static-thread-api.txt` | `CreateThread` adapter and COM worker entry |
| S2 | `bdzshl-preview-static-deadline.txt` | `GetTickCount - start > 0x7d0` timeout predicate |
| S3 | `bdzshl-preview-static-worker-object.txt` | Worker-state loader/interface destruction and `FreeLibrary` |
| S4 | `bdzshl-preview-static-constructor.txt`, `bdzshl-preview-static-thread-thunk.txt` | Generic completion clears preview-state handle; CRT parameter construction and thread entry |
| S5 | `bdzshl-preview-static-cancel-writes.txt` | Full-`.text` destination scan for owner cancellation field `+0x230` |
| S6 | `bdzshl-preview-static-thread-end.txt`, `bdzshl-preview-static-thread-exit.txt` | CRT exit helper and its optional parameter-block handle, distinct from the caller-returned thread handle; import names retained in `bdzshl-preview-thread-exit-api.json` |

## Replay

- REA replay: `evidence/raw/recipes/rea-bdzshl-preview.json`. It opens the exact DLL using the matching `bdzshl-viewcontrols-session-snapshot.json`, writes the canonical records above, saves the independent private snapshot `evidence/raw/bdzshl-preview-session-snapshot.json`, and closes the session.
- Static replay uses `evidence/raw/tools/inspect-pe-static.py` against `bdzshl.x64.dll`. Requests for S1–S4 are `function:0x180042f64 function:0x180013278`, `function:0x18001f370`, `function:0x18001f000 function:0x18001f0f4`, and `function:0x180015802 function:0x1800135ac`. S5 records the full-`.text` destination-operand scan for displacement `+0x230`. S6 uses `function:0x180043040 function:0x180042e9c`; RIP-relative call operands resolve through the shipped PE import table.

## Limits and remaining edge

- The exact Explorer popup-close → COM `Release` → owner-cleanup timing is not established statically. The DLL has no independent menu-close cancellation write: cooperative cancellation begins only when `FUN_180015efc` runs. A popup may therefore close while enumeration continues until a natural limit or later owner cleanup.
- `FUN_18001dde8` is the only completion path that kills the 200-ms timer and clears `DAT_18007f118`; a later `QueryContextMenu` kills a prior timer before replacement. The inspected owner-cleanup path does neither. If Explorer releases the owner after popup close before the completion tick, the global timer's state-pointer lifetime is unresolved and requires runtime lifecycle verification.
- The two-second rule and cancel flag depend on Ark invoking the member callback. They do not bound an Ark call that does not return to that callback. The 30-second owner-cleanup wait and forced termination are the only observed fallback.
- The stop-triggering member's inclusion remains dependent on the Ark callback contract. No claim is made about actual installed/enabled Shell registration, live menu timing, archive compatibility or reader behavior beyond the calls made by this DLL.

