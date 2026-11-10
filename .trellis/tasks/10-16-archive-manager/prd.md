# Archive manager: Bandizip-style compression module (压缩)

**Status: implementation authorized; first release covers Phases A–F.** The latest user approval confirms RAR4/RAR5 read/extract/test, ZIP editing, genuine split archives and native drag-out, navigation, documented dimensions/defaults and necessary dependency evaluation. No product completion is implied. Release/install/real registry/default-association/service changes, commits and pushes remain separately restricted.

## Current continuation requirements — 2026-10-28 planning revision

User requests the original task be replanned, NOT an implementation prompt. The consolidated
[continuation requirements/design/order/acceptance](continuation-plan.md) owns UX01–UX05,
EDIT01/EDIT02 and HZ2–HZ5; root sections below retain the original A–F baseline/history.
This amendment promotes internal MD/TXT/image preview to required, adds ordinary single-volume
7z/TAR-family editing and default-off gated source-space reclamation plus real-process crash
testing/batch summary. Earlier optional-preview/ZIP-only/general-recovery exclusion wording
is superseded only for that explicit continuation scope. RAR encoding remains excluded.
Approved current Kit0.8.0/pre0.3.9, one tab host, official ListItem fallback and later UI contracts
supersede historical0.7.1/per-archive-window/DataTable wording; no framework re-spike/reset.
HZ1 AC01–03 is accepted; preserve it. This revision schedules future work, does not authorize
product code or destructive execution; unresolved product choices are listed once in the
continuation plan. Prior implementation approvals and acceptance remain at their original scope.
Current execution status remains solely [checkpoint](phase-a-checkpoint.md).

## Goal

Add a Bandizip-style archive manager to NxTools.Next. It has four parts:

1. A new **压缩** (Compression) module in the main-window sidebar. It holds the archive settings and the quick entry points.
2. An independent **archive browser window**. It opens whenever the user opens an archive (from the module, the CLI, Explorer double-click/menu, drag-drop or Quick Find), following the user-supplied Bandizip screenshot layout.
3. **Extract and compress dialogs**, plus a progress window, with Bandizip-equivalent options.
4. A Rust **archive engine** crate that is safe against path traversal and archive bombs, streaming, cancellable and testable.

The work is staged in canonical A–F order; intermediate phases are checkpoints, not a reduced first release. Before coding, read the reference note
[docs/reference-code/archive.md](../../../docs/reference-code/archive.md), this task's
[design.md](design.md) and [implement.md](implement.md), and the shared specs listed in `implement.jsonl`.
Detailed executable scenarios: [acceptance.md](acceptance.md). Read [handoff index](README.md) first; the retained screenshot is [assets/bandizip-layout-reference.png](assets/bandizip-layout-reference.png).

## Confirmed evidence

- **Bandizip 7.32.0.1** is installed at `D:\Tool\Bandizip`. Its public resources (language file, `bz.exe` help, icons, format strings) were inventoried in the reference note.
- **Its EULA §3 restricts reverse engineering except where applicable law permits it.** No Bandizip decompilation was performed in this research. Public resources and licensed open-source projects supplied enough evidence; this handoff proposes no proprietary code reuse.
- **Archiver** (MIT) and **SupaZip** (MIT/Apache-2.0) were source-reviewed. Their engine patterns and pitfalls are recorded in the reference note. Neither GUI (Slint/egui) is reusable.
- **Workspace facts:**
  - The workspace already depends on `zip` (caret requirement 8.3.0, 8.6.0 locked, Deflate feature only) and `encoding_rs` 0.8.35.
  - The main navigation is `MainModule` in `rust/apps/nxtools-desktop/src/shell.rs`: 下载, 速启, 剪贴板, 扩展, 其他工具.
  - Settings use `AppConfig::update_section` in `rust/crates/nxtools-core/src/lib.rs`.
  - The C# boundary already provides `PickDirectory`, `OpenShellTarget`, `BeginFileDrag` (single path) and `SetTaskbarProgress`.
  - A second desktop launch currently only summons the main window (`runtime.rs#summon_running_desktop`).
- **Locked Kit 0.7.1 provides:**
  - `Tree`/`TreeState`
  - `DataTable`/`TableDelegate`, whose `TableSelection` is single-row; `TableEvent` has `DoubleClickedRow` and `RightClickedRow`
  - `h_resizable`
  - `Toolbar` (its `child` forces `ghost().compact()`)
  - `DropdownButton`, `DropdownMenu for Button`, `StatusBar`, `Settings`, `Form`, `Dialog`, `AlertDialog`, `Progress`, `PopupMenu`, `AppMenuBar` (global app menus)
  - GPUI `ExternalPaths` for files dropped into a window
