# Archive manager reference note (Bandizip / Archiver / SupaZip)

Status: **Phase A reference**; [task](../../.trellis/tasks/10-16-archive-manager/prd.md) owns scope/evidence.
Follow [workflow](README.md); source inspection is not runtime acceptance.
[Conflicts](../../.trellis/tasks/10-16-archive-manager/research/stream-conflict-sources.md).

## 1. Evidence sources and legal boundary

| Source | Location | What was inspected | Boundary |
| --- | --- | --- | --- |
| Bandizip 7.32.0.1 (x64) | `D:\Tool\Bandizip` | Initial Phase A: `langs\SimpChinese.lang` (all1201 lines), `bz.exe` help, versions, config, icons, extension strings and EULA. Selected-entry follow-up: exact-member ZIP extraction plus bounded REA/Ghidra12.1.4 native drag, extraction-coordinator, directory-picker and factory-loader dossiers (§1.1). | EULA§3 restricts reverse engineering except where applicable law permits. Native pseudocode is reconstructed artifact evidence, not original source; no proprietary code reuse is proposed. Resource inventory does not verify every feature or edition. |
| Archiver (MIT, Ke Sheng Da) | `D:\Code\Archiver-main` | `core/src/lib.rs`, `gui/src/{main,view,shell,args}.rs`, `gui/ui/app.slint`, `installer/archiver.iss`, `bench/` | MIT: code may be adapted with attribution. Its `unrar` dependency carries the UnRAR license (extract only). GUI is Slint, not reusable. |
| SupaZip (MIT OR Apache-2.0) | `D:\Code\supazip-master` | `supazip-core/src/{traits,error}.rs`, `formats/*.rs`, GUI and CLI sources, ROADMAP/DESIGN/plan | Its docs claim more than the code does; only source is evidence. GUI is egui, not reusable. |
| User screenshot | [retained layout reference](../../.trellis/tasks/10-16-archive-manager/assets/bandizip-layout-reference.png) | Bandizip Professional title, `main (5).zip` open, tree/list/comment/status | User-supplied shape reference, not an app-running or license-state check. Original image SHA256 `da5f63eacb75b9ce508d39671e2d4d4180d33d5fd40958b740634545d5ab9488`. |

Legacy follow-up: `git -C D:\Code\NxTools grep -n -I -E 'ZipArchive|SevenZip|SharpCompress|压缩|解压' -- '*.cs' '*.xaml' '*.csproj'` found package/FFmpeg ZIP and image/video compression, not a general archive browser. This covers tracked matching sources only, not all untracked/generated files. `Services/BrowserExtensionPackageService.cs` and `Services/FfmpegInstallerService.cs` are unrelated packaging/install owners; image/video compression is not archive behavior. No old archive-window/native lifecycle was identified to migrate.

2026-10-28 feedback seam verification: `ArchiveOutputSession.BeginFile/Partial/CommitJournal` retains native committed transaction parent handles, and `TrackDirectory/CreateExclusiveDirectory` retains actual Name/Smart collision-directory handles. `ArchiveWorkerRunner.RunAsync` disposes resources before freezing terminal journal; result-location facts must therefore be captured before that disposal, without changing cleanup/publication semantics. `ArchiveJobSession.OnRunnerEvent` is the typed terminal bridge. Desktop `jobs.rs::job_terminal` currently opens request.destination (base only) and discards Shell completion; `ClipboardShellTarget.LaunchCore` ordinary open is Process.Start, whereas `ExplorerShellActivation` owns bounded target/foreground readback. No legacy NxTools general archive owner was found (see original search above). A missing result location is unknown, not authority to guess the base/cache/external drop directory.
Preview default-on integration additionally verified `browser_preview.rs::sync_preview_selection` -> `entry.rs::start_preview` used generic current-connection command enqueue although browser drag already retained original read connection. New passive requests validate that same origin atomically under the original IPC router lock; no new queue/reservation. `browser.rs::suspend_tab` only suspended gestures/marquee and did not invalidate preview tokens/content. Hide retires the local token immediately and defers original cancel; next visible render reseeds the remembered preview preference. Explicit-open lease tests match exact Release amid legitimate passive-preview control messages, not assume an otherwise empty queue.

Contextual HZ5 continuation: `application.rs::consume` retires each read after its terminal and owns the top-level source/generation; `jobs.rs::JobRecord` retains original request paths/destination plus attempt and actual cleanup terminal. `launch_entry.rs` previously opened an empty continuation form from the home page. The object-scoped flow uses the same `OperationForm`, `ArchiveBrowser` and `JobProgressView`, with correlated read-only recovery inspection owned by `ArchiveApplication`; fresh Continue still enters the original Job Start queue. Native header stores target identity, not a target path, so unidentified targets require selecting and identity-checking the original folder. Browser refresh/close/cancel/disconnect retire pending discovery; view callbacks defer cross-entity writes. No legacy general archive owner or independent native show/hide boundary is introduced.
Bandizip is the **interaction reference**, not a pixel or copy source.

### 1.1 Selected-entry runtime and native follow-up

The [research record](../../.trellis/tasks/10-31-archive-research/research.md) retains the supplied ZIP facts, production-boundary reproductions, timings, REA Evidence IDs and repair boundaries. REA uses `D:\Tool\ghidra_12.1.4_PUBLIC`; this follow-up is distinct from the initial resource inventory.

| Boundary | Verified entries / invariant |
| --- | --- |
| Drag | `FUN_1401130a0` / `FUN_140147200` → `FUN_140053460` → `DoDragDrop` at `0x1400535bc`; data object uses `CF_HDROP`/`TYMED_HGLOBAL` and balances data/drop-source references. The main path retains selected items and separates ordinary/fast drag. Static evidence does not select the sample's GUI branch. |
| Selected extraction | Successful fast-drop branch → `FUN_1401481d0` → `FUN_14013ce80`; selection indices and destination enter the common operation coordinator. Engine metadata calls remain indirect; the proprietary payload engine's implementation is not recovered by this chain. |
| Directory Browse | `FUN_1400fa3d0` (`TEXT_SELECT_DEST_FOLDER`) → `FUN_140024f10` → `SHBrowseForFolderW` at `0x140024ff0`. Owner HWND and current directory are passed; callback `FUN_140024ee0` seeds initial selection; only a nonempty result updates the destination. |
| Engine factory | `FUN_1400ee3e0` loads the module and resolves `CreateArk`/`CreateArkCompressor`, with explicit module/export/factory failure results. |
| Dual progress | `CExtractDlg` constructor `FUN_1401170c0` → initialization `FUN_140117a10` retains two HWNDs (`0x3ee`/`0x3f5`) and sets each range to1000. Timer100ms → `FUN_140118c90` → `FUN_140119a40` samples two distinct percentage fields under the existing critical section, updates both with `PBM_SETPOS`, and resets member elapsed when the displayed filename changes. Aggregate percentage also feeds taskbar/tray. REA pseudocode evidence `ev_f99bf996bcb344013f46d9ae578d82f75db6d5dfc4de4d8c0564e5f8a4cc5a1d`; static artifact evidence, not a live GUI measurement. |

For `companion\dist\bcprov-jdk18on-1.78.1.jar` in the supplied 137,737,166-byte ZIP, `bz.exe x` selected-member extraction took110.731ms including startup. A separate NxTools owned-test-package/Debug run measured browser10,195.0772ms, materialize24,536.2446ms, and ordinary selected extraction30,130.4285ms ending `timeout` after publishing one8,324,412-byte file. Both extracted outputs compare byte-for-byte with the decoded member. These are separate operations, not a reproduction of one production UI operation exceeding60s.

Next source findings: metadata preserves literal ZIP backslashes while Desktop tree/drag split only on `/`; ordinary extraction accepts both. Contextual Extract captures source/selection but always renders the source chooser. Picker submission uses an uncorrelated envelope that the C# parser rejects before the native dialog, and Desktop does not settle picker pending state on the rejection's `CommandResult`. Whole-archive revision scans, including a per-selected-file scan, and synchronous JSON chunk output are confirmed costs; their individual runtime shares remain unmeasured. Preserve ordinal/source/output binding, native ownership, CRC and actual cleanup while repairing these boundaries.
Drag feedback correction: `browser_drag.rs::drag_move/drag_result` writes `DragState.message`, and `browser.rs::render` appends `archive-drag-feedback` as a banner above the list. Transient gesture errors must instead use the existing deferred NotificationCenter ingress; persistent archive read/stale/recovery state remains in the page. Current progress wire carries aggregate bytes and a member name but omits member bytes/size. The existing shared output boundary has both facts; preserve100ms snapshot coalescing and original generation/terminal authority when adding current-file plus overall Progress controls.

Bandizip native facts above are static artifact observations; real directory-dialog focus/cancellation, external OLE behavior and proprietary engine virtual-call targets remain unverified. No legacy NxTools general archive-window lifecycle has been identified.

## 2. Bandizip feature inventory (from public resources)

Section names refer to `SimpChinese.lang`. This inventory is not an approved scope decision.
Approved required/optional/excluded delivery and exact capability gates live in the task PRD/design; old MVP/P2 recommendations are superseded.

### 2.1 Formats

- Format strings found in `ark.x64.dll`: 7z ace alz apk arj br bz2 cab deb dmg egg gz img ipa iso jar lha lz lz4 lzh lzma msi nsis rar rar5 sit tar tbz tbz2 tgz txz tzst udf war wim xar xpi xz z zip zipx zpaq zst zstd.
- **Evidence limitation:** extension strings and icon names are candidates, not verified list/extract/create support. The initial inventory did not exercise format fixtures. The §1.1 follow-up verifies one exact ZIP member through `bz.exe`; other formats, GUI paths and edition-specific availability remain unverified.
- File-association icon set (`icons\default`): 001 7z alz bz2 cab egg gz iso lha lzh rar tar tbz
  tbz2 tgz xz zip zipx zpaq.
- `bz.exe -fmt:` create formats: zip, zipx, exe(SFX), tar, tgz, lzh, iso, 7z, gz, xz.
  Shell "压缩为" menu adds zpaq.
- Edit (add/delete/rename) is ZIP only, and is refused for split, SFX or broken archives
  (`TEXT_ERR_EDIT_ZIPFILE_ONLY_SUPPORTED`, `…SINGLEFILE…`, `…SFX…`, `…BROKENARCH…`).
- gz/xz cannot hold a folder or several files. The user is told to tick "compress each item
  separately" instead (`TEXT_MSG_GX_XZ_CANT_ARCHIVE_*`).

### 2.2 Archive window (`[MENU]`, `[SKIN]`, screenshot)

**Menus:**

| Menu | Items |
| --- | --- |
| 文件 | Open Ctrl+O; recent files; close F4; new archive Ctrl+N; new window (normal or admin); save as; delete archive (Recycle Bin) Ctrl+Del; delete permanently Shift+Ctrl+Del; open archive location; test Ctrl+T; malware scan Ctrl+I; extract Ctrl+E; exit |
| 编辑 | Rename F2; add Ins; delete Del; archive comment; settings F5; password manager; find Ctrl+F; copy files to clipboard Ctrl+C; paste files Ctrl+V; copy full paths |
| 视图 | Start screen F6; folder tree F7; comment pane F8; image preview; status bar; toolbar (large, small, classic, hidden, button picker); list style (icon, list, details, large icons); column settings; always on top; colour settings |
| 工具 | Repair (ZIP only); password recovery |

