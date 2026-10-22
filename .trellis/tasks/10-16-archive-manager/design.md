# Archive manager — technical and UI design

Status: **approved implementation contract; product development and isolated verification authorized**. First release covers canonical Phases A–F; documented defaults/dimensions/navigation and technical direction are approved. Necessary dependency additions require exact capability/license/Windows build/resource evidence; no arbitrary upgrades or GPUI Kit change. Release/install/real registry/default-association/service changes, commits and pushes remain separately restricted.

Read [PRD](prd.md), [reference evidence](../../../docs/reference-code/archive.md), [implementation order](implement.md) and [acceptance cases](acceptance.md). If an earlier draft sentence conflicts with the more precise safety/lifecycle rules here, these rules govern implementation; record any actual scope change rather than silently dropping a feature.

## 1. Change boundary and source map

Current behavior: no first-class archive manager exists in the inspected Next navigation. ZIP is used for clipboard and extension-package archives, not as a general browser. Legacy NxTools archive behavior was **not exhaustively searched**; before implementation, check the old product for an existing compress/extract interaction and update the reference note if found.

Target gap: typed archive operations and settings behind one reusable Rust module; independent GPUI archive windows; native actions through C#. Do not put codecs in MainWindow, Windows calls in domain code or unrelated package-import policy in a general extractor.

| Existing source | Required integration / retained responsibility |
| --- | --- |
| `rust/apps/nxtools-desktop/src/shell.rs` | Add Archive after Extensions in the approved six-module order; preserve Clipboard→Extensions adjacency. |
| `rust/apps/nxtools-desktop/src/main_window.rs`, `main_window/shell.rs` | Settings projection, six focus handles, render dispatch; navigation itself must not start jobs or apply native changes. |
| `rust/apps/nxtools-desktop/src/main.rs`, `runtime.rs` | Compose ArchiveApplication; parse launch intent before single-instance return; preserve silent/background/browser launch modes and protected relaunch arguments. |
| `rust/apps/nxtools-desktop/src/platform_events.rs` | Route correlated picker/native terminals and desktop launch intents to original owners on the next app turn. |
| `rust/crates/nxtools-core/src/lib.rs`, `paths.rs` | serde defaults, owned archive section update; existing AppPaths cache/data separation. |
| `rust/crates/nxtools-protocol/src/lib.rs` | Versioned command/event payloads, validation and wire tests, not codec types. |
| `rust/crates/nxtools-ipc/src/lib.rs`, `local_pipe.rs` | Bounded authenticated secondary-client intent forwarding; existing `handle_summon_connection` accepts PlatformCommand only today. |
| `csharp/NxTools.PlatformHost/Protocol/PlatformCommandParser.cs`, `PlatformPipeClient.cs` | Decode/dispatch new native requests; detached bounded blocking operations. |
| `csharp/NxTools.PlatformHost/Windows/NativeFileDragCoordinator.cs`, `NativeFileDragService.cs` | Retain one C# OLE loop and pressed-source-HWND policy; not a ready-made virtual-file drag. |
| `csharp/NxTools.PlatformHost/Windows/ShellIconExtractor.cs` | Reuse extension/attribute-based cached icons for archive entries without extracting them. |
| `rust/apps/nxtools-desktop/src/quick_launch.rs`, `quick_launch/actions.rs` | Explicit internal open route only when enabled; keep usage tracking, close/failure policy and other Shell actions. |
| `installer/nxtools-next.iss` | Later approved association/menu cleanup only; no service or protected deployment edits. |

Proposed new files (names are recommendations, not claims that they exist):

```text
rust/crates/nxtools-archive/
  src/lib.rs             # narrow public request/result interface
  src/model.rs           # entry IDs, metadata and capabilities
  src/engine.rs          # open/execute, resource admission, backend dispatch
  src/path_policy.rs     # portable archive name validation + output plan
  src/encoding.rs        # raw name provenance and explicit/automatic decoding
  src/control.rs         # pause/cancel, prompt response, coalesced progress
  src/transaction.rs     # owned partial files, commit journal, safe cleanup
  src/formats/{zip,sevenz,tar,stream}.rs
  tests/{roundtrip,security,cancel,encoding,transaction}.rs
rust/apps/nxtools-desktop/src/archive.rs
rust/apps/nxtools-desktop/src/archive/
  browser.rs, dialogs.rs, jobs.rs, native_actions.rs, window_lifecycle.rs
  tests.rs, visual_preview.rs
rust/apps/nxtools-desktop/src/main_window/archive_page.rs
csharp/NxTools.PlatformHost/Windows/ArchiveFilePicker.cs
csharp/NxTools.PlatformHost/Windows/ArchiveNativeFileOperations.cs
```

