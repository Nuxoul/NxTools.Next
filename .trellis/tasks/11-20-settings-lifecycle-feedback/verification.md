# Settings and lifecycle verification

## Outcome

- Clipboard feature enablement retains metadata baseline0 instead of rejecting the transition. Queued/old content is excluded; a later changed sequence captures through the original owner. Real operation exceptions retain failure/native codes and temporary pause remains independent.
- Quick Launch main-popup bounds request actual native8DIP-scaled corners on the existing size operation. Input/state/IME, dimensions and preview remain unchanged; unrelated bounds consumers omit the field.
- Ordinary installer preparation no longer elevates merely because an indexer service registration exists. Actual access-denied fallback and explicit Bootstrap maintenance retain authorization; migration runs once with both switches.
- Markdown Reader settings use the original compact feature shell, horizontal official fields and reading/association/preview order, retaining all actions, immediate saves and feedback.
- About's combined caption/download regression passes. The user-confirmed installed main-window hiding initiator remains unproved. Close-source/target diagnostics are present and correlated to the original native hide command; no forced re-show or quit-policy change is applied.

## Compiled checks

| Check | Result | Evidence boundary |
| --- | --- | --- |
| C# test-project build |0warnings/0errors |Debug Host/test assembly, no installed replacement |
| Clipboard pause/enable fixtures |6pass/0skip/0fail |Production EventWindow dispatch/application with injected operations; zero baseline/new change/native exception, pause/retry/queue/cancel/privacy |
| Window bounds corners fixtures |2pass/0skip/0fail |Real owned HWND/GDI through actual parser->adapter, resize/reset/preservation/frame compensation/nonactivation; no global input |
| Desktop About |12pass/0fail/0ignored |Actual caption click with MainWindow/About, original paused-queue downloader, close/reopen, stale/retired owner and accepted task retention |
| Desktop Markdown settings |10pass/0fail/0ignored |3layout plus7notification regressions; Light/Dark460x320, official bounds/states, scrolling, font limits, immediate persistence and association feedback |
| Desktop Quick Launch placement |4pass/0fail/0ignored |Original hidden/receipt/frame/session path plus96/120/144DPI corner factory |
| Desktop Quick Launch polish |11pass/0fail/0ignored |Original input identity/focus, results/actions/preview/keyboard and fixed geometry |
| Protocol library |123pass/0fail/0ignored |Optional radius/legacy omission/reset/roundtrip plus original wire tests |
| Protocol strict Clippy |PASS |Library -D warnings |
| Final Desktop type check |PASS |cargo check --locked -p nxtools-desktop --bin nxtools-desktop --tests, including correlated close diagnostics |

Desktop related total is37unique tests. C# exact selection is8/run8/pass8/skip0/fail0/not_run0. These are focused results, not full-workspace or release acceptance.

Commands use the workstation CARGO_BUILD_JOBS=1, CARGO_INCREMENTAL=0 and RUSTFLAGS=-C codegen-units=256, one Cargo process:

```powershell
dotnet build csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-restore
dotnet run --project csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-build --no-restore -- --fixtures --list "clipboard pause fixture" "window bounds corners fixture"
dotnet run --project csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-build --no-restore -- --fixtures "clipboard pause fixture" "window bounds corners fixture"
cargo test --locked -p nxtools-desktop --bin nxtools-desktop main_window::about:: -- --test-threads=1
cargo test --locked -p nxtools-desktop --bin nxtools-desktop tools::markdown_reader::settings:: -- --test-threads=1
cargo test --locked -p nxtools-desktop --bin nxtools-desktop quick_launch::monitor_presentation:: -- --test-threads=1
cargo test --locked -p nxtools-desktop --bin nxtools-desktop quick_launch::polish_tests:: -- --test-threads=1
cargo test --locked -p nxtools-protocol --lib -- --test-threads=1
cargo clippy --locked -p nxtools-protocol --lib -- -D warnings
cargo check --locked -p nxtools-desktop --bin nxtools-desktop --tests
```

## Source checks

Installer source gate passed ownership/recovery/runtime-admission/original-user constraints plus ordinary authorization and one migration invocation. PowerShell Parser.ParseFile accepted both modified scripts without executing them. Spec navigation/contracts, component catalog, coherent GPUI release, pinned Markdown editor provenance and About modeless/own-Close policy gates passed. Scoped rustfmt completed. One bounded read-only review found no concrete in-scope defects.

## Limitations and existing findings

Desktop production strict Clippy still reports five existing findings: unnecessary lazy evaluation in main_window/about/emblem.rs:68, large enum variant in main_window/extensions_page/activity.rs:16, nonminimal booleans in tools/markdown_reader/history.rs:170 and mod.rs:1153, and unit-value binding in settings.rs:112. These pre-existing statements were not changed by this slice; no warning suppression was added.

The installed report is hidden-to-tray, not process exit. Current-source TestWindow coverage does not reproduce its native initiator; matching Windows caption/visibility evidence remains required. Actual GPUI screenshots, Win10 installed appearance, physical IME/input, mixed-DPI and native accessibility/provider acceptance remain unexecuted. Ordinary GPUI Group semantics are not validated by Base control snapshots.

Installer source/syntax evidence does not certify real UAC or running-upgrade behavior. The isolated Inno runtime fixture was updated but not compiled/executed; the existing exact-target termination/wait mechanism remains. First service enrollment, protected writes, explicit maintenance and real access denial can still require administrator. No package, installer execution, installed-program replacement, live clipboard/global input, Registry/service mutation or commit was performed.