**List context menu:** extract all / extract selected / extract here; open (Enter); open with
associated app; edit F3; open with Bandizip (nested archive); new folder; add; delete; rename;
select all Ctrl+A; parent folder Backspace.

**Tree context menu:** delete archive; open containing folder.

**Extract split-button menu:** smart extract Ctrl+Shift+S; extract here Ctrl+Shift+C; extract to
"archive name" folder; extract to desktop; smart extract to desktop.

**Code page menu:** default; Simplified Chinese; Traditional Chinese; Japanese; Korean; 1252; 1250;
1251; 1254; 737; Arabic; Hebrew; Baltic; Thai; UTF-8; auto-detect.

**Columns (`TEXT_COLUMN_*`):** name, packed size, original size, type, modified, method,
encryption, CRC, attributes, comment, extra, path.

**Toolbar (screenshot, left to right):** 打开▾, 解压▾, 新建, 添加, 删除, 测试, 扫描, 查看 (column chooser).

**Layout (screenshot):**
- menu bar, then large icon-over-label toolbar
- left folder tree whose root is the archive file
- right details list with a `..` parent row
- an optional comment pane under the list, showing the archive comment (a 40-hex git hash in the screenshot)
- a status bar with view-style toggles and `文件: 50, 文件夹: 15, 压缩文件大小: 176 KB`

### 2.3 Dialogs

| Dialog id | Purpose | Controls |
| --- | --- | --- |
| IDD_SELECT_FOLDER | Choose extract path | Destination path; all files or selected files; extract into an archive-named subfolder; delete archive after extract; remember settings; new folder |
| IDD_ARCHIVE | New archive | File list (add, delete, total size); file name + browse; destination folder; format; split size (none, 3/4/5/10/20 MB, 25 MB Gmail, 100/300 MB, 650/700 MB CD, 4 GB FAT32, 4.7 GB DVD, custom); level (0 store, 1 fast, 2 normal, 3 maximum); set password; test after compress; delete source after compress; each item to its own archive; more options; SFX settings; start |
| IDD_ARCHIVE_ADVANCED | Per-format advanced | **ZIP:** method, encryption algorithm, CPU threads (auto), store symlinks, file comment. **7Z:** method, solid block size (non-solid or solid), encrypt file names. **TAR:** store symlinks, keep hard links |
| IDD_PASSWORD_INPUT | Password | Password, re-enter (create mode), show password, password manager, apply to all, wrong-password message |
| IDD_ASK_DUPEFILE | Name conflict | Existing file vs incoming file (size/date); skip, overwrite, or rename (shows the new name); apply to all |
| IDD_ASK_DROPFILE | Files dropped on an open archive | Open as archive, new archive, or add to current archive; password; delete source after |
| IDD_ARCHIVING / IDD_EXTRACT | Progress | Pause, stop, close; "don't close when finished"; open destination; minimise to tray; low priority; delete archive after extract |
| IDD_EDIT_COMMENT | ZIP comment | Text; save as UTF-8; load from .txt |
| IDD_ASK_EXECUTABLE | Opening an executable from an archive | Warning; don't ask again |
| IDD_FIND | Find in archive | Query; extract or delete matches |
| IDD_PASSMAN | Password manager | List of (password, display name); add, edit, delete; master password |

### 2.4 Settings pages (`[IDD_SETTING_*]`, tabs: 常规 文件关联 上下文菜单 解压 压缩 查看 安全 高级 语言)

**General:**
- Esc exits
- test archive after add/delete/rename
- show `[..]`
- grid lines
- full-row select
- single-click opens
- window snapping
- update channel (X)

**Association:** per-extension list with groups (archives, UNIX, other, CD/DVD); basic selection; select all; clear; apply now; icon pack; Windows default-apps page.

**Context menu:**
- Double-click action: open with Bandizip, smart extract, show context menu, smart extract to a folder, or extract to "name" folder. Option to delete the archive after a double-click extract.
- Extract verbs: here, smart, to "name", each of those plus delete, choose path, as admin, test, open, preview, preview limit, tree preview.
- Compress verbs: to name.zip, plus delete, plus email, add to, zipx, exe, tar, tgz, lzh, iso, 7z, gz, xz, zpaq, each item to its own zip or 7z, the dialog, as admin, custom name rule (`${NAME}` `%Y %m %d %H %M %S`).
- Explorer background menu: new folder, cmd (X).
- Other: disable the menu; cascade it; hide the icon; detect archives by content; show the compress menu even on archives; use the clicked item's name when several are selected; drag-and-drop into a zip in Explorer.

**Extract:**
- enter the single root folder automatically
- open the folder after extracting
- keep the progress window open
- fast drag-drop for large files
- extract TGZ/TBZ in one pass, even big ones
- smart extract puts a single file into a new folder
- macOS NFD→NFC
- Unix zip uses UTF-8
- parallel extract
- default code page
- types to "extract all then run" on double-click

**Compress:**
- don't store the root folder for a single folder
- UTF-8 names in ZIP; UTF-8 extra field; UTF-8 in tar
- force ZIP64
- exclude Thumbs.db
- NTFS timestamps in ZIP
- "fast compress" (skip incompressible files)
- keep the progress window open
- default level
- test after compressing
- new-archive folder: last used, or a fixed folder
- exclude filter

**View:**
- image preview without extracting
- show folder sizes
- fonts (X)
- default columns
- image viewer choice (X)
- external editor path
- colours (X)

**Security:** copy Zone.Identifier from downloaded archives; AMSI scan before extract.

**Advanced:**
- export settings
- temp folder
- extra extensions treated as archives
- delete temp files when the child process exits
- delete to Recycle Bin
- recent file history
- long paths over 260 characters
- allow Explorer to delete an open archive (FILE_SHARE_DELETE)
- auto-expand the tree
- modal operations (X)
- show the start screen
- full path in the title bar
- delete extracted output on cancel
- preallocate output (SetEndOfFile)
- compress files locked by other processes (backup semantics)

### 2.5 Behaviors and copy worth matching

**Smart extract (`-target:auto`):** the console help names automatic target choice, and the single-file-folder setting proves a selectable special case. The commonly expected one-top-level-item vs multiple-item rule below is a **proposed Next behavior**, not a runtime-verified Bandizip algorithm: preserve one directory directly, place one file directly unless configured otherwise, and put several top-level items in an archive-stem directory. `-target:name` explicitly selects an archive-name folder.

**Overwrite policy (CLI):** `-aoa` overwrite all, `-aos` skip, `-aou` auto-rename to `name (2).txt`.
The GUI prompts per file, with "apply to all".

**Pre-extract checks:** missing destination prompts to create it
(`TEXT_ASK_MAKE_FOLDER`). No write permission prompts to continue or elevate. Low free space
shows destination, free space and expanded size (`TEXT_ASK_DEST_FOLDER_SPACE_NOT_ENOUGH`).

**Delete source after compress:** if "test" is off, Bandizip asks to turn it on, because deleting
the source without verification can lose data (`TEXT_ASK_DELSRC_TURN_ON_TEST_ARCHIVE`).

**Opening an entry:** resource text exposes temp extraction and a modified-file write-back prompt (`TEXT_ASK_APPLY_MODIFIED_FILE`), plus an executable warning. This remains resource-level evidence for opening/editing. The selected-entry follow-up in §1.1 inspects native drag/extraction coordination; executable-open temp lifetime and modification-watcher behavior remain unverified.

**Volumes:** the user is told to open the first volume, or offered to jump to it
(`TEXT_ASK_OPENERR_FIRST_VOLUME`). Missing `.z01` volumes are reported.

**Mojibake hint:** suggests the code-page menu, then auto-detect (`TEXT_NOTICE_AUTODETECTCODEPAGE`).

**Typed error vocabulary (`TEXT_ARKERR_*`):** wrong password, password required, CRC error,
truncated or corrupt, needs next volume, unsupported method, unsupported encryption, disk full,
access denied (with UAC variant), path longer than 260, cannot create a symlink (needs admin),
SFX over 4 GB, out of memory (lower the level), file locked by another process.

## 3. Open-source engine findings

### 3.1 Archiver (`core/src/lib.rs`, MIT)

**Dependencies:**

| Crate | Version / features |
| --- | --- |
| `zip` | 9.0.1 (aes-crypto, bzip2, deflate-flate2-zlib-rs, deflate64, lzma, ppmd, xz, zstd) |
| `sevenz-rust2` | 0.23.0 (compress, aes256) |
| `unrar` | 0.5.8 (bundled C++; needs `advapi32` link fix) |
| `tar` | 0.4.46 |
| `flate2` | 1.1.10 |
| `bzip2` | 0.6.1 |
| `liblzma` | 0.4.8 (static, parallel) |
| `zstd` | 0.14.1 (zstdmt) |
| other | `walkdir` |

**Formats:**
- Read and write: ZIP (AES-256 write; ZipCrypto and AES read); 7z (LZMA2, one solid block, AES-256 plus header encryption when a password is set); tar, tgz, tbz2, txz, tzst; single-file gz, bz2, xz, zst.
- RAR is read-only. No explicit volume workflow or SFX support was found; underlying library volume handling was not proven.
- Detection is by extension only.

**API shape:**
- Free functions `list`, `extract(selected)`, `test`, `delete`, `create(CreateOptions{format, level, password})`.
- `Progress.cancel`/`ProgReader` checks after reads at EOF/1MiB(lib.rs152–165,417–442); blocked I/O is not interrupted. Next [cancel boundaries](../../.trellis/tasks/10-16-archive-manager/worker-cancel-contract.md).
- `Entry { name, size, compressed, modified: String, is_dir, crc32, encrypted }`.

**Patterns to adapt:**
- `view::archive_children(entries, prefix)` for pure virtual-folder derivation (`gui/src/view.rs:31`).
- `safe_join` with tests (`lib.rs:203,1288`).
- ZIP delete through `raw_copy_file` into `<archive>.rewrite.tmp` and rename.
- Remove the partial output on create failure or cancel.
- Parallel ZIP: per-thread temp ZIPs joined with `merge_archive`.
- Multi-member parallel gzip; xz `memlimit_threading`.
- Pure-function HKCU registry layout with tests (`gui/src/shell.rs:63`).
- Bench data generator (`bench/gen_data.py`).

**Pitfalls (do not copy):**
- Password-needed is detected by matching error strings.
- Timestamps are pre-formatted strings.
- 7z `encrypted` is always false.
- Extraction overwrites silently and leaves partial files after cancel.
- No code-page handling.
- Listing a single-file `.gz` decompresses the whole file.
- 7z is always one solid block.
- Archiver's checked-in benchmark reports (not rerun here) show about1.2–1.4GB peak RAM for multi-threaded7z/xz on its stated benchmark machine.
- Classic shell verbs start one process per selected file and sit under "Show more options" on Windows 11.

### 3.2 SupaZip (`supazip-core`, MIT OR Apache-2.0)

**Worth adapting:**
- A small typed error: `ArchiverError { Io, InvalidArchive, PasswordRequired, WrongPassword, Cancelled, UnsupportedFormat, TooLarge }` (`error.rs:12-41`).
- [B3 readmission](../../.trellis/tasks/10-16-archive-manager/research/zip-output-readmission-sources.md).
- Compound-extension detection in `detect_format` (`formats/mod.rs:20-78`).
- `safe_join` that rejects Prefix and RootDir (`formats/sevenz.rs:141-162`).
- Password: [B2 seam](../../.trellis/tasks/10-16-archive-manager/research/extract-password-sources.md).