Split modules by actual ownership, not arbitrary file-size quotas. Do not add a public trait for each helper. Multiple format adapters justify one private format seam; the domain interface hides backend-specific constructors and options.

## 2. Ownership, interface and lifecycle

### 2.1 Public domain interface (illustrative, not released Rust code)

```text
ArchiveEngine.open(OpenRequest, OperationControl) -> ArchiveSnapshot
ArchiveEngine.execute(ArchiveRequest, OperationControl, InteractionPort) -> ArchiveOutcome

ArchiveRequest = Extract | Create | Test | Modify(Phase E)
OperationControl = operation_id + cancel/pause control + latest progress sink
InteractionPort = bounded password/conflict/risk request + one-shot typed response
```

`ArchiveSnapshot` is immutable metadata, not a GPUI entity or open UI window. It contains a source revision (path plus size/mtime and, where available, stable file identity), format/capabilities, entries, aggregate totals and bounded comment. Workers open `File` with seek support; never copy whole input archives into a Vec. The engine detects source replacement before executing against an old snapshot and returns `SourceChanged` rather than extracting a different entry at the same index.

`ArchiveEntry`:
- `entry_id`: stable only within `archive_session_id + revision`; usually backend ordinal, never display text.
- `raw_name` with encoding provenance; `display_path` separated from validated `output_path`.
- kind: regular file / directory / symlink / hardlink / special / unsafe.
- size and compressed size: `Option<u64>`; unknown or shared solid-block packed size is **not zero** and is not repeated as per-entry packed size.
- modified: typed time plus original precision/zone knowledge; ZIP DOS times must not be relabeled UTC without conversion policy.
- optional method, encryption, CRC, attributes, comment.
- warnings and extraction eligibility. Do not hide dangerous/duplicate entries from listing and pretend the archive is empty.

`ArchiveCapabilities` advertises each operation and option: read/write/test/modify, password modes, header encryption, split read/write, determinate byte progress, cooperative pause/cancel. Detect a format from a bounded signature probe plus compound extensions. Extensions alone cannot prove a valid archive. ZIP-family containers (JAR/APK/CBZ) may browse as ZIP; this does not authorize editing signed application packages. GZ/BZ2/XZ/ZST are streams, TAR.GZ etc. are nested containers; do not decompress the whole stream merely to populate a size column.

### 2.2 State owners

- `ArchiveApplication`: settings snapshot, keyed window registry, bounded job manager and native pending map.
- Browser entity: local navigation/selection/filter/column draft plus immutable archive snapshot. `current_folder`, focus, multi-selected IDs and range anchor have one owner, not Tree/Table plus a mirrored business controller each.
- Job manager: requests and workers outlive browser presentation. Closing a browser cannot silently cancel independent extraction/compression. A progress window close means background; Stop is explicit cancellation. Application exit asks about active jobs and waits for acknowledged cancellation/cleanup, not an unbounded join on the UI thread.
- Worker callbacks never update GPUI synchronously. First install pending ID/generation, then dispatch. Apply results on a later turn with weak handles and session/revision checks. Cache-hit/immediate completion obeys the same rule.

Browser lifecycle:

```text
Requested -> Opening -> AwaitingPassword? -> Ready | OpenFailed
Ready -> Refreshing -> Ready(new revision) | LastGoodWithError
Any -> Closing -> Retired
```

An initial failure keeps a useful error surface (Retry/Open another), not a blank list. A refresh failure preserves last-good content as stale/read-only. Reopening a canonical path restores/focuses its existing live window; source replacement prompts refresh. Canonical path is the first-release key; hardlink aliases are not claimed to deduplicate unless stable-identity tests exist.

Job lifecycle:

```text
Accepted -> Queued -> Preflight -> Running
Running <-> Paused
Preflight/Running -> AwaitingPassword | AwaitingConflict | AwaitingRisk -> resume
nonterminal -> Cancelling -> Cancelled
Running -> Verifying? -> Committing -> Succeeded | SucceededWithWarnings | Failed
```