- **The user requires:**
  - the new 压缩 category with its settings
  - a dedicated archive window like the screenshot
  - UI built from GPUI Kit components according to the local component-first rules

## Approved first-release scope

- **Required first release** = Phases A–F. A–D alone is not completion.
- RAR core support is Phase A; split support and ZIP editing remain Phase E; Explorer integration and genuine native drag remain Phase F.
- Optional enhancements below remain explicitly optional, not substitutes for required work. Necessary dependency additions are authorized only after version/capability/license/Windows build/resource verification; no arbitrary existing dependency upgrade or GPUI Kit upgrade.

| Capability | Delivery |
| --- | --- |
| Read/list/extract/test ZIP (Stored, Deflate, Deflate64, BZip2, LZMA, XZ, Zstd; ZipCrypto + AES), 7z (LZMA/LZMA2/BCJ/AES, incl. header-encrypted), TAR, TGZ/TBZ2/TXZ/TZST, single-file GZ/BZ2/XZ/ZST | Required; Phase A engines |
| Read/list/extract/test RAR4/RAR5 | Required; Phase A, read-only UnRAR license/C++ build/resource review; never RAR creation |
| Create ZIP (Store/Deflate, levels, AES-256, UTF-8 names), 7z (LZMA2, solid/non-solid, AES-256, optional name encryption), TAR, TGZ, TXZ, single-file GZ/XZ | Required |
| Create genuine split volumes (`.zip` + `.z01…`, `.7z.001…`) | Required; Phase E |
| Read split volumes (`.7z.001`, `.zip`/`.z01`, `.part1.rar`) | Required; Phase E |
| Transactional ZIP add/delete/rename/new folder/comment | Required; Phase E; reject split, SFX, broken and signed application containers |
| Classic HKCU Explorer verbs and per-user OpenWith registration | Phase F source/isolated fixtures required; real application/testing separately authorized |
| Genuine Explorer drag-out, including unmaterialized selected entries | Required; Phase F delayed-rendering OLE; cannot substitute prepare-then-drag |
| Password manager; bounded image preview; explicit temp-edit write-back | Optional enhancements with their own acceptance gates; no plaintext secret persistence |
| SFX/ZIPX/LZH/ISO/ZPAQ creation; RAR creation; repair/recovery; AMSI; email; icon packs/skins/language/updater; Windows11 top-level IExplorerCommand menu | Excluded |

## Requirements

### R1. Compression module (压缩) in the main window

- Add `MainModule::Archive`, label `压缩`, description `压缩、解压与压缩包浏览`. Approved order: 下载, 速启, 剪贴板, 扩展, 压缩, 其他工具, preserving Clipboard→Extensions adjacency.
- The page uses the official `Settings → SettingPage → SettingGroup → SettingItem → SettingField` compound. Its pages are 常规, 解压, 压缩, 查看, 集成 and 高级. Field lists and defaults are in design §5.
- Above the settings, a quick-action row has outlined Medium buttons: 打开压缩包…, 新建压缩包…, 解压到…. There is also a recent-archives list with a bounded size (default 10).
- Saving follows the existing section-owned config contract (`AppConfig::update_archive_settings`). Saved values are not the same as applied ones: a shell-integration change reports its own native result.

### R2. Archive browser window

- One independent GPUI window per opened archive, keyed by canonical archive path. Reopening the same path focuses the existing window and never creates a duplicate.
- The layout follows the screenshot:
  - client titlebar `<archive name> - NxTools`
  - menu row (文件, 编辑, 查看, 工具)
  - toolbar: 打开▾, 解压▾, 新建, 添加, 删除, 测试, 查看▾
  - resizable split of a folder `Tree` (left) and a file `DataTable` (right)
  - optional comment pane
  - `StatusBar`
- Default client size is 960×640 logical px, minimum 720×480. It is resizable, and bounds are remembered.
- The left tree's root is the archive file name, and its folders are derived from entry paths, including implicit folders. Selecting a tree node navigates the list. "Single root folder auto-enter" is on by default.
- **Right list columns:**
  - default: 名称, 原始大小, 压缩后大小, 类型, 修改日期
  - optional: 压缩方法, 加密, CRC, 属性, 路径
  - the column chooser is the 查看▾ menu
  - sortable, with folders first
  - a `..` parent row (toggle in settings)