**Pitfalls:**
- ZIP and 7z copy the whole archive into memory.
- Errors are matched by string.
- Creation is non-recursive and loses folder structure; ZIP can store absolute `C:/` paths.
- A fake Brotli-in-ZIP format.
- The per-entry limit actually counts the cumulative total.
- No encoding, overwrite or smart-extract support.
- `sevenz-rust` 0.6 cannot do solid archives or header encryption.

## 4. NxTools.Next mapping (Rust / C# / GPUI)

| Concern | Owner | Notes |
| --- | --- | --- |
| Formats, listing, extract/test/create/modify, encoding, path safety, limits, progress/cancel | Rust: new `rust/crates/nxtools-archive` | No GPUI, no Windows API. Streams from `File` (`Read + Seek`), never whole-archive buffers. |
| Job queue, browser/dialog state, settings projection | Rust desktop: `ArchiveApplication` entity | Same pattern as `DownloadApplication`. One bounded job manager. |
| Rendering | GPUI Kit 0.7.1 | Component record in task `design.md` §6. |
| Pickers, Shell open/reveal, multi-file OLE drag-out, Recycle Bin, file association, Explorer menu registration, ANSI code page, taskbar progress | C# PlatformHost through typed `PlatformCommand`s | Extends the existing `PickDirectory`, `OpenShellTarget`, `BeginFileDrag` and `SetTaskbarProgress`. |
| Second-instance intents (`--archive-open`, `--archive-extract`, `--archive-compress`) | Desktop launcher → Rust-owned local IPC server → running desktop | Existing `runtime.rs#summon_running_desktop` sends visibility/activation via Rust `nxtools-ipc::handle_summon_connection`; C# is the native pipe client, not the archive-intent router. A typed bounded Rust intent route is new work. |

Existing in-repo ZIP code that must **not** be repurposed as the archive engine:
- `rust/crates/nxtools-extensions/src/archive.rs` is the extension-package validator. It keeps its own Stored/Deflated allowlist and package budgets.
- `rust/crates/nxtools-clipboard/src/archive.rs` handles clipboard export/import.

Both show workspace `zip` usage. Manifest caret requirement `zip = 8.3.0` (lock8.6.0, feature
`deflate-flate2-zlib-rs` only). `ZipFile::name_raw`, `enclosed_name`, `encrypted`, `is_symlink`,
`ZipArchive::by_index_decrypt` and `ZipWriter::{raw_copy_file, merge_archive, set_comment,
new_append, abort_file}` exist in the 8.6.0 sources (`read.rs:808,849,981,1005`,
`zip_archive.rs:521`, `write.rs:621,828,1034,1468,1563,1691`).

## 5. Open questions (unverified)

- **Classic verbs, `MultiSelectModel=Player`:** whether it delivers every selected item to one
  process on Windows 10/11 is **unverified**. The design must coalesce per-item launches either way.
- **Volume support:** exact sevenz-rust2 0.23.0 generic Read+Seek/Write+Seek and solid/header APIs have since been inspected; project discovery/missing-part/segmented global I/O remains required Phase E, not delivered by the current small single-volume fixtures.
- **RAR crate:** exact unrar_sys0.5.8 bundled UnRAR7.1 C++ Windows build and narrow advapi32 linkage passed. Independent RAR4/5/encrypted open/test/output fixtures passed; broad memory/performance/volume acceptance remains open.
- **Modern Explorer menu:** a Windows 11 top-level menu (`IExplorerCommand` plus a sparse package)
  is excluded from the approved first release; classic integration and genuine drag are required Phase F work.

## 6. Implementation source verification (Phase A)

- Locked `zip-8.6.0/src/read/zip_archive.rs::SharedBuilder::build` inserts entries into an IndexMap keyed by raw name; duplicate names overwrite. Therefore listing/security preflight must enumerate original central records independently, retain all ordinals and refuse unsafe backend ordinal execution when duplicates exist. `Config` only exposes archive offset, not metadata/allocation limits.
- `ZipArchive::by_index_raw` avoids password/decode but is still downstream of allocator-heavy construction. `by_index` password rejection uses UnsupportedArchive(PASSWORD_REQUIRED); use independently read encryption flags for required-password typing rather than matching raw error text. InvalidPassword can be probabilistic for ZipCrypto, so ambiguous authentication stays AuthenticationOrDataError.
- Initial archive implementation reuses existing zip8.6.0 Store/Deflate only; no new codec features/version upgrade yet. Reference sources inspected: Archiver `core/src/lib.rs` public types/list/safe_join and exact registry ZIP readers/config/result. Existing clipboard/extension policies remain independent.

Exact codec follow-up: [capability gates](../../.trellis/tasks/10-16-archive-manager/research/codec-capability-gates.md) records sevenz-rust2=0.23.0 public memory-limit gap, unrar=0.5.8 public cancel/allocation gap, ZIP8.6.0 multi-disk read/write gap and stream candidates. User-approved C# Job-isolated Rust worker has since built on Windows and passed actual independent RAR/7z/native-output fixtures. SupaZip entry paths are under nested `supazip/supazip-core`, not a root-level crate directory.

Current ZIP test/extract no longer constructs ZipArchive a second time after preflight: immutable central record locations validate local signature/flags/method/name/sizes/CRC/bounded extents and stream Store/Deflate via existing flate2/crc32fast. A real mismatched local-name fixture was accepted by ZipArchive before correction and rejected after. Safe no-replace native output is separately fixture-verified; full codec/overwrite/product acceptance is not implied.

Historical Rust30/C#16 and strict-CAS native32 results are retained in task evidence. Current approved overwrite/ZIP preparation and exact counts live only in [checkpoint](../../.trellis/tasks/10-16-archive-manager/phase-a-checkpoint.md);none imply Phase A/first-release completion.

Worker codec source verification: `unrar0.5.8` exposes stream test/typed Code+When but no actual-byte callback in public wrapper; native `UCM_PROCESSDATA` callback can count/cancel and is required before actual-output-budget acceptance. `sevenz-rust2 0.23.0` ArchiveReader.for_each_entries supplies bounded caller streaming, public stream_map/block AES coder for encryption, and typed password/checksum errors. Avoid read_file/read() whole-entry Vec. Exact upstream small fixtures retained under archive tests/fixtures/upstream with hashes and UnRAR license; bsdtar independently listed plainRAR4/RAR5/7z. Archiver core/build.rs confirms narrowly linking advapi32 for bundled UnRAR on Windows without altering upstream sources.

7z extraction follow-up: exact sevenz-rust2 0.23.0 `reader.rs::ArchiveReader::for_each_entries` iterates blocks, then empty entries; block callbacks consume bounded per-file readers with optional CRC wrappers. A callback returning false does not stop the outer block loop, so the caller must retain first error and reject later callbacks. Block integrity can be later than an individual callback; stage exclusive output tokens until full decode and source recheck, then commit. No upstream unpack/read_file (whole-entry Vec) is used. Independent bsdtar extraction verified `lzma.7z/file.txt` as the literal15bytes `this is a file\n`, SHA256 `e0ef7229e64c61596d8be928397e19fcc542ac920c4132106fb1ec2295dd73d1`; solid/non-solid fixture payload hashes were independently observed before implementation.

RAR extraction follow-up: exact UnRAR7.1 `Documentation/RARCallback.md` states `UCM_PROCESSDATA` exposes a read-only buffer for both testing/extraction; return1 continues,-1 cancels. Use RAR_TEST with null native destination, split callback data into <=64KiB acknowledged ArchiveOutput writes while the native buffer is live. Mutex-serialized callback state borrows only a Send output, retains first failure, and never unwinds across C ABI. First enumerate/validate all output names; second header sequence must match expected immutable ordinals before partial creation. Stage tokens until successful native integrity result and final source recheck. Independent bsdtar payload literals: RAR4 VERSION=`unrar-0.4.0`; RAR5 Unicode entry=`foobar\n`. No RAR native filesystem writer or creation API is used.

2026-10-17 acceptance remediation: C# ArchiveSourceLease now protects the same no-reparse
source namespace/file object before Rust reads and through actual worker cleanup; RAR's two
path opens are constrained by it, not merely compared headers. Exact UnRAR7.1 dll.cpp/file.cpp
chain RAROpenArchiveEx→FMF_OPENSHARED→CreateFile was rechecked.7z public block/Coder::properties
and Archive::read can preflight a bounded synthetic normal-header wrapper containing encoded
header StreamsInfo without decoding payload; data/encoded-header dictionaries and declared
decoded header size are checked before relevant decode. Rust owns those budget/format decisions;
C# owns stable Windows handles and Job memory. Process budget is coherently work+metadata+64MiB
overhead/default832MiB, not an arbitrary512MiB ceiling. No upstream source patch or dependency
change. Independent7-Zip24.09 exact encrypted file counts/hashes are recorded in archive fixture
README; system libarchive's unsupported encrypted reads were not used as an oracle. Current
Rust31/31,C#16/16+6/6; atomic overwrite separately remains FAIL32. See
[remediation record](../../.trellis/tasks/10-16-archive-manager/acceptance-remediation.md).
This supersedes only current implementation evidence, not historical failures or A–F scope.

## Reference overwrite follow-up

[Exact source comparison](../../.trellis/tasks/10-16-archive-manager/research/reference-overwrite-comparison.md): Archiver extraction uses File::create (lib.rs609/630), ZIP rewriting finishes a sibling .rewrite.tmp then fs::rename818; SupaZip extraction similarly truncates directly, while its CLI create syncs a NamedTempFile then persist397. Matching tempfile3.27.0 Windows persist uses MoveFileExW(MOVEFILE_REPLACE_EXISTING), with no expected-old-target identity. Neither reference proves identity-conditioned atomic replacement. Next's non-share-delete target lease is a stronger added contract and causes the independently reproduced native32 self-conflict with its current replacement path. Source inspection is not reference runtime verification; no overwrite semantic amendment is approved by this comparison.

## Approved path-overwrite implementation2026-10-17

The user now approves reference-like complete validated sibling+immediate target recheck+single
atomic path publish, explicitly accepting target-close→publish TOCTOU; strict target-ID CAS is
superseded. No truncation/delete/backup-first. C# closes only target check; source lease and
held no-reparse ancestor/temp capabilities stay live. Windows SDK10.0.26100.0 minwinbase.h /
winbase.h verifies FileRenameInfoEx22(flags1|2), FILE_RENAME_INFO offsets0/8/16/20 and full
FileIdInfo18. Bounded local probe found Win32 relative RootDirectory87, full name success;
an unterminated counted name published suffixTAIL, explicit WCHAR NUL fixed it. Main API is
SetFileInformationByHandle with parent-handle-derived VOLUME_NAME_GUID path/RootDirectoryNULL;
no MoveFileEx fallback. Native result/state and post-ID/content checks prevent partial cleanup
after publication. Original renamed+6new fixtures7/7; actual Rust worker proves CRC failure,
cancel before/during/after and published-but-unverified output. This supersedes current status,
not historical strict-CAS FAIL32 or earlier reference inspection. FAT/SMB/old systems untested;
no absolute race/power-loss guarantee. [Results](../../.trellis/tasks/10-16-archive-manager/overwrite-remediation.md).
Final integrated29/29 (safe16+source/budget6+overwrite/ABI7), no skips. Native publication count
is recorded before close/hash, capability50 and uncertain64 retain exact terminal state/code;
one bounded review's concrete findings were red-tested/fixed. No whole Phase A acceptance.