Terminal is once-only. Cancel wakes paused and prompt waits. Retry receives a new operation ID linked to the prior one; it must not silently overwrite previously committed extraction output. Conflict/password responses include job ID + prompt ID + generation, and late responses are ignored.

## 3. Engine selection and format contract

### 3.1 Recommended adapter choices

| Need | Recommendation | Evidence / gate |
| --- | --- | --- |
| ZIP | Reuse workspace `zip` requirement `8.3.0` (caret, locked 8.6.0); add only the needed features to archive consumer | Do not upgrade to Archiver's 9.x just to match its example. Feature unification requires regression tests of extension/clipboard ZIP rejection rules. |
| 7z | Evaluate `sevenz-rust2` 0.23.x, `compress` + `aes256` | Archiver uses 0.23.0; inspect exact licensed upstream sources and build on Windows before committing a version. Solid/non-solid, header encryption, typed error and cancellation behavior are capability-spike gates. |
| TAR | Evaluate `tar` 0.4.46 | Entry streaming; do not call unrestricted unpack without project path/link policy. |
| Gzip / BZip2 | Existing/transitive `flate2` 1.1.9, `bzip2` 0.6.1 candidates | Prefer maintained compatible versions; versions quoted are observations, not new pins. |
| XZ / Zstd | Choose maintained `lzma-rust2` or `liblzma`, and `zstd`, after Windows memory/license review | Archiver uses liblzma0.4.8/zstd0.14.1; SupaZip whole-buffer XZ is rejected. Do not invent a crate API without source review. |
| Names / filters / temp | Existing `encoding_rs`, compatible `globset`, `tempfile`, `walkdir`; evaluate `zeroize` | CP437 is not an encoding_rs encoding; add a reviewed decoder or use backend decoding. `chardetng` is optional, not proof of correct filenames. |
| Time restore | Portable timestamp facility or reviewed `filetime` adapter | DOS attributes, MOTW and Windows reparse/identity checks remain C#. |

The app must run without Bandizip installed. Never bundle or call `ark*.dll`/`bz.exe` as the product backend. Also do not depend on a user-installed 7-Zip/unrar executable or place passwords on subprocess argv.

### 3.2 RAR / volume decision gate

RAR4/RAR5 browse/extract/test is **required in Phase A** by explicit user approval. Evaluate the read-only `unrar`0.5.x adapter, UnRAR source restriction and C++ Windows build/resource costs there. RAR creation is excluded. No further scope reapproval is needed; exact backend capability must still be proved.

No examined project yet proves all required split capabilities. Phase E must deliver real `.7z.001`, ZIP split `.z01 + .zip`, RAR volume reads and ZIP/7z split creation. Naive concatenation/ordinary-ZIP chopping is not a split ZIP implementation. A concrete library gap requires an evidenced backend decision, not silent omission.

### 3.3 Target capability matrix

| Format / method | Required read + extract | Required create | Test meaning | Phase / notes |
| --- | --- | --- | --- | --- |
| ZIP Stored/Deflate | yes | yes | stream/check backend CRC/authentication | A; raw-copy modifications E |
| ZIP Deflate64/BZip2/LZMA/XZ/Zstd | yes, fixture proof required | no default write UI | stream/check CRC/authentication | A |
| ZIP ZipCrypto/AES | both | AES-256; ZipCrypto write only after public API review | no string-matched password inference | A |
| 7z LZMA/LZMA2/BCJ/AES | yes, coder/header fixtures | LZMA2, solid/non-solid, AES, optional encrypted names | decode/check supported integrity | A; split E |
| TAR | yes | yes | headers/full stream, **no payload checksum guarantee** | A; link preservation optional |
| TAR.GZ/BZ2/XZ/ZST | yes | GZ/XZ initially | outer codec integrity + TAR validation | A |
| Single GZ/BZ2/XZ/ZST | yes | GZ/XZ | codec integrity; unknown size remains unknown | A; single regular file only |
| RAR4/5 | yes, required | never | decode/CRC as supported | A core; volumes E |

Create levels are **semantic choices** Store/Fast/Normal/Maximum, not a universal 0/1/5/9 mapping. Each adapter publishes supported choices and maps to its own valid level/method. TAR has no compression level; stream codecs without Store disable that choice. Unsupported encryption/solid/link options are absent or disabled with an explicit reason. A method being recognized is not the same as being decodable.

