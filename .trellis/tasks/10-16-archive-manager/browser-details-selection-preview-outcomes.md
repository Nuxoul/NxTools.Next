# Browser correction — partial implementation, UI/preview NOT complete

2026-10-27. User authorized folders, default details, marquee and internal image/text/code/Markdown
preview. This round is **PARTIAL / NOT COMPLETE**, not a revised smaller scope. Sole current status
remains [phase-a-checkpoint.md](phase-a-checkpoint.md). Installation error1 root cause stays OPEN.
All earlier outcomes/checkpoints retain their historical conclusions.

## Actual retained source

### Default detailed list and compatible migration

- Original ArchiveBrowser constructor now defaults to Details. Display order is Name, Packed size,
  Original size, Type, Modified; semantic column indexes, saved widths and sorting remain unchanged.
- Original header/row resize tracks and horizontal viewport retained. Name minimum240DIP; unknown
  file metadata remains dash, directories retain blank sizes. Existing type presentation uses file
  extension (e.g. XML file), not the internal File enum; no new OS type-description lookup claimed.
- UiState version2, default details=true. Original bounded background loader accepts valid v1:
  missing/false mode promotes to Details and first five visible columns; explicit v1 true preserves
  selected visibility. Both preserve widths, sort/direction, geometry, recent, Tree/comment.
- Original invalid preferences are validated before migration; corrupt/null/future/invalid files
  stay preserved and write-protected. Original writer saves v2 on the next save. Explicit v2 false
  remains a durable optional name-list choice. No whole configuration reset or new writer/cache.

### Rectangle selection

- New browser_marquee.rs is private transient state of the original ArchiveBrowser, not a new
  selection controller. BrowserModel owns final selection/focus/anchor through clear/rectangle/
  restore methods. Parent navigation rows are excluded from rectangle selection and batch output.
- Real empty-body left press/move/release paints a passive theme-derived rectangle. Normal replaces;
  Ctrl XORs against the fixed press baseline, so repeated moves do not repeatedly flip selection.
- Content-space mapping reads original vertical scroll and measured body origin (including actual
  horizontal placement). All projected rows are eligible without mounting them. One virtual blank
  tail row is presentation-only, absent from BrowserModel/extraction.
- Original ListItem row press stops body propagation and retains original held>=4DIP native drag.
  Root release retires candidates; original double-click/keyboard/context/sort/resize/scroll remain.
- Escape/focus loss restores baseline; directory/filter/sort/refresh/source/snapshot/close retire
  obsolete gesture. Bounds/selection state is local, cross-owner behavior stays deferred.
- Low-noise accepted/terminal correlation, outcome/elapsed only; no move/frame/content/path logs.

### Safe directory icon backend — INCOMPLETE UI

- Original TypeIcons now supports one fixed `nxtools-folder-type:directory` key. Existing right-row
  queue can request it; PNG validation,8pending/128resources/15seconds and all stale/close guards stay.
- C# fixed target resolves before member filesystem/shortcut logic; SHGFI_USEFILEATTRIBUTES and
  FILE_ATTRIBUTE_DIRECTORY, same bounded STA/IconCache.Shared. Owned-cache fixture injection only.
- **NOT DONE:** Tree does not yet consume queue_folder_icon/folder_icon_path. Right rows still use
  the old16DIP size and outline fallback. Clear pending/failure visual correction is not complete.
  Passing backend tests is not the user-requested recognizable folder experience.

### Internal preview — NOT DELIVERED

- Scope updated to confirmed images plus text/code/Markdown, excluding PDF/Office/audio/video,
  executable execution and edit/write-back. No repeated scope question remains.
- A background/materialization/purpose/decode/controller/UI draft was attempted but could not be
  connected to browser.rs after its mandatory edit stop. To avoid broken compilation or deceptive
  dead production routes, every new preview production extension was removed again precisely.
- entry.rs, application.rs, archive.rs, Rust archive_browser protocol and C# ArchiveEntrySession
  return to their opening whole-file anchors; original user changes remain. No preview Shell
  fallback, placeholder success callback or alternate runtime was retained.
- Unwired draft snapshot is diagnostic-only in current session scratch
  `archive-browser-correction/unwired-preview-draft/`; it is NOT required release material or a
  tested implementation. It must be reviewed/retested before future use. No preview budget,
  latest-selection, password cancellation, lease or decoder acceptance is claimed.

## Mandatory edit stop / genuine remaining blocker

Browser.rs Edit failures in this turn: EDIT_LINES_UNSEEN on a multi-op batch (28unseen anchors,
including15/92/93/206), then module insertion anchor16, then UI integration anchor2374. After the
third failed edit, no more writes/format/delegate/alternate-tool edits were made to that path.
The earlier successful detail/marquee edits remain. UI integration and browser formatting stop
here under the project rule; this is not a missing user scope decision or capability rejection.
No shell patch/reset/whole-file rollback was used. The stopped preview delegate could not resume;
parent removed only its additions using current anchors. No unrelated dirty files were reset.

