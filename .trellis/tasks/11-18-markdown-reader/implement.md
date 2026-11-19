# Markdown Reader Implementation Plan

The reader stages share one owner. Add a red-capable test at each production boundary before changing its behavior.

## 1. Reading

- Add the Other Tools tile, one activatable reader window and the document/tab owner (`main_window/tools_page.rs`, Desktop `tools/markdown_reader`); reuse the caption and window-root helpers. Write real tile → window → reopen/cancel/close tests and verify resize, focus and no reentrancy with a cached document.
- Implement native open selection at the existing PlatformHost modal STA boundary, typed request/results and desktop routing. Add open/drop, duplicate tab, relative image, outline, committed-text find and external reload. Tests include Chinese and duplicate heading matches, normal editor atomic rename saves, stale watcher delivery, missing files, and preservation of the last-good viewport.
- Finish with one reader scroll owner, local-image policy, theme/reading settings and minimum-window geometry. Keep all unrelated preview paths unchanged.

## 2. Editing

- Add production header behavior tests before the edit entry: both named buttons are in the preview title bar, visibly outlined, accessible at minimum width, single-dispatch by click and keyboard activation, and the two modes have no F2/F3 bindings. Verify caption drag/control and post-mode focus separately.
- Keep one source draft/Editor across Code and Split, read-only TextView in Preview, dirty close confirmation and guarded atomic save. Exercise unsaved mode transitions, undo, failure/retry, unchanged BOM/line endings and post-transition focus.

## 3. Export

- Add the reader's frozen source → GFM standalone HTML pipeline and owner-bound Save dialog. Fixtures check embedded local image, outline links, tables, code, Chinese text, offline open and draft/source independence. Cancellation and failure cannot publish a partial output or overwrite an existing one.
- Add the PlatformHost WebView2 SDK/loader path for PDF only, staged file transfer and bounded native completion. Start with contract tests for cancel, missing runtime, renderer failure, owner/reply retirement and existing-output preservation. Separately, with authorization, run an isolated native fixture against actual `PrintToPdfAsync`; open the resulting multi-page PDF and verify selected text and image placement. Missing runtime is not a passed PDF acceptance test. No release package/install changes outside the export dependency boundary.

## 4. Version history

- Integrate SQLite capture with the actual read/save/restore owner, timeline/name actions and official Diff surface. Test close/restart, unchanged deduplication, retention and comparison of two selected versions, including a saved external edit.
- Add guarded restore with durable pre-restore snapshot and one real file replacement. Test failure and blocked draft/conflict paths, exact bytes and recoverability of the version replaced by the restore. Run the combined workflow from the Other Tools entry to draft → save → compare → restore → HTML/PDF export.

## 5. Tool configuration and Explorer entry

- At the existing reader's `Tree` row boundary, reproduce overlapping long Chinese/nested headings with production layout bounds; then constrain one-line labels, preserve click/keyboard navigation and verify full accessible names/tooltips. Keep the two preview-title-bar editing actions unchanged.
- Replace the launch-only tile with the existing status/configuration pattern. Add the feature settings window and an owned Core section for activation and reading defaults. Test open/reuse, durable Apply/failure, and configuration-only card clicks; native association controls may never report effective default based on dispatch alone.
- Register only Markdown in the current user's Windows application choices via PlatformHost, reuse the fixed Default Apps action and read effective per-extension status. Add isolated owned-key, pre-existing handler, cancellation, replacement, notification and remove-while-default fixtures. Real Windows registry/default-app/Explorer acceptance remains a separately controlled test, not a side effect of development.
- Add explicit `--markdown-open` parsing and cold/warm single-instance routing without changing archive startup behavior. Verify Unicode and spaces, disabled/missing/shared-path input, no unwanted main/config window, actual reader tab selection, and repeat/race/late-request handling with a bounded transport fixture. Finish with a real Explorer double-click on each selected extension when authorized.

## 6. Winc-inspired preview and Win11 default-app handoff