## 4. Safety, extraction and transactions

### 4.1 Name validation and encoding

Preserve entry ordinals and original name provenance. Prefer explicit UTF-8 flag / valid Info-ZIP Unicode Path extra field, then a job/session-selected code page, then bounded auto candidates. Do not override a valid authoritative Unicode name. GBK/Shift-JIS auto-detection is ambiguous for short paths: mark uncertain, show original/manual alternatives, never claim a universal detection guarantee. NFD→NFC is opt-in and followed by collision checks. Explicit code-page changes rebuild tree/list projection and output plan for the same entry IDs; archive I/O, when needed for original header bytes, is worker-only.

For every decoded output component reject traversal, empty/`.`/`..` where unsafe, roots/drive-relative paths/UNC/device namespaces, ADS colon, NUL/control characters, Windows device basenames (also with extensions), trailing-dot/space ambiguity and invalid Windows characters. Detect duplicate decoded outputs, file/directory prefix collisions and case-insensitive collisions before writes. A duplicate path inside the archive is not a normal existing-destination overwrite prompt. Do not auto-sanitize an unsafe name into a potentially colliding name.

Lexical `starts_with` and `enclosed_name()` are necessary helpers, **not sufficient protection** against pre-existing junctions/symlinks or a destination swapped during extraction. The native adapter must provide a reviewed no-reparse/final-path/identity check or handle-based output capability; Rust owns the policy and plan, C# owns Windows handles. Revalidate parent identity at open/commit. Until that native seam passes adversarial fixtures, extraction safety is not accepted. Source creation must also avoid following reparse points, recursion cycles and including the output archive in its own source tree.

Links/special entries are listed with a warning but skipped by default. First release creates regular files/directories only. Optional link storage needs explicit policy/native checks; never automatic elevation. Long paths/UNC require real tests, never truncation.

### 4.2 Bounded budgets (approved defaults; configurable with explicit finite risk prompts)

- max metadata entries 1,000,000 and metadata memory 256 MiB; bounded signature/central-directory preflight before allocator-heavy backend calls.
- decompression dictionary/work memory max 512 MiB per job; heavy jobs serialized under a global budget, even if two lightweight jobs may run.
- expansion warning at 1000× for entries over 64 MiB; both declared and actual counts are checked. Unknown solid-block compressed size is not treated as zero or used in a bogus per-entry ratio.
- default actual output budget 100 GiB/job and 32 GiB/entry; users can approve larger finite budgets. Sane highly compressible files trigger a choice, not a permanent unsupported-format error.
- selected-file temp-open budget 512 MiB, nested archive depth 3, bounded image dimensions/decode memory (P2); no recursive automatic extraction.
- one active metadata load per browser, queue max32 operations, max2 active jobs, per-archive mutations serialized and conflicting output commits serialized.

Budgets are approved policy values, not measured blanket codec behavior. User now explicitly approves a C#-managed OS-memory-capped Rust archive worker for codecs without public allocation/cancellation limits: suspended child creation, Job Object admission before resume, bounded private channel, no secret argv/persistence, cooperative cancellation then termination of only the owned worker after grace, no slot release before actual cleanup. Rust retains codecs/state, C# owns Windows process/Job APIs. No third-party source patch or GPUI upgrade. Exact evidence: [codec gates](research/codec-capability-gates.md).

### 4.3 Output and rollback

Approved overwrite amendment2026-10-17: **complete validated sibling temporary + immediate target recheck + one atomic path publication** replaces the earlier strict identity-conditioned/CAS requirement. The user explicitly accepts the TOCTOU interval after closing the old-target recheck handle and before replacement. This is not an atomic compare-and-swap of the confirmed file ID and is not a universal power-loss guarantee. Earlier FAIL32 evidence remains in the native gate.

