# Release38 archive drag and automatic preview repair

## Behavior boundary

Normal files, explicit/implicit/empty directories and supported multi-selection use the original held-pointer gesture, immutable source/snapshot/ordinal, connection-locked command admission and one C# OLE/async-content session. Optional encryption metadata and unknown sizes do not by themselves prohibit drag-out. Confirmed encrypted files retain explicit source-bound preauthorization; directory descriptors never request content. Actual per-file/aggregate output accounting and original cancellation/lease retirement remain authoritative.

Windows descriptor names are presentation paths, not extraction authority. Resolve representable case/name/file-directory conflicts in the frozen descriptor tree while preserving every regular-file ordinal. The original worker still reads original member paths into separate owned leases. Unrepresentable member names, links/special kinds, stale identities and finite limits report the actual member and specific restriction; they never silently omit a selected file. Whole-archive extraction collision policy is not relaxed by single-member browser reads.

Preview is automatic on supported single-file selection. Historical `show_preview=false` cannot disable it. No toolbar, View-menu or pane-close enable/disable action remains. Original selection generation, preview controller, cache/lease owner and passive rendering remain; selection replacement, unsupported selection, tab suspension, source invalidation, close and cancellation retire old presentation and reject late results. Preview does not activate a window or move list focus.

Encoded archive-source admission, selected expanded output, codec work/spool allowance and final text/image decode limits are separate quantities. An 80-byte supported UTF-8 member remains previewable when its containing archive exceeds the text/image output limit. Genuine refusal identifies the exhausted stage rather than claiming the selected file is oversized.

The download row's original official PopupMenu builds existing capability-driven actions first and appends the existing single-selection View Details callback last. No leading separator, alternate action entry or enablement change.

## Reference and component ledger

Read `docs/reference-code/archive.md`, `archive-virtual-drag.md`, `download.md`, the original browser preview contract and Release38 installed-upgrade outcomes. Legacy NxTools `DownloadCompletedWindow.axaml.cs:78` -> `WindowsFileDragService.cs:29` is a pressed real-file OLE reference, not a virtual-content implementation. Archiver `gui/src/main.rs:115` -> callbacks -> `ui.run:158` and `view_item:697` -> selected extraction -> success-only open do not supply a reusable safe cache or preview controller.

ABDM desktop `HomeComponent.kt:272` retains selection by task ID before opening item options. Legacy NxTools `MainWindow.axaml.cs:1335/1452` retains the original context-request/menu command route and ends with properties. Current Next `download_page.rs` owns the official menu and deferred `open_download_detail` callback.

UI shape remains the original browser split pane and official ListItem/Tree/TabBar. Locked Kit/Component0.8.0 `PopupMenu::item/separator` preserves menu identity; no new menu owner. Preview uses the existing `h_resizable/resizable_panel`, passive MarkdownDocument/image and official Empty/Spinner; remaining source/retry Buttons are Small outlined outside Toolbar.child's ghost/compact adaptation. No vendor, dependency, native focus or window-placement change.

## Regression boundaries

- Production ListItem down/move -> original platform queue: nullable metadata, warning-only conflict, repeated/case-colliding file names, all original ordinals and no eager content read.
- Original native ZIP read -> source binding -> virtual content demand -> real worker/native IStream: each colliding member returns its own bytes and original leases retire.
- Original preview request/worker plus tiny text in a larger source: selected output and source admission remain independent; true output/decode limits remain bounded.
- Production browser restored false -> automatic current selection, no opt-in controls, unchanged list focus, tab retirement and late-result rejection.
- Production secondary-click menu -> details after normal actions -> original detail scope; multi-selection and capability behavior retained.

## Causes and final implementation

1. Drag admission treated missing optional encryption facts, unknown size and OutputCollision as
   whole-selection refusal. File selection and downstream codec metadata could also conflate names.
   Freeze files by original ordinal and directories by original subtree; resolve descriptor names
   only. Fold original directory keys consistently so file a plus A/x and a/y has one renamed parent.
   The fresh browser-member ExtractionPlan alone permits namespace collisions; general extraction
   still rejects unsafe/colliding outputs. ZIP and 7z content remain original-source/ordinal reads.
   7z callback association follows actual block/empty-entry traversal, never a filename HashMap.
2. Download View Details was constructed before normal actions. The original single-selection
   callback is now appended last; capabilities, disabled state and normal actions are unchanged.
3. The reproduced small-member failure is compressed TAR: a real 80 B member coexists with 128 KiB
   other content, while outer expansion incorrectly used the selected text's 64 KiB limit for the
   entire intermediate TAR. The original finite metadata/spool allowance now owns that intermediate
   stage; selected output and decoder budgets are unchanged. An 80 B ZIP member in a source larger
   than 8 MiB already worked. Native source resource failures retain their typed cause through runner
   and cleanup instead of becoming source_unavailable/io_failure; UI separates read, file bytes,
   text lines, image dimensions, decode memory and document presentation limits.
4. Preview enablement state and all enable/disable controls are removed. Legacy off fields are
   ignored by the actual loader/writer. Existing selection generation/controller/cache/deferred
   lifecycle supplies automatic preview. Name-only rows track the actual remaining viewport rather
   than retaining a 240 DIP clamp after preview allocation. Official EmptyTitle's public child carries
   the visible title's semantic label; no extra focus or controller is introduced.

