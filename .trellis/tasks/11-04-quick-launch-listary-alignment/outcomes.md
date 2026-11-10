# Quick Launch Listary alignment outcomes

## Delivered

All four requested code changes are implemented:

1. Finder uses application/history/match categories instead of an unconditional executable6500boost. Formal good application matches remain prominent; weak applications and ordinary executable files do not automatically displace strong general results.
2. Exact and prefix keyword history use recency. Application/indexed path aliases share usage and query learning without clearing or bulk rewriting existing SQLite records. Match-before-merge prevents nonmatching aliases from hiding valid results; duplicate application shortcuts fold while ordinary same-name files and command argument variants remain distinct.
3. Launcher parent paths use released measured middle ellipsis via Kit Label/GPUI text APIs. Stored strings, action targets, tooltip and accessible names retain full content.
4. Launcher-specific52DIP rows/32DIP icons and16/15/12DIP title/subtitle hierarchy accompany content-sized height. The470DIP results viewport scrolls when full; default separated mode has116DIP one-result,228DIP three-result and514DIP capped window heights. Compact empty/loading state is140DIP; errors reserve36DIP footer. Attached-dialog/action compact metrics,620DIP width,42DIP header, current4/0gaps and palette remain.

Default-first versus explicit selection, retained-row normal paint with stale actions blocked, current-result early publication, original input/IME/focus, passive preview and correlated native owners are preserved. Real resize uses existing C# SetWindowBounds/SwpNoActivate, coalesced one in flight with current content re-read after completion. No vendor, protocol, C# or indexer service change is part of this slice.

## Verification

| Command / boundary | Result |
| --- | --- |
| `cargo test -p nxtools-finder -- --test-threads=1` | 95 passed,0 failed. Includes10new category/history/merge regressions and original command, query, pinyin, homophone, calculator, environment and package suites. |
| `cargo clippy -p nxtools-finder --tests -- -D warnings` | Passed. |
| `cargo test -p nxtools-desktop quick_launch:: -- --test-threads=1` | 185 passed,0 failed. Includes original smooth-refresh/default-selection tests plus measured path text,0/1/many published sizes, overflow reachability, footer, gap styles, coalesced native commands and hidden-session protection. |
| `cargo clippy -p nxtools-desktop --tests -- -D warnings` | Passed. |
| `cargo check --workspace --tests` | Passed. |
| `cargo test -p nxtools-desktop -- --test-threads=1` | 1718 passed,10 failed; failures listed below. Integration target did not execute after the unit target failed. |
| Independent bounded Finder review | No concrete in-scope defects reported. |
| `tests/architecture/quick-launch-lifecycle.ps1` | Passed, including content-sized footer and deferred native resize boundaries. |
| GPUI catalog/release, Markdown editor provenance and spec-contract gates | All passed. |
| Scoped rustfmt and git diff whitespace | Passed for modified Finder/Quick Launch sources and documentation. |

Every Cargo command inherited CARGO_BUILD_JOBS=1, CARGO_INCREMENTAL=0 and RUSTFLAGS=-C codegen-units=256. Final Quick Launch tests cover the prior smooth-refresh slice as well as this geometry/text slice.

## Known verification failures

The full Desktop unit run is not green:

- Three `archive::application::jobs::tests::ac_repair_tests` tests require the C# production-progress wire export absent from this test environment.
- Four `archive::application::jobs::tests::opt03_result_tests` tests require the effects-audited native fixture and `NXTOOLS_OPT03_CREATE_WIRE`.
- `archive::application::jobs::tests::hz3_retry_uses_original_native_recheck_once_before_new_attempt` and `hz3_rendered_eligible_retry_click_rechecks_once_then_original_new_attempt` observed2where1was expected.
- `main_window::tests::download_context_menu_omits_actions_that_cannot_run_for_the_selection` failed its unavailable-action visibility assertion.

These failures are outside the modified Quick Launch/Finder owners. No archive or download behavior was changed to satisfy them.

## Acceptance boundary

Compiled TestWindow render, measured width/bounds, synthetic keyboard/pointer and original typed native-command tests passed. TestWindow native size changes explicitly deliver a viewport event, and native bounds assertions use its actual scale factor. These do not certify physical mixed-DPI/IME/native-HWND operation or user visual acceptance. No package, install, service restart, installed-product launch, commit or push was performed.
