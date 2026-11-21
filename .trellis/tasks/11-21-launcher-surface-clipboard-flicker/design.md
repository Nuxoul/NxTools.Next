# Search surface and Clipboard presentation

## Search proposal

The [research](launcher-research.md) separates Win11 system rounding, Win10 shadow artifacts and compositor clipping. The supplied square-shell screenshot remains rejected. User accepts the [single-surface direction](../../../docs/ui/quick-launch-win10-proposal.html):620x60 visible bar,8px outer radius, one neutral border/shadow,20/28px regular type and the existing family. Adoption is limited to an isolated real GPUI example before product integration.

Exact released Component0.8.0 input/group.rs supplies InputGroup::new/input/addon and InputGroupControl from the original Input. The compound owns the only frame; public Styled/Sizable and addon alignment arrange the existing icon, InputState and feedback. No custom ordinary input, new search engine, dependency/palette swap or vendor edit is introduced. Native geometry, shadow space, real GPUI pixels and Win10 behavior must be reconciled before product adoption. The prior region fixture alone cannot approve this surface.

## Statistics presentation

ClipboardOverview.has_snapshot distinguishes no successful response from a valid zero-count response. Only a successful original worker reply sets it. ClipboardOverview.metadata is the sole header-text projection: initial loading/unavailable stays explicit; a loaded snapshot remains numeric during background refresh; a real refresh failure retains counts and adds visible failure copy. Known errors remain until a successful reply. Missing worker handling preserves last-good values instead of resetting them to zero.

Original request generation, bounded/coalesced dirty handling, subscriptions, query cadence, worker ownership and history remain unchanged. This is not a new cache/index/transaction owner. MainWindow continues to render ModuleHeader and official Settings/Switch controls using the same entities.

## Status lane

The existing clipboard-feature-status element has stable identity and an elastic min-width-zero lane. Its width is allocated by the existing row, not by the current label length. Original feature_configuration_status still distinguishes desired intent, pending native application and confirmed success/failure; no pending/error suppression or global animation policy change.

The source-confirmed changes address loading-copy churn and status reflow. They do not establish or repair an unobserved native text-opacity/compositor failure.

## Diagnostics and regression boundary

Original correlated feature accepted/save/configuration/terminal events remain the diagnostic owner. No per-frame/pointer/content logging is added. A cfg(test)-only worker barrier runs on the existing bounded job queue and releases on normal or failed test teardown; it is not present in production.

The real Root/MainWindow/Clipboard page fixture settles a valid empty history, clicks the actual Switch, holds counts pending, injects the matching native ACK and verifies the same header/status bounds before, during and after the request with normal and reduced motion. It checks correct status labels, one ConfigureClipboard per click, original entity identity and no presentation commands in the closed-popup fixture. Existing pointer/Space persistence and motion regressions remain separate. Snapshot-state tests cover initial/unavailable, valid zero, nonzero and failed refresh values.

## Isolated Kit example

`rust/apps/nxtools-desktop/examples/quick_launch_surface.rs` is an independent Cargo entry with no imports from product main/runtime. Released0.8.0 InputGroup Large owns the only frame; public Styled overrides width620/height60/radius8 and its input slot owns20/28Regular typography. InlineStart addon owns the20DIP official Search icon. Its public background slot uses the current opaque popover token: the released group otherwise has a30% dark/transparent light backing, unsuitable as an independent desktop surface. No Toolbar or manual input frame. Current Catppuccin Latte/Frappe config and embedded Sarasa are shared assets, not a new palette/font owner.

One public Kit open_window creates Base Root. The instance-only transparent Root refinement runs on the next app turn after construction and theme changes, preserving Kit overlays/focus without leased-entity reentry. Windows PlatformWindow reports Server decorations, bypassing Component's client WindowBorder. Transparent WindowKind::PopUp uses existing GPUI composition. The preview runner's small C# helper matches only its freshly launched PID and exact preview title, applies the existing PlatformHost SetWindowShadow(false) DWM policy and reads DWMWA_NCRENDERING_ENABLED=false. Rust does not call Windows APIs; production C# and vendor remain unchanged. Visible bar620x60 sits16DIP inside an explicit652x92 transparent client for a single8DIP-blur shadow. The gutter is preview-only and not a product placement decision.

InputState is stable across F2 theme changes; Escape removes the window and last-window close exits the isolated process. No synthetic search result execution, persisted user data or background service owner is constructed. Tests use the same Render through real Root, checking actual single-contour quads/radii, opaque theme backing/no external fill, dimensions/control containment, addon pointer focus, text/entity preservation and Escape retirement. The runner builds only this example or its tests with required single-job Rust variables and the existing Cargo-lane guard; NoBuild skips Cargo. One run ID connects Rust start/close and C# native-frame start/readback/elapsed output, without text telemetry. Browser, TestWindow, real Windows11 composed output and Win10 acceptance remain distinct evidence levels.

## Transparent edge composition

User accepts the overall native example but reports visible stair-stepping on white. Stock GPUI already supplies signed-distance antialiasing; the demonstrated composition defect is additive destination alpha in two Windows blend states. The verified merged ZedPR65571/e18de65b changes these to source-over without changing shaders, radius, RGB factors or render passes. A command-local temporary backend copy and separate candidate executable preserve the ordinary preview and production source graph. Native GetDesc regression covers both factories; the same InputGroup/InputState tests cover the candidate. White-backdrop A/B shows a lighter dark fringe at the same silhouette. This does not eliminate every low-DPI pixel staircase or certify Win10/user acceptance. Exact sources, flow and scope: [source-over research](source-over-research.md).

## Direction-aware coverage

The continued isolated candidate addresses the user's rejected dark arc with the referenced L1 SDF pixel footprint. The temporary quad shader uses analytic circle/axis normals, normalizing the original linear coverage ramp at both border interfaces and border admission. Geometry, palette and straight-axis coverage remain. Source-over stays enabled. No checked-in framework change or product integration. See [coverage method](pixel-coverage.md).

The example's optional gpu-capture argument schedules explicit draw/readback after Root construction leases. The existing public Window::render_to_image returns real renderer pixels; premultiplied data is saved separately from a once-composited white view. Capture starts unsuccessful and becomes successful only after readback and all files complete; terminal process status and fixed correlation/stage diagnostics distinguish failure. Normal preview input/window behavior is unchanged.

The runner's PixelCoverage and CaptureDirectory options are fixture controls, not persisted product settings. Both backend variants and the ordinary preview stay separate; all temporary edits are recreated from the checked-in backend. Native blend-state and component tests, real GPU arc bounds/straight-pixel invariance, export failure and source provenance remain distinct gates. Final dark/light and Win10 acceptance is the user's, not inferred from test counts.

## Background coverage compensation

UnderBorder is a separate fixture candidate, mutually exclusive with directional widening. A temporary core adds the background marker and passes border data from Style::paint; a temporary Windows shader precompensates only that background and returns before the outer multiplier. Child and foreground order remain. The opaque fully-covered denominator case is defined. A separate pinned manifest/private lock avoids modifying root dependency files; the actual example source is reused.

NoShadow is capture-only and records its state in frame metadata. The single-coverage oracle checks all256corner pixels for the current opaque solid InputGroup, not a few coverage samples. Source-over remains. Original-shadow visual frames, lifecycle/component tests, native blend-state, strict candidate lint and production-source gates are separate evidence. Formula and limits: [background compensation](under-border.md).
