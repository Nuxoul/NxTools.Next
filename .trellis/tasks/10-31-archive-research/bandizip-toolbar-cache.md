# Bandizip 7.32.0.1 — toolbar settings and catalog-size cache

**Boundary.** Read-only inspection of shipped `D:\Tool\Bandizip\Bandizip.exe`, x64 version7.32.0.1, using Ghidra12.1.4 reconstructions and PE resource/fixed-table parsing. This is static implementation evidence, not original source or observed GUI behavior. The EULA§3/applicable-law boundary in `docs/reference-code/archive.md` applies. No target process, archive operation, live configuration, registry, user log, build, package or product code was exercised or changed.

## Toolbar customization applies on each click

The main menu and toolbar context menu expose command `0x8244` (`MENUSTR_SET_TOOLBAR_ITEM`). Main dispatcher `FUN_14013fc40`, case `0x31`, creates owner-modal `CToolbarSettingDlg` with template `0xe7` and retains the archive-window owner at dialog `+0x40`. PE resources contain nine auto-checkboxes `0x407..0x40f` and one `IDOK` button. There is no separate Cancel button in this template. The resolved vtable at `0x1402653a8` has event handler `FUN_14019ffe0` in slot0. [T1,T2,S1]

On `WM_INITDIALOG`, `FUN_1401a0200` localizes the nine actions and calls `FUN_1401a0450`. That function sends `BM_SETCHECK` from the existing settings fields. A checkbox `BN_CLICKED` reaches `FUN_1401a0610`, which reads **all nine** check states with `BM_GETCHECK` into settings `+0x47c..+0x49c`, then the event handler immediately calls `FUN_140161240` on the retained archive-window owner. There is no pending copy waiting for OK. [T2–T5]

| Control | Action | Settings field |
| --- | --- | --- |
| `0x407` | Open | `+0x47c` |
| `0x408` | Extract | `+0x480` |
| `0x409` | New archive | `+0x484` |
| `0x40a` | Add | `+0x488` |
| `0x40b` | Delete | `+0x48c` |
| `0x40c` | Test | `+0x490` |
| `0x40d` | Scan | `+0x494` |
| `0x40e` | Column popup | `+0x498` |
| `0x40f` | Code-page popup | `+0x49c` |

`IDOK` and the handler's `IDCANCEL` branch merely end the dialog with results1 and2 respectively; neither restores a settings snapshot. After `DialogBoxParamW` returns, the main caller invokes settings writer `FUN_14016a570` **without testing the modal result**. Changes already applied by checkbox clicks therefore survive either terminal handler result and enter the ordinary settings write. This is immediate customization with a close boundary, not transactional OK/Cancel editing. [T1,T2,T9]

## Toolbar style and visibility are separate

Main commands map `0x812e` Normal→style0, `0x812f` Mini→1, `0x814d` Old/classic→2 and `0x8130` Hide→3 in settings `+0x624`. They call `FUN_1401609d0`, which selects visibility of `STATIC_TOOBAR_NORMAL`, `STATIC_TOOBAR_MINI`, `STATIC_TOOBAR_OLD` and `STATIC_TOOBAR_HIDE`, then calls `FUN_140161240`. [T1,T7,S1]

`FUN_140161240` selects the matching normal/mini/classic resource-key table and applies the nine action flags. Open and Extract each have a main and dropdown resource bound to the same respective action flag; the other actions have their own resources. The classic table has no Scan entry even though its preference remains one of the nine settings. Hide is a separate parent presentation state, not a fourth button design or clearing of those preferences. The [navigation report](bandizip-browser-navigation.md) retains the already connected column/code-page/view command owners. [T6,T7]

## Folder-size cache belongs to the current catalog tree

`bShowFolderSize` at settings `+0x30c` gates `FUN_1401a8630`'s detail-row size display. With the setting enabled and a folder node present, it calls `FUN_1401470e0`. Regular nodes add nonnegative retained member sizes/counts. A directory with null node `+0x40` allocates a zeroed aggregate, recursively visits its children and stores totals there; later queries reuse those totals. It does not recursively stat files on disk, start a background job or establish a cancellable/deadline-bounded calculation. [C1,C2]

The cache lifetime is connected at the model boundary, not inferred from a ListView repaint:

