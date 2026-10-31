# Queue 02 — Archive Settings search diagnosis outcome

## Outcome

**Resolved as a test locator defect, not an Archive Settings product/search/save defect.** The exact historical missing-bounds assertion was reproduced with official GPUI scheduler seed3. A minimal test-only locator correction makes that seed and the bounded0–7 sequence pass. No production view, theme, Settings/vendor, native, persistence or archive engine source was changed.

The current canonical archive checkpoint was read as `Current checkpoint — 原始参考图已实际查看；最新界面与物理交互待核对`. This task changes no phase/checkpoint or visual/native/user acceptance. Original plan titles/order remain authoritative; this is only the separately requested test diagnosis.

## Feedback loop and observed root cause

The original exact focused test passed at default seed0, and the original ArchiveSettings group passed19/19. Those greens did not explain the historical MainWindow185/1 failure and were not represented as a repair.

A bounded run of that same original test with `SEED=0 ITERATIONS=8` failed at seed3 on the original `archive-test-after-create` bounds assertion. A second run pinned to seed3 with a temporary test-local `DEBUG-ARCHIVE-SEARCH` node observer reproduced the same failure. The observer showed two real TextInput nodes before the click:

1. A NumberInput descendant in the right-hand settings page (`value="10"`,bounds x809/y473.5,width62/height32).
2. The real official search Input under `archive-official-settings/settings-sidebar` (`label="Search..."`,empty value,bounds x188/y144,width136/height32).

The original `.find(role == TextInput)` selected the first node, which was **not the search input**. After its pointer click and simulated text, the actual sidebar search still had an empty value and `focused=false`; filtering never received the intended query. This accounts for the absent target without invoking a save, theme leak, delayed search task or virtual-list timing explanation.

The exact released Component0.8.0 Settings source confirms that `render_sidebar` owns this Input under the stable `settings-sidebar` element; Settings::render reads its InputState value synchronously and computes SettingsFilter, and SettingPage uses its existing virtual ListState for matching groups. There is no debounce search task here. The verified correction changed only locator specificity and assertions; event delivery, click/type ordering and draw count remain unchanged. That controlled result rejects focus-frame or virtual-measurement delay as the cause of this reproduced failure. It does not claim every possible unrelated UI timing problem is impossible.

## Minimal change and retained assertions

Only `main_window/archive_page_tests.rs::archive_settings_official_search_never_saves` changed:

- Locate TextInput nodes whose actual semantic path contains both `archive-official-settings` and `settings-sidebar`, and require exactly one match.
- Continue using the original real pointer click and `simulate_input`; do not call set_value, query/filter helpers or business save methods.
- Before checking the original result bounds, assert that the actual sidebar TextInput is focused and contains the complete `压缩后测试` query.
- Run the test with `#[gpui_kit::test(iterations = 8)]`, covering official seeds0–7 in ordinary test execution, including the proven failing seed3.

The original positive result assertion, subsequent Ctrl+A/nonmatching-query negative assertion and exact temporary configuration-byte equality assertion remain. No assertion was deleted, weakened, ignored or replaced with a direct method invocation. No new sleep, clock advancement, retry loop or extra draw was added. All temporary DEBUG diagnostics were removed from live source after their raw logs and source snapshot were retained as evidence. All other tests in the file remain byte-identical to their task-start portions.

## Performed verification

All Rust invocations used CARGO_BUILD_JOBS=1,CARGO_INCREMENTAL=0,RUSTFLAGS=-C codegen-units=256 and ran serially after natural Cargo/rustc idle checks with adequate timeout. No process was killed/paused and no resource reservation was requested. SEED/ITERATIONS environment overrides were restored after each controlled run.

| Check | Actual result |
| --- | --- |
| Original exact focused test, default seed0 | PASS1/1,0.41s |
| Original ArchiveSettings group | PASS19/19,22.87s |
| Original exact test, SEED=0 ITERATIONS=8 | FAIL at seed3, original missing target bounds;1.44s |
| Original locator plus temporary node observer, SEED=3 | FAIL again, same target assertion; actual search empty/unfocused;0.20s |
| Corrected locator, SEED=3 ITERATIONS=1 | PASS1/1,0.57s |
| Corrected standard test (iterations8,no environment override) | PASS one test with all seeds0–7;3.42s |
| Corrected ArchiveSettings group | PASS19/19,27.81s; includes the eight-seed search test |
| cargo clippy -p nxtools-desktop --tests -- -D warnings | PASS exit0 |
| cargo check --workspace --tests | PASS exit0 |
| Scoped rustfmt --edition2024 --config skip_children=true --check archive_page_tests.rs | PASS exit0 |
| Scoped git diff --check | PASS exit0 |
| GPUI catalog,release contracts,Markdown editor provenance,spec contracts | All PASS; source/document checks only |

The original focused/group compilation establishes that the preceding queue task's compile blockers were absent for these runs. No complete Desktop or MainWindow suite was run. This new causal evidence closes this specific locator failure; it does not rewrite the historical185/1 log into a past green or certify the whole Desktop suite. The independent Extensions import-preview failure and later queue tasks are outside this result.

## Scope, dirty-state and historical evidence

Task-start dirty baselines were captured for archive_page_tests.rs,archive_page.rs,theme.rs,archive reference and official Settings source. One exact reversible own-delta covers only the target test. Prefix and suffix outside that function are verified unchanged. Product ArchiveSettingsView,theme,archive reference and vendor Settings source still match their baseline hashes. Current selected official source files and historical failure references have independent hashes in the evidence manifest.

The original broad failing log at `.trellis/tasks/10-25-rem-role-consistency/evidence/main-window-related.log` remains untouched, as do prior parent reviews/manifests and cancellation/blocker histories. No reference implementation was executed or proprietary source inspected; no migrated product behavior changed, so no new native lifecycle or reference-policy amendment was necessary.

## Limits and stop

No remaining confirmed blocker for this specific test-locator task is known after the bounded red/green sequence. Evidence is compiled production-view synthetic pointer/text behavior plus temporary configuration-byte checks only. Physical keyboard/IME,real native focus/HWND,DPI/mixed-monitor,UIA/reader,visual/user/product/installed/release acceptance: NOT RUN.

No subagent/independent session,real archive or user configuration,product/PlatformHost/service/native/Shell/Registry/clipboard/network operation,package/install,commit or push occurred. Stop after this queue task; subsequent tasks are dispatched by the queue rather than started here.
