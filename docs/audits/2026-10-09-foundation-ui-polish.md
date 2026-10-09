# Foundation UI polish — first product-source delivery

Date: 2026-10-09. Existing owner: `.trellis/tasks/10-02-staged-desktop-ui`, exact phase **Foundation**. User explicitly requested implementation after the download-window recurrence. This is product UI source work, not another standards-only delivery. Preserve unrelated working-tree changes; no commit/archive or branch switch.

## Visible source changes

- Approved palette is now the product Theme mapping: base `#18181b`, work surface `#1f1f23`, recessed well `#121215`, raised/hover `#27272a`, accent/focus `#0ea5e9`, hover accent `#38bdf8`, primary/secondary/muted text `#f4f4f5/#a1a1aa/#71717a`. Semantic green/amber/red values retained. Selected and keyboard-highlight tints remain distinct; pressed neutral surfaces also have their own role.
- Production startup and visual fixtures share `theme::apply_component_theme`: fonts/type roles, radius, focus/caret/selection, primary/secondary buttons, lists/popovers/scrollbars. Existing transparent window background/native alpha boundary remains unchanged. No scattered color rewrite or dependency/vendor patch.
- Shared control ladder is now 24/24/28/32 logical pixels; compact/default aliases share24, existing action/input remain28/32. Control radius3, floating6 retained. Primary cyan controls use dark well text rather than white for legibility; hover is bright cyan, pressed returns to the main accent, not the same hover color.
- Main module rail remains208px wide and titlebar36px. Navigation rows become48px, with4/8/16 spacing, selected2px accent rail, secondary readable descriptions, stable2px focus border and no focus-induced layout shift. Module controller/state ownership and typed native titlebar commands are preserved.
- Tools is the representative work surface: shared48px module header and a full-width continuous80px-minimum tool entry with icon, title/purpose, enabled state and configure copy. Replaces the isolated240px card grid, not a new feature. Pointer/Enter/Space open or reuse the same registered per-tool settings window; settings persistence remains its original deferred owner.

## Interaction boundary and references

Read GPUI reference note before Zed ButtonLike/ListItem and legacy NxTools MainWindow constructor/Open/Closed/OnClosing/tray flow plus WindowAnimationService Open/cancel-first-close/final-close. Avalonia native/base source is not present. Current MainWindow titlebar still routes Windows operations through C#; no show/hide/resize/native lifecycle change.

Pinned GPUI0.2.2 `elements/div.rs` generates focused `ClickEvent::Keyboard` on **KeyUp** for Enter/Space. The shared `focused_action` reuses track_focus/on_click; it does not add a second activation handler. Tab/Shift+Tab use current GPUI focus navigation; held keydown is suppressed, modifiers retain the built-in non-activation rule, disabled/busy eligibility rejects pointer and keyboard. Module switch clears orphaned page-menu focus then returns it to its navigation entry. No synchronous cross-owner callback write introduced.

Initial reasoning from the vendor Button leaf alone incorrectly suggested no automatic keyboard activation. Deeper GPUI inspection corrected that before implementing any duplicate activation handler. TestAppContext simulate_keystrokes supplies KeyDown only, so tests explicitly send actual KeyUp and repaint after focus transitions. Do not weaken the production boundary to accommodate a shallow test input.

## Red/green and integrated defects corrected

1. Approved palette regression failed before implementation: actual base `#1e1f22`, expected `#18181b`.
2. Production sidebar keyboard-switch regression failed on old sidebar: QuickFind remained active after navigation gesture instead of Clipboard. Actual focus plus KeyUp regression passes after the shared boundary migration.
3. First full desktop run: **439 passed / 5 failed**. Four content-budget failures were caused by an attempted global settings row42->48 change (DASH355->361, media error462->480, network error/footer overlap and trailing inset). That global change was withdrawn; existing fixed-client settings row42 is staged migration debt, not an approved grid exception. No ordinary176/278 or media window-height workaround. The fifth failure exposed selected and keyboard-highlight colors becoming equal; semantic tints were separated. All five exact cases passed individually afterward, without relaxing footer/overlap assertions. The network toggle's expected boundary now follows approved shared24px rather than old22px.
4. The single directed SOL review found two concrete issues: explicit global tab indices interleaved index0 page controls before Quick Find, and primary hover/pressed were equal. Added actual Downloads->QuickFind and reverse-focus coverage plus distinct pressed assertion. Pre-fix focus navigation fell into a page input and hit the test fixture's absent Root path; pressed assertion failed directly. Removed global indices in favor of render order; separated active/hover in both component and custom mappings. Expanded regressions pass. No further review loop.

## Verification performed

Final source candidate:

- Foundation focused tests **6/6**: exact approved palette, shared component palette/pressed separation, shared density, actual module Tab/Shift+Tab + Enter/Space selection, Tools minimum-window geometry/settings reuse, one-dispatch/held/modifier/disabled action behavior.
- Tools actual GPUI bounds at920x480 and1060x480: continuous entry has16px content inset, status/configure do not overlap description, navigation rows remain inside client, focus does not change hit bounds. These are actual production render geometry assertions, not native screenshots.
- Final full desktop suite **444/444** after concrete corrections; all download/clipboard/quick-launch and settings tests included. Earlier full failures retained above. A second final run was performed after the two directed review corrections, not blind retry-to-green.
- `cargo clippy -p nxtools-desktop --tests -- -D warnings`: pass, no new warning suppression.
- `cargo check --workspace --tests`: pass.
- `cargo fmt --all -- --check`: pass.
- Eight final architecture gates pass: ui-token-boundaries, ui-consistency, ui-state-lifecycle, desktop-shell-lifecycle, download-ui-lifecycle, quick-launch-lifecycle, clipboard-lifecycle, clipboard-ui-lifecycle. Token gate now enforces approved shared dimensions/palette/component focus mapping while preserving prior boundary scans.
- Document navigation/scenario checks and diff checks pass. Rust commands use CARGO_BUILD_JOBS=1, CARGO_INCREMENTAL=0, RUSTFLAGS=-C codegen-units=256; one Cargo at a time.

