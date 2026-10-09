# Quick Launch Result-Panel Flicker Findings

## Status

Investigation-only. No Quick Launch product behavior, animation, native window ordering, or gesture code was changed.

The existing source and lifecycle gates are green, but the PRD-level visual repro is not complete because the packaged smoke harness cannot compile under the current Windows PowerShell 5.1 environment. The temporary compatibility edit made while diagnosing that harness was reverted; `tests/runtime/quick-launch-search-smoke.ps1` is unchanged from the start of this investigation.

## Evidence inspected

- `docs/reference-code/quick-launch-research.md`
- `rust/apps/nxtools-desktop/src/quick_launch.rs`
- `rust/apps/nxtools-desktop/src/quick_launch/panel.rs`
- `rust/apps/nxtools-desktop/src/quick_launch/search.rs`
- `rust/apps/nxtools-desktop/src/quick_launch/actions.rs`
- `csharp/NxTools.PlatformHost/Windows/ModifierDoubleTapDetector.cs`
- `csharp/NxTools.PlatformHost/Windows/WindowsPlatformAdapter.EventWindow.cs`
- `csharp/NxTools.PlatformHost/Windows/WindowsPlatformAdapter.NativeMethods.cs`
- `rust/crates/nxtools-protocol/src/lib.rs`
- `rust/crates/nxtools-ipc/src/lib.rs`
- existing Rust/C# Quick Launch tests and `tests/architecture/quick-launch-lifecycle.ps1`
- existing payload-free desktop/PlatformHost logs under `%LOCALAPPDATA%\\NxTools\\NxTools.Next\\data\\logs`

## Verified call chain

```text
double Ctrl
  -> ModifierDoubleTapDetector
  -> low-level hook posts WmQuickLaunchGesture
  -> PlatformHost EventWindow captures foreground/cursor/DPI
  -> quick_launch_panel_requested IPC event
  -> PlatformEventDispatcher
  -> QuickLaunchApplication::handle_platform_event
  -> toggle_panel
  -> open_popup / show_existing_popup
  -> GPUI QuickLaunchPanel
  -> native visibility/topmost/activation commands
```

The request carries timestamp, target context, cursor and DPI, but no summon correlation ID and no native-show/first-GPUI-frame acknowledgement.

## Historical synchronized evidence that is available

A real log sample from `2026-09-28` provides a partial lifecycle, but not a visual frame trace. Payload-bearing fields were excluded from this report.

| Time (+08:00) | Transition | Evidence |
| --- | --- | --- |
| 23:08:42.426 | popup opened | Rust desktop lifecycle log |
| 23:08:42.429 | native topmost requested; window already reported visible | PlatformHost native window diagnostic; collapsed bounds `969,345,1591,391` |
| 23:08:42.445 | topmost/foreground confirmation | PlatformHost native window diagnostic |
| 23:08:42.818 | query generation 3 queued | Rust provider lifecycle log |
| 23:08:42.890 | query generation 5 queued | Rust provider lifecycle log |
| 23:08:42.950–42.953 | generation 5 batches accepted and merged | Rust provider lifecycle log |
| 23:08:44.522 | generation 7 queued | Rust provider lifecycle log |
| 23:08:44.645 | generation 9 empty query committed | Rust provider lifecycle log |
| 23:08:44.657 | generation 7 late batches rejected | Rust provider lifecycle log |
| 23:08:45.750 | foreground-triggered auto-dismiss | Rust desktop lifecycle log |
| 23:08:45.750 | native hide queued | Rust desktop lifecycle log |
| 23:08:45.756 | native hide confirmed | PlatformHost/Rust correlated lifecycle; elapsed about 5 ms |

Important limitation: the first-summon path uses `WindowOptions.show = true`, so this sample has no explicit native `visible=true` event that can be compared with the first committed GPUI frame. The result-surface mount and 120 ms animation are also not logged with start/end timestamps.

## Source-level lifecycle facts

- `prepare_for_show` clears the query/result state, sets `is_open = true`, sets `results_visible = false`, and resizes the panel to the collapsed height.
- A non-empty query or action/recent-directory mode changes `results_visible` to true, increments `results_appearance_generation`, and resizes to the expanded height.
- The result surface is conditionally mounted and keyed by `results_appearance_generation`.
- The entrance animation is 120 ms and moves the surface by 1 px; it does not animate opacity.
- Reuse currently sends native `SetWindowPosition` and `SetWindowVisibility(true)` before the GPUI `prepare_for_show` update.
- First creation uses `WindowOptions.show = true` while the GPUI panel is created in the window callback.
- Dismissal immediately marks the panel closed and defers the application hide operation; native visibility hide is tracked and retried once if unconfirmed.
- Query/index generations reject stale provider results, but `results_appearance_generation` is not explicitly bound to a popup session ID.

## Ranked falsifiable hypotheses

### 1. Native show before the first committed GPUI frame — highest suspicion, not proven

