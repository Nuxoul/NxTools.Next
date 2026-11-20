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
`ThemePreference::default`, AppConfig defaults and initial Desktop UI preference are System. Explicit saved choices remain authoritative. `SystemSettings.ui_font_family` defaults to `DEFAULT_UI_FONT_FAMILY` (`Sarasa UI SC`); the original global transaction validates and saves it, then defers presentation publication. A missing available family retains saved intent but renders with the bundled default. Searchable official Select lists system/private English/Chinese families; no CSS fallback list, font-size/palette change or mono-font rewrite. Tests: `repo:rust/crates/nxtools-core/tests/global_theme_defaults.rs`, Core font settings tests and Desktop font draft/theme/paint regressions. Reference: [UI fonts](../../../../docs/reference-code/global-font-settings.md).

## Scenario: Bundled Sarasa Regular default
### 1. Scope / Trigger
User-approved private Regular-only font bundle and default selection. Evidence: `repo:rust/apps/nxtools-desktop/src/assets.rs`, `repo:rust/apps/nxtools-desktop/src/main.rs`, `repo:rust/crates/nxtools-core/src/lib.rs`.
### 2. Signatures
`DEFAULT_UI_FONT_FAMILY: &str = "Sarasa UI SC"`; `default_ui_font_family() -> String`; unchanged `SystemSettings.ui_font_family: String`; Desktop `assets::register_fonts(&App) -> anyhow::Result<()>`.
### 3. Contracts
Embed only unmodified SarasaUiSC-Regular.ttf1.0.43 (24,049,996bytes, family Sarasa UI SC, Regular400) as borrowed static bytes. Register through released TextSystem::add_fonts before Kit init, theme application and window creation. This uses the existing DirectWrite private collection, not Windows font installation. Include full copyright/OFL1.1 in assets and original About license scroll. Existing official Medium32 searchable Select labels the family as default; no new control, font size, theme or monospace policy. Explicit saved families remain authoritative; no config rewrite or migration flag is introduced. Missing preferences use the new default. Registration emits one correlation/start/terminal success/elapsed pair with count, not font data/path/catalog text. No new IPC or environment key.
### 4. Validation & Error Matrix
Missing preference -> Sarasa UI SC; explicit saved family -> unchanged; invalid new selection -> original validation error; unavailable saved family -> preserve intent/default rendering; registration error -> failed lifecycle event and fixed startup failure before windows show. There is no partially initialized font-dependent UI.
### 5. Good/Base/Bad Cases
Good: a new computer uses the embedded default without a system font install. Base: explicit Microsoft YaHei UI or another selected family survives roundtrip. Bad: ship every weight, resolve a private face from the user's temporary directory, overwrite explicit choices, or apply its family before registering bytes.
### 6. Tests Required
Core ui_font_settings_tests covers new/missing defaults, explicit old/English/Chinese choices and atomic transaction failure. Desktop assets tests cover borrowed embedded data, one-face catalog, full license and public registration call; font draft/theme/paint/About tests retain original behavior. `python tests/architecture/bundled-ui-font.py` checks actual TTF family/style/weight, source startup ordering and OFL wiring. The default GPUI TestPlatform uses NoopTextSystem; passing its registration call is not native DirectWrite or visual acceptance. Scope limits belong in the task verification.
### 7. Wrong vs Correct
Wrong: change only a family literal, load from D:\Temp at runtime or add a font-install command. Correct: embed original Regular plus OFL -> register static bytes privately before UI use -> apply new Core default while preserving explicit preferences.