1. Singleton `CArchiveMan` from `FUN_1400ef2a0` embeds its root node at manager `+0x30`. Node constructor `FUN_1400eed60` sets the node marker `0x20210104`, child containers and member/parent fields, including cache pointer `+0x40=0`. [C3,C4]
2. Open worker `FUN_1401b2100` calls `FUN_1400ef810` after the reader-open attempt. The builder checks that the reader is usable, releases its old index array, and calls `FUN_1400eee90(manager+0x30)` **before** enumerating the current catalog. It builds the path tree through `FUN_1400f0590 → FUN_1400f0bd0`. The insertion function allocates each missing node as `0x48` bytes, calls `FUN_1400eed60`, then binds name, member metadata and parent. Existing sibling lookup is reuse within this newly constructed tree. [C5–C8]
3. `FUN_1400eee90` recursively cleans child nodes, releases their containers and name storage, frees each node's `+0x40` aggregate as `0x20` bytes and sets the cache pointer to zero. Its repeated recursive cleanup calls see the already-cleared fields; they do not preserve an aggregate for later catalogs. [C9]
4. Ordinary archive Close `FUN_140157560` clears list/tree HWND rows and maps before calling the same model cleanup on manager `+0x30`. Open coordinator `FUN_140157cc0` calls this Close path before starting its new reader attempt. Failed or canceled Open therefore cannot carry an old directory aggregate into the next admitted catalog. [C10,C11]
5. Successful Open refresh `FUN_1401584c0 → FUN_1401aa280` binds the TreeView to that rebuilt root and selects it; the normal tree-notification path rebuilds the list. Edit publication's already connected `FUN_140154650 → FUN_1401579a0` reopen uses this same close/open/model-rebuild path. Rename, deletion, directory insertion and changed-file replacement do not rely on an old cached total surviving a repaint. [C12,C13,C14]

Ordinary navigation within one retained tree can reuse aggregates. Reopen and successful edit publication obtain a new tree/cache state; there is no separately versioned on-disk folder-size cache in this chain. The measured latency of the first synchronous aggregate over a large catalog remains unobserved.

## Evidence locator

All canonical files below are under `evidence/raw/` and have subject `D:\Tool\Bandizip\Bandizip.exe`, recorded SHA-256 `b6c0eefb56b6d08e627ecea84dbd09bfa265ff7b3a08d35c742b60e747f95324`. IDs are copied from the retained records, not derived from addresses.

