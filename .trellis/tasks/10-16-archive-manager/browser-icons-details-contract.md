# Browser safe folder-type icons and Details v2 migration

## Parent convergence status — partial, UI integration blocked

Parent actual checks: original archive-related run156PASS/1FAIL (activity-consent queue assertion;
later narrowed to permit only one read-only .hta type-icon request), C# final build0warnings/0errors,
exact LIST2/RUN2/PASS2/skip0/fail0 for the two folder fixtures. New migration/icon Rust tests passed
inside that archive run. Not an observed red for these nine tests. Source gates passed separately.
Browser.rs hit the required three-Edit-failure stop: Tree folder PNG,24DIP/filled fallback and
preview pane integration are NOT implemented. Right list requests the new directory type through
its existing queue, but keeps old16DIP size/fallback. Do not interpret the intended parent render
recipe below as completed source. Full visual/user acceptance remains NOT ACCEPTED.

2026-10-27. Scoped source implementation; compiled/red/green/native/visual acceptance is **NOT RUN** by this subagent. Parent owns browser/render/application integration and the single Cargo verification lane. No package, install, product launch, Registry/service changes or commit.

## Authority / reference evidence

Read AGENTS; current UI component selection/release/table/index; archive reference; browser-details-selection-preview-request; historical render/name outcomes; applicable shared/Desktop/PlatformHost contracts. The later user request supersedes historical default-name-mode behavior, not retained preferences or native lifecycle.

Rechecked original reference sources: `D:\Code\NxTools\Services\BrowserExtensionPackageService.cs:220-235` is ZIP package creation and `Services\FfmpegInstallerService.cs:87-103` installs FFmpeg; neither is a general archive browser. `D:\Code\Archiver-main\gui\src\view.rs:107-147` separates parent/folder/type presentation and directories-first sorting. `gui\src\main.rs:115-159` constructs AppWindow, wires callbacks, runs Slint and removes its shared temporary root; that unsafe shared cleanup is not copied. No proprietary Bandizip source was inspected/reused.

Released Component0.8.0 `vendor/gpui-component-080/src/icon.rs:104-138,175-190` verifies public Icon::new/path/data and size behavior. Existing official Tree/ListItem/uniform_list and original native owners stay. This slice introduces no painted controls/variants/parent adaptations, window, focus or selection owner. Intended parent work is24DIP recognizable folder PNG/filled fallback in32DIP rows; that render integration is BLOCKED/NOT DONE as recorded above. Missing fallback review is not a capability gap.

## Integration APIs / invariants

- `rust/apps/nxtools-desktop/src/archive/browser_icons.rs:27-41`: directories now map to exactly one `nxtools-folder-type:directory` key, regardless of names/extensions. Parent/warning rows still do not request types. Original safe ASCII-alphanumeric <=16-byte extension targets stay `nxtools-file-type:.ext`.
- `browser_icons.rs:53-65`: existing `icon_path(&Row)` / `queue_type_icons(&[Row], cx)` support directories. New `folder_icon_path() -> Option<&PathBuf>` and `queue_folder_icon(cx)` let Tree/root reuse that exact cache. `None` means keep official fallback for absent/pending/failure/offline/budget states; no separate status API is needed for that paint rule.
- Queue work stays deferred; cached completion does not synchronously call another entity. Original TypeIcons owns cache, <=8 pending, <=128 total resources, 15-second deadline, generation/snapshot/epoch/retirement/disconnect guards and background PNG validation. No member path, extraction, executable inspection, shortcut resolution or Shell-open command is introduced. Parent calls queue_folder_icon while rendering Tree and uses folder_icon_path for its root/children; row renderer uses icon_path before fallback.
- Existing `archive/application.rs:1172-1185` routes icon receipts on the next app turn. No application/protocol change is required for the folder target.
- `csharp/NxTools.PlatformHost/Windows/ShellIconExtractor.cs:178-198`: fixed reserved target resolves before environment expansion/filesystem/shortcut logic to `ShellIconTarget("nxtools-folder", null, true, FILE_ATTRIBUTE_DIRECTORY=0x10)`. Extension types are explicitly validated and resolved directly; malformed reserved targets fail closed.
- `ShellIconExtractor.cs:78-84,151-165`: attribute-only targets never call File.Exists/Directory.Exists/GetLastWriteTimeUtc on the target; SHGetFileInfo receives SHGFI_USEFILEATTRIBUTES plus fixed directory attributes. Cache-file IO remains necessary/owned. No native directory member probe, .lnk COM resolution, Shell launch or archive materialization. Native failures now populate NativeErrorCode rather than only interpolating the code into error copy.
- Production still uses IconCache.Shared and original bounded STA. Optional internal `Extract(command, cancellationToken, IconCache? cache = null)` permits only fixture cache injection; tests never reach production cache. Public callers/typed IPC stay unchanged.
- Diagnostic chain stays `archive.type_icon.started/finished` -> original correlated IPC -> `shell.extract_shell_icon.started/completed/failed`, elapsed/native code at the owning boundary. No paths/member names/icon payloads/frame logs are added.

## UiState v2 compatibility

`rust/apps/nxtools-desktop/src/archive/ui_state.rs:18,79-104,404-426`:

1. `pub(crate) const VERSION = 2`; UiState::default details_view=true, original five primary columns visible.
2. details_view remains the existing optional field, no new required fields. Absent v2 mode uses true; explicit v2 false persists. Null still rejects.
3. Original bounded typed loader accepts valid v1, reads the legacy optional bool with its original false default, sets version2, validates the original preferences **before** changing visibility, then promotes absent/false legacy mode to details=true and visible[0..5]=true. Name-hidden/invalid width/geometry/recent/version/corrupt documents are not repaired/overwritten.
4. v1 already explicitly details=true keeps its selected column visibility. Both migration cases preserve widths, sort/direction, geometry/maximized, recent, tree/comment and all other existing fields. No full reset.
5. Migration is in-memory on the original background loader; the next original enqueue_save/flush writes v2 through existing sibling-temp/sync/rename. Read alone does not overwrite v1 bytes. Without a save a future load can derive the same migration again; once v2 persists, explicit false is never re-promoted.
6. Parent must default ArchiveBrowser::new details_view=true (prefer UiState::default().details_view). `snapshot_from_observed` already uses `..UiState::default()`, so version2 is automatic. Explicit struct constructors must use VERSION or Default, not hardcode1. No new required field breaks callers.
7. Parent updates obsolete default-name-list browser assertions / absent-mode serde assertion in browser_tests.rs; original v2 true/false durable round-trip coverage stays. This subagent did not edit browser.rs/browser_tests.rs/application.rs/entry.rs/protocol archive_browser.rs.

## Tests authored before production repair

`browser_icons_tests.rs` (included from browser_icons.rs under cfg(test)) adds five tests: safe shared directory key; real ArchiveBrowser deferred command/dedup/validated PNG/cache hit; generation/snapshot/epoch/close/disconnect stale completion; failed-result fallback/no-redraw-spin/epoch retry; original pending8/types128/15-second deadline budgets. No native worker/Shell runs in Rust fixtures.

`ui_state_migration_tests.rs` (included from ui_state.rs under cfg(test)) adds four tests against actual load/UiStateStore/writer/reopen: v2 defaults/optional field; v1 absent/false migration with exact full-preference retention and subsequent durable v2 false; v1 explicit details column retention; corrupt/null/future/invalid existing byte preservation and save refusal.

`csharp/NxTools.PlatformHost.Tests/ArchiveTypeIconTests.cs` adds two tests. Pure reserved-target/attributes/actual flags checks plus real Windows directory HICON->PNG/STA/repeated cache reuse/cancel using GUID-owned injected IconCache. Exact registrations are Program.cs:499-500; TestEffects.cs explicitly classifies both function symbols Fixture with reviewed scope. No broad name heuristic or old restricted-test reclassification.

### Honest red baseline handoff

No Cargo or C# test/build was executed here. Tests were authored before fixes; **expected** failures are not claimed as observed red:
- Original key excludes directories; row-only folder command test receives no request.
- Original UiState default is version1/name mode and loader has no migration; v2 defaults/migrated snapshot assertions fail.
- Original native resolver returns null for fixed folder target; directory-attributes assertion fails.

Baseline copies are at `C:\Users\Administrator\.pi-desktop\scratch\2b4b441f-ea46-480a-b5b5-84691667d060\browser-icons-details-baseline\`. Exact original production files plus original Program.cs/TestEffects.cs are retained. `browser_icons.red.rs`/`ui_state.red.rs` are original implementations with new test includes; `browser_icons_tests.red.rs` is the four initial tests using only pre-existing APIs (no new Tree APIs), so parent can construct an isolated baseline without a missing-symbol pseudo-red. `ui_state_migration_tests.red.rs` is the initial same-boundary migration suite. Do not roll back the dirty workspace; use an isolated source overlay/copy and keep tests. C# test code additionally needs new flags/cache seams: for a meaningful parser red retain those seams but revert only fixed-target dispatch, or use only the existing ResolveIconTarget assertion; missing methods do not count as behavioral red.

## Verification / reproducible commands

Passed here: scoped direct rustfmt (edition2024/skip_children=true) and --check; scoped git diff --check (only existing Program.cs CRLF normalization warning); Python gpui-release-contracts, markdown-editor-contracts, spec-contracts (114docs/14owners). These prove parsing/format/source/provenance, not Rust/C# compile or behavior. No catalog/vendor/dependency edits.

Parent's Rust lane (jobs1/incremental0/codegen256, locked/offline, serial tests) should run focused browser::icons::tests and browser::ui_state::migration_tests, then integrated browser/application/drag suites and strict Desktop test lint. These are new tests, not results.

C# commands after coordinating the build lane (NOT RUN here):

```powershell
dotnet build csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-restore
dotnet run --project csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-build --no-restore -- --fixtures --list "archive folder type icon fixture"
dotnet run --project csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-build --no-restore -- --fixtures "archive folder type icon fixture"
```

Expected selection2; require actual RUN/PASS counts and no skips/failures before claiming fixture green. This invokes the test harness only, not product/adapter/global hooks. Native folder/icon extraction fixture is not real product HWND, physical input, mixed DPI, screenshot, user approval or installed/release acceptance.
