# Bandizip 7.32.0.1 — browser navigation and command handlers (static reference)

**Boundary.** Read-only static inspection of shipped `D:\Tool\Bandizip\Bandizip.exe` x64 7.32.0.1 (recorded SHA-256 `b6c0eefb56b6d08e627ecea84dbd09bfa265ff7b3a08d35c742b60e747f95324`), its PE menu/accelerator resources and `langs\SimpChinese.lang`. Ghidra 12.1.4 pseudocode is reconstructed implementation evidence, not original source or observed GUI execution. No target/helper process, archive, Explorer integration, registry, live configuration, credentials, user logs or service was exercised. The EULA §3/applicable-law boundary recorded in `docs/reference-code/archive.md:7–14` applies.

## Connected command and notification mechanisms

### Column popup: selection applies immediately; there is no OK stage

The shipped main menu and list popup expose commands `0x8049/4b/4d/4f/51/53/55/68/57/820c` for compressed size, original size, type, modified date, compression method, encryption method, CRC, attributes, comment and extra information. Toolbar/list-header routing reaches `FUN_14014d120`, which loads menu resource `0xa9`, localizes/checks it, and calls `TrackPopupMenu(..., TPM_RETURNCMD)`. A zero result destroys the menu and returns without changing state. A selected command is passed directly to `FUN_14014d1f0`; there is no chooser dialog and no OK→Apply boundary. [N1–N3,S1]

`FUN_14014d1f0` first forces list view mode `settings+0x62c = 0` (Details), applies the corresponding native ListView style, recalculates image-list mode, and rebuilds the current tree-node projection. It then flips exactly one enabled-column flag at `settings+0x438..+0x45c`, invokes `FUN_1401a7550` to delete all rows and columns and reinsert the enabled column set, and invokes `FUN_1401a68d0` to repopulate/sort the current folder. The handler changes in-memory settings and the visible list immediately; it does not call `FUN_14016a570`, so durable settings write remains a later lifecycle action. [N3,N4,N7]

### View style menu: concrete command mapping and capability rejection

PE resources route both the main View submenu and list context menu to `0x8186` Icon, `0x8187` List, `0x8188` Details, and `0x8227` Large Icons/preview; the accelerator table also maps these to Ctrl+Shift+2/3/4/1 respectively. All four dispatch through main handler `FUN_14013fc40` to `FUN_1401419b0`. [N1,N5,S1]

The handler writes these values to `settings+0x62c`: Icon=`2`, List=`1`, Details=`0`, Large Icons=`3`. It updates the native `LVS_TYPEMASK` bits, toggles extended style `0x02000000`, calls `FUN_1401a7e10`, then obtains the selected tree node and runs `FUN_1401a68d0`, followed by main-UI state refresh `FUN_140160520`. The menu path does **not** call `FUN_1401a5990`; it inlines the same setting/style/recalculation work and then explicitly rebuilds the current projection. [N5,N7]

Large Icons has an additional stopping result: when the product capability field at `FUN_1401285c0()+0x120` is zero, the handler opens `CLicenseGuideDlg` in mode `3` and returns before changing the list mode or rebuilding it. Other recognized modes have no confirmation/cancel stage. Like the column handler, this path does not immediately call the settings writer. [N5]

### Tree selection notification → list rebuild → path history

The main window sends `WM_NOTIFY` to `FUN_140156740`. For tree control ID `0x51`, notification code `-451` (`TVN_SELCHANGEDW`) and suppression flag `main+0x1b0 == 0`, it reads the selected tree node with `FUN_1401aa4c0`, calls `FUN_1401a68d0(list, node)`, derives the selected internal path with `FUN_140155d50`, and passes that path to navigation-history owner `FUN_1401acb30`. It returns notification result zero; there is no cancellable selection-change prompt. A nonzero suppression flag skips this rebuild/history branch. [N6,N7,N10]

`FUN_1401a68d0` starts with `LVM_DELETEALLITEMS`, clears its row maps/current-node pointer, inserts the new node's children, sorts, and rebuilds original-catalog-index mapping. This complete tree-change branch does not restore arbitrary prior selected rows or call `SetFocus`. Returning to a folder through the tree or path history therefore restores location only in this inspected chain; it does not restore that folder's prior multi-selection or list HWND focus. [N6,N7]