## Standalone Gzip continuation source check

Rechecked tracked legacy NxTools archive terms: only package/FFmpeg ZIP and image/video owners,
no general archive UI/lifecycle identified (not an exhaustive generated/untracked search).
Archiver core/src/lib.rs230–275 decoder/encoder and987–1075 create use streaming Gzip/TAR,
but list inflates standalone streams and create writes final paths directly; neither is copied.
Exact already-locked flate2=1.1.9 (MIT/Apache-2.0,zlib-rs,Windows-built) exposes Decompress
total_in/total_out/Status::StreamEnd. Its Gzip convenience header reader retains variable names/
comments, so Next will parse bounded RFC1952 headers itself, discard untrusted FNAME/COMMENT,
stream fixed buffers and require each member's CRC/ISIZE plus explicit Deflate end. Standalone
listing derives one safe local output name and leaves expanded size unknown; no full inflate to
populate metadata. Source revision check precedes native output commit; cancellation/failure
aborts only owned token, final published bytes remain native-owned. No UI/focus/native show/hide
behavior changes. Independent CPython gzip.compress fixtures and bounded mutations are the oracle.

ZIP12 follow-up: exact bzip2=0.6.1/src/mem.rs exposes Decompress::new(small),
decompress,total_in/total_out,Status::StreamEnd/MemNeeded. Default libbz2-rs-sys (lock0.2.5)
is pure Rust, no C toolchain; wrapper MIT/Apache-2.0, libbz2-rs-sys bzip2-1.0.6 license,
no zip feature/version change. Full derived-work license is retained in THIRD_PARTY_NOTICES.md.
Small decode has bounded block-size<=900000 and documented approx2300KiB state; Next reserves
conservative8MiB before constructor, fixed input/output and chunk cancellation. Use low-level
explicit end rather than convenience reader EOF; ZIP central/local/extents/name/CRC/actual-byte
checks remain authoritative. CPython ZIP_BZIP2 independently supplies fixture, not self writer.

TAR continuation: inspected exact tar0.4.46 (MIT/Apache-2.0,no build.rs,default xattr
disabled; Windows dependency filetime) archive.rs285–448 and header.rs193/291/319/364/733.
Normal entries() read_all's GNU longname/longlink/PAX before caller budgets; do not use it.
Reuse fixed512-byte Header methods only, independently bound extension bytes/count and checked
offsets, apply PAX size/path to the real next entry, reject sparse semantics explicitly.
Archiver561–575/674–685 uses Archive/entries/unpack_in; SupaZip formats/tar.rs uses bounded
walk but leaf creation is nonrecursive. Neither unpack/create path is copied. Next lists links
as warnings/skips and extracts regular files through held native tokens, never follows links.
TAR structural/header/full-stream validation is not a payload-checksum guarantee. Independent
CPython tarfile.PAX_FORMAT Unicode/empty-dir/file/link fixture drives public red tests.