**Prediction:** if this is the cause, a frame-synchronized capture will show the native HWND becoming visible before the first collapsed GPUI frame, especially on the first summon or reused-popup path. Reordering reveal after the first committed collapsed frame should remove the flash without changing query results.

**Evidence:** first creation uses `show = true`; reuse sends native visibility before `prepare_for_show`; no first-frame acknowledgement exists.

**Confidence:** medium. The ordering is directly observable in source, but no synchronized screen/frame trace exists.

### 2. Immediate hide overlaps the result entrance animation — high-risk race, not proven

**Prediction:** opening, typing/receiving a result, and dismissing within 120 ms should produce a higher flash rate than a settled open. A close-time animation invalidation or session guard should eliminate the artifact while leaving steady-state results unchanged.

**Evidence:** the result surface has a generation-keyed entrance animation; close sets `is_open = false` and queues native hide, but there is no explicit animation cancellation tied to hide/session.

**Confidence:** medium. The state transition permits the overlap, but no immediate-dismiss recording or frame trace exists.

### 3. Stale/restarted result generation — lower suspicion for the observed sample

**Prediction:** a late result from an older query/session would need to mount or restart the result surface after a close/reopen. A session-bound appearance generation should make the failure disappear.

**Evidence:** the existing provider log shows generation 7 batches rejected after generation 9 became active. Existing tests cover stale index batches and preserve committed local results.

**Confidence:** low-to-medium. Query generation handling is strong, but it is not the same as a cross-summon animation/session guard.

### 4. Duplicate summon/dismiss event — possible but unobserved

**Prediction:** two accepted panel requests for one physical gesture would toggle open → hide or hide → reopen. A gesture/request correlation and duplicate suppression would remove the transition.

**Evidence:** the native detector rejects repeat/intervening-key cases, but the IPC request has no correlation/sequence identity and Rust uses toggle semantics. No duplicate-request trace was captured.

**Confidence:** low-to-medium.

### 5. Ordinary entrance animation is itself the visible flash — least supported

**Prediction:** the artifact would occur on settled opens without a hide race, while a frame capture would show only the 120 ms 1 px entrance. A no-motion diagnostic build would remove it.

**Evidence:** animation is a small vertical settle with no opacity; source inspection alone does not establish what the user sees.

**Confidence:** low until a visual artifact is recorded.

## Regression coverage and limitation

The following existing checks pass and are useful state/lifecycle coverage:

- `tests/architecture/quick-launch-lifecycle.ps1`
- Rust production-window geometry and collapsed/expanded surface tests in `quick_launch/tests.rs`
- stale index generation rejection tests
- dismissal command-order, activation retry, focus restoration and hide self-healing tests
- C# double-Control detector and protocol round-trip tests

They do **not** prove the absence of a visible flash. There is currently no automated gate that drives the real double-Control gesture and asserts the ordering of native visibility, first GPUI frame, result-surface mount, animation start/end, and native hide. Automated state coverage and manual visual acceptance must remain separate until that seam exists.

## Reproduction protocol blocked in this session

Attempted command:

```powershell
./tests/runtime/quick-launch-search-smoke.ps1 -UseRealPlatformHost -CapturePreview -ScreenshotPath <scratch>\quick-launch-baseline.png
```

The script failed before application startup because its embedded `Add-Type` code imports `System.Text.Json`, which is unavailable to Windows PowerShell 5.1. A temporary compatibility rewrite was attempted, then fully reverted after the legacy CodeDom compiler exposed additional syntax/structure incompatibilities. No product process was left running by this attempt and no screenshot was produced.

User-run capture protocol once the harness is run under PowerShell 7/.NET or its C# bridge is repaired:

1. Start the exact packaged desktop and matching PlatformHost in an interactive display session.
2. Record screen frames at 60 fps plus payload-free desktop/PlatformHost logs.
3. Run 20 settled double-Control open/close cycles.
4. Run 20 immediate-dismiss cycles with dismissal inside 120 ms.
5. Run 20 cycles where a query/result arrives while dismissing.
6. Run 20 empty-query → first-keystroke cycles.
7. For each cycle record only gesture accepted, request timestamp/session, native show/activation, result-surface mount/appearance generation, animation start/end, dismissal reason, and native hide confirmation. Do not record query text or result labels.

## Smallest fix proposal for approval

Do not change behavior until the visual protocol confirms the ordering hypothesis. If the capture confirms native visibility before the first committed collapsed GPUI frame, the smallest candidate is:

1. keep the primary popup hidden during first creation/reuse preparation;
2. commit the collapsed GPUI state first;
3. reveal the native HWND only after that committed frame, with one session/correlation guard;
4. invalidate any pending result entrance on dismissal.

If the capture instead shows a duplicate request, fix request identity/deduplication rather than animation. If it shows only a normal entrance frame, do not change animation duration or theme based on this investigation.

No product fix has been applied. Stop point: review this evidence and approve the narrowest behavior change before implementation.
