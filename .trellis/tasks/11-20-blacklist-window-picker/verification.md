# Blacklist window selection verification

## Delivered behavior

The original Application Blacklist editor retains manual process-name input and adds the official
Kit0.8.0 Small Outline `选取窗口` button beside it. During selection the action becomes `取消选取`.
One passive transparent GPUI window outlines the hovered native window and displays its executable
name and original device-pixel dimensions. Left click selects; Escape or the original button cancels.
The confirming target click and Escape retain matching-release suppression.

Selection only refills the original process input and restores its focus. Scopes and existing rules
are unchanged. Original Add/Update edits the local draft; original Apply is the sole durable/live
publication path. Cancel, page leave, close, matching-connection disconnect and timeout retire the
session. Old session events and old-connection disconnects cannot mutate or cancel a newer draft.
The rendered outline intersects the virtual desktop so maximized/off-edge borders and the caption
remain visible; displayed dimensions remain native facts. No target window is moved or resized.

## Reference and ownership

Legacy `D:\Code\NxTools\Views\SettingsWindow.axaml.cs` provides original scoped Apply/Cancel and
window lifecycle behavior. The window picker is an explicit new capability; no legacy picker
implementation was found. The verified call chain is indexed in `docs/reference-code/system-shell.md`.

Rust owns the original settings draft/session and GPUI layout. C# owns HWND ownership checks,
hit-testing, process facts, input hooks and native presentation. The original PlatformBridge,
PlatformPipeClient, event channel and protocol1 envelopes remain. Native Start prepares the overlay
hidden, initializes layered alpha255, verifies outer/client bounds and installs hooks. Rust requests
session-qualified Show only after a GPUI frame. Native terminal handling hides before publication,
then drains intercepted releases on the same owner. Diagnostics contain operation/state/fixed
reason/elapsed/native-code facts, not process names, paths, titles or hover streams.

## Verification results

| Check | Command / assertion | Result |
|---|---|---|
| Protocol | `cargo test -p nxtools-protocol --lib` | 122 passed, 0 failed |
| Global settings | `cargo test -p nxtools-desktop --bin nxtools-desktop network_settings:: -- --test-threads=1` | 49 passed, 0 failed |
| Protocol strict lint | `cargo clippy -p nxtools-protocol --lib -- -D warnings` | Passed |
| Desktop strict lint | `cargo clippy -p nxtools-desktop --bin nxtools-desktop -- -D warnings` | Existing five findings below; none in picker code |
| C# build | `dotnet build csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-restore -v:q` | 0 warnings, 0 errors |
| Reviewed C# fixtures | `dotnet run --project csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-build --no-restore -- --fixtures "process window pick fixture"` | selected12/run12/pass12/skip0/fail0/not_run0 |
| Rust formatting | Scoped `rustfmt --edition 2024 --check --config skip_children=true` | Passed |
| Spec contracts | `python tests/architecture/spec-contracts.py` | Passed |
| GPUI provenance | `python tests/architecture/gpui-release-contracts.py` | Passed |
| Editor provenance | `python tests/architecture/markdown-editor-contracts.py` | Passed |
| Scoped diff | `git diff --check` for affected tracked files | Passed |

All Cargo verification was serial and inherited `CARGO_BUILD_JOBS=1`, `CARGO_INCREMENTAL=0`,
`RUSTFLAGS=-C codegen-units=256`.

The six new production-boundary Desktop tests assert:
- Actual original-editor Button layout, pointer action and keyboard accessibility; manual input
  remains usable and failure releases selection state.
- Qualified Start command/correlation, deferred source-view refill, unchanged scopes/rules/config,
  and explicit original Add without implicit persistence.
- Page/button cancellation, close and late-selection rejection.
- Old-session/connection rejection, stale-disconnect isolation, current disconnect and native failure.
- Negative-origin/scaled geometry and maximized/off-edge clipping at1/1.25/1.5/2 scale.
- A TestWindow overlay frame produces one qualified Show; the actual125-second timer cancels and rejects
  late results.

The twelve C# fixture registrations were reviewed in `TestEffects.cs` and listed with
`--fixtures --list "process window pick fixture"` before execution. Injected native APIs exercise
actual ownership/style/placement/hook policy and partial-install/alpha-initialization failure,
coalesced hover, fresh clicked HWND, initial/source passthrough, paired click/Escape releases,
stale Cancel/Show, connection/disposal/deadline retirement and payload-free diagnostics. The actual
pipe route, event forwarding and framing run over a GUID-owned local pipe. No fixture installs real
global hooks, reads user windows or launches product applications.

## Known issues and acceptance limits

Desktop strict lint remains blocked by pre-existing findings outside this slice:
- `main_window/about/emblem.rs:68`: unnecessary lazy evaluation.
- `main_window/extensions_page/activity.rs:16`: large enum variant.
- `tools/markdown_reader/history.rs:170` and `tools/markdown_reader/mod.rs:1153`: nonminimal Boolean expressions.
- `tools/markdown_reader/settings.rs:110`: unit-value binding.

Full Desktop/workspace suites were not run for this slice. Real global-input selection, hidden-HWND
first-frame delivery, visual/user acceptance and physical mixed-DPI/multi-monitor behavior remain
unverified. Compiled TestWindow and injected native fixtures do not certify those interactions.

This task performs no packaging, installation, installed-version replacement, Registry/service
mutation or commit. Source implementation and relevant controlled checks are complete; installed
native/user acceptance remains open.