Compound Gzip/BZip2 disk-spool/integrity/source evidence is preserved in
[stream research](../../.trellis/tasks/10-16-archive-manager/research/extraction-metadata-sources.md#preserved-compound-stream-reference-evidence); no full-memory archive, early inner publication, mutable-source reopen or BZip2 creation.

Exact XZ0.21.0 source/API/allocation/index/encoder evidence and current boundary:
[XZ capability note](../../.trellis/tasks/10-16-archive-manager/research/xz-capability.md).
Single-source verified creation and public XZ/TAR.XZ/.txz reading now pass; exact public/native
red→green and safety evidence: [XZ read slice](../../.trellis/tasks/10-16-archive-manager/xz-read-evidence.md).
Zstd exact reference/API/window/error/license/oracle evidence and bounded-read verification:
[Zstd capability](../../.trellis/tasks/10-16-archive-manager/research/zstd-capability.md).

ZIP methods/crypto: [reads](../../.trellis/tasks/10-16-archive-manager/research/zip-read-capability.md),
[AES/noEOS fixes](../../.trellis/tasks/10-16-archive-manager/zip-read-remediation.md),
[create APIs/source contract](../../.trellis/tasks/10-16-archive-manager/zip-create-contract.md).

7z/TAR/TGZ/TXZ: [source/APIs](../../.trellis/tasks/10-16-archive-manager/research/tree-create-capability.md), [evidence](../../.trellis/tasks/10-16-archive-manager/tree-create-evidence.md). Current status only in task checkpoint.

Extraction metadata: [preserved contract/source/fixture/TARmtime navigation](../../.trellis/tasks/10-16-archive-manager/research/reference-metadata-navigation.md).

Source recycling: [verified SDK/API/reference chain and limits](../../.trellis/tasks/10-16-archive-manager/research/source-recycle-native-capability.md).

## Product/resource source navigation
[Product](../../.trellis/tasks/10-16-archive-manager/product-read-contract.md),
[resources](../../.trellis/tasks/10-16-archive-manager/research/resource-boundaries.md),
[finite100k](../../.trellis/tasks/10-16-archive-manager/product-100k-contract.md).

## Windows long-path source follow-up (2026-10-20)
[Verified sources and limits](../../.trellis/tasks/10-16-archive-manager/research/windows-long-path-sources.md).

ZIP filename encoding: [verified reference chain, candidate and reprojection boundary](../../.trellis/tasks/10-16-archive-manager/research/zip-encoding-sources.md).

## Phase B settings source verification (2026-10-21)
Rechecked tracked legacy archive terms before new UI work: still packaging/FFmpeg ZIP and image/
video compression, no general archive settings/window lifecycle identified. Bandizip public
langs/SimpChinese.lang IDD_SETTING_EXTRACT714–724 and IDD_SETTING_ARCHIVE729–734 retain single-root,
codepage/root-strip/default-level/exclusion reference vocabulary; Next defaults remain design§5.
No proprietary native lifecycle or icon is inferred. Existing Next MainWindow constructor→render
dispatch→official Settings keyed state and original module focus owner are the settings lifecycle;
no new HWND/show/hide boundary. Presets are configuration only, not browser/operation acceptance.
Design§5 destination is a strategy, not a fixed path; no settings picker is added. Existing typed
PickDirectory→WindowsPlatformAdapter→bounded STA/StaDialogRunner remains unchanged for later forms.
Exact slice invariants/API ledger: [Phase B contract](../../.trellis/tasks/10-16-archive-manager/phase-b-settings-contract.md).

## Phase C browser/entry reference follow-up (2026-10-21)
Tracked legacy archive search rechecked; no general archive browser lifecycle identified, not
an exhaustive untracked/generated claim. Archiver gui/src/view.rs::archive_children31,
parent_prefix195,sort_items109 derive implicit children/parent/directories-first details.
gui/src/main.rs AppWindow::new115→wire_callbacks→ui.run158 is its Slint native boundary;
activate669→view_item697→selected extract711→Msg::Done/After::OpenFile→poll300→open_path967
only opens after extraction success. Temp_view_dir is PID-shared and main159 recursively removes
it after loop exit; this is NOT safe lease/source-binding evidence and is not copied.
SupaZip nested supazip/supazip-gui main.rs App/default→eframe::run_native682, UI drains controller
events and lib.rs AppController::open_archive460 owns busy/read state; no GPUI transplant or
accepted nested/temp-lease contract inferred. Exact retained layout/lifecycle/cancel/focus/failure
invariants and new owner seams: [Phase C contract](../../.trellis/tasks/10-16-archive-manager/phase-c-browser-contract.md).

Phase C production entrances verified this turn: main.rs register -> archive/application.rs
ArchiveReadClient via archive_client_bridge -> original archive read dedicated transport ->
ArchiveReadSession/ArchiveWorkerRunner. Browser callback -> next-turn application.open_entry ->
typed ArchiveEntry materialize -> existing source lease/worker selected ordinal/protected output ->
actual exited cleanup -> ArchiveEntryCache.Seal -> current Rust identity check -> Shell command or
child browser. Native picker uses existing detached dispatcher and bounded STA/StaDialogRunner;
the test pre-cancel path does not display a window. Cache release validates file identity/hash/
metadata/MOTW and only retires owned empty ancestry, never recursive user/config paths.
Exact executed checks and remaining window-memory/encrypted-entry/expiry-startup gaps are in
[Phase C partial outcomes](../../.trellis/tasks/10-16-archive-manager/phase-c-browser-outcomes.md).
These new production entrances are not proof of legacy general-archive support or native UI acceptance.

Phase C closeout verified additional current entrances (2026-10-21):
ArchiveApplication register -> independent UiStateStore/background load -> keyed open restored
WindowBounds/ArchiveBrowser.apply_restored_state; native close/F4 snapshot -> deferred retirement
-> bounded background sibling-temp/sync/rename; no AppConfig.archive geometry or archive paths.
ArchiveEntryCache constructor -> one background startup sweep; version2 manifest -> fixed-parent
retire record -> exact root/directory identity + empty deletion, including real restart recovery.
Browser-only read command -> original ArchiveReadSession/runner/private InteractionPort -> nonsecret
password ticket event -> Rust masked Dialog -> private bounded reply -> matching original runner.
Native source identity on password read retries retains original control, never diagnostic attemptID.
Legacy Archiver source re-read at AppWindow115/ui.run158/recursive cleanup159 and view_item697-715
confirms selected-success-before-open, but its shared root/global password/recursive delete are NOT
lease recovery or private-ticket evidence. Exact delivered checks and921/1 integrated/font-gate/
native-acceptance limits: [closeout outcomes](../../.trellis/tasks/10-16-archive-manager/phase-c-closeout-outcomes.md).

## Phase D operations/launch reference follow-up (2026-10-21)

Rechecked tracked legacy archive terms before Phase D: packaging/FFmpeg ZIP and image/video
compression, not a general archive operation UI. Archiver gui/src/args.rs::parse28–51 keeps
OsString/arity; main.rs85–158 parses before AppWindow::new115, wires callbacks136, dispatches
initial action151–156 then ui.run158. extract_dest24 preserves compound archive stems. Its
spawn274–298 creates one background task/progress/cancel, poll300–385 owns completion and
hide/quit339–342; do_extract718–748 snapshots selected paths/destination, start_create485–508
filters options by capability before invoking core create. These are behavior references, not
permission to copy its global password, unbounded std channel, path overwrite or recursive temp
cleanup159. Smart extraction remains the explicitly approved Next rule, not a verified Bandizip
algorithm. SupaZip nested supazip/supazip-gui lib.rs AppController::default192, apply268,
dispatch_entry_action335, cancel477, start_progress499/finish507 and main.rs671–695
App::default→eframe::run_native682 are the controller/native boundaries; its close resets busy/
progress without worker acknowledgement and its placeholder TestEntry is not copied.

Next Phase D reuse boundary: ArchiveApplication owns immutable drafts/FIFO/presentation; typed
execution session translates into original ArchiveWorkerRunner.Start→immutable admission→fixed
held helper→suspended Job→worker/source acquire→original codec/ArchiveOutput→actual exited cleanup.
Native publication/uncertain/partial-output facts outrank late cancellation. No browser close or
private complete releases a running slot. Existing extraction plan supplies All/Entries and
Here/Name/Smart; new worker selection must bind original source revision before ordinal remap.
Password/conflict/risk/readmission stay original finite InteractionPort mechanisms. Progress
coalesces at<=10Hz, prompt/terminal lossless/bounded, existing NotificationCenter/taskbar only.

Startup verified main.rs517–546 currently parses launch modes then ordinary summon early return;
runtime.rs631 summon uses Rust IPC lib.rs handle_connection/handle_summon_connection, where
client_name alone is not peer authentication. New archive intent must use actual same-user/session/
executable identity and bounded Rust admission/readiness/ack before that return, never C# roundtrip.
Protected redirect forwards original args_os, silent/browser/background paths retain their
presentation policy; archive-origin cold startup creates main hidden/unfocused rather than hide
after show. Quick Find panel deferred selection→QuickLaunchApplication::execute_native_result
retains accepted-close and success-only usage/failure owner; intercept only ordinary file Open,
not Reveal/opaque OpenWith Shell action. Exact implementation/evidence belongs to Phase D records.

Phase D format-policy follow-up: Archiver core/src/lib.rs519–568 was rechecked for original
metadata/extraction flow. Locked sevenz reader.rs784–796 keeps only the first packed stream;
archive.rs477–487 exposes the real mapping. Sum independent-block packed streams only; shared
solid blocks have no per-member ratio. Locked UnRAR arcread.cpp322–335,807–809 and
rartypes.hpp30 establish unknown-size sentinel0x7fffffff7fffffff; unknown/split/solid member
sizes do not justify guessed risk ratios. Next reuses original finite output_readmission and
ZipRisk/InteractionPort, not new codec execution. Known planned sizes authorize at most one
output-only readmission after zero commits/actual cleanup/source binding. Unknown streams keep
actual counters. Verified compressed-TAR discovery uses the existing anonymous outer spool,
bounded by max(output,metadata); overflow before known inner sizes remains ResourceLimit, not a
shortage. Destination checks use original held native namespace/volume before directory effects;
no probe write, elevation or guessed free space. Bandizip evidence remains resources only.

## Phase E ZIP capability follow-up (2026-10-22)

New exact-source and small real-format evidence is in
[ZIP capability gate](../../.trellis/tasks/10-16-archive-manager/research/phase-e-zip-capability.md).
This verifies limitations, not product support. No earlier-phase research/cache/font/Root repair
was repeated. Archiver core/src/lib.rs::delete763–825 uses by_index_raw782→raw_copy_file784→
finish/flush787→path rename818; predictable File::create779/path rename are not Next transactions.
SupaZip nested core yielded no split/raw-copy/edit implementation in this bounded follow-up.

Locked zip8.6.0 read/zip_archive.rs::read_central_header190–193 rejects unequal end/central disks;
equal disk numbers do not implement member disk mapping. write.rs::write_central_and_footer
1896–1958 fixes disk0/ZIP64one-disk; types.rs691–718 fixes member disk0; ZIP64 extra disk_start
is unused25–35. Current Next formats/zip.rs84/101/127/258–268 rejects nonzero disks and
engine.rs31–61 opens one File. A generic segmented sink alone cannot supply genuine ZIP volumes.

Two spec-encoded252byte ZIP sets (2disks and3disks, different central/end placement; NOT ordinary
ZIP chopping) independently pass stock hash-verified7Zip26.00 list/test/extract6commands and
all4payload hash checks. Existing-artifact API probe: concatenated2disk parses but first member
fails local header;3disk explicitly unsupported; current public engine returns MissingVolume for
both last ZIPs despite present siblings. This is not a fresh source build or production Job test.

Raw-copy precision: write.rs1691→1623→1635 regenerates records from read.rs::options1046–1069,
then copies compressed ranges. Plain CPython control loses0x5455timestamp extra and original
DOS/internal attrs. Existing independent ZipCrypto control keeps packed SHA256s but loses
encryption mask0x0001; API returns success, independent7Zip t exit2/Data Errors. AES metadata
has a public preservation field, but AES preservation was not runtime-tested in this gate.
Do not treat raw_copy_file or Archiver's pattern as P203 metadata/encryption acceptance.

Required P202/P203 now needs the explicit evidenced backend decision before implementation:
recommended locked-codec project ZIP disk-relative adapter/record-preserving rewriter, not an
arbitrary upgrade or external executable backend. Retain immutable set revision/entry IDs,
bounded exact-volume identity leases, original queue/worker/Job/cancel/native commit and real
partial-publication facts. Same-source native replacement with held lease/POSIX_SEMANTICS needs
a real regression; share flags alone prove neither success nor failure. No native expansion
is authorized/inferred. Scope, actual commands/failures and unrun cases stay in the gate/results.

## Phase E approved adapter and actual same-source native gate (2026-10-22)

The user approved project-owned ZIP container/record adaptation with locked dependencies unchanged;
the earlier ZIP backend decision is CLOSED. [Implementation contract](../../.trellis/tasks/10-16-archive-manager/phase-e-implementation-contract.md)
records within-E order/new-target-only split publication. No backend/vendor/crypto engine change.
Partial zip_container.rs now maps real disk-relative records to a bounded encoded anonymous spool;
ArchiveEngine::with_zip_volumes explicitly opts in. Production worker does NOT opt in before C#
all-volume leases exist. zip_volume_candidates returns unpublished anonymous encoded candidates,
rewrites member/central/end disk coordinates and preserves existing payload codec ownership.
Small source-generated6volume ZIP independently7Zip list/test/extract succeeds,600byte payload;
public read/missing/replaced/order/count/budget tests are container evidence,not native P202 delivery.

Actual same-source chain newly verified: ArchiveSourceLease.Open32–48 -> held no-reparse parents/
read-only file via OutputNative.Open -> CaptureTargetIdentity -> BeginFile(expected,true) -> complete
candidate/readback -> ArchiveOutputSession.Commit319–379 -> Flush/full target recheck ->
FileRenameInfoEx flags REPLACE_IF_EXISTS|POSIX_SEMANTICS (OutputNative.RenameReplace662–685).
The source file handle remains live; actual call fails replace_file/Win3232/FailedBeforeCommit.
No inference from flags: three exact native fixtures LIST then eachRUN observe1PASS/2FAIL,0skip/
not_run. Same-path success and late-cancel publication both fail32; changed-native-identity refusal
passes. Precommit cancellation assertions execute before the late-cancel subcase,not a whole-case PASS.
Failure proof verifies old archive hash unchanged and held identity stable,zero replacement fact.

Source/archive transaction production code/rights remain unchanged. This is a new concrete native
boundary decision,not reopening ZIP backend choice. An edit-only final source recheck/lease handoff
inside the existing one-shot native commit is a proposal requiring confirmation; no globally widened
sharing,delete/truncate/backup/retry is authorized. Full evidence,retained failures and unrun P203/
UI/7z/RAR/native volume work belong to the additive implementation results/current checkpoint.

## Phase E current authorized delivery follow-up (2026-10-22)

The historical native32 observations above are unchanged. User now explicitly authorizes edit-only
final-source-FILE handoff/accepted TOCTOU; this closes the old decision, not the historical failures.
Current chain: ArchiveSourceLease.OpenForEdit captures fullID/version/hash/target and bounded MOTW;
worker_zip_edit binds immutable browser source/ordinals, generates/verifies record-preserving ZIP,
closes original readers, exact-token readback then edit_reads_complete. Original C# output session
flushes candidate, rechecks still-held source/target, final cancellation, releases FILE only and does
one existing FileRenameInfoEx replacement. Parent/candidate ownership and ordinary sharing stay.

Current entry points: zip_edit.rs::candidate/structure/renamed_record/write_records;
zip_container.rs::discover/normalize/split_encoded/relocate/finish_single; worker_volume_output::publish;
ArchiveVolumeSources constructor/Verify; ArchiveWorkerOutputBridge.PhaseE::HandleExtendedCall;
ArchiveOutputSession::CommitVerifiedEdit/CommitJournal; Desktop archive draft/forms/jobs/browser.
ZIP/7z source routing activates only after all-member native acquisition. All split candidates
readback/flush/verify before first no-replace publication; native journal survives cleaned candidates
and distinguishes real publication/uncertainty, never set-wide atomicity. Encoder spool is not a volume.

Record support: unchanged local/central extras/comments/attrs/raw names/encrypted bytes/descriptors
are preserved separately; rename regenerates dependent Unicode/name fields, relocation ZIP64/offsets.
Known finite extra allowlist; unknown dependent structures,SFX,split,signed application containers
refuse. Newly added entries use plain Deflate/Stored directories, visibly disclosed; existing crypto
is not changed. Native edit preserves MOTW/ordinary DOS bits, not arbitrary ADS/ACL/filesystem flags.

Independent oracle identity is hash-verified stock7Zip26.00 (SHA256
4BCC005758C5E604C0AAA257B6097846A4A708E62A9C0C7471FA360FAD863F1F).
Genuine RAR fixtures come from libarchive79c571f845dbdd250d69bff9d93ae034f674135e with license/
uuencoded originals/manifests retained; RAR4three volumes20111bytes and RAR5solid four volumes117398bytes.
These fixture sources are independent data/oracles, not a replacement product backend. Final current
results live in phase-e-delivery-outcomes and sole checkpoint. Physical GUI/DPI/UIA/user acceptance
remains NOT RUN; no F,source recycle,release/system mutation or first-release completion claim.

## Noninteractive CLI reference follow-up (2026-10-24)

Verified nested SupaZip `supazip/supazip-cli/src/main.rs`: Clap22-118 -> main/run136-215 ->
resolve_backend259-290 -> cmd_list296/cmd_extract323/cmd_test404 -> core traits::ArchiveFormat.
`output.rs5-34` serializes list/test success only; extract ignores output mode and errors staystderr.
`StderrProgress496-523` cancellation AtomicBool is never set; Ctrl-C is a TODO comment, not implemented.
Password argv Option<String>, extension-only backend selection (compound tar suffix gap), ignored
--all, direct truncating File::create and inconsistent missing-selection/unsafe-name success are not
copied. Exact extraction/test paths: core formats/zip.rs205-446,564-625; sevenz.rs238-460,539-612;
tar.rs41-149,209-258. Core traits.rs334-410 is contract only, not evidence of safe native publication.
Tracked legacy NxTools Program.cs17-27/72-112 exposes only browser extension export/package options;
BrowserExtensionPackageService220-235/FfmpegInstallerService87-103 ZIP owners are unrelated.
Next CLI must reuse product ArchiveReadClient/ArchiveReadSession and ArchiveJobSession/original
worker/Job/source/output. Exact approved shape, prompt/secret/cancel/cleanup/output requirements:
[CLI contract](../../.trellis/tasks/10-16-archive-manager/archive-cli-contract.md). No native drag substitute.

## 7z risk/readmission exact metadata follow-up (2026-10-24)

Locked sevenz-rust2 0.23.0 archive.rs45-123/459-487 and reader.rs661-825/831-1028:
file_block_index maps streamed members; first_pack indices count referenced packed inputs.
ArchiveEntry.compressed_size is only FIRST file/FIRST packed stream, not a universal member size.
Current supported single-input chains admit a denominator only for a single-member independent
block with exactly one uniquely attributed packed stream (checked sum of that one stream). Shared
solid members and ambiguous/orphan/multi-input spans stay unknown, not zero/divided. Global
archive.is_solid is anyshared, so inspect each block; this slice adds no multi-input support.
reader.rs955-978 derives finalsubstream size from declaredblockoutput minus previoussizes; declared
max/sum can be computed without payloaddecode, not integrityproof. Fullcontent/CRC/source checks
remain required before originalstagedpublication. For finalblock, pack_sizes.len can include orphan
declaredstreams, so must prove actualreferencedcount before a denominator is known. No newbackend.
SupaZip sevenz.rs64-101/308-452 copies misleadingpackedfield and compares operationcumulativebytes
with memberratio; traits.rs273-297 ratio assumptions/unrestricted values are not copied. Original
legacy packageZIP owner remains unrelated. New finite7z scope/invariants:
[7z slice contract](../../.trellis/tasks/10-16-archive-manager/sevenz-risk-readmission-contract.md).

## RAR risk/readmission source-admission follow-up (2026-10-24)

Original run_rar -> SourceSet::open -> metadata rar::execute -> validate_metadata/browser/codec_plan
-> extraction_preflight already exists; no missing-hook rewrite. Actual upstream RAR4 VERSION11bytes
and RAR5 Unicode7bytes at total1 both failed before output_budget_shortfall in original worker
rar_readmission_contract (02-red), not a decoder-only simulation. Original ResourceLimits::default
model.rs196-212 and C#ArchiveWorkerBudget.Capture52-54 agree on finite100GiB default; metadata/work/
Job remain separate. RAR-only extract+readmission may use max(original,that finite default) for
encoded source acquisition to reach original declaration preflight, never for actual output.
Native signature classification must occur under original held source lease; RAR4/5 signatures and
original RarNext/ordered family/sourceidentity checks stay distinct from 7z. No non-opt-in/open/TEST/
otherformat expansion. Original worker_password keeps finite no-output preparation and Required/
explicitWrongPassword policy; Risk only confirms original current ceiling. Exact bounded scope:
[RAR slice contract](../../.trellis/tasks/10-16-archive-manager/rar-risk-readmission-contract.md).

Locked UnRAR7.1 RARHeaderDataEx.md88-124/143-145 and RAROpenArchiveDataEx.md70-79 distinguish
member split-part packed bytes from full member unpacked size/last-part CRC. DLL PackSize is not a
volume-aggregated denominator. ROADF_SOLID is archive-level; RHDF_SOLID means current member uses
previous data, so even first member may have RHDF_SOLIDfalse in a solid archive. Existing rar.rs
gate requires neither archive/member solid nor member split; no metadata-policy repair indicated.
INT64NDF remains0x7fffffff7fffffff (arcread.cpp315-335/803-809),notzero. Archiver corelib.rs535-550
listsRAR compressedNone and642-660 uses nativeextract_with_base; SupaZip formats/mod.rs19-45/
153/184 has noRARbackend. Neither is a source for replacing current callback-budget/native chain.
Fresh fixed libarchive79c571f845dbdd250d69bff9d93ae034f674135e uuencoded testdata follow source
test_read_format_rar.c450-550 and test_read_format_rar5.c188-284: compress_normalRAR4(14617bytes,
SHA7d32e2ef511d0ade372345440838165df828197eff4d7816880dafde690cce03), compressedRAR5(436bytes,
SHA68f9d7e2662cd5bc5607ba9c6bf1edbf121cf3b08011a7d8643091791750a7c8), multiple_files_solidRAR5
(677bytes,SHA0aa2c652c06304dedd93019663c1d52d5e85055d5fdb8045bfcc4345b90b4a0f). Originaluu,
BSDnotice,stock7Zip26 l/t/memberx-so and literal/hash evidence retained in RAR round scratch; no
RARcreation/compressorcopy. RAR4regular20111/7091 andRAR5regular1200/361 provide actual tiny
positive ratio at lowered DEBUG(1,1); production64MiB/1000x unchanged. Solid/shared ratios stayunknown.

## TAR / stream declaration follow-up (2026-10-24)

Exact source check: TAR fixed/PAX regular payload size known,memberpackedNone; outer compressed
length has no member attribution. Gzip ISIZE is per-member modulo2^32 (lockedflate2crc.rs84-93),
BZ2 BZh is block/work selector,notoutputlength. Existing XZ scan checks allIndexrecords/headerCRC/
blockextents/LZMA2chunkcounts and accumulates expanded_total without payloaddecode; ZstdFCS requires
ALLstandardframes known,skippable contributeszero,anyunknown meanswholeunknown. FCS canlie,full
decoderstillchecksframeend/actualsize/applicablechecksums. Exact lzma-rust2Index API is private;
zstd-safe7.2.4 get_frame_content_size is perframe,find_decompressed_size experimental notenabled.
Actualworker tar_stream_readmission_contract03RED: knownXZ/knownZstd/twoknownZstd rejected before
originalshortfall; unknownstreamguardPASS. Minimalprivateextract-readmission open may reuse original
scans' declarations,notpublicverifiedsize or newfulldecode/index. Publicopen/TEST remainunknown,
allstructure/work/dictionary/metadatachecks and actualoutputlimits remain original. SupaZipcompound
compressed_size=size and wholememoryTXZ are not copied; Archiverstandalonelistfullinflate notcopied.
Scope/invariants:[TAR/stream contract](../../.trellis/tasks/10-16-archive-manager/tar-stream-risk-readmission-contract.md).

## Directed settings/browser correction reference verification (2026-10-24)

Re-read the current component-selection rules,0.8.0 release delta, catalog/API index and task
PRD/design§5/ui-spec/Phase B–C execution sections before edits. Tracked legacy NxTools search
again found packaging/FFmpeg and image/video compression, not a general archive-window owner.
Bandizip public UTF-8 language resources IDD_SETTING_GENERAL/ASSOC/SHELL/EXTRACT/ARCHIVE/ETC
verify the named options, not runtime implementation or permission to copy proprietary assets.
Seven newly supplied PNGs are the user's authority, but the screenshot viewer failed; loaded
image dimensions and native Paint AX tree are NOT visual inspection. No new visual match claimed.

Current production call chains reviewed: ArchiveSettingsView::save -> Core owned atomic archive
section -> deferred settings_updated; ArchiveApplication::open -> Kit independent activating
window -> ArchiveBrowser -> native close snapshot/deferred retirement. Browser Tree observes ID
and defers navigation; projection_changed defers original tree/vertical scroll writes. Preserve
source/snapshot/selection/password/native-drag generation guards, last-good/error/retry, existing
focus and close/cancel behavior. No new native show/hide owner is needed for presentation edits.

Exact locked public sources: Component0.8.0 tree.rs::Tree::new/ListItem slot delegates behavior
to Base0.8.0 tree.rs (public depth/is_folder/is_expanded; outer TreeItem owns click+toggle).
Component button/button.rs public child,accessibility_label,tooltip and Toolbar.content preserve
official outline/Medium; Toolbar.child still forces ghost/compact. GPUI pre0.3.9 div.rs remaps
orthogonal wheel input on single-axis overflow unless restrict_scroll_to_axis is set (Base's
public lock_scroll_axis helper delegates to it). The production vertical-wheel regression was
observed red (-64->0 horizontal offset) before fixing the original containers; same regression
then passed. UniformList uses its public style property, horizontal Div the public method before
test-support wrapping. Official Toolbar Medium is48DIP, so icon-over-label64DIP content needs the
public h_auto parent adaptation; real menu-overlap red/green protects this, not child sizing alone.

Safety option review: zip_edit.rs::execute flushes then verify before returning a candidate to
worker_zip_edit.rs native commit; the reference's optional test switch cannot disable this owner.
ClipboardShellTarget.cs::TryCreate/IsSafeOpenTarget accepts HTTP(S) or existing filesystem paths,
not ms-settings URIs; no default-app Button is falsely wired through this rejecting command and
no allowlist/Registry/COM changes were made. Current integration payload only classic_cascade,
open_with,extensions/revision; double-click/individual-verb presentation requires a separate
native contract, not empty settings. Temp-root/lease/source cleanup owners remain unchanged.

## Archive settings stale-draft/publication repair (2026-10-25)

UI-AC01 original production Settings repro: form-owned exclude_patterns save -> old Settings
ordinary-switch save merges new rules -> unchanged textarea rows Save silently overwrote them.
Keep a separate editor baseline; only explicit changed rules submit. Compare baseline/latest/draft
under original Core config lock; conflict retains disk+draft and explicit official reload is the
only discard action. Clean editor refresh is deferred and rechecks content before replacement.

UI-AC02 original remember_accepted -> background Core update -> old merged snapshot callback could
publish after newer foreground save. Both archive save routes now return process-local ordered
SavedArchiveSettings, allocated inside existing update_section lock and delivered only on success.
ArchiveApplication rejects older results before app/browser publication; no config schema/Core API
or native lifecycle change. The regression pauses actual remember completion via cfg(test) async
barrier after save, publishes later foreground save, then releases older callback; disk/app/browser
must retain newer values. No arbitrary external-process config watching or general framework claim.
Evidence/limits: [bounded repair outcomes](../../.trellis/tasks/10-16-archive-manager/ui-ac-settings-repair-outcomes.md).
Original negative UI acceptance remains historical; missing settings/reference/physical gates remain.

## 2026-10-25 — audit #4 decision-object display (bounded implementation index)

This is stabilization within `Phase A — Engine contracts and safety foundation`, not phase/UI
acceptance. Re-read allowed Bandizip `langs/SimpChinese.lang:579`–588: existing/incoming labels,
Skip/Overwrite/Rename/apply-all are resource evidence only, not live layout/native behavior.
Re-read licensed Archiver `core/src/lib.rs:230` decoder and `:665` single-file final `File::create`;
that truncating output/progress-name callback is NOT a safe confirmation or publication source.
No old general NxTools archive UI was identified in the existing tracked-source investigation.

Verified current producer chain: formats ZIP/TAR/standalone Gzip/BZip2/XZ/Zstd and isolated
7z/RAR `begin_extracted_file` -> `worker_output.rs::PrivateOutput` (ordinary format receives
plan-prefixed target; isolated codec maps the current member through `planned_unique`) ->
`worker_conflict.rs::ExtractionConflict::begin` -> original InteractionPort pending ticket ->
private conflict frame -> `ArchiveJobSession.OnFrame/Prompt` -> `ArchiveJobEvent` -> original
Desktop job record/`refresh_job` -> `ProgressSnapshot` -> `JobProgressView::show_prompt`.
CreateOverwrite uses `ArchiveWorkerRunner.ExecuteAsync` original native observation and
`ArchiveJobSession.ConfirmCreateOverwrite`, bound to the immutable operation Command's
Destination/Create.Name, not progress/selection. `ArchiveJobSession.Reply`, Desktop `reply_job`
and original InteractionPort retain single reply/ticket/generation/cancel authority.

Display strings are UI-only snapshots, never an output path grant or native token. Preserve
source revision/ordinal, plan/path validation, native observed identity/signature, budgets,
temporary verified output/publication, password clearing and original cleanup-bound lifetime.
New optional display is omitted when absent; new receivers accept old payloads with explicit
unavailable-object fallback. Old strict receivers reject new populated fields: no mixed-version
compatibility claim, unknown-field relaxation or version change. New payload Debug/ToString
must redact the names; fixed correlated production diagnostics remain unchanged.

Exact released Component0.8.0 `dialog/dialog.rs:735` limits the Dialog to viewport, `:769`–786
scrolls body separately from footer. `description_list.rs::vertical/columns/item` provides read-only
metadata slots. Keep original official Button/Checkbox and Dialog close/confirm/deferred reply
owners and 560x300/default,480x260/min native client. Task evidence belongs to
`.trellis/tasks/10-25-archive-prompt-object/`; source/compiled/synthetic is not physical/native,
reader/DPI/visual or user acceptance.

## 2026-10-25 — audit #5 confirmation/default-submit source boundary

Rechecked the allowed Bandizip IDD_ARCHIVE480–494 and IDD_ASK_DUPEFILE579–588 text resources;
these identify the decision fields/actions, not verified native default-key behavior. Licensed
Archiver gui/ui/app.slint401–408 uses LineEdit plus a primary create-ok Button;430/442 explicitly
route LineEdit accepted to find/password actions. gui/src/main.rs::start_create485–508 snapshots
and validates options, clears/closes the draft then calls the original background spawn;
::do_extract718–748 snapshots selected source/destination; AppWindow::new115 -> callbacks ->
ui.run158 is the Slint lifecycle boundary. Framework native internals were not inspected. The old
NxTools tracked-source investigation did not identify a general archive operation UI.

Exact current Component0.8.0 dialog/footer.rs::DialogAction122–132 has only an outer div click
dispatcher and a private anchor; it exposes no public Button trigger. Component button/button.rs
857–865 forwards activation only when Button.on_click exists, while Base Button/div own the real
keyboard-release boundary. A local composition may use official Button public child/on_click and
public FocusHandle::dispatch_action with exported dialog::Confirm, exactly one activation source
feeding the original Dialog on_ok/deferred reply. A zero-size non-Tab child focus node keeps
pointer activation routed to its own Dialog even when pointer press intentionally preserves focus.
Do not use an empty click callback, private anchor API, vendor change or second reply controller.
Base Dialog installs a root Enter Confirm keybinding; CreateOverwrite must additionally require
this prompt's explicit button activation, not blindly approve when only root/body has focus.

Original OperationForm::submit/capture, busy/pending_format/retirement and deferred submit_form
remain the admission owner. Default submission is confined to explicitly allowed focused editing
controls, after child component handling, with IME/modifier/held/busy/layer guards. Existing masked
password, Textarea, Select and format-discard Dialog keep their local semantics. The original
form client/scroll/fixed-footer budgets, source/draft/ticket/native identity and diagnostic privacy
remain unchanged. New task evidence: `.trellis/tasks/10-25-default-submit-keyboard/`; previous
audit #4's unresolved diagnostic/history stays immutable and is closed only by new #5 evidence.

## Directed remaining reference UI continuation (2026-10-25)

Re-read public Bandizip SimpChinese.lang314/407–408/791–799: Recent/Clear/empty history and
temp-folder/browse/child-process-exit vocabulary only; no proprietary process lifetime is inferred.
Tracked legacy `git grep ZipArchive|SevenZip|SharpCompress` still finds packaging, not a general
archive history/window owner. Archiver gui/src/main.rs AppWindow115 -> ui.run158 -> recursive
temp_view_dir cleanup159 and view_item697–715 -> selected extraction -> open_path967 are reference
behavior only; shared PID temp/global recursive deletion are not safe ownership to copy.
Current history must extend original ArchiveApplication -> successful generation-scoped read ->
UiStateStore bounded writer; browser geometry saves must not overwrite newer recent data/clear.
Nested temp archives do not enter top-level history. Configuration remains Core section locked
changed-field save and original ordered/deferred publication. Existing closed UIAC/Shell slices
are not reopened. Original PNGs are not reliably visually inspectable in this environment; no
new image/physical GUI claim. Temp lifecycle changes require the original cache/lease/native
open boundary, never arbitrary recursive cleanup or launch-return-as-child-exit.
Current Kit0.8.0 public Settings/Page/Normal Group/Item/Field, number_input/dropdown/render slots,
ListItem and Small/Medium outlined Button/DropdownButton compose actual history choices/list/menu;
parent layout arranges controls, no custom painted substitutes or Toolbar.child variant changes.

Temp owner trace verified before implementation: `entry.rs::materialize` -> strict
`ArchiveEntryContract/ArchiveEntrySession` -> `ArchiveEntryCache.Create` -> original runner output
and exited/cleanup_complete -> Seal -> Desktop current identity -> existing native Shell boundary
`ClipboardShellTarget.Launch/Process.Start`. The latter previously discarded Process; worker exit
is NOT external viewer exit. Existing Cache manifestv2/full fileID/hash/MOTW/directory identity,
fixed-parent retire record/exact-file+empty ancestry deletion and bounded8+8startup/admission sweep
remain deletion authority. Settings affect new leases only; old bases need bounded retained discovery
inside the same cache owner, never user-directory recursion or scan. New typed materialize policy
and lease-only open resolve owned path natively, not an arbitrary output/executable. Safe UI labels
disclose tracked-process-only exit attempts, null-process fallback, edited/locked retention and
expiry-as-eligibility. Exact source chain from read-only exploration is preserved in session scratch.
Original named browser cells need full-label Tooltip when ellipsized; official public slot only.
Recent-clear failure must not turn a ready archive into a stale snapshot. Production handle/scrollbar
press-move-release regressions will determine concrete layout defects, not direct offset injection.

## User-rejected installed browser rendering (2026-10-26)

The supplied TrafficMonitor ZIP screenshot is explicitly rejected, not visual approval. Online
GPUI Kit Design Guides and component/Toolbar were read at0.8.1; runtime remains pinned0.8.0.
Guides require platform UI font, stable icon/name/metadata lanes, comparable numbers right-aligned,
shared header/row geometry and real task/resize/focus validation, not component-count acceptance.
Sources: https://gpui-kit.com/zh-CN/docs/design-guides/ and /zh-CN/component/toolbar/ (GPUI Kit,
CC BY4.0 document concepts paraphrased here; implementation APIs verified against locked source).