- **Interactions:**
  - multi-select with Ctrl/Shift; Ctrl+A
  - Enter or double-click opens a folder or a file
  - Backspace parent; F2 rename and Del delete (Phase E); Ctrl+E extract; Ctrl+T test; Ctrl+F filter; F4 close archive
  - right-click context menu (design §6.4)
- **Status bar:** left side shows `文件: N, 文件夹: M, 压缩包大小: X`, or the selection summary when a selection exists. Right side shows format, solid, encrypted and comment indicators.
- Opening a file entry extracts it into a per-window temp workspace, then hands it to Shell `Open`. Executables (`.exe .com .bat .cmd .ps1 .vbs .js .msi .scr .lnk`) need a confirmation unless 不再询问 was saved.
  - Optional explicit editable-ZIP temp-file write-back has a separate gate; never automatic.
  - On browser close, retire the owned temp workspace; active child-browser/external-use leases and locked files defer deletion to bounded later cleanup. Never delete another session or user-owned file.
- Drop-in offers open/create/add when writable. Required Phase F genuine drag-out enters one native OLE loop from the still-pressed pointer callback with source HWND and bounded delayed materialization. Never start after asynchronous extraction and mouse-up. See design §7.
- A code-page menu offers 自动检测, UTF-8, GBK(936), Big5(950), Shift-JIS(932), Korean CP949, CP437, 1250, 1251, 1252. Preserve authoritative Unicode names. Rebuild names/tree/output validation on the worker when needed; auto-detection is heuristic and uncertain names retain a manual choice.

### R3. Extraction

- **Commands:**
  - 智能解压
  - 解压到当前文件夹
  - 解压到"<压缩包名>\"
  - 解压到…: shows the extract dialog with destination, all or selected files, extract into an archive-name subfolder, delete the archive after extraction, open the folder when done, and remember settings
  - 解压到桌面
- **Smart extract:** if the archive has exactly one top-level entry, extract it directly. When that entry is a single file and the setting "单文件也创建文件夹" is on, create a folder instead. Otherwise create a folder named after the archive stem, with a non-colliding suffix.
- **Conflicts:** the conflict policy is one of 询问, 覆盖, 跳过 or 自动重命名 (`name (2).ext`).
  - 询问 shows a conflict dialog comparing existing and incoming size/date, with 应用到全部.
  - The policy is per job, and "apply to all" is per job too.
- **Pre-flight checks:**
  - destination exists, or offer to create it
  - destination writable
  - free space versus total uncompressed size, with a continue/cancel prompt
  - required password present
- **Safety (non-negotiable):**
  - reject traversal: `..`, absolute paths, drive and UNC prefixes, ADS `:`, reserved device names
  - links never escape the destination, and symlinks are not created by default
  - enforce entry-count, memory, actual expanded-byte and ratio budgets, with explicit finite budget escalation rather than misclassifying legitimate data as unsupported
  - ordinary extraction opens each validated final relative name under the C# held no-reparse parent with `CREATE_NEW`; there is no sibling `.nxpart`, final rename, `FlushFileBuffers`, per-write hash, output fingerprint, per-file `VerifyDirectory` or published-file reopen
  - Ask/Overwrite/Skip/AutoRename and current-attempt ApplyAll remain Rust-owned. An existing `CREATE_NEW` result reports `target_exists` plus existing bytes; overwrite directly truncates the same observed path, AutoRename retries `CREATE_NEW`, and cancel/failure deletes only current incomplete owned output(s), never successfully closed files/directories, skipped pre-existing targets or outputs not owned by the attempt
- Restore modification times. Restore the read-only/hidden attributes when the archive provides them.
- Optionally copy `Zone.Identifier` from the archive to the extracted files (default on). This is the Mark-of-the-Web, a security setting.
- Preserve source archive on any failed/skipped entry or selected-only extraction. Delete-after-extract requires full successful extraction and explicit per-job consent, then C# Recycle Bin with per-path outcomes.
- Encrypted archives prompt for a password with show/hide. A wrong password re-prompts with an error and keeps the job's other options. 应用到全部 reuses the password for the rest of the batch.

### R4. Compression

- **Entry points:** 新建压缩包… (module, archive window, CLI, Explorer verb), and files dropped on the module page or an archive window.
- **The compress dialog fields:**
  - source list with add, remove and total size
  - archive name and destination folder
  - format: ZIP, 7Z, TAR, TGZ, TXZ, GZ, XZ
  - semantic level: 仅存储 / 快速 / 标准 / 最大; adapter-specific mappings, with unsupported choices disabled (TAR has none)
  - real split size (Phase E; required before first release)
  - password with confirm and show
  - test after compress
  - delete source after compress
  - each item to its own archive