## Tests and retained failures

Evidence root: current session scratch `archive-browser-correction/`. Every Cargo used jobs1,
incremental0, RUSTFLAGS=-C codegen-units=256, locked/offline, idle shared-lane check, serial tests.
No Cargo timeout, duplicate build or unrelated process kill.

| Log / actual command | Result / scope |
| --- | --- |
| 01-red-marquee-details.log: cargo test -p nxtools-desktop --locked --offline --bin nxtools-desktop archive::browser::marquee_tests -- --test-threads=1 | 0PASS/2FAIL observed before fix: missing default detailed header and selection remains empty after real body drag |
| 02-marquee-details.log | Compilation failure: missing move capture plus in-flight preview test/API mismatch. NOT behavioral red |
| 03-marquee-details.log | 4PASS/1FAIL; disconnected row-release fixture expected multi-selection retention even though ordinary release legitimately clicks the row. Changed to still-pressed movement assertion; original connected native drag tests retained |
| 04-archive-related.log: cargo test -p nxtools-desktop --locked --offline --bin nxtools-desktop archive:: -- --test-threads=1 | 156PASS/1FAIL; original consent-cancel fixture assumed entirely empty command queue. Narrowed to permit at most one exact read-only .hta32px type-icon command, all materialize/Shell/other commands still fail |
| 10-final-archive.log: same archive:: command | 157PASS/0FAIL/0ignored,1188filtered,48.80s; includes5new marquee/default,5icon,4migration and original native-drag/selection/sort/resize/long-name/minimum/LightDark boundaries |
| 11-strict-tests.log: cargo clippy -p nxtools-desktop --tests --locked --offline -- -D warnings | PASS exit0; test-target strict, not a separate production strict claim |
| 12-workspace-check.log: cargo check --workspace --tests --locked --offline | PASS exit0; not workspace tests/Release/native acceptance |
| 13-full-desktop.log: cargo test -p nxtools-desktop --locked --offline -- --test-threads=1 | PASS1345unit+10integration,0fail/ignored;500.78s+2.68s |
| 06-dotnet-final-build.log: dotnet build csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-restore | PASS0warnings/0errors; prior preview-delegate build failed CS0136 in icon resolver, fixed typeExtension local |
| 07-folder-list.log /08-folder-run.log: dotnet run --project csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-build --no-restore -- --fixtures [--list] "archive folder type icon fixture" | exact LIST2=RUN2=PASS2,skip0/fail0/not_run0 |
| 09-fmt-check.log: cargo fmt --all -- --check | FAIL exit1: changed browser.rs formatting plus unrelated existing page_rem/network formatting. No formatting bypass after browser edit stop |

Other touched Rust files scoped rustfmt PASS. git diff --check PASS (existing CRLF normalization
warnings only). spec-contracts114docs/14owners, catalog79historical/78entries, coherent GPUI0.8.0/
pre0.3.9 and pinned Markdown57file provenance gates PASS again after final documentation changes.
Icon/migration tests were authored before repair but their original implementations were not
executed red; do not invent red evidence from expected failure or baseline copies.

Original task validation PASS (82implement/79check entries),8existing context-size warnings.
Checkpoint original1540-line suffix SHA256 remains
509A3C5900ED201C68214D94CCDB013DBD47CEFF6B9F2CF2A85ECC69B538BCD2,
matching its opening #BCD2 whole-file anchor; all earlier history is retained.
## Files retained for this round

- Desktop archive/browser.rs, browser_tests.rs, browser_marquee.rs, browser_marquee_tests.rs,
  model.rs, entry_tests.rs, browser_icons.rs, browser_icons_tests.rs, ui_state.rs,
  ui_state_migration_tests.rs.
- C# Windows/ShellIconExtractor.cs; Tests/ArchiveTypeIconTests.cs, Program.cs, TestEffects.cs.
- Original task request, selection/layout and icon/migration contracts, this outcome, checkpoint;
  archive reference appended with current correction. No dependency/vendor/catalog changes.

## Still required / NOT RUN

1. Finish Tree/list24DIP recognizable folder PNG/fallback/pending/failure UI.
2. Implement and integrate actual resizable readonly preview, resource bounds and original lease/
   password/latest-selection/retirement lifecycle with red-capable boundary tests.
3. Resolve browser.rs formatting in a later permitted turn; preserve unrelated user formatting.
4. Actual repaired HWND pixels, real GUI/physical mouse/keyboard, held native OLE to Explorer,
   scrollbar/trackpad,100/125/150%/mixedDPI,IME/UIA/provider and user visual acceptance.

No package/install/product startup/real GUI/service/Registry/COM/Explorer operation, signing or
release overwrite, installation fix, commit/push. Current installer/diagnostic package unchanged.
Full test green does NOT complete four requested functions or accept overall browser visuals.