Verified reuse: Downloads request_icons -> ExtractShellIcon; QuickLaunch request_icons same command;
WindowsPlatformAdapter.ExtractShellIcon -> bounded STA ShellIconExtractor -> IconCache.Shared, also
used by Clipboard ForegroundContextProvider. Archive browser instead hardcodes IconName::File and
never requests type resources. Use extension-only synthetic target, not archive member filesystem
path, materialization, shortcut resolution or executable parsing. Existing PNG validation retained;
bounded pending/results and snapshot/retirement/deferred cross-entity guards required.

Locked Component0.8.0 DropdownButton::render constructs its own menu Button at semantic32px; main
custom64px child height does not propagate. Actual red top74/main versus90/caret. Size::Size64
equalizes heights but makes a64px-wide menu trigger; final composition uses original official
action Button plus narrow24x64 Button.dropdown_caret/DropdownMenu with public slots, no vendor edit.
Base resizable distribution was investigated, but actual header AND row geometry matched at
720/960/1200; no independent-column-width defect confirmed. Browser explicitly adopts shared
UI_FONT_FAMILY, keeps original selection/virtual-list/Tree/native/drag/consent owners.

The two original PNGs were successfully read as image attachments, not OCR/dimensions. They show
a single name-list main surface with recognizable folder/type icons, not five equally promoted
metadata columns. Default name-list mode remains unimplemented in this partial repair. Mode
field/test draft was reverted because browser.rs editing was stopped after repeated
EDIT_LINES_UNSEEN (unseen anchors149/175/297/517/536); do not report mode switching or reference
layout completion. UiState schema/file remains unchanged; no user layout reset. Current actual
post-repair screenshot/native/physical/user visual acceptance not performed.