- **Advanced section:**
  - ZIP: Deflate/Store, AES-256; ZipCrypto writing only after a reviewed public backend capability is proved; comment and threads only where implemented
  - 7z: solid/non-solid, encrypted names and threads after the capability spike
  - Link storage is optional opt-in capability work, never implicit symlink following or creation.
- GZ/XZ cannot take a folder or several items. The dialog says so and offers 分别压缩 (one archive per item).
- Turning on 压缩后删除原文件 while 测试 is off asks whether to enable 测试. Source deletion happens only after a successful test and moves files to the Recycle Bin through C#, never deleting them permanently.
- Archive creation output, unlike ordinary extraction output, retains a unique exclusive sibling candidate, complete verification/readback and verified replacement. Cancel/failure preserves the prior target. Confirm overwrite; reject source/output self-inclusion, source changes and recursion through links.
- **Name rules:** a single folder named X becomes `X.zip`. Several items take the first item's name, or the parent folder's name when no first-item name exists. "只压缩单个文件夹时不创建根文件夹" (default off) strips the root.
- ZIP names are written as UTF-8 with the language-encoding flag set. Thumbs.db/desktop.ini exclusion is on by default, and an exclude filter accepts glob patterns.

### R5. Jobs, progress and cancellation

- One bounded job manager owned by `ArchiveApplication`: at most 2 concurrent jobs, a FIFO queue of up to 32 jobs, each with a `JobId` (UUID) correlation ID.
- Every extract, compress, test and modify job shows a compact progress window. It has a `Progress` bar (determinate when totals are known), current item, bytes done/total, speed, elapsed and remaining time, and 暂停/继续, 停止, 后台 and 完成后不关闭 controls. It sets taskbar progress through the existing C# command.
- Progress is a latest-value snapshot coalesced to at most10 per second; prompt/terminal events are lossless and bounded. Byte thresholds never override the time-based UI rate limit.
- Pause/cancel are cooperative chunk-boundary capabilities. Target cancellation acknowledgement≤200ms in bounded synthetic I/O tests; do not promise interruption of arbitrary blocked OS/codec calls or release a stuck worker slot on timeout.
- Terminal outcomes distinguish full success, success with warnings, cancellation and failure. Existing NotificationCenter provides meaningful per-job actions. Failure retains actionable retry; native code is shown only when provided by its owning boundary. Background/close presentation never silently cancels a job.

### R6. Windows integration

- **CLI intents** on `NxTools.exe`:
  - `--archive-open <path>`
  - `--archive-extract <mode> <paths…>`, where mode is `smart`, `here`, `name` or `dialog`
  - `--archive-compress <paths…>`
  - `--archive-test <path>`
  - A bare archive path in argv equals `--archive-open`.
- A second instance forwards a typed intent through the Rust-owned local pipe before the summon early-return. Deduplicate IDs and coalesce only a provably related same-action/mode selection; a300ms window alone is not enough to merge unrelated commands. Preserve cold/silent/background startup behavior.
- **Phase F Explorer integration:** HKCU-only classic cascading `NxTools` verbs: open/smart/here/name/dialog/test for archives; name.zip/name.7z/create-dialog for sources. Native registration/removal returns typed observed outcomes; uninstall removes owned verbs only. Source and isolated plans are authorized; real apply needs separate permission.
- **Phase F file association:** per-user `OpenWithProgids`; open Windows default-apps settings, never silently take defaults.
- Quick Find: opening a recognized archive result routes to the archive window when the setting "使用 NxTools 打开压缩包" is on (default on).

### R7. Diagnostics and privacy

- Every job logs `archive.job.accepted`, `archive.job.started`, phase transitions (`listing`, `extracting`, `compressing`, `testing`, `awaiting_password`, `awaiting_conflict`, `paused`) and one terminal `archive.job.finished` event.
- The terminal event carries `job_id`, kind, format, entry count, total bytes, elapsed ms, outcome and the typed error code. C# native operations log their own HRESULT/Win32 code under the same correlation ID.
- Never log: passwords, archive or entry names, paths, comments, or file contents. Log counts, sizes, the format and the extension class only.
- Never log per-chunk progress.

### R8. UI component compliance

- Every surface is recorded in the component-selection ledger (design §6) before it is implemented.
- Official Kit compounds are used. No hand-painted Button/Input/Switch/Checkbox/Radio/Select substitutes.
- Header and quick actions are outlined Medium Buttons composed through `Toolbar.content` when a Toolbar is used.
- The Kit Default Light/Dark themes are used, with no palette overrides.