### Backspace parent: parent selection plus one explicit child-row focus/selection

The accelerator table maps virtual key `0x08` Backspace to command `0x8031`; `FUN_14013fc40` dispatches it to `FUN_1401574b0`. That handler asks the tree for the current caret item (`TVM_GETNEXTITEM`, flag `9`) and then its parent (flag `3`). At root, the null parent makes the command a no-op. Otherwise it saves the old child-node pointer, synchronously selects the parent with `TVM_SELECTITEM`, and then calls `FUN_1401a73c0(list, old_child)`. [N1,N8,N9,S1]

After the tree notification has rebuilt the parent's rows, `FUN_1401a73c0` scans those rows for the saved child-node pointer. On a match it sends `LVM_SETITEMSTATE` with state/mask `3` (`LVIS_SELECTED | LVIS_FOCUSED`) and `LVM_ENSUREVISIBLE`. This is a deliberate single-row result: moving up selects and focuses the folder just left. It is not restoration of that parent's previous multi-selection, and it does not explicitly call `SetFocus` on the list HWND. [N8,N9]

### Alt+Left/Right location history: branch truncation, no row-selection history

The accelerator table maps Alt+Left to `0x8224` and Alt+Right to `0x8223`. Main-dispatch cases `44` and `43` move the current pointer in the path-history linked list only when an adjacent entry exists, copy that entry's path into current-history state, and call `FUN_1401aa610` to select the corresponding tree path. At either end, the command is a no-op. `FUN_1401aa610` selects the deepest component that still exists, so a stale history path can land on its deepest surviving ancestor. The resulting synchronous tree notification performs the list rebuild described above. [N1,N6,N11,S1]

When a tree selection produces a genuinely new path, `FUN_1401acb30` removes every forward entry after the current node before appending the new path and making it current. If the new path equals the current path, or equals the retained tail under its existing condition, it does not append a duplicate. History therefore stores paths and current/forward position, not selected rows or focused list items. [N10]

### F6 start screen and the two distinct recent lists

PE resources map `MENUSTR_VIEW_STARTSCREEN` and F6 (`VK_F6=0x75`) to command `0x811e`. `FUN_14013fc40` toggles `main+0x2b8` through `FUN_14015e630`: state `0` shows the start-screen owner at `main+0x140`, hides the browser owner at `+0x148`, and rebuilds recent-archive presentation with `FUN_14015e7b0`; nonzero state reverses the visible owners. This is a presentation toggle. It does not call `FUN_140157560` and does not close or clear the currently opened archive. By contrast, command `0x8014` is the shipped **Close archive** menu/F4 command and is the direct caller of `FUN_140157560`. [N1,N12,N13,S1]

The start-screen recent archive owner reads `settings+0x660`, not recent folders at `+0x5b8`. `FUN_14015e7b0` binds actionable archive commands `0x8113..0x811b` to retained paths, disables/clears those controls when unavailable, and enables Clear only when it has archive items. Settings load/write retain ten names `RecentArchive0..9`, while the inspected actionable command array contains nine IDs. Resource `BUTTON_RECENT_9`/`0x811d` exists and reaches the same dispatcher case, but `FUN_14015f0d0` does not recognize it because it scans only the nine-ID array; this static chain therefore establishes nine actionable start-screen archive entries, not ten. [N13,N14,N18,N19,S1]

Selecting a recognized recent-archive command validates its index against the currently bound path vector and calls `FUN_1401579a0`. Success returns `1`; the ordinary open path refreshes tree/list and moves the path to the front of `+0x660`. Failure returns `0`. Only when failure is followed by a zero result from the one-second bounded path check does `FUN_14015f0d0` unlink the matching `+0x660` entry and rebuild recent controls; a nonzero result, including timeout `0x102`, keeps the entry. A stale command/index is a no-op. [N14–N17]

