# Onboarding, settings and native interaction reliability

## Goal
Make first-install file search useful sooner and remove the reported settings, archive entry-point, storage, uninstall and native-selection friction without changing unrelated product behavior.

## Requirements and confirmed evidence

| ID | Required outcome | Current evidence |
| --- | --- | --- |
| R1 | Diagnose and reduce initial indexing latency while preserving complete/correct search and durable restart. Report first-search readiness separately from all-volume completion. | `rust/apps/nxtools-indexer/src/mft.rs:731` hydrates each enumerated record with a native open/read/close (`mft/time_metadata.rs:28`); `lib.rs:1936` scans volumes serially and `:2647` publishes one aggregate completion. Native stage timings are unavailable. Listary evidence is managed boundary only. |
| R2 | Appearance and category rules use one vertical page scroll owner, with every control/action reachable. | `network_settings/theme_chooser.rs:206` adds a 244-DIP inner scrollbar; `downloads/settings_view.rs:1193` and `:1351` add master/form scrollbars inside a fixed 310-DIP block at `:1494`. Released SettingPage already scrolls (`vendor/gpui-component-080/src/setting/page.rs:228`). |
| R3 | Present each effective theme once; retain existing saved preference compatibility and Follow System. | `network_settings/theme_chooser.rs:6`, `:12` include default and named Latte/Frappe choices that resolve identically at `:21`; the current approved defaults are recorded in `docs/reference-code/graphite-theme.md:15`. |
| R4 | Ordinary files, folders and mixed multi-selection expose Add to Archive through the existing Shell creation workflow. | `rust/crates/nxtools-core/src/archive.rs:138` omits Create from defaults used at `:179`. C# `ArchiveSelectionHandlerPlan.cs:29` already maps create to --archive-compress; `:54` registers selected create for * and Directory. Actual user configuration/registration has not been read. |
| R5 | Display the effective absolute archive temporary/cache path for default and custom settings. | `main_window/archive_page.rs:71` shows raw config with only a generic cache placeholder. Native default resolution is in `ArchiveEntryCache.cs:34`/`:40`; custom resolution in `ArchiveEntryContract.cs:41`. |
| R6 | Complete uninstall removes application-owned personal/runtime data and owned background components; offer an explicit retain-personal-data choice. Preserve download outputs and unrelated files/processes. | `installer/nxtools-next.iss:366` only conditionally stops Indexer. It has no history/cache cleanup or retention choice. `runtime.rs:178`/`:190`/`:204` open clipboard/download stores; `:262` supports a separate background download owner. |
| R7 | Every supported file/folder/save picker remains accessible above its originating window and never fails just because selection takes time. Cancel, owner retirement and connection loss are distinct outcomes. | `BrowserExtensionExportDialog.cs:51` is ownerless; `DirectoryPicker.cs:42` uses a worker-time foreground window. IPC `lib.rs:36`/`:179` expires most pickers after 10 seconds and ArchivePicker after 120; `ArchiveJobPicker.cs:33` adds a native two-minute limit. |

Desktop-relative evidence paths in this table are under `rust/apps/nxtools-desktop/src`; native basenames are under `csharp/NxTools.PlatformHost/Windows` or `Protocol` as identified in [research](research.md). Indexer/IPC paths refer to their owning Rust package. The research document records the full paths, reference provenance and additional supported failure cases.

## Acceptance criteria

- AC1/R1: Obtain an authorized isolated first-build baseline with scan/construct/persist/reopen/publish durations and item/volume counts before claiming improvement. Add regressions for eliminating the selected expensive path and publishing a completed volume while later ones build. Preserve true file times, journal replay, last-good search, and final Ready correctness. Packaged LocalSystem restart/base+delta recovery/idle and real search remain separate release gates.
- AC2/R2: At supported minimum sizes, wheel input over theme previews or category fields moves the page, not a nested viewport. Last choices, maximum supported categories, complete editor fields and Apply/Cancel are pointer/keyboard reachable.
- AC3/R3: Latte and Frappe appear once each; saved default/named aliases select the same effective preview. Opening/cancelling does not rewrite settings. Follow System and other palettes retain behavior.
- AC4/R4: Explicit Shell Apply gives an ordinary file, a folder and a mixed selection one creation invocation with the complete ordered selection; Revert/uninstall removes only owned registration. Existing intentional custom command selections remain respected.
- AC5/R5: The displayed default/custom absolute session path equals the native cache path. Default reset preserves a blank default-policy setting. Displaying a path performs no cache IO, migration or deletion.
- AC6/R6: Disposable uninstall fixtures cover standard removal and explicit retention, active background downloads, custom clipboard/archive locations, owned/protected processes/services and unrelated sentinels. Download outputs survive even if saved inside an application data tree. Upgrades/repairs preserve user data.
- AC7/R7: Real router/caller/STA regressions cover delayed choices beyond old deadlines, normal Cancel, owner close, disconnect, stale/replaced/edited owners, no duplicate dialog and unrelated command responsiveness. Separate native acceptance verifies actual owner/Topmost/focus/keyboard/DPI behavior. Export destination selection does not consume bounded package-generation time.

## Approved UX decisions

- Natural-height Light/Dark preview columns with one choice per palette and Follow System.
- An official category selector above the existing full category editor, replacing the embedded master-list/form scrollbars. Category identity, order and draft ownership stay unchanged.
- Show the actual temporary path with default/custom status; retain the existing base-directory policy.
- Default uninstall removes personal application data, with an explicit option to retain it. Never delete downloaded files or arbitrary custom parent directories.
- All human pickers wait until selection/cancel/owner retirement, rather than increasing their deadline. Ordinary background commands keep timeouts.

## Scope and constraints

Implementation and isolated regression checks are approved. Preserve unrelated worktree changes. Rust owns state/layout, GPUI rendering and C# Windows APIs. Keep the current GPUI release and existing operation/settings/Shell owners.

In scope: the seven reported requirements, their direct cross-layer contracts and targeted regressions. Indexer optimization targets its first-build path and partial publication, not a new search architecture. Native picker changes cover all listed families, not only browser export.

Separate operational approval is required for packaging/install, real registry mutation/uninstall, deletion of current user data, real-volume scans and service restart. Implementation approval does not grant these effects. No dependency/vendor upgrade, new archiver/backend, broad persistence migration, global timeout removal or indiscriminate process termination.

## Risks and deferred evidence

Current native first-build stage timings and a comparable Listary timing are unknown. Raw-MFT optimization must retain supported NTFS record/time/identity semantics and USN catch-up; choose it from measured attribution and red-capable fixtures. Existing standalone PowerShell service probes no longer satisfy current approved-Desktop admission and must be updated before release acceptance. Compilation and screenshots do not replace live interaction or installed/native acceptance.

## Artifacts

[Research](research.md), [design](design.md) and [implementation plan](implement.md) define the owning boundaries, ordered changes and verification. Their implementation/check context manifests retain real spec/research entries. The task is in progress; workstream outcomes distinguish source/fixture evidence from outstanding native acceptance.