Create unique same-directory partials with CREATE_NEW semantics relative to continuously held no-reparse parents; never truncate/open an existing temporary or final target. Finish full codec/solid-block integrity and source checks, then FlushFileBuffers. Immediately before commit reopen the destination relative to the protected parent without following reparse; compare volume/full128-bit fileID/size/mtime and existing fingerprint with the confirmed snapshot. Missing/appeared/changed/directory/reparse/denied/busy are distinct typed failures, not implicit overwrite approval. Close only the target recheck handle; retain source lease, parents and temporary capability. Publish once using SetFileInformationByHandle(FileRenameInfoEx, REPLACE_IF_EXISTS|POSIX_SEMANTICS). No delete-first, truncate-first or old-target backup move. Compatibility fallback is disabled unless separately capability-classified and safety-verified; sharing/access/target changes are not unsupported capability.

State: Preparing→Verified→CommitInFlight→Committed, or FailedBeforeCommit; preserve CommittedButVerificationFailed/TargetChangedAfterCommit and CommitOutcomeUnknown separately. The final cancellation checkpoint precedes CommitInFlight; cancellation after this point cannot undo or relabel a successful publication as cancelled/unchanged. Mark publication immediately after native success before post-verification; a post-check failure must not retry, delete by path, roll back or treat the published handle as a partial. Post-check may observe the destination ID matching the temporary ID; it does not close the accepted TOCTOU interval.

Extraction is per-file committed, **not globally atomic**. Only owned uncommitted partials are removed on precommit failure/cancel. Optional cleanup may remove unchanged newly-created outputs after identity+hash verification, never pre-existing replacements or externally modified files. Published-but-unverified and uncertain outcomes are retained without destructive recovery. A failure before publication preserves the prior destination; a postcommit/unknown outcome must not claim the old file was preserved. Source lease, budgets and all codec/integrity protections remain unchanged.

Smart extraction is an approved Next rule, not a runtime-verified Bandizip algorithm: derive logical top-level items including implicit folders; one directory preserves it, one file goes directly unless single-file-folder is selected, several items use a unique archive-stem directory. Compound suffixes are one unit. Selected extraction preserves archive-relative structure, including selected ancestors. Empty archives create no destination.

If any entry fails or is skipped, use `SucceededWithWarnings` or Failed according to committed output; never delete the source archive. Delete-after-extract is allowed only after **all** intended entries of a full extraction succeed, never selected-only extraction. Delete-after-compress is allowed only after final archive commit plus a successful full test and source identity check. C# Recycle Bin returns per-path results; a recycle failure is a warning after archive success, not a false compression failure or automatic permanent deletion. Do not recycle a live browser temp source or a parent directory containing an output.

### 4.4 Open entry / temporary workspace

Temp roots: `AppPaths.cache_dir/archive/sessions/<session-id>/`, managed manifest/version and locked active lease. Only sweep owned expired sessions with no live lease; reject reparse roots. Never recursively clean arbitrary paths from config. Browser close retires temp data; failed deletion due to external apps is retained for bounded later cleanup, not forced process termination. Shell Open may hand the file to an existing process and return no usable child lifetime; do not infer app exit from launch return.

Warn before executing active content, preserve MOTW where supported, do not auto-run macros/scripts, and render archive comments as bounded plain text. Nested archives open a child browser with a temp lease held independently of the parent. P2 write-back is explicit on return/refresh, not an idle polling loop: compare baseline identity/hash, prompt, serialize a ZIP replacement against its original revision, and retain user edits if commit fails. No automatic write-back on Shell process exit.

## 5. Settings specification

Use a real official Settings hierarchy; no duplicate project-selected-page state. Approved pages and defaults:

| Page/group | Field key / Chinese label | Default and validation | Apply policy |
| --- | --- | --- | --- |
| 常规 / 打开 | `open_in_nxtools` 使用 NxTools 打开压缩包 | true, internal Quick Find route only; does not claim OS default association | immediate owned section save |
| 常规 / 浏览 | `auto_enter_single_root` 自动进入唯一根文件夹; `show_parent_row` 显示上级目录; `show_tree` 显示目录树 | all true | next view update |
| 常规 / 最近 | `remember_recent`, `recent_limit` 记录最近压缩包 | true,10; range0–30, Clear explicit | paths stored locally, never diagnostic logs |
| 解压 / 目标 | `default_destination` 默认目标策略 | Ask; shortcuts still select Smart/Here/Name explicitly | per-job immutable snapshot |
| 解压 / 冲突 | `conflict_policy` 同名文件处理 | Ask; Overwrite/Skip/AutoRename alternatives | per-job scope |
| 解压 / 完成 | `open_after_extract`, `keep_progress_open`, `smart_single_file_folder` | true,false,false | delete-source is deliberately not a sticky default |
| 解压 / 文件名 | `filename_encoding` 默认代码页 | Auto plus explicit menu choices; uncertain hint | projection + output validation |
| 压缩 / 默认 | `create_format`, `compression_level`, `test_after_create` | ZIP,Normal,true; capability-validated | drafts seed new dialog only |
| 压缩 / 来源 | `strip_single_root`, `exclude_patterns` | false; Thumbs.db and desktop.ini basenames | glob validation, no UI-thread scan |
| 查看 / 列 | `visible_columns`, `column_widths`, `sort` | five default columns; Name ascending, dirs first | preserve hidden-column options |
| 查看 / 面板 | `show_comment`, optional `preview_images` | true when nonempty; optional images=false | no idle decode |
| 集成 / Shell (Phase F) | extension list + per-verb enabled flags | all off until explicit Apply | draft + native Apply; persisted != applied; real mutation separately authorized |
| 集成 / 默认应用 (Phase F) | 打开 Windows 默认应用设置 | action only; never UserChoice edits | correlated C# result |
| 高级 / 安全 | `copy_motw`, `warn_active_content` | true,true | native result/error visible |
| 高级 / 清理 | `cleanup_new_outputs_on_cancel`, temp retention | false,24h proposed; only owned files | no rollback claim |
| 高级 / 资源 | finite limits, concurrency | §4.2; advanced validated numeric fields | future jobs, no mid-job mutation |
| 高级 / 隐私 | Clear recent; Clear inactive owned temp | explicit actions only | no user files / active temp deleted |

Fields without implemented capability are not saved as pretend-working switches. Do not add an archive-specific palette, font selector, theme setting or updater. Reuse global theme and CJK/mono fonts. Optional explanations use the shared Kit Tooltip; validation and native errors stay visible.

`AppConfig.archive: ArchiveSettings` uses serde defaults **only when the new field is absent in otherwise valid old config**. `update_archive_settings` uses lock/reread/normalize/atomic merge. Malformed JSON or a missing update target returns an error without recreation. Do not deserialize an invalid existing section to defaults and save away the user's data. Recent history/window geometry belong to separate bounded local UI state, not a whole-AppConfig overwrite. Secrets never persist in either file.

## 6. GPUI Kit component-selection ledger

The full authoritative ledger for this task is [ui-spec.md](ui-spec.md); it is split out to fit the Trellis32768-byte per-file injection limit without truncating native/safety sections. Read it before any UI implementation.

### 6.1 Reference shape and target appearance
See ui-spec§1 for retained screenshot, tree/details/comment/status shape, approved new browser dimensions and official Kit DefaultLight/Dark grammar. Do not copy Bandizip skin or change existing windows.

### 6.2 Released component map
See ui-spec§2 for exact0.7.1 sources/public interfaces, visible variants, size tiers, Toolbar.content parent adaptation, Settings/Form/Dialog composition and each required compatibility test.

### 6.3 DataTable multi-select spike (blocking before browser implementation)
See ui-spec§3. Locked TableSelection is single-row; DataTable overwrites row ARIA and owns click/keyboard behavior. Prove accessible Ctrl/Shift multi-select via public slots or obtain a narrow fallback approval; no silent loss of behavior, duplicate selection owner or dependency patch.

### 6.4 Commands, focus and window behavior
See ui-spec§4 for full menus/keyboard/focus and activating browser lifecycle, ui-spec§5 for extract/create/password/conflict/progress layouts and validation, and ui-spec§6 for the visual/state/native acceptance matrix.

## 7. Native interfaces, launch routing and real drag

Names below are **proposed typed contracts**, not existing PlatformCommand variants. Add symmetric Rust/C# decoding/tests and capability advertisement; retain protocol compatibility rules rather than silently changing existing payloads.