## Acceptance criteria

### A1 Engine (Phase A)

- For each advertised create-capable format, create/list/extract and byte-compare at valid semantic levels. For read-only methods, use independently produced fixtures. Include AES ZIP and AES7z with/without header encryption; never pretend all read formats support creation.
- Missing passwords yield typed PasswordRequired; wrong-password results are typed only when the backend distinguishes them. Ambiguous authentication/corruption yields AuthenticationOrDataError, never a string-matched guess. Test retry preserves destination/selection/options.
- Malicious fixtures are rejected or contained, with no write outside a temp destination:
  - `../`, absolute, `C:\`, UNC and ADS names
  - reserved device names
  - symlink escape
  - case collision
  - extreme expansion, actual-byte overflow and metadata allocation limits, with typed risk/limit handling
  - entry-count overflow
- Cooperative cancellation target≤200ms on bounded1GiB synthetic I/O; paused/prompt waits wake on cancel and only current incomplete owned ordinary outputs are deleted. Record backend limitations honestly.
- Explicit GBK/Shift-JIS/CP437 fixtures round-trip names. Auto cases include both detectable and ambiguous input; authoritative Unicode cannot be re-decoded, ambiguous input must retain manual selection.
- Benchmark100k-entry ZIP listing (target≤1.5s on recorded workstation/warm-cache conditions), metadata RSS, first-paint and cancel latency. This target is not an observed result or permission to write a tautological timing test; enforce finite metadata budgets.

### A2 Module and settings (Phase B)

- The sidebar order and labels match R1.
- Settings persist through update_archive_settings with stale-section tests. Only an absent archive field in valid legacy config gets defaults; malformed config or missing update target fails without overwriting/recreating the file.
- The quick actions open the native picker. Cancel has no effect; a selection opens the archive window or the dialog.

### A3 Archive window (Phase C)

- Opening `main (5).zip`-like fixtures shows a tree and list equivalent to the screenshot.
- `..` navigation, tree/list sync, sort, multi-select, the keyboard map, the context menu, the code-page switch and the status-bar counts are all covered by GPUI tests against the real production views.
- Reopening the same archive focuses the existing window.
- Entry-open uses leased owned temp storage into correlated Shell Open and active-content warning. Close cleans only safe retired files, and locked/child-leased temp remains available until deferred cleanup.
- Drop-in proves its choice dialog. First-release Phase F drag-out additionally proves still-pressed OLE initiation, selected-item completeness, delayed-materialization cancellation and command-reader responsiveness.
- Light/Dark, default and minimum sizes, long CJK names: screenshots are generated and individually inspected. User visual acceptance is recorded separately.

### A4 Dialogs and jobs (Phase D)

- Extract-dialog and compress-dialog options map one-to-one onto the typed job requests. Unit tests cover the smart-extract and naming rules.
- All conflict-policy and password-retry paths are covered by GPUI and lifecycle tests, including Escape and close as cancel, and no double submit.
- Progress pause, resume and cancel work, the queue is bounded, terminal notifications appear, and the taskbar progress command is sent and then cleared.
- Delete-source happens only after a successful test, and goes to the Recycle Bin through C#.

### A5 Integration (Phases D/F)

- Single and second-instance CLI intents open or forward correctly. A multi-select burst is coalesced. A typed rejection is returned when the desktop is not running and cannot start.
- Phase F registry plans and rollback/uninstall are pure/isolated-fixture tested. Real registry apply and actual Shell invocation require separate authorization; no HKLM/admin/UserChoice edits.

### A6 Verification gates

- Rust: `cargo fmt --check`, strict Clippy `-D warnings`, workspace tests (with the `CARGO_BUILD_JOBS=1` memory rules from AGENTS.md); C# tests; `python tests/architecture/spec-contracts.py`; `python tests/architecture/gpui-component-catalog.py`.
- Report source review, compiled behavior, inspected screenshots, native/DPI and user acceptance as separate items. Never claim one from another.

## Out of scope / constraints

- No Bandizip proprietary-code/icon/skin reuse or decompilation is proposed or performed by this task; licensed public/source references suffice for the handoff.
- No HKLM writes or service changes; installer scope is only owned Phase F verb cleanup. No Release packaging, installation, real registry/default-association application, commit or push without separate authorization.
- No new GPUI-stack dependency or Kit upgrade (Kit stays exactly 0.7.1).
- A new archive crate dependency needs a check of its license and Windows build cost, recorded in design §3.
- The existing extension-package ZIP validator and the clipboard archive stay independent and unchanged.
