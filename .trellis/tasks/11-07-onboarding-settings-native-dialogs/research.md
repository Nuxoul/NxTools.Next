# Source evidence

## Settings scrolling and duplicate themes

- `rust/apps/nxtools-desktop/src/network_settings/theme_chooser.rs:6` includes both Light and CatppuccinLatte; `:12` includes both Dark and CatppuccinFrappe. `:21` resolves these pairs to the same palette. `:100` labels the aliases as defaults. The later approved default-pair amendment in `docs/reference-code/graphite-theme.md:15` establishes Latte/Frappe as the current defaults.
- The same chooser creates a fixed 244-DIP inner scroll viewport at `theme_chooser.rs:206`. It is placed into the Appearance SettingPage by `network_settings.rs:1239`; the released `vendor/gpui-component-080/src/setting/page.rs:228` already creates the page list and vertical scrollbar.
- Download categories have a scrolling master list at `downloads/settings_view.rs:1193`, a second scrolling form at `:1351`, and a fixed 310-DIP master/detail wrapper at `:1494`. The enclosing `Settings` is created at `:2368` and category rules are embedded in its General page at `:2767`.
- Existing `network_settings/kit_migration_tests.rs:670` requires every persisted ThemePreference to appear as a separate control. `:795` specifically tests scrolling the inner theme viewport. These assertions do not protect the requested single-scroll and nonduplicate presentation.

### Component and compatibility boundary

Retain Kit/Base/Component/Assets 0.8.0 and GPUI pre0.3.9. Use Settings / SettingPage / SettingGroup / SettingItem / SettingField for settings; official Radio / RadioGroup for theme previews, a released dropdown/Select for category selection, Form / Field for category edits, and outlined Small Buttons outside Toolbar.child transformations. Natural-height content belongs to the page scroll owner. Do not modify vendor code or introduce a second settings controller.

## Native file selection and human waiting

- `csharp/NxTools.PlatformHost/Windows/BrowserExtensionExportDialog.cs:28` starts a standalone STA, and `:51` calls ownerless SaveFileDialog.ShowDialog().
- `ProgramPicker.cs:49`, `ClipboardArchiveFilePicker.cs:69` and `:93`, `ClipboardStorageDirectoryPicker.cs:38` and `ExtensionManifestPicker.cs:108` also use ownerless presentation. `ExtensionRuntimePicker.cs:23` reuses ExtensionManifestPicker.ShowDialog.
- `DirectoryPicker.cs:42` supplies the foreground HWND observed on the worker thread, not the initiating source window; `:64` obtains it through GetForegroundWindow.
- Older PickDirectory/PickProgram/PickExtensionManifest/PickExtensionRuntime/PickClipboardStorageDirectory contracts lack source HWND at `rust/crates/nxtools-protocol/src/lib.rs:684`. ExportBrowserExtension also lacks it at `:992`. Source ownership already exists for extension settings (`:715`), ArchivePathPicker and ArchiveJobPicker.
- `rust/crates/nxtools-ipc/src/lib.rs:36` sets the general deadline to 10 seconds and the archive picker deadline to 120 seconds. `CommandRouter::enqueue` at `:179` assigns the general deadline to all other picker commands, including extension settings whose C# runner is already infinite.
- `expire_pending_commands` at `:639` removes expired pending operations. `send_timeout_results` at `:677` delivers a failed result but does not cancel the native modal dialog. The export caller in `downloads.rs:3176` consumes this result, clears its pending operation and displays the reported failure.
- `ArchiveJobPicker.cs:33` additionally imposes a native two-minute wait. `ExtensionSettingPathPicker.cs:16` already demonstrates the existing bounded STA reservation with Timeout.InfiniteTimeSpan and real modal cleanup ownership. `BoundedStaOperationRunner.cs:37` admits existing slots and releases them at actual worker completion, not logical timeout.
- `downloads.rs:254` and `:2067` also impose a separate ten-minute FFmpeg picker timer; remove human deadlines at both router and feature owners, not just one layer.
- `PlatformCommandSequenceDispatcher.cs:11` already detaches pickers/export from the serial pipe reader. Preserve this; indefinite human waiting must not block unrelated commands.

### References inspected

AB Download Manager was inspected first: `D:\Code\ab-download-manager-master\desktop\app\src\main\kotlin\com\abdownloadmanager\desktop\pages\addDownload\shared\LocationTextField.kt:46` binds folder selection to its field and passes attachToWindow=true. `desktop\app\src\main\kotlin\com\abdownloadmanager\desktop\ui\util\FilePickerUtils.kt:54` supplies FileKitDialogParent.awt(LocalWindow.current). FileKit/AWT native implementation was not inspected.

