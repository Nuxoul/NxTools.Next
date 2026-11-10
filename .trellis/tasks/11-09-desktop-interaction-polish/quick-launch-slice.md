# Quick Launch result entrances

## Layout and released APIs

Expanded results use the stable 620×530 DIP viewport, clamped by the existing monitor/placement owner. Result counts, provider completion, action/recent-directory contents and error footer do not resize it. Empty input outside those modes, hide and reopen retain the 60 DIP search-only lifecycle. The 58 DIP header, 52 DIP result body, 32 DIP icons and optional 4/0 DIP external row gap remain.

Settings use the existing Kit0.8.0 Settings → SettingPage → SettingGroup → SettingItem → SettingField::scrollable_dropdown. Its released dropdown.rs renders an outlined Button at the existing Settings Medium/32 DIP size, with official PopupMenuItem choices and no Toolbar adaptation. The callback defers the original QuickLaunchApplication saver, changing only result_entrance in AppConfig::update_section. Failed persistence does not change the live setting.

Rows retain the existing approved focusable Base Button shell, virtualized uniform_list, Label and Input owners. Motion only offsets/fades row content inside the stable hit/focus/layout rail. Released Base0.8.0 motion::Timing with Easing::EaseOut supplies a single finite 120 ms Tween; no spring, repeating animation, new animation engine or native bounds interpolation. Modes are slide_up (default), slide_from_right, fade and none. Slide Up begins 8 DIP below; Slide From Right begins 12 DIP right; animated modes fade to full opacity.

Admission is queued by the existing result publication owner and consumed at the first exposed viewport paint. The real GPUI viewport and original scroll offset decide visible identities; offscreen identities are recorded final. While the native expansion is pending, no entrance frame/timer is requested, and its latency cannot consume the entrance. Repaints only sample existing released Timing values. Terminal diagnostics are aggregate generation/correlation/count/category/elapsed transitions, never per-frame events or payloads.

## Reference and lifecycle invariants

Listary managed call chain rechecked: LauncherSearchWindowFactory.cs18 → LauncherSearchWindow.cs24/base constructor → SearchWindowBase.cs268 event wiring, native ShowWindow333–348/ActivateWindow323–329 and base.Hide274. Launcher positioning remains LauncherSearchWindow.cs50–69; one adjacent PreviewPopup remains owned by the launcher. SearchMode retains committed results and cancels stale generations. Native engine source is not available; legacy NxTools has no Quick Launch owner.

SwiftList secondary source: QuickSearchWindowController.cs71–75 constructs position/foreground owners; ShowWindow127–207 prepares layout/position before Show191 and restores activation/input focus; HideWindow314–404 resets preview before query reset and token-checks deferred Hide361/focus restoration367. Existing Next primary activation and passive preview contracts remain unchanged.

The user-approved fixed expanded viewport supersedes count-driven geometry and 11-05 height settling. Each newly admitted identity is recorded once per query generation, including delayed publication; offscreen admissions render final and scrolling, selection, repaint, icon completion and reorder cannot restart motion. New query, motion suppression, disabled mode, hide, close and disconnect finish immediately. No animation task/timer survives retirement. Cross-owner preview/native/settings publication stays on the next application turn; motion decisions remain local to the panel.

## Verification

Behavioral red evidence supplied by the parent: legacy result_entrance missing instead of slide_up; content height and first native SetWindowBounds requested 132 rather than 530 DIP. Focused filters: Core quick_launch_result_tween_; Desktop quick_launch_result_tween_, quick_launch::tests::refresh_tests, quick_launch::tests. Parent owns serialized compiled tests, Clippy/fmt and architecture/provenance gates. No product launch, packaging, native/physical pointer/DPI or user visual acceptance is implied.

Implementation adds Core legacy/default/all-mode/unknown-mode persistence coverage, actual Settings dropdown pointer-to-durable-save coverage, production panel delayed-admission/nonreplay/viewport-wait/suppression/hide/close/disconnect tests and finite-state/stale-generation tests. Existing geometry regressions now assert the fixed expanded viewport and unchanged compact/header/scroll/focus boundaries. Owned Quick Launch Rust files were formatted with direct rustfmt and skip_children; Core's shared file was not whole-file formatted. Compiled tests, lint, architecture/provenance gates, physical/native/DPI and user visual acceptance remain with the parent.
