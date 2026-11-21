# Quick Launch Windows10 outer corners

## Result

The existing native bounds operation applies an optional rounded outer-window region after client sizing/readback. Main Quick Launch bounds carry an8DIP radius at the current device scale. Compact, results and action presentations retain the same input, dimensions, state and native owners; auxiliary previews and other applications leave their window regions unchanged.

## Component and reference evidence

- Current graph: Kit/Base/Component/Assets0.8.0, default gpui-pre0.3.9. Mandatory selection/release/table/index/UI-authority documents were reviewed.
- Current layout is the authority: `rust/apps/nxtools-desktop/src/quick_launch.rs` defines620DIP width,58DIP search header,60DIP compact client and530DIP expanded client. Historical42DIP/InputGroup geometry is not the current design.
- `quick_launch/panel.rs:2059-2103` retains the existing stable InputState and official `gpui_kit::component::input::Input::new(...).aria_label(...).focus_bordered(false).with_size(Size::Large)`44DIP frame, current typography/padding, icon/feedback/actions and parent search-group layout. There is no Toolbar adaptation, added control, duplicate input frame or change to input focus/IME ownership.
- Released `vendor/gpui-component-080/src/root.rs:542-559` fills the Root background and wraps it in WindowBorder. `window_border.rs:19-22` explicitly documents rectangular child content masks and zero generic wrapper radius. Native region clipping reaches the real HWND without a framework patch or rounded-child substitute.
- Canonical Listary managed chain was rechecked at `D:\Code\Listary.Decompiled\6.3.5.94\managed\Listary\Main\Views`: factory18-20 → launcher constructor24-32 → SearchWindowBase constructor25-35/event wiring318-336 → Show383-398/base.Show394, direct Hide324/Close326; separate native foreground/base.Activate373-379. Launcher placement50-69 is reached from SourceInitialized91-96 and preserves NOACTIVATE|NOSIZE|NOZORDER. WPF's Window is the recorded managed base boundary; Listary native engine source is unavailable. Legacy NxTools has no Quick Launch implementation.
- Updated reference index: `docs/reference-code/quick-launch-research.md:35-73`.

## Protocol and native API

- `rust/crates/nxtools-protocol/src/lib.rs:989-999`: additive `corner_radius: Option<u32>`, device pixels, serde default and skip-None. Missing/null preserves existing region; explicit0 clears it. Existing protocol/command identity is unchanged.
- `csharp/NxTools.PlatformHost/Protocol/PlatformCommandParser.cs:62,380-392`: optional `int? CornerRadius = null`; accepts absent/null/nonnegative integer pixels and rejects negative, fractional, boolean, string and signed-int overflow values before native mutation.
- Existing routing remains `PlatformPipeClient.cs:981-982` → `WindowsPlatformAdapter.cs:585-592` → `WindowsPlatformAdapter.NativeMethods.cs:676-765`.
- Native bounds retain real outer x/y, live outer/client frame compensation, positive/widened size validation, SWP_FRAMECHANGED|NOACTIVATE|NOZORDER and requested-client readback. Only a provided radius invokes the native region operation before success.
- The region uses the resized current outer rectangle. GDI receives ellipse diameters clamped with64-bit arithmetic to the smaller outer dimension. The GDI lower/right construction extent includes the extra pixel needed for an actual region bounding box equal to the outer rectangle; GetRgnBox and bottom-edge assertions verify it.
- `CreateRoundRectRgn` failure and `SetWindowRgn` failure return typed failure/native error. Successful transfer belongs to Windows and is never deleted by the caller. Failed transfer deletes the caller-owned HRGN after capturing the original native error. Explicit0 calls SetWindowRgn with a null HRGN. There is no retained HWND registry, listener, poller, additional owner or protocol command.

## Rust call-site coverage and lifecycle