Legacy NxTools: `D:\Code\NxTools\Views\AddDownloadWindow.axaml.cs:20` attaches animation in the constructor, `:27`/`:34` own open/close events, and `:179` captures the original view model before awaiting its StorageProvider.OpenFolderPickerAsync. `Views\MainWindow.axaml.cs:2044` builds export options, `:2069` suspends application-window Topmost states, `:2084` selects the settings window's StorageProvider, `:2085` awaits SaveFilePickerAsync and restores prior Topmost states in finally. The Avalonia native base implementation is not checked into this reference tree.

The archive reference `docs/reference-code/archive-picker.md:9` follows the saved Bandizip constructor/dispatcher to owner-bound SHBrowseForFolderW, preserving cancel-without-selection; it explicitly does not establish the entire reference form creation chain. Current Next ArchiveJobPicker uses owner-bound WinForms/IFileOpenDialog calls at its actual native modal boundary.

### Required invariants

Carry the actual initiating HWND and authenticated desktop peer through typed commands. Validate the original owner before showing and before applying results. Keep the picker above that owner and prevent other product-owned Topmost windows from obscuring it; restore only the presentation state suspended by the same live operation. Do not make it globally permanently topmost or steal foreground from unrelated applications.

Human waiting has no elapsed deadline. User cancellation, original owner retirement, process shutdown and connection loss remain terminal events. Retain one admitted native operation through actual modal return and cleanup; a logical cancellation cannot permit a replacement dialog to overtake a still-live one. Preserve view/window/operation/edit-revision checks and deferred GPUI cross-entity callbacks.

Separate destination selection from browser package export so machine work keeps a bounded failure policy without charging the user selection time. Carry one correlation ID through selection, export and native diagnostics. Log only operation/stage/count/elapsed/native code, not selected paths, filenames or payloads.

## Archive temporary directory

- `main_window/archive_page.rs:71` initializes the path input from the raw configured base string and supplies only the placeholder `默认：NxTools 缓存目录`.
- `main_window/archive_temp_settings.rs:105` renders that raw input and controls, but no effective path.
- `csharp/NxTools.PlatformHost/Windows/Archive/ArchiveEntryCache.cs:34` appends archive/sessions to its cache root; Production at `:40` uses LocalApplicationData/NxTools/NxTools.Next/cache. The default effective session directory is `%LOCALAPPDATA%\NxTools\NxTools.Next\cache\archive\sessions`.
- `csharp/NxTools.PlatformHost/Protocol/ArchiveEntryContract.cs:41` appends `NxTools.Next\archive\sessions` to a custom base; `ArchiveEntryCache.cs:400` selects default versus admitted custom roots.
- Read-only known-folder lookup on this workstation returned `C:\Users\Administrator\AppData\Local`, making the default `C:\Users\Administrator\AppData\Local\NxTools\NxTools.Next\cache\archive\sessions`.

Display the effective absolute directory with default/custom status and a real open/copy action, keeping the blank configuration value as the default policy. Display-only work must not move, clear or take ownership of existing caches or active leases.

## Verification status

Evidence above is source/reference inspection, not a live UI reproduction, native focus acceptance or compiled behavioral pass. Implementation must add symptom-specific production-boundary regressions before changing the corresponding product behavior. No product build, launch, picker presentation, registry/service operation or user-data deletion was performed for this planning task.

## Archive creation Shell entry

