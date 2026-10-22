# Phase B — Compression module and settings: implementation handoff

2026-10-21. **Scoped implementation delivered; independent acceptance pending. Not full Phase B,
Phase A or A–F PASS.** The single current status owner remains [checkpoint](phase-a-checkpoint.md).
User dependency amendment is in implement.md. No Phase C/D/F work was started.

## Delivered behavior
- Formal six-module order: 下载 → 速启 → 剪贴板 → 扩展 → 压缩 → 其他工具. Six independent
  focus handles; actual existing Sidebar/Button navigation and new Google Material Symbols
  folder_zip SVG. Existing Apache2 notice covers the asset; no proprietary icon reuse.
- Actual production ArchiveSettingsView uses locked Kit0.7.1 Settings/Page/Group/Item/Field,
  built-in search/scroll, default Medium Switch/Input/outlined Save and official theme grammar.
  Six approved pages remain. No Card/Grid, dependency fork, second page/scroll state owner.
- Saveable existing-engine presets: destination/conflict/codepage, create format/semantic level,
  test-after-create/root stripping, exclusion globs and MOTW. Original defaults remain Ask/Ask/
  Auto/ZIP/Normal/true/false/[Thumbs.db,desktop.ini]/true. Gzip/XZ omit Store; TAR level disabled.
  These are defaults only, not a claim that browser/forms/jobs already execute them.
- Selection changes immediately save only archive. Exclusions use an explicit Save and a
  lossless JSON string-array Input (spaces, semicolons and escaped newlines retained). Invalid
  JSON/glob/finite limits show local error; failed persistence retains previous live values and
  draft. No render/search save, fake Saved message, native Apply or Shell registration.
- AppConfig.archive defaults only for the wholly absent legacy field. A present section requires
  valid typed fields; null/malformed/missing/invalid fields fail without default replacement.
  update_archive_settings reuses original lock/reread/merge/sync/rename and preserves other owners.
  normalize does not repair archive corruption. History/window geometry are not added to config.
- MainWindow retains the view/InputState/saved values across module switches. Official Settings
  resets to General after unmount; no invented controlled-selection API or duplicate controller.
  Callbacks defer weak owner work to the next application turn.
- Save diagnostics: archive.settings.save_started/save_finished, one UUID, outcome/elapsed/fixed
  safe reason only; no paths/rules/config payload. No per-render/frame logging.

## Not added / honest scope
Browser-related open/recent/column/tree/comment, progress/operation forms, scheduling, cleanup and
resource interfaces remain readonly unavailable information, not operational switches. Shell
association/context menus explicitly say default off/not connected; no Apply action. Design§5 has
strategy destination, not a fixed-directory field, so there is no necessary settings picker. No new
Protocol/C# picker or archive picker cancel/stale test is claimed. Original typed picker/STA seam is
unchanged; its compatibility/cancellation tests were run. Open/save/source pickers stay with C/D.
No new generic framework/runtime/history DB, retry or system config modification.

Core now directly reuses already locked globset=0.4.19/default-features=false (MIT OR Unlicense),
the engine's existing GlobBuilder grammar; no version upgrade/new resolved package. Bounds32rules,
256characters/rule and aggregate8192UTF8bytes; invalid relative-path/glob syntax rejected. Lockfile
adds only Core's edge to the already existing package. Existing notices cover the dependency.

## Actual verification (serial Cargo, offline)
All Rust commands inherited jobs1/incremental0/RUSTFLAGS=-C codegen-units=256; one machine lane.
No100k/large input/long-path/performance/all-codec fixture was rerun.