- `quick_launch.rs:136,158-160,1934-1941`: one semantic8DIP radius converted through the current GPUI window scale on dynamic resize.
- `quick_launch/monitor_presentation.rs:118-132,231-251`: the existing bounds factory carries the same radius at target monitor DPI for staging, target sizing and final placement. Existing operation/session/HWND/connection generation, hidden preparation, readback-before-show, activation and cancellation/error retirement remain unchanged.
- Non-main-popup call sites use None only: Quick Launch preview; Clipboard popup; link placement; Clipboard preview; Archive drag progress. Existing exact Clipboard patterns assert None without changing Clipboard enablement, visibility, capture or interaction semantics.
- Added Rust tests: `bounds_corner_radius_is_optional_and_round_trips_device_pixels` (legacy omission, opt-in/reset roundtrip, invalid types), and `popup_bounds_always_carry_scaled_outer_corners` (96/120/144DPI →8/10/12pixels through all three factory size steps).

## Diagnostics

This extends the original correlated operation, not a new diagnostic stream. `monitor_presentation.rs` retains presentation_accepted/presentation_started/presentation_finished with one operation UUID across the bounds/query/show envelopes and native results; terminal fields include stage/outcome/native_error_code/elapsed_ms. Dynamic resize retains quick_launch.popup.resize.accepted/rejected/completed with its request UUID, native error and elapsed time. C# original command.received/command.completed (`PlatformPipeClient.cs:565-670`) records that correlation, command, success, native_error_code and elapsed_ms. Region failure returns through that same terminal owner. No query/input/clipboard data, paths, titles, frames or pointer motion are logged; no temporary probes are present.

## Verification

C# command:

```powershell
dotnet build .\csharp\NxTools.PlatformHost.Tests\NxTools.PlatformHost.Tests.csproj --no-restore
dotnet run --project .\csharp\NxTools.PlatformHost.Tests\NxTools.PlatformHost.Tests.csproj --no-build --no-restore -- --list 'window bounds corners fixture'
dotnet run --project .\csharp\NxTools.PlatformHost.Tests\NxTools.PlatformHost.Tests.csproj --no-build --no-restore -- 'window bounds corners fixture'
```

- Build passed:0warnings/0errors, including PlatformHost and test assembly.
- Exact reviewed registrations in `Program.cs:22-23` and `TestEffects.cs:8-10`: `window bounds corners fixture strict optional wire`; `window bounds corners fixture owned region resize reset`. Listing selected2/run0. Final execution selected2/run2/pass2/skip0/fail0/not_run0.
- `WindowBoundsCornerTests.cs` invokes the actual ParseSetWindowBounds → WindowsPlatformAdapter.SetWindowBounds entry. The fixture uses only self-owned STATIC HWNDs and GDI objects; the uninitialized stateless adapter entry avoids ordinary adapter startup, hooks, clipboard, Shell and Registry/service effects. Only its own popup is briefly shown via SW_SHOWNOACTIVATE, then hidden/destroyed; no global input is injected.
- Native assertions cover four excluded corners, retained center/current bottom edge, exact outer region box, client/placement preservation, hidden60→530→60 and visible530→60 resize, framed client compensation,9×5 clamp with int.MaxValue radius, missing/null preservation, explicit0 reset, invalid size/HWND failure and unchanged foreground.
- Red-capable boundary evidence: the owned-HWND regression rejects an unrounded bounds implementation at GetWindowRgn with “actual HWND must have a rounded native region”. This distinguishes actual outer shape from a rounded descendant or successful SetWindowPos alone.
- `python tests/architecture/gpui-release-contracts.py`: passed, coherent0.8.0/pre0.3.9 and exactly two framework-file deltas.
- `python tests/architecture/markdown-editor-contracts.py`: passed, pinned57-file editor provenance and approved substitutions/seams.

## Limitations

Rust verification passed: protocol123/123 with strict library Clippy, main-popup placement4/4 including96/120/144DPI radius, and Quick Launch polish11/11; scoped rustfmt completed. The two owned Windows HWND/GDI fixtures passed without skip. These do not accept real GPUI panel appearance, physical input/IME, Win10 installed behavior or mixed-DPI rendering. No product launch, package/install, service/Registry mutation or commit was performed.