- Baseline presentation: [Winc-derived typography, code actions and table layout](../../../docs/reference-code/markdown-reader.md#winc-markdown-presentation-reference) use the reader-local Kit slots. The initial narrow Popover, optional wide sidebar and two edit actions are the starting point for the interaction changes below.
- For Markdown only, open the Windows per-user registered reader page, and distinguish opening Settings from selecting an effective default. Keep the previous archive action unchanged and do not directly write the Windows default. Verify the fixed URI, per-extension status, and an isolated reader regression before live Explorer acceptance.

### Reader interaction and color verification

1. Add actual header/editor/save coverage for Code, Split and read-only Preview. Verify selected state, ignored Preview typing, caption targets, minimum size, keyboard activation, preserved draft/undo and no implicit save. New Explorer documents select Preview.
2. Prove committed TextView viewport/source mapping with a long document containing wrap, duplicate headings and fences. Implement the left outline rail and one shared tree, delayed hover plus explicit opening, pin with narrow fallback, and active-section rules for normal scroll, explicit jumps and document end. Verify pointer/keyboard cancellation and focus, including no focus transfer on hover.
3. Use the same revision-matched mapping for mode location and both split-scroll directions. Exercise source edits, stale parse rejection, mode switching, and reflected-scroll suppression. Add low-noise mode/edit and pin transition diagnostics at the reader owner; do not emit pointer, scroll or per-frame events.
4. Check real source and fenced-code token colors in Light/Dark, plus plain unsupported/unlabelled fences and exact code copy. Compare the same long Chinese technical document in Winc and NxTools in all three modes. Report compiled interactions, inspected renders and physical/native acceptance separately. Preserve export, history and association behavior; do not run packaging or change Windows defaults for this slice.

### Winc Markdown visual and Explorer activation correction

1. Use the supplied dark `INSTALL.md` comparison immediately. Capture the isolated NxTools reader with that same fixture, viewport and theme; obtain additional Winc Light/Dark and Preview/Split references via its PNG fixture or a live test window. The supplied baseline is sufficient to begin the dark visual correction without installing or changing Winc’s toolchain or any packaged application. Keep extra captures in scratch.
2. Add actual-reader regressions for compact chrome without an Open button, centered article without preview line numbers, and Winc’s hover-card lifecycle: dwell, in-place expansion, pointer handover, section jump, dismissal and pinning. Adapt the existing reader composition with locked public Kit slots; compare matched captures and preserve the approved mode/draft/scroll semantics.
3. Capture the existing Shell failure first: on cold start, warm reuse and minimized reuse, use the real double-click and read back the foreground HWND and current document. Separate protected redirect, forwarding/admission, show/restore and frontmost readback under the same operation. Add a red-capable launch regression at the real boundary before changing activation; fix the confirmed handoff using the Windows adapter without touching associations or other launch surfaces. Repeat the real gesture with an authorized installed or isolated fixture, and distinguish that result from compiled launch tests.
4. Run focused reader/startup and applicable native adapter tests plus component/provenance checks. Report what the screenshots and HWND readback actually prove. Do not package or change Windows defaults as part of this correction.
5. Correct Source and Split against the [Winc layout and input reference](../../../docs/reference-code/markdown-reader.md#winc-source-and-split-layout) and the dark `INSTALL.md` baselines. Add a reader-window regression for the equal initial split, proportional drag/resize, double-click reset and both 240-DIP minimums before changing composition. Apply borderless styling to the single existing Editor, keep its source/caret/undo and the committed preview scroll owner, then recapture both modes in Light and Dark. Keep standalone Preview and native association behavior unchanged.
6. Exercise actual A−/A+ clicks against source line geometry in Code/Split and paragraph geometry in Preview. Feed the same font request into both renderers, keep Preview read-only, and retain official tabs without an unconditional file-list dropdown. Recapture the supplied `INSTALL.md` comparison after font changes and mode switches.
7. Run the outline pointer regression through the same Base Root/text-selection layer as the reader window. Prove normal text dragging, click and release a card row overlapping prose, move into the article without a pressed button and assert unchanged selection. Repeat in Preview and Split and preserve keyboard jumps, hover/pin lifecycle and ordinary selection afterward.
8. Route settings-operation results through the existing NotificationCenter. Exercise actual settings buttons and correlated association completion, durable save/failure, default-app readback, automatic-query silence, explicit refresh, refusal/failed submission/disconnect and duplicate/superseded receipts. Preserve page status, fixed copy, source-window routing and deferred lease boundaries; no notification subsystem or real default-association changes.


## Verification

- Run project component and provenance gates relevant to the reader: `python tests/architecture/spec-contracts.py`, `python tests/architecture/gpui-component-catalog.py`, `python tests/architecture/gpui-release-contracts.py`, `python tests/architecture/markdown-editor-contracts.py`. These detect broken navigation, catalog/API drift and unauthorized framework/editor source changes, not native acceptance.
- Set `$env:CARGO_BUILD_JOBS='1'`, `$env:CARGO_INCREMENTAL='0'`, `$env:RUSTFLAGS='-C codegen-units=256'` in each direct or background Rust invocation. Run scoped reader and protocol tests during each stage, then `cargo test -p nxtools-desktop -- --test-threads=1`, `cargo test -p nxtools-protocol`, `cargo clippy -p nxtools-desktop -p nxtools-protocol --tests --no-deps -- -D warnings`, `cargo check --workspace --tests` and `cargo fmt --all -- --check`. Scope any remediation to this slice; distinguish baseline dirty-tree failures.
- Run affected PlatformHost build and reviewed picker/export fixtures. Validate physical input, IME, Light/Dark, minimum sizing, mixed DPI and owned picker/PDF runtime separately from automated tests. Native/live product launch and packaging require their own authorization; never infer them from source or synthetic GPUI results.