Archive Clear command `0x80bb` in the main dispatcher empties `settings+0x660`, immediately calls `FUN_14016a570`, and rebuilds archive recent controls. The recent-archive popup returns zero on dismissal and performs no clear/open. Separately, recent-folder popup `FUN_14014d980` enumerates `settings+0x5b8`, opens a selected folder through `FUN_14014b180`, and intercepts its own `0x80bb` result to clear `+0x5b8` and save settings. The shared command ID and label do not merge the two lists: ownership of the popup decides which container is cleared. [N1,N13,N19,N20]

## Evidence locator

All canonical records below have subject `D:\Tool\Bandizip\Bandizip.exe`, PE x86-64, recorded digest `b6c0eefb56b6d08e627ecea84dbd09bfa265ff7b3a08d35c742b60e747f95324`.

| Ref | Exact canonical filename in `evidence/raw/` | Actual evidence_id | Subject / procedure |
| --- | --- | --- | --- |
| N1 | `bandizip-create-p-0x1401401fb.json` | `ev_62909cf3d746bcd5b408319b252fc9d385facb256bfa349b3e90ce32ef35288e` | `Bandizip.exe`, containing dispatcher `FUN_14013fc40` |
| N2 | `bandizip-nav-follow4-p-0x14014d120.json` | `ev_dedb747b78d4663587de2062c5b8a4697c6b5885eb7456bb09b1c82b0ff8bcb2` | `Bandizip.exe`, column popup `FUN_14014d120` |
| N3 | `bandizip-nav-main-p-0x14014d1f0.json` | `ev_ff7bb2b433a38fd63fec520fe4b63f221721c8018b0fef7d720a97a9a1823383` | `Bandizip.exe`, immediate column toggle/apply `FUN_14014d1f0` |
| N4 | `bandizip-viewcontrols-follow6-procedure_pseudo_code-0x1401a7550.json` | `ev_0eb1030e150a5b23199a8fbbcb724dbfda5e23efb00cae056e11850106ff28d8` | `Bandizip.exe`, column rebuild `FUN_1401a7550` |
| N5 | `bandizip-nav-main-p-0x1401419b0.json` | `ev_2ca362d7f9b9e1f21bafcbcc568b01360a0e52fa1fdb27f572268e7780a9e5e1` | `Bandizip.exe`, view-style handler `FUN_1401419b0` |
| N6 | `bandizip-foldercommit-notify-p-0x140156740.json` | `ev_301bcb75db9ddab2d3345b7f29d934fc787c9ad7f0ae42ea7a0780fd7f1cac35` | `Bandizip.exe`, main browser notification handler `FUN_140156740` |
| N7 | `bandizip-viewcontrols-follow5-procedure_pseudo_code-0x1401a68d0.json` | `ev_05cbd80c558f62471d30a2a92934fe7492e5a2fdbd64bfb698db331e7def6e97` | `Bandizip.exe`, list projection rebuild `FUN_1401a68d0` |
| N8 | `bandizip-nav-main-p-0x1401574b0.json` | `ev_f821af17bfd323f7a2ea995caab86b2a063aac70b7e7bb7a3145fb1fd4d99b40` | `Bandizip.exe`, parent command `FUN_1401574b0` |
| N9 | `bandizip-nav-follow1-p-0x1401a73c0.json` | `ev_e97cc9cc9e9d720ed542335345b9098d23bd65436ef761687d2f657c01fe3464` | `Bandizip.exe`, child-row select/focus `FUN_1401a73c0` |
| N10 | `bandizip-nav-follow3-p-0x1401acb30.json` | `ev_67cf5691c9356b60ba7afaca639c7481af326e046c81d92f4851ddc34a5b0e1a` | `Bandizip.exe`, path-history append/truncate `FUN_1401acb30` |
| N11 | `bandizip-viewcontrols-follow3-procedure_pseudo_code-0x1401aa610.json` | `ev_9889459447e0637bf71ef5157d6b8814145cd4a3a2d7004e79344601021c582a` | `Bandizip.exe`, tree path selection `FUN_1401aa610` |
| N12 | `bandizip-management-view-p-0x14015e630.json` | `ev_175bb857309de5c37c2413af55e8bf4e0050eb80160bd8380d45cb5f6c4760f1` | `Bandizip.exe`, start/browser owner switch `FUN_14015e630` |
| N13 | `bandizip-management-view-p-0x14015e7b0.json` | `ev_758092f73521b6d7cd2c2470ed2332e6cffcdb6fe93c4c59c053945b916f9dae` | `Bandizip.exe`, recent-archive command projection `FUN_14015e7b0` |
| N14 | `bandizip-nav-main-p-0x14015f0d0.json` | `ev_e0356d1d09ad48d7ed8207367e6aae4beebc960481e169b8740f8a4c91a4bee7` | `Bandizip.exe`, recent-archive selection `FUN_14015f0d0` |
| N15 | `bandizip-nav-follow1-p-0x140125470.json` | `ev_6426fee3a59b94087f31e3781a28a64e4670041cb5931089cbc778ba87852a00` | `Bandizip.exe`, one-second bounded path check `FUN_140125470` |
| N16 | `bandizip-nav-follow2-p-0x140125450.json` | `ev_601ca067552f57409f6cc49c6bbdf407f28a5fc16fcd36109a924c2814a06e92` | `Bandizip.exe`, bounded-check worker `FUN_140125450` |
| N17 | `bandizip-viewcontrols-analysis-0x1401579a0.json` | `ev_809dc4a22e243a139fb325197933a7c83b00f97329742f8dd8bc92d138796d73` | `Bandizip.exe`, archive open/recent update/refresh `FUN_1401579a0` |
| N18 | `bandizip-browser-code-from-0x140165b52.json` | `ev_b337340b1014fec23f907058bc13566c3ca7f897618dd5ea619147d55fb9f634` | `Bandizip.exe`, settings load `FUN_140164e40` |
| N19 | `bandizip-browser-code-from-0x14016ae74.json` | `ev_f9fb926fd126cdfa26cdb64777d5f3c89aa2efa8a5ca0940a26c332774e531d2` | `Bandizip.exe`, settings writer `FUN_14016a570` |
| N20 | `bandizip-viewcontrols-follow3-procedure_pseudo_code-0x14014db9a.json` | `ev_0c5799437cd8264f89f5d148f6244b5b3e8b1f400214299a21d5d8da5e66f8a0` | `Bandizip.exe`, recent-folder popup/select/clear `FUN_14014d980` |