## Reference name-list continuation (2026-10-27)

User explicitly resumes the incomplete default name-list slice. Original BrowserModel selection,
ListItem/uniform_list32px rows, Tree/resizable, public PopupMenu checked entries and UiStateStore
remain owners. Add serde-defaultfalse details_view independently from existing visible/widths;
legacy missing mode opens name-list, explicit details persists; no config reset or metadata loss.
Name-only content uses actual viewport prepaint bounds followed by deferred local view update,
not synchronous leased-entity mutation or guessed window-minus-sidebar widths. Public Base
ElementExt::on_prepaint and Window::defer are already used by notification/feedback surfaces.
Header, row, hit region and scroll content share one mode-aware width. Switching retains immutable
selection/focus/sort and detailed column preferences; clears only obsolete horizontal offset.
Retain original UI font/Shell type icon changes and native read/open/drag/consent boundaries.
Continuation implemented: actual default pane geometry720/960/1200 and segmented splitter
gesture, original checked View menu pointer switch/selection/width preservation, legacy/null
and both modes durable reopen regressions now pass. Final related67 and --tests strict ClippyPASS;
see browser-name-view-outcomes for raw failures and measured interval. This closes the prior
unimplemented name-mode source gap only, not real HWND/reference visual/user acceptance.

## Later user correction: detailed default / directory type / marquee (2026-10-27)

The latest approved task explicitly supersedes the preceding name-list DEFAULT only; optional
name mode stays. Requested order Name/Packed/Original/Type/Modified uses existing semantic column
indexes and original header/row/horizontal owner. UiStatev2 migrates validv1 absent/false mode to
Details+fivecolumns, preserving widths/sort/geometry/recent/panels;v1true/customvisibility andv2
explicitfalse remain. Invalid original JSON/preferences are preserved, never reset/repaired.

Reference follow-up reconfirms legacy BrowserExtensionPackageService and FfmpegInstallerService
only package/install ZIPs; Archiver view.rs107-147 separates folders/type/parent; AppWindow
constructor115->callbacks->run158->sharedtempcleanup159 is not a safe lease model to copy.
Next TypeIcons->ExtractShellIcon->ShellIconExtractor originalboundedSTA->IconCache.Shared now has
fixed nxtools-folder-type:directory -> SHGFI_USEFILEATTRIBUTES/FILE_ATTRIBUTE_DIRECTORY before
filesystem/shortcut resolution. No archive-member path/materialization/shortcut/executable probe.

Current marquee uses officialListItem actualdown (retains originalnativecandidate), emptybody
down/move/up, publicuniformlist/scrollhandles/on_prepaint and originalBrowserModel selection.
Ctrl XORs fixedbaseline, navigationparent excluded, Escape/focusloss restores; projection/source/
refresh/close retires. Original stillpressed4DIP/OLE/sourceHWND boundary remains unchanged.
Locked0.8.0 list/list_item.rs,tree.rs,resizable.rs were rechecked; no dependency/vendorchange.

This round is PARTIAL: Tree folderPNG/24DIP/recognizablefallback not integrated; internal readonly
preview NOTDELIVERED after required browserEditstop. Unsafe/untested unwired draft removed from
production. FullDesktop1345+10 and archive157/strictTests/checkPASS are not fourfeature/visual
completion;fmtFAIL and physical/native/user gaps retained in owning browser correction outcomes.

## Browser preview and folder completion (2026-10-27 continuation)

The prior partial paragraph is historical. Current user resumes the blocked source integration.
Original officialTree/ListItem32rows now consume one sharedtypePNG at24DIP, archivefilledMaterial
fallback and statusTooltip. Original boundedIconCache/STA/syntheticdirectoryattributes stay; no
member filesystem/shortcut/executable/extraction probe. Same detailmigration/selection/nativeOLE.
View->readonlypreview uses officialResizable/SmalloutlineButtons, unified passiveMarkdownDocument
and owned staticdecodedimage, not externalShell or ClipboardItem fiction. Rust/C# Preview/PreviewText
purposes narrow originalactualoutput8MiB/64KiB; originalworker/source/password/cache/lease preserved.
Actualread/decode finishes beforelease release; UI token/row/source/snapshot protects latest/ABA/
retirement; maskedpreviewdialog cancels onlyitsown scope. ExplicitOpen cancels preview then retains
original consent/materialize/Shell boundary; previewlease cannotnativeOpen. Readimage02510 limits
beforedecode: allocationnon-strict requires boundedWebPadmission/staticonly and conservativeJPEG/
WebPcanvas; no vendoredit/globalRSSclaim. Exactbudget/sourcechains in browser-preview-contract.
Current archive181/protocol82/Desktop1369+10 and23ownednativefixturesPASS are source/isolated evidence,
not physicalGUI/Registry/visual/release acceptance. Productionstrict/globalfmt unrelatedblockers
and complete failures retained in browser-completion-outcomes; currentinstallationerror1staysOPEN.