| Ref | Canonical filename | Actual evidence_id | Boundary |
| --- | --- | --- | --- |
| T1 | `bandizip-create-p-0x1401401fb.json` | `ev_62909cf3d746bcd5b408319b252fc9d385facb256bfa349b3e90ce32ef35288e` | Main command, modal dialog, unconditional save and styles |
| T2 | `bandizip-present-p-0x14019ffe0.json` | `ev_84aade73b9a27295c4798a7f0c3a81f0654d34653c9e9e8dfaf8a420823938c4` | Dialog message/checkbox/close handler |
| T3 | `bandizip-present-p-0x1401a0200.json` | `ev_46f7f683516821c9bba3452c74b623ce82153421b4d3102dcd340f3a33da57f7` | Initialization/localization |
| T4 | `bandizip-present-p-0x1401a0450.json` | `ev_c8019de524c860a9d8ab53ef7afb1cb001b4e6f8ecde600cc8bfc638c455c581` | Settings to checkboxes |
| T5 | `bandizip-present-p-0x1401a0610.json` | `ev_af30431b74ba7898f3974cd2ab68d8acb4c835e022ecb4fd0adb19e509f5b386` | Checkboxes to settings |
| T6 | `bandizip-viewcontrols-follow2-procedure_pseudo_code-0x140161357.json` | `ev_045d8617f6397b7d8e9deb26e6d305a2f26d7535d6c6ccf662ebffe2f85acbed` | Button resources and action flags |
| T7 | `bandizip-present-p-0x1401609d0.json` | `ev_66806ab602dcdf336b570c4cf31c1e7d8b71eff55599fe5f688eb8c0381a3c1b` | Parent style visibility and rebuild |
| T8 | `bandizip-present-name-toolbar-vftable.json` | `ev_0392284c23ffc804359f7266164cd0995aceb521b843b54ed86451accc8f83b4` | Exact dialog vtable identity |
| T9 | `bandizip-browser-code-from-0x14016ae74.json` | `ev_f9fb926fd126cdfa26cdb64777d5f3c89aa2efa8a5ca0940a26c332774e531d2` | Durable settings writer |
| C1 | `bandizip-viewcontrols-detail-procedure_pseudo_code-0x1401a8630.json` | `ev_524744a381d3fdc5ed7d9dbde2c20b45ddb417fa49ca06507677bf3a9f25b64a` | Size display gate |
| C2 | `bandizip-viewcontrols-foldersize-procedure_pseudo_code-0x1401470e0.json` | `ev_855985b131d014a94973cd09313c30253905f91613e8bc493aa4641e1ed98810` | Lazy catalog aggregation |
| C3 | `bandizip-present-p-0x1400ef2a0.json` | `ev_71a45747e00e00c61662e2133ea78814c67b027ff9d0aa648bfc7c2dd6e5d434` | Manager/root ownership |
| C4 | `bandizip-present-p-0x1400eed60.json` | `ev_d1fb05109390bb54ad5a2403f413765f43d201b6ccae87a8cffa631e89b7aec1` | Node initialization |
| C5 | `bandizip-create-p-0x1401b2100.json` | `ev_7e63ea66fc75a2a8e1d33c389036fac91b3040187c5435f2fccbe8b69a96313f` | Open worker to tree builder |
| C6 | `bandizip-present-a-0x1400ef810.json` | `ev_fba503732b7ab4721ed51a206e5d004100a2c9f47da3c06281202c40cdc1a7b0` | Clear then rebuild from current catalog |
| C7 | `bandizip-present-p-0x1400f0590.json` | `ev_f34c777b2421b02792608e898ed6e4137c94415c1405004c9cd874c0aee408b7` | Path traversal to insertion |
| C8 | `bandizip-present-p-0x1400f0bd0.json` | `ev_ca57f0b77d722e4fa5db10b4a70085c44c2a9b65074df44783a7bbf03bea667e` | Missing-node allocation/constructor/binding |
| C9 | `bandizip-present-a-0x1400eee90.json` | `ev_ce30b0f0975fb014640a5a6b8b8a89865a83a4dd1d6d06fc4363e308c9ab0a87` | Recursive release and cache clear |
| C10 | `bandizip-viewcontrols-follow2-procedure_pseudo_code-0x140157560.json` | `ev_ed2bc6cb84e03d0d6bee845eb83802601e4f363d3814ce08e901e319cdc9dbf7` | Browser Close/model cleanup |
| C11 | `bandizip-create-p-0x140157cc0.json` | `ev_fd828c460cd299f6931bf2da0cc4fd23035aefb5e9d25ae67394473bed03715d` | Close before a new reader attempt |
| C12 | `bandizip-management-setting-p-0x1401584c0.json` | `ev_6c94dd2d85d404b0896bb1dd66c66295ceffb904ba82c285772867ef3bb0f486` | Successful-open projection |
| C13 | `bandizip-present-p-0x1401aa280.json` | `ev_d7781d843ce4f225f67299a14ba0ebda059482a6853ea4b6df4162388954176d` | TreeView bound to rebuilt root |
| C14 | `bandizip-browser-code-from-0x140154b1f.json` | `ev_23defda9d8520169930e8fdcdf50b333d77edf140f471e6aa2c306ac3dd6e915` | Edit publication/reopen owner |

**S1.** `evidence/raw/bandizip-present-static-resources.txt` retains the menu commands, full template0xe7 controls and four vtable pointers. It is generated by `evidence/raw/bandizip-present-static-resources.py` from the PE and intentionally has no REA Evidence ID.

## Limits and replay

The connected paths establish static field writes, native messages, cache initialization/release and reopen invalidation. They do not establish rendered toolbar appearance, actual settings-write success, large-catalog latency, focus/DPI/accessibility behavior or crash recovery. The absence of a Cancel button and immediate checkbox commit are resource/handler facts, not a user-observed usability judgment. No stale-cache defect is established in the inspected close/open/edit-reopen path.

Run `python .trellis/tasks/10-31-archive-research/evidence/raw/bandizip-present-static-resources.py` for resource replay. REA recipes are `evidence/raw/recipes/rea-bandizip-present-{entry,owners,state,model,build,nodes,insert}.json`, with independent `bandizip-present-session-snapshot.json`, `GHIDRA_HEADLESS_MAXMEM=768M`, `REA_GHIDRA_STARTUP_TIMEOUT_MS=900000` and a direction-specific session label. Every recipe closes its binary session; the entry recipe seeds from the matching existing navigation snapshot.