**S1.** Static command/resource mapping is retained in `evidence/raw/bandizip-nav-resource-routing.txt`; it is generated by `evidence/raw/bandizip-nav-resource-routing.py` from PE resources and fixed image data and intentionally has no REA Evidence ID.

## Limitations

- The connected paths establish static command routing, state writes, native messages and no-op/cancel branches. They do not establish rendered appearance, actual keyboard focus after Backspace, accessibility behavior or runtime acceptance.
- Backspace has a specific single-child row result; no general per-folder multi-selection/focus cache was found in the complete tree-notification/list-rebuild/history chain. Selection restoration owned by edit completion or Find navigation is separate and does not change this result.
- The Large Icons capability field's product/edition policy was not decoded beyond the concrete `CLicenseGuideDlg` rejection path.
- Settings persist ten recent-archive names while the inspected start-screen actionable array contains nine IDs. The static tenth-button mismatch is established; visible runtime treatment of that dormant resource was not exercised.
- Navigation-history maximum length and UI enabled/disabled painting for Alt+Left/Right were not followed. Boundary behavior is still explicit: missing adjacent nodes produce no action, and a new path after going back truncates forward entries.

## Replay

Static resource routing:

```powershell
python .trellis\tasks\10-31-archive-research\evidence\raw\bandizip-nav-resource-routing.py
```

REA records are replayed with the checked-in helper and the private recipes `evidence/raw/recipes/rea-bandizip-nav-main.json` and `rea-bandizip-nav-follow1.json` through `rea-bandizip-nav-follow4.json`, using `GHIDRA_HEADLESS_MAXMEM=768M`, `REA_GHIDRA_STARTUP_TIMEOUT_MS=900000`, an independent `REA_SESSION_LABEL`, and the retained private `evidence/raw/bandizip-nav-session-snapshot.json`. Every recipe closes its binary session.

