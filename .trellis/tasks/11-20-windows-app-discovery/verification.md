# Windows application discovery verification

## Outcome

Quick Launch combines existing Start Menu/Desktop/file scanning with a current-user Windows AppsFolder packaged-application snapshot. Windows supplies localized names and AUMID launch targets; unknown installed packages need no product registration list.

Known Windows packages have bilingual search aliases, including Calculator, Paint, Notepad, Snipping Tool, Photos, Camera, Recorder, Clock, Terminal, Sticky Notes, Store and Media Player. Aliases do not create absent package entries. Classic Calculator/Paint retain calc.exe/mspaint.exe fallback routes when no corresponding native package or known shortcut supersedes them.

Finder keeps root and installed snapshots separately, preserves history across refresh, and accepts installed replacements atomically. Native failure retains last-good data; successful empty snapshots remove stale installed entries. The original QuickLaunchApplication coalesces requests and binds results to their request/connection. Disable, disconnect and timeout retire ownership. Current success refreshes only the live ordinary query; closed/action-mode panels are not reopened or replaced. AppsFolder applications use ordinary Shell Open without file preview, reveal, path-copy or elevation.

Native enumeration owns its COM objects on the existing bounded STA runner, is detached from the serial pipe reader, and returns no partial data after failure. Diagnostics preserve correlation, elapsed time, stable failure kind and native error codes without names/AUMIDs.

Reference inspected: `D:\Code\Listary.Decompiled\6.3.5.94\managed\Listary\Core\Launcher\Class291.cs`, application enumeration and STA cleanup. This is managed application-provider evidence, not Listary's unavailable native search engine. The verified call chain is recorded in `docs/reference-code/quick-launch-listary-results.md`.

## Verification

| Check | Result |
| --- | --- |
| `cargo test -p nxtools-finder -p nxtools-protocol --no-fail-fast` | Finder103 passed; Protocol132 passed,1 ignored;0 failed |
| Finder installed-application integration tests |4 passed: bilingual/partial search, unknown/absent packages, fallback precedence, root-refresh/history preservation, atomic replacement/removal and native target parity |
| Desktop related regression invocation |29 passed;0 failed/ignored, including5 new installed-application tests |
| `cargo clippy -p nxtools-finder --tests -- -D warnings` | Passed |
| `cargo clippy -p nxtools-protocol --lib -- -D warnings` | Passed |
| C# test-project build, no restore | Passed;0 warnings/errors |
| Exact `installed applications fixture` selection | selected8/run8/pass8/skip0/fail0; list-only selection verified before execution |
| Rust formatting,9 affected files with child traversal disabled | Passed |
| Spec contracts, GPUI release contracts, Markdown editor provenance | Passed |
| `git diff --check` | Passed |

Desktop filters: `windows_apps_`, `catalog_refresh_`, `w2_`, `w6_`, `feature_disable_`, `feature_total_switches_`, `floating_search_`, `r1_popup_presentation_`, `multimonitor_`. New tests exercise actual original request/result routing, coalescing, last-good/empty replacement, connection/disable retirement, the real timeout task, live/closed panel refresh and action construction. Discovery never writes activation history or creates a panel.

Native fixtures use injected enumeration and process start. They verify localized Calculator/Paint/Camera and future applications, complete JSON, missing/invalid facts, failure without partial publication, logical timeout with retained native admission, detached routing, exact original pipe results, ordinary Shell launch parameters and rejected filesystem/elevated actions.

All Cargo verification used CARGO_BUILD_JOBS=1, CARGO_INCREMENTAL=0 and RUSTFLAGS=-C codegen-units=256.

## Limitations

- Strict Protocol test-target lint remains blocked by the existing `unnecessary_map_or` at `rust/crates/nxtools-protocol/src/archive_browser.rs:565`. The file has no working-tree diff.
- The final strict Desktop invocation remains blocked by5 existing production findings: About emblem `unnecessary_lazy_evaluations`, extension activity `large_enum_variant`, Markdown reader history/module `nonminimal_bool`, and Markdown reader settings `let_unit_value`. No final diagnostic points to the application-discovery or Quick Launch files.
- The full Desktop suite and whole-workspace test-target check were not rerun for this slice. Prior full-suite findings remain recorded in `.trellis/tasks/11-20-multi-monitor-window-controls/verification.md`.
- Actual installed AppsFolder COM enumeration and application activation were not executed. Synthetic results do not certify installed-native acceptance.
- No packaging, installation, live Registry/service operation or commit was performed. The installed application has not been replaced by this source change.
