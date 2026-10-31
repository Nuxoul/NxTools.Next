# Queue 02 — Archive Settings search instability

## Boundary

Diagnose only the historical archive_settings_official_search_never_saves missing archive-test-after-create bounds assertion. Keep original real pointer/text search and temporary config-byte non-save assertion. No archive product/engine/native changes, vendor patch, visual redesign, phase advancement, fixed sleep, ignored/removed assertion or direct set_value shortcut.

Current canonical checkpoint: Current checkpoint — 原始参考图已实际查看；最新界面与物理交互待核对. This diagnostic task does not reopen or claim visual/native/physical acceptance. Plan phase names/order remain unchanged.

## Baseline and feedback loop

Captured current dirty bytes before edits for archive_page_tests.rs, archive_page.rs, theme.rs, archive reference and official Component0.8.0 Settings source. Preceding queue compile closeout plus the original focused1/1 and ArchiveSettings group19/19 runs compile current Desktop; no old transport compile error remains in these runs.

A bounded official GPUI seed run (SEED=0, ITERATIONS=8) reproduces the exact historical assertion at seed3. The loop is the original exact focused Cargo test with SEED=3, ITERATIONS=1. No complete Desktop/MainWindow rerun is needed.

## Ranked falsifiable hypotheses

1. The test's first-TextInput lookup identifies a different input. Prediction: observed role/path/bounds/focus differs from the official Settings sidebar search; an exact actual search-node match fixes the failure without extra time.
2. Pointer focus changes before the new text input handler is painted. Prediction: the typed query is missing or truncated at seed3; drawing the focused frame between click and typing restores full query and result without waiting or changing Settings filtering.
3. Search query is complete but virtual Settings page measurement has not been published. Prediction: the query is complete in the rendered search node while target is missing, then bounded drawing alone publishes it with unchanged input/focus.

Inspect one variable at a time. Temporary synthetic test diagnostics use a DEBUG-ARCHIVE-SEARCH marker and are removed after causal confirmation. Failure to identify a cause must be reported unresolved, not represented as fixed by isolated green.

## Likely edit boundary and acceptance

Only archive_page_tests.rs test lifecycle may need a local correction if the actual product chain is sound; any product change requires a reproduced product-boundary defect and reference-first review. Preserve full original assertions and every unrelated test byte. Confirm controlled seed3 red/green, original0–7 sequence, actual theme/search/navigation/empty/clear transitions as applicable, and exact temporary config bytes unchanged. Run relevant ArchiveSettings tests and scoped lint/check/fmt/doc gates. No full Desktop suite, real archive/config/native/product/service/packaging/install/commit/push. Rust verification jobs1/incremental0/codegen-units256, natural idle and serial commands only. English evidence retains historical failed logs and dirty attribution.