| Check | Result |
| --- | --- |
| Core full library |69pass/0fail/0ignored,exit0; includes9archive filesystem tests |
| Desktop scoped production settings |4pass/0fail,exit0; save/reload/failure/return/search/no-write/real synthetic pointer+KeyDownUp/scroll/default-minimum×LightDark/lossless patterns/stream choices |
| Desktop full final |871pass/0fail/0ignored,exit0 |
| Existing actual navigation focus/geometry subset |2pass/0fail,exit0; six handles and one release dispatch |
| Protocol full library |54pass/0fail/0ignored,exit0, including unchanged generic directory picker DTO |
| C#build |exit0,0warnings/errors |
| C#exact LIST then same RUN |2selected/run/pass,0skip/fail/not_run,exit0: canceled STA dialog drops its late result; generic directory picker preserves title and current directory |
| Core/Desktop strict Clippy --tests |exit0,-Dwarnings,final rerun after review fixes |
| Workspace cargo check --tests |exit0 |
| cargo fmt --all -- --check / final Debug build |both exit0 after review fixes; Debug has2existing dead_code warnings |
| Source gates |desktop-shell-lifecycle,ui-state-lifecycle,ui-token-boundaries,ui-consistency,ui-preview-isolation all exit0 |
| Documentation gates |spec-contracts/catalog/task validation exit0; native-contracts and archive-reference injection-size warnings recorded, sources read directly |

Red evidence retained: six-entry navigation failed before implementation (1failed); malformed glob
validation failed before correction (1failed); lossless exclusion regression failed before review
fix (1failed). Initial fullDesktop869/1failure was an old five-entry Tab assertion; updated for the
approved Archive→Tools order and final871all pass. First page return assertion incorrectly expected
Kit internal page retention; actual unmount behavior was documented and saved-state assertion kept.
One optional bounded review only: two concrete findings above fixed and production regression green.

### Genuine remaining check blockers
**Production-only strict Clippy fails** (exit101), unrelated pre-existing code unchanged:
- clipboard/extension_adapter.rs:275 extension_adapter dead_code;
- clipboard_worker.rs:431/453 query_category_page_async/query_category_page_async_with_cursor dead_code;
- ui/components/project_menu.rs:153 needless_return.
--tests Clippy passing does not make this production command green. Debug build succeeds with two
existing dead_code warnings. No broad allows or unrelated fixes were made.

## Actual page / native evidence limits
The existing Debug-only --ui-preview foundation entry now has NXTOOLS_PREVIEW_ARCHIVE_SETTINGS,
NXTOOLS_PREVIEW_THEME=light|dark and NXTOOLS_PREVIEW_ARCHIVE_MINIMUM. It creates only GUID-owned
PI_SCRATCH_DIR config and actual MainWindow/ArchiveSettingsView; runtime/capture/indexer/platform
remain false. No second mock page or official installed product startup.

An earlier compiled production page printed ready at96DPI/scale1 and stayed alive during an owned
30second wait. The launching shell's child did not remain available between tool calls; no crash
cause is established. Computer Use window lookup found no target, so **no screenshot was obtained
or inspected and no real native pointer/keyboard/scroll action was delivered**. An explicit scratch
preview launcher was rejected by Computer Use as blocked app(self); the safety limit was respected,
not bypassed. Default/minimum Light/Dark live checks, physical/DPI/provider and user visual acceptance
remain pending. The compiled production TestWindow four-way bounds/actions tests are separate and
must not be described as screenshot/user/native acceptance.

## Stop boundary / evidence
Raw commands/logs/red-green evidence remain in session scratch163f435b-1ee5-4c07-8102-31a2c7441b9b:
phase-b-check.ps1, phase-b-check-results.json, phase-b-*.log and archive-*.log. The C# test harness
completed; no archive codec worker or formal PlatformHost runtime was launched. No Cargo/preview process remains.
Temporary launchers/configs remain scratch. Final scoped git diff --check exits0.
No packaging/install/signing/formal startup/registry/service/source recycle/delete/commit/push.
Phase A remaining other-format risk/budget/readmission, trusted UNC and strict source recycling
are unchanged; public prompts/firstpaint/scheduling/Shell/deployment retain original C/D/F/native
ownership. Stop for independent acceptance; do not continue Phase C or add research/test matrices.
