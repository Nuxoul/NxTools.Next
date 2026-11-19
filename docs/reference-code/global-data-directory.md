# Global software data directory

## Source review
Legacy `D:\Code\NxTools\Views\SettingsWindow.axaml.cs`: constructor attaches WindowAnimationService; OnOpened calls the Avalonia base then ConfigureForCurrentScope; SaveButton_Click executes ApplySettingsCommand then closes; CloseButton_Click/OnClosed execute CloseSettingsCommand. The Avalonia native base implementation is unavailable. The legacy settings surface does not implement a global software-data relocation command; this is a new capability.

Current entry chain: `rust/crates/nxtools-core/src/paths.rs::AppPaths::discover` is shared by Desktop, CLI and BrowserHost. DesktopRuntime::initialize opens ClipboardHistory through the serialized worker, FinderCatalog, DownloadEngine and ExtensionService. `main.rs::run` exits GPUI, drains downloads, extensions and Clipboard, then drops runtime. Global Settings uses one official Settings compound and an atomic Apply/Cancel owner. `PickDirectoryCommand` carries the source HWND/process; C# PickerOwner and the original directory picker own native selection and correlated result.

Clipboard's `ClipboardHistory::migrate_storage` in `rust/crates/nxtools-clipboard/src/storage.rs` uses a SQLite backup, copies managed assets, rewrites image/source-icon/native-format paths and checks the resulting database before publication. Reuse this boundary before copying remaining data. Do not rewrite Clipboard contents, external file lists, download artifact paths or independently configured storage.

## Relocation invariants
- Retain the existing default config/data/cache/log layout until explicitly changed.
- Custom roots contain config and data subdirectories. Fixed runtime cache/locks/logs/native registration/indexer data remain at existing system paths; one small fixed bootstrap locator selects the active root for every Rust process.
- Native picker stays owned by the original Settings HWND; cancellation, stale result, close and disconnect cannot initiate relocation. Cross-entity callbacks are deferred.
- Reject nonabsolute/nonlocal paths, drive roots, nonempty targets, installation directories and overlapping source/destination trees. Validate write access before exiting.
- Copy only after normal feature drain; a migration lock blocks new startup discovery during publication. Never publish the locator before data and configuration are complete.
- Keep source data as the last-good copy. Failure leaves the old locator authoritative. Restart reopens the selected committed root and reports failure honestly.
- No window placement, primary activation, preview focus or native show/hide changes. No new IPC command or native picker implementation.

Installed extension code directories and approved internal EXE runtime paths use the canonical Windows source root when rebasing. Preserve the existing approval/content identities and external runtimes; only paths to copied software-managed files move. A validation request retains its original ID/window through background validation; disconnect and timeout retire that ownership before a delayed result can request shutdown.