| Operation | Proposed request / result | Ownership and failure |
| --- | --- | --- |
| Open/save/source picker | PickArchivePath{purpose,current_path,source_hwnd}; PickArchiveSources{files,folders}; typed Selected(paths)/Cancelled/Failed | C# bounded STA; save path does not create a file; late reply checks draft generation |
| File preflight/native commit | ArchiveFileOperation{job,action,expected_identity,...} | C# reparse/identity/MOTW/DOS attrs/Recycle Bin; Rust chooses plan; no arbitrary privileged operations |
| Multi-real-file drag | new BeginFilesDrag{paths,source_hwnd}, or backwards-compatible extension | existing single BeginFileDrag stays; C# bounded admission/dedicated STA, capture release, initiating button check |
| Archive launch intent | DesktopArchiveIntent{request_id,action,paths,mode} and acknowledgement | Rust local secondary-client route; no codec work in C# |
| Shell integration (Phase F) | ApplyArchiveIntegration{desired_revision,verbs,extensions}; observed result | HKCU transaction, exact app path/rollback/SHChangeNotify; real apply separately authorized; no UserChoice hijack |

The current platform pipe server lives in **Rust nxtools-ipc**; C# PlatformHost is its client. Secondary `nxtools-desktop-summon` currently forwards PlatformCommands. A desktop archive intent should be consumed by a bounded Rust launch-intent route on that server (or an explicitly justified current-user desktop pipe), **not sent through C# just to come back**. Authenticate current-user/session/peer executable with existing pipe facilities; client_name is not authentication. Deduplicate request IDs, cap32 pending intents and256 paths/request within the existing frame limit; don't use argv passwords. A cold launch queues initial intent until ArchiveApplication exists and shows archive UI without flashing the main settings window. A secondary launch forwards before the existing summon early-return. No blind300ms union of unrelated user operations: coalescing requires same action/mode, bounded time and originating request/session grouping; otherwise independent intents remain independent.

### Real Shell drag constraint

Do **not** asynchronously extract and then call BeginFileDrag after mouse-up. It fails the existing still-pressed pointer requirement and is not a true native gesture. For already materialized selected files, start directly from the live pressed pointer callback with source HWND; C# cancels capture, rechecks the initiating button and enters one OLE loop on its dedicated bounded STA worker. Never block serial commands/event thread.

For unmaterialized archive rows, Phase F must first prove virtual-file delayed-rendering OLE (or obtain a specific alternative native-design approval), then deliver genuine drag-out before first release. Enter while pressed, coordinate Rust materialization without blocking IPC, and bound cancellation/lifetime/apartment behavior. Existing BeginFileDrag or explicit extraction is not evidence. Never silently substitute a two-click prepare-then-drag workflow.

## 8. Diagnostic and error schema

Operation IDs run through Rust worker/UI and every child native command. Use distinct command IDs under the same job correlation ID. Accepted/start, meaningful state changes, terminal outcome, elapsed and native error where applicable. No per-byte/frame/pointer logs. Bounded nonblocking sink, no logging failure changes feature result.

Domain codes: UnsupportedFormat/Method/Encryption, PasswordRequired, WrongPassword, AuthenticationOrDataError (when backend cannot distinguish), CorruptArchive, MissingVolume, UnsafePath, OutputCollision, ResourceLimit, DestinationDenied, DiskFull, SourceChanged, Busy, Cancelled, IoFailure. Preserve internal source without logging its raw Display/Debug. User-visible filename context may be shown locally; no paths/names/passwords/comments in diagnostics. Native error code optional, never fabricated for codec errors.

Progress uses one overwriteable latest snapshot at≤10Hz (terminal/prompt events lossless), not an unbounded event queue. Time/byte sampling coalesced; speed/ETA only with enough observations. 200ms is a **cooperative chunk cancellation target**, tested with bounded synthetic I/O, not a guarantee to interrupt arbitrary OS/codec calls. A stuck native call retains its worker slot. No timeout-spawned replacement threads.

## 9. Approved decisions and remaining capability gates

1. First release is A–F. RAR core is Phase A; split/ZIP editing remain E; native drag remains F. No phase renumbering or A–D completion claim.
2. Approved navigation: 下载 → 速启 → 剪贴板 → 扩展 → 压缩 → 其他工具.
3. Approved browser960×640/min720×480, compact official toolbar and documented defaults/technical direction.
4. Necessary compression dependencies are authorized after exact version/features/license/resource/cancellation and Windows build checks. Do not upgrade unrelated dependencies or Kit0.7.1.
5. Before browser coding pass public DataTable multi-select fixture or request the narrow fallback approval. Extraction needs real no-reparse/transaction fixtures before acceptance.
6. Source/isolated verification is authorized. Real registry/Shell mutation tests need separate approval; no Release/install/default-association/service changes, commit or push.
