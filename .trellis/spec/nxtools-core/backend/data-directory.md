# Software data location and appearance preferences

## Scenario: Global software data relocation

### 1. Scope / Trigger
Global Settings Data page selects an empty local folder and immediately requests migration plus restart. Default paths remain unchanged until an explicit successful migration.

### 2. Signatures
`AppPaths::discover/default_locations/at_root/validate_data_destination/begin_data_migration/commit_data_location`; Desktop `DataMigration::prepare/execute` and `NetworkSettingsApplication::complete_data_validation`.
Evidence: `repo:rust/crates/nxtools-core/src/paths.rs`, `repo:rust/apps/nxtools-desktop/src/data_directory.rs`, `repo:rust/apps/nxtools-desktop/src/network_settings/data_directory.rs`, `repo:rust/apps/nxtools-desktop/src/main.rs`.

### 3. Contracts
One fixed runtime-cache locator selects config/data subdirectories for every Rust process. Cache, instance locks, logs, native registration/signing material and LocalSystem indexer storage retain their system paths. Discovery refuses a missing selected configuration instead of silently recreating defaults. An exclusive migration lock blocks fresh discovery through drain/copy/commit.

The original owned native directory picker carries one request ID and source HWND. Retain current request/window ownership through background validation; cancellation, timeout, disconnect and old results cannot request shutdown. Official Medium outlined Button and original Settings hierarchy own presentation; no new native picker or focus controller.

Normal shutdown drains downloads, extensions and Clipboard before copying. Reuse ClipboardHistory migration for managed image/icon/native-format references. Copy remaining config/data without merging nonempty targets; canonical-source rebasing updates installed code directories, approved internal EXE paths and internal runtime paths without changing approval identities. External files/runtime locations and independently configured Clipboard storage stay independent. Commit the locator only after destination writes succeed; source remains a last-good copy. Restart reports actual success/failure through the original notification owner. Unapplied settings drafts are not saved by directory selection.

### 4. Validation & Error Matrix
Relative/network/root/installation/system/overlapping/nonempty/unwritable target -> refuse before shutdown. Resource drain/copy/config/locator failure -> no new location, source retained. Missing selected directory -> actionable startup error, no default reset. Retired validation result -> release its resources without changing newer UI state.

### 5. Good / Base / Bad Cases
Good: migrate A to B then C, including an approved staged EXE, with unchanged runtime locks and valid new package/Clipboard references. Base: unconfigured startup retains the existing Windows directory layout. Bad: live-copy SQLite, publish before durable writes, compare verbatim package paths with a noncanonical root, or commit a validation result after disconnect.

### 6. Tests Required
Core paths tests cover default/custom resolution, missing selected configuration, migration admission and destination rules. Desktop tests cover real temporary Clipboard assets, two migrations, actual staged EXE approval, failed copy/locator publication and production Settings picker cancellation/stale validation. Compiled fixtures do not establish physical picker/restart or installed-data acceptance. Reference: [global directory](../../../../docs/reference-code/global-data-directory.md).

### 7. Wrong vs Correct
Wrong: move only the database file or replace the active path before all writers drain. Correct: retain original ownership -> validate -> drain -> migrate managed references and copy -> sync configuration -> commit locator -> restart.

## Appearance preferences
`ThemePreference::default`, AppConfig defaults and initial Desktop UI preference are System. Explicit saved choices remain authoritative. `SystemSettings.ui_font_family` defaults to `DEFAULT_UI_FONT_FAMILY`; the original global transaction validates and saves it, then defers presentation publication. A missing installed family retains saved intent but renders with the default. Searchable official Select lists installed English/Chinese families; no CSS fallback list, font-size/palette change or mono-font rewrite. Tests: `repo:rust/crates/nxtools-core/tests/global_theme_defaults.rs`, Core font settings tests and Desktop font draft/theme/paint regressions. Reference: [UI fonts](../../../../docs/reference-code/global-font-settings.md).