- `rust/crates/nxtools-core/src/archive.rs:138` defines legacy_menu_commands without Create; fresh defaults call it at `:179`. C# `ArchiveIntegrationPlan.cs:44` also defaults to five archive-only actions. The existing UI exposes all commands through `main_window/archive_page.rs:589`.
- `csharp/NxTools.PlatformHost/Windows/Archive/ArchiveSelectionHandlerPlan.cs:29` maps create to --archive-compress; `:54` adds the selected create verb to HKCU Software/Classes/* and Directory. Archive verbs go under SystemFileAssociations. No selected create means no ordinary-file/directory creation keys.
- `ArchiveIntegrationPlan.cs:119` requires ClassicCascadeEnabled; `:126` keeps creation on the grouped selection helper rather than a per-file %1 command. `docs/reference-code/archive-shell.md:168` records the legacy-five compatibility decision and the selected-only Apply chain. The current user's saved selection and live registry were not inspected.

This is a default/discoverability gap, not missing creation support. Include Add to Archive in fresh recommended defaults and make files/folders applicability explicit. Preserve consciously saved custom selections and explicit Apply/Revert; do not register at startup or launch one operation per selected item.

## Uninstall and runtime storage

- `installer/nxtools-next.iss:366` only conditionally runs sc.exe stop NxToolsIndexerV1. It has no personal-data choice/cleanup, service-registration deletion, native-messaging/Shell revert or protected deployment cleanup. Installed-file records (`:45`) and the startup-value removal (`:76`) do not own runtime databases.
- The exact-owned shutdown script is embedded at `installer/nxtools-next.iss:48` and called by PrepareToInstall, not uninstall. `installer/close-owned-processes.ps1:17` computes installed/protected targets and `:205` owns native image/exit/stop handling. Reuse these identity principles, not upgrade file transactions.
- `rust/crates/nxtools-core/src/paths.rs:17` derives config/data/cache/log roots. `runtime.rs:178` selects default or custom clipboard storage, `:186` owns clipboard-images, `:190` opens clipboard.db, `:200` opens quick-launch.db and `:204` opens downloads.sqlite3. Include app-owned DB WAL/SHM and assets in cleanup.
- `runtime.rs:262` starts the background download owner over the same database; `main.rs:647` routes Background mode there. Active transfers keep this process alive (`runtime.rs:289`), so no visible desktop does not mean the database is unlocked.
- `runtime.rs:219` normally uses USERPROFILE/Downloads and falls back to data_dir/downloads. Configured download destinations may also overlap application roots. Preserve final downloads even inside the application tree; do not delete the whole data root indiscriminately.
- Custom clipboard cleanup targets its DB/sidecars and owned asset subtree, not the selected parent. Custom archive cleanup targets the appended NxTools.Next/archive/sessions subtree and recorded owned roots after actual lease/process shutdown, not user extraction outputs.

Complete uninstall needs a standard remove-personal-data path plus explicit retain choice, exact-owned desktop/background/helper shutdown, owned Shell/native-messaging revert and authorized protected service/deployment removal. Upgrades/repairs preserve data. No real uninstall or cleanup was exercised.

## Indexer initial build

- `rust/apps/nxtools-indexer/src/mft.rs:731` hydrates LastWriteTime for every enumeration batch. `mft/time_metadata.rs:28` performs OpenFileById, GetFileInformationByHandleEx and CloseHandle for each record. `.trellis/tasks/10-14-winshun-w2-w5/w4-outcomes.md:10` already records this added initial/rebuild cost; current native stage durations remain unmeasured.
- `nxtools-indexer/src/lib.rs:1926` builds requested volumes serially at `:1936`; `:2647` sends one BuildCompleted after all volumes, applied at `:2472`. Desktop queries Ready or a nonzero snapshot (`quick_launch/search.rs:344`), so the first completed volume is not searchable while later volumes build.
- `mft.rs:544` persists before publication; `mft/cache.rs:828` sorts/writes/syncs/replaces and exactly reopens mmap. Auxiliary data is built both before persistence and after reopen. Its actual share of latency is unknown.
- Discovery/cache restore (`lib.rs:401`) and the lazy single service runtime (`windows_service_host.rs:57`) do not ordinarily rebuild on desktop reconnect. Valid base plus persisted delta restores directly; historical restoration logs are not current LocalSystem first-build evidence.
- Listary evidence is the managed state/gRPC boundary indexed by `docs/reference-code/quick-launch-research.md:1514`, not native engine source. The reported seconds-level behavior has not been measured in a same-machine controlled comparison.
- Secondary reference SwiftList uses Parallel.For across volumes (`D:\Code\SwiftList-main\Core\Indexer\Usn\Journal\IndexBuilder.cs:31`), 8-MiB sequential raw MFT blocks with FILE-record names/standard times (`Core\Indexer\Mft\MftIndexScanner.cs:71`) and per-volume durable/mmap publication (`Core\Indexer\Usn\UsnIndexerBuildExtensions.cs:110`). These are reference directions, not Listary implementation claims.

Measure scan/construct/persist/reopen/publish with item counts. Replace initial row-by-row native time hydration with sequential MFT metadata extraction once baseline attribution confirms the cost, and publish each durable reopened volume without claiming final Ready early. Preserve true last-write times, scan-start journal cursor/USN replay, versioned base/delta and last-good search. Measure this reduced-IO path before adding multi-volume concurrency.

### Performance and regression gaps

- `tests/performance/indexer-scale-baseline.ps1` uses synthetic MftEntry vectors, not native MFT/metadata opens/full persistence, and does not assert a real first-build threshold.
- Existing failed_first_build_without_a_snapshot_never_publishes_ready, a_slow_initial_snapshot_does_not_block_handshake_or_ping and mapped_publication_ tests protect correctness/liveness/publication, not native first-build latency.
- `tests/performance/indexer-idle-cpu.ps1:72` connects directly from PowerShell, while `windows_service_host.rs:407` admits only an authenticated approved Desktop. `.trellis/tasks/11-06-demand-service-lifetime/outcomes.md:32` requires retaining that Desktop owner for acceptance. Adapt probes to that path before using them as release evidence.
- Current LocalSystem first-build stage logs were unavailable. Authorized first-build measurement and packaged idle/restart/search acceptance remain required. No scan, service operation or Cargo verification was executed.