Scoped source baseline/current deltas live in session scratch `foundation-before/` and `foundation-turn-diff.log`; theme baseline includes newly added red test but unchanged production palette. Logs: `foundation-red.log`, `foundation-focused-green.log`, `foundation-bounds-actions.log`, `foundation-integrated-readable.log` (first439/5 failure, decoded from mixed PowerShell encoding), `foundation-retest-*.log`, `foundation-review-red.log`, `foundation-review-green.log`, `foundation-desktop-final.log`, `foundation-workspace-check.log`, `foundation-clippy-final.log`, `foundation-format-final.log`, `foundation-static-gates-final.log`. No DEBUG probes remain.

## Honest visual and release boundary

**No actual before/after GPUI screenshots were captured.** No product launch, package/install, service or DOpus operation was authorized/performed in this UI turn. A browser mock or diagram would not be native GPUI evidence and was not substituted. The current installed09:19 Validation package contains neither this Foundation work nor the download recurrence follow-up. Native pressed-pointer/probe/release/DPI/footer acceptance remains pending under its existing window owner.

Foundation is a source delivery, **not visually accepted or phase-complete**. Still pending: supported100/125/150% native screenshots, real keyboard/pointer/focus review and user approval; common reduced-motion preference propagation; pinned GPUI provider/name-role capability; older caption/custom-action parity; module-local2/6 and measured fixed-client row migration; large radius8 role migration. This does not certify whole-app accessibility or every component state.

Canonical next stage remains **Downloads**, but it may start only after Foundation's required review/acceptance. No Quick Find/Clipboard + Tools/Global settings phase started, no adjacent utility implemented. Shared palette/control inheritance is not a claim that module-specific layouts were redesigned. Existing staged task remains in_progress. Task start recorded status in degraded session-pointer mode; no branch switch to satisfy the task tool's same-base warning.

## Native preview follow-up — 2026-10-09 11:00 session

User authorized showing/checking the actual interface. Added explicit Debug-only `--ui-preview foundation` interception before protected redirect/instance/runtime startup; Release rejects this option before runtime. MainWindow/Root/theme are real production views with invalid business refs, no AppPaths/config/clipboard worker/platform/indexer connection. Separate download fixture uses the real NewDownloadView/window options, an in-memory engine and dropped platform endpoint with synthetic example.test metadata; no probe or submission was invoked. This is isolated native rendering, not the real product startup/service workflow.

Captured only owned HWND client images via PrintWindow(flags3) after GetWindowThreadProcessId equality; no full-desktop capture. Final current Debug build at native scale1 produced artifacts/ui-foundation/foundation-native-100.png1060x650, foundation-native-minimum-100.png920x480, foundation-native-keyboard-100.png920x480 and download-confirmation-native-100.png560x278. Gallery: artifacts/ui-foundation/index.html. Images contain actual native GPU rendering, not HTML redraws. Window-message pointer selection produced Downloads, then Tab/Enter KeyDown+KeyUp produced QuickFind; fixed low-noise preview diagnostics confirm these states. This is message-level input, not physical pressed-pointer drag acceptance.

The first detached launch disappeared before a subsequent tool call without terminal/error evidence; no screenshot occurred because ownership check rejected its dead HWND. Subsequent captures launch/validate/capture/close within one bounded call. Final run reports ui.preview.stop; no leftover preview/PlatformHost/Indexer processes. No real clipboard/DOpus/service/installer operation. Existing installed package is unchanged.

The browser work-panel screenshot tool repeatedly returned `browser guest is not available` even after navigation succeeded. Therefore the agent could not directly inspect image aesthetics or declare footer visibility from its own visual observation; sampled colors only reject a uniformly blank image, not certify UI. Actual PNGs are delivered to the user for viewing. The download fixture starts directly at confirmed278 with synthetic metadata: it does not exercise176->278 probe/move/release or the C# recovery command. Real recurrence acceptance remains pending.100% current-state screenshots now exist; historical before screenshots and125/150% physical DPI remain absent.

Final checks after preview additions: desktop445/445, strict desktop Clippy, workspace check --tests, fmt check, five source gates (preview isolation/token/shell/download/Quick Launch) pass. Debug native build succeeds; Release rejection is source-gated, not claimed as a new Release build/runtime check. Normal startup/regression tests remain green. Preview lifecycle diagnostics are intentionally Debug-only, bounded state events, not temporary high-frequency probes. No extra review loop or phase advance. Session evidence: native-preview-final-{tests,clippy,check,format,build,gates}.log, native-preview-interaction.stdout.log; capture scripts remain scratch-only.

## Consolidated continuation supersedes pause/current-image status
The user requested all remaining modules/common capabilities before one closeout. See [consolidated audit](2026-10-09-consolidated-ui-polish.md) for final474 desktop tests, strict production/tests lint,11 gates, updated native100% PNG hashes and successful gallery viewing. Earlier per-stage pause/guest-unavailable/current-image notes above are dated historical evidence, not today's implementation stop. Provider/native/full-budget acceptance remains incomplete; no package/install or release certification.