## Verification (2026-10-30)

| Evidence class | Executed boundary | Result |
| --- | --- | --- |
| Simulated production GPUI/model | Original row/Tree down/move -> platform command; complete selected ordinals, historical off, preview states/focus/cancel/tab/close, real viewport bounds and download right-click/details | 347 related archive/download-page tests passed, zero failed/ignored |
| Isolated native / real worker | Original metadata/source/Job/output/cache/native COM IStream; duplicate ZIP/7z names, two 7z blocks with interleaved empty files and reordered descriptors, unknown size, remaining output, async marshal/End/cancel/abort/disconnect | LIST12/RUN12/PASS12/SKIP0/FAIL0/not_run0 |
| Isolated preview / real worker | 80 B ZIP and TAR text, real 128 KiB selected-member refusal with no path, original source-admission resource failure and preview-no-Shell authority | Included in the native12; exact 80 B output and true resource_limit preserved |
| Compiled engine integration | Original unsafe paths/collisions, source identity, actual stream/CRC, finite budgets and general extraction plan | 15 engine + 5 extraction-plan tests passed |
| Build/lint/format | Matching isolated-codecs Debug worker; C# tests build; strict Archive and Desktop/tests Clippy; 22 scoped Rust files | Passed; C# zero warnings/errors |
| Source/document contracts | spec-contracts.py, gpui-release-contracts.py, markdown-editor-contracts.py and scoped git diff --check | Passed; documentation/provenance only, not product acceptance |

The regression fixtures were executed before their fixes: ordinary nullable/conflicting drag,
historical preview-off and menu ordering failed; native same-name ZIP and 7z demand failed; the
80 B compressed-TAR read failed; the directory-parent regression observed two parents instead of
one. Corrected behaviors pass with the original owners and constraints, not changed expectations.
Scoped logs: release38-behavior-red.log, release38-native-drag-red.log, release38-native-7z-red.log,
release38-directory-red.log, release38-final-native.log, release38-final-desktop-related.log,
release38-final-archive-contracts.log and release38-final-*-clippy.log in the task session scratch.

Reproducible filters:
```text
cargo test -p nxtools-desktop --bin nxtools-desktop -- archive:: main_window::download_page:: --test-threads=1
cargo test -p nxtools-archive --features isolated-codecs --test engine_contract --test extraction_plan_contract -- --test-threads=1
dotnet run --project csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-build --no-restore -- --fixtures "archive Phase F virtual tree" "archive preview fixture" "archive Phase F virtual async"
```
Every Rust command uses CARGO_BUILD_JOBS=1, CARGO_INCREMENTAL=0 and RUSTFLAGS=-C codegen-units=256;
one shared lane and the original 6 GiB commit-headroom preflight. Native fixtures require the freshly
built matching Debug isolated-codecs worker via NXTOOLS_ARCHIVE_WORKER_FIXTURE.

## Actual cleanup inventory

| Deleted / replaced item | Current coverage / disposition |
| --- | --- |
| rust/apps/nxtools-desktop/src/archive/virtual_drag_spike.rs and its archive.rs test module registration | Removed obsolete disconnected spike tests; keep production row/Tree/IPC/native demand and async lifecycle regressions |
| tests/architecture/archive-phase-f-spike.py | Removed obsolete string gate that required the faulty generic admission; no current script/build or reference-index call remains |
| Empty untracked root Program.cs | Removed unused temporary stub; real C# test Program.cs remains and describes ZIP/7z fixture effects |
| Preview toggle/menu/pane-close-only tests and redundant first-open coverage | Removed with the deleted enablement feature; retain pointer/keyboard generation, passive focus, split sizing, decode and independent lifecycle cases |
| Historical-off/toggle-close assumptions | Replaced by actual legacy loader/save/reopen automatic-preview, tab/page retirement and stale-result assertions |
| Unknown-encryption/conflict blanket rejection assertions | Replaced by complete original-ordinal/name-resolution tests; unsafe path/link/kind/source/finite-budget negatives remain |
| Hidden-preview-only viewport setup and fixed wide name-lane assumption | Actual automatic-preview viewport, visible target intersection and measured width assertions |

No current build/test registration or active source documentation points to the deleted files.
Historical task evidence mentioning spike work, installer recovery records, user data, references,
real failure logs, wire compatibility alias and valid native/password/async tests are retained.
No new temporary diagnostic probe remains in the production changes.

## Contracts and limitations

Owner specs: product-read.md in the Archive backend; ui-experience-contracts.md and its Desktop
index; the shared UI design amendment. Reference call chains are in archive-virtual-drag.md and
download.md; this record is linked from the archive task index.

The original reported archive and exact selection were not supplied. Fixtures establish reachable
causes, not the original archive's format. No physical Explorer/foreign-target drag, actual held
HWND/capture gesture, installed-product interaction, mixed-DPI/Windows focus or user visual
acceptance was performed. Controlled COM consumers do not certify those rows. No package,
installation, installed-product launch, commit/reset or HZ5 work was performed.