## Approved single-host tabs continuation (2026-10-27)

The user explicitly supersedes the historical independent-window presentation with one browser
host and official TabBar/Tab. Re-read Archiver main.rs AppWindow115 -> run158, activate669 ->
view_item697, SupaZip nested GUI run_native682 and legacy tracked archive grep; no general legacy
browser/native lifecycle or reusable tab controller identified. Shared recursive temp cleanup is
not copied. Current launch_entry background canonicalize -> ArchiveApplication.open_canonical ->
original ReadClient/consume deferred install remains the request seam. Original per-browser session,
generation/depth/parentlease/preview/entry cleanup remain, only HWND presentation is shared.
Exact Component0.8.0 TabBar selected_index/track_scroll/menu/max_width and Tab suffix accept an
official close Button; no Tab close builder or private API is inferred. One caption per host;
tab close retires one source, native X retires all, last tab closes host. Inactive result installs
must not change active tab/focus. Existing C# ActivateWindow owns foreground; no new Rust WinAPI.
Contract, component selection, diagnostics and acceptance limits:
[continuation boundary](../../.trellis/tasks/10-16-archive-manager/window-tabs-icons-drag-contract.md).

## HatchZip five-capability research (2026-10-27)

Read [HatchZip index](hatchzip.md) and [future-task handoff](../../.trellis/tasks/10-16-archive-manager/hatchzip-research-handoff.md) for verified space-plan/progress/batch/crash-test/destructive-reclamation source maps. The supplied local0.1.3directory has noGitmetadata; source hashes and symbol coordinates are retained. This is research only: no runtime/test/benchmark/installation or archive mutation. In-place truncation/punching is a separate destructive capability, not existing post-success recycling; strict native source/output ownership, bounded recovery and ordinary extraction protection cannot be relaxed. No current UI/native acceptance or canonical phase ordering changes.

## Caption, progress and submitted preferences source follow-up (2026-10-27)

Current constructor chain verified: ArchiveApplication::open_form/show_job -> original
WindowOptions -> OperationForm/JobProgressView -> one shared controlled_titlebar; browser host
owns one caption above official TabBar and original browser contents omit their own caption.
caption_window_options selects transparent native titlebar+Client decoration; duplicate server/
client caption is removed at window creation, not by hiding text. Shared34DIP officialButton/
WindowControlArea composition retains each old close/cancel/background/tray owner; passive
previews/menus/notifications/editor overlays are not ordinary independent caption windows.
Exact inventory/officialAPI/narrowprivateTitleBarclose limitation and tests:
[window outcomes](../../.trellis/tasks/10-16-archive-manager/window-progress-preferences-outcomes.md).

Statistical source verified before UI repair: worker_job.rs::ProgressWriter::flush publishes
OperationControl processed bytes/files at>=100ms, nullabletotal/files_totalNone; C# original
ArchiveJobSession.Run/Finish derives committed/skipped from runner/native journal and verified
counts separately. Do not equate processed and committed or invent terminaltotals. Submitted
form preferences use original save_settings -> AppConfig update_section locked reread/changed
field merge/background publication; only explicit progress checkbox saves its ownkeep field.
No password/source/grant/ordinal/requestselection/runstate persistence. Original cancel retains
committed outputs and nativecleanup-owned slot. Nativecaption/DPI/physical/useracceptance unrun.

## Approved two-role icon consumption follow-up (2026-10-27)

Legacy NxTools.csproj ApplicationIcon/embeddedIcon and MainWindow TrayIcon load were rechecked;
current GPUI Windows platform load group1 -> WNDCLASS icon and original DesktopExecutableIcon ->
NotifyIcon remain owners. New Desktop resourcegroup1=approvedblueapp,group2=approvedorangearchive;
ProgID DefaultIcon/classiccascade use explicitnegativeID-2. No new loose runtime file, cache reset,
UserChoice or realregistration. Original Desktop/extension hashes carry assets through release
approvals; installerSetupIcon uses approvedblue. All9frames independently decoded. Exact map,
reference coordinates and rebuild/installed limits in
[icon outcomes](../../.trellis/tasks/10-16-archive-manager/icons-integration-outcomes.md).
Main-session browser root/Tab prefix additionally consume brand/archive.png; windows/taskbar/tray
remain softwareblue, not a new per-HWND orange requirement. Original PNGs immutable. Offline
Light/Dark size sheet actually inspected by main session, not realShell/DPI/uservisualacceptance.

## WAC01–03 password / tab focus lifecycle follow-up (2026-10-27)

Verified locked source entries: Component0.8.0 `src/root.rs::open_dialog` captures previous focus,
`close_dialog` restores it synchronously; `defer_close_dialog` removes the top Dialog and restores
previous focus after original animation only if the dialog count still matches. `dialog/dialog.rs`
uses that original deferred-close boundary for official actions. WindowState/ActiveDialog remain
private. gpui-pre0.3.9 `src/window.rs::on_focus_out` is a public retained Subscription (activated
next app turn), suitable for event-driven current-modal focus completion; no vendor patch needed.

Current chain: original handle_drop/ArchivePassword notification -> original passwords ticket ->
per-original-host coalesced application defer -> precise retired Dialog scopes -> public focus-out
event -> revalidate current host/active session/generation/operation/ticket -> original prompt or
current browser focus only if no modal. Ordinary entry_result now calls original retirement as
read terminal already does; its lease/source/cleanup/failure-notice semantics are unchanged.
Covered retired scope cannot close a newer modal. No second password queue, polling, animation
timer, Root-before-construction query or leased-entity synchronous writeback. Reference evidence
does not imply proprietary Bandizip/native window lifecycle inspection. Exact RED/GREEN commands,
12-case GPUI boundary coverage, related253+window8PASS and remaining native/user limits:
[WAC repair](../../.trellis/tasks/10-16-archive-manager/wac-password-focus-fix.md).

## Browser image-slot / HZ1 follow-up (2026-10-28)

Exact Component0.8.0 Tab puts12DIP label padding inside public prefix/suffix: symmetric public-slot
adaptation gives12DIP outer inset/8DIP content gaps,16DIP orange canvas; officialTab/labelellipsis/
menu/wheel/close owners remain. Tree20DIP root/directory slot keeps original32DIP row,16DIP arrow/
indent,4DIP gaps and held-native drag. Additive caption prefix supplies blue20DIP image within
original34DIP caption, not a second titlebar. Existing32/40px image aliases preserve whole aspect/
alpha; no source/derived asset bytes changed. Exact alpha/pixel/96synthetic matrix/8behavior checks:
[icon layout outcomes](../../.trellis/tasks/10-16-archive-manager/icon-layout-repair-outcomes.md).

HZ1 extends original held-directory observation and form/job result owners, not the execution
worker/queue/output budget. Read-only capacity/nativeIDs/allocation -> bounded Rust original
snapshot estimate -> officialForm/DescriptionList/refresh; reserve0 and original start-time
continue/cancel confirmation retained. Current result is PARTIAL with closed-form pending cleanup
and one strictlint still OPEN; native12PASS/core11PASS/related273PASS1FAIL are distinct evidence.
[HZ1 outcome](../../.trellis/tasks/10-16-archive-manager/hz1-space-preflight-outcomes.md) records
limitations, source-reference provenance and exact RED/GREEN; no physical/user/release acceptance.

## Multi-format edit reference follow-up (2026-10-28)

User-requested local-source review of five pinned clean Git checkouts is recorded in
[archive editing](archive-editing.md) with [source hashes](archive-editing-manifest.json).
It confirms ordinary single-volume7z add/delete/rename UpdateItems paths, Ark TAR/filter
streaming rewrite and PeaZip external backend orchestration; it also records source-level
delete-old-before-move/unchecked-commit and capability limits rather than copying them.
Archiver delete763-825 includes a non-ZIP extract-kept/recreate branch; previous ZIP-only
raw-copy discussion is not a claim that its entire delete API is ZIP-only. No product edits,
new dependencies/backend, reference builds/runtime tests, package/install or phase advancement.

## 2026-10-29 HZ4-TERM01 — owning native failure versus cleanup cancellation

Before the bounded repair, the recorded legacy NxTools tracked archive search was rerun: package/FFmpeg ZIP and image/video owners, no general archive Job. Existing indexed NanaZip `CWorkDirTempFile::MoveToOriginal`, `WorkDir.cpp:85–92`, was read again: failed native replacement returns its HRESULT, not user cancellation. No third-party source/build/dependency change.

Newly verified Next chain: `ArchiveWorkerOutputBridge.Execute` produces operation/call-bound `ArchiveNativeOutputFailure` -> `ArchiveWorkerRunner.Run.FailNative` first-failure CAS stores `ArchiveFault.OutputFailure` -> `ResolveTerminal` emits the owning category/native/state -> `ArchiveJobSession.OnRunnerEvent`. Independent `OnFrame(failure/Cancelled)` may arrive before exit because failed native output requests worker stop; that frame is a cleanup consequence, not a new user cancellation cause. The deterministic original-boundary regression calls real FailNative/ResolveTerminal and real OnFrame before OnRunnerEvent, reproducing failed/native5 -> cancelled at Job before repair. Preserve genuine/native cancellation, timeout/cleanup/unknown/postpublication/source-change facts; no native-code-nonnull blanket mapping or new wire authority.

## Selected preview password-preparation boundary (2026-10-30)
Actual retained native logs show preview_text -> password.running -> attempt_finished(ResourceLimit)
before selected output. Current Rust start_preview sets PasswordInteraction=true. Native fixture
previously left that false. With the same real 80 B ZIP member and larger peers, turning only this
flag on reproduces resource_limit. ArchiveEntrySession.Start -> original runner -> worker_password::
execute makes a no-output TEST preparation request by copying the member's 64 KiB final output
limits but omitting browser selection. Whole-archive validation is not selected output and must use
the existing finite decode-work allowances, then restore the original selected extraction limits.
Keep password attempts, CRC/integrity, source lease, cleanup and no-output password validation.
For browser-only formats with no password codec, allow-password intent routes directly to the
original attempt rather than rejecting TAR/streams as UnsupportedOperation. No new codec/runtime.
Preview layout is automatic but initially absent: only a supported single-file selection reveals
the existing resizable pane; empty/multiple/directory/unsupported selections release its width and
clear prior content. Keep original selection generations, passive focus and cancellation owners.

### On-demand preview split composition (2026-10-30)
Locked Base0.8.0 `resizable/panel.rs::ResizablePanel::render` returns an empty div for
`visible(false)` before updating its slot bounds. Group synchronization still counts that child;
`resizable/mod.rs::resize_panel_at_handle` sums all stored slot sizes, including the hidden slot.
The browser therefore conditionally adds the existing official preview panel as the last child
of the original `h_resizable` group. With no supported single-file selection there is no preview
slot, handle or reserved width. Original Tree/details slots, official dragging, preview controller
and focus owner remain. Both existing real splitter regressions cover the initially absent pane;
the on-demand preview regression covers supported/unsupported selection transitions.
