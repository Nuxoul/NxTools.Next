# R2 history visibility — red handoff

## Status and owned files
Tests-first checkpoint only; production visibility/layout behavior is unchanged.
Parent must run red before authorizing the smallest fix. No Cargo, rustfmt, build,
product/native execution or user history access was performed by this owner.
Existing dirty changes in history.rs and clipboard.md were retained.

- `rust/apps/nxtools-desktop/src/ui/history.rs`: test-only no-ink/absolute Canvas
  descendants at the actual action rail and state-icon containers, plus registration.
- `rust/apps/nxtools-desktop/src/ui/history_feedback_tests.rs`: five synthetic
  production-render/paint, real Root/Tab/release and asset-raster regressions.
- `docs/reference-code/clipboard.md`: append-only R2 DrawRow/constructor evidence.

## Reference and invariants (before fix)
Complete `docs/reference-code/gpui-kit-component-index.md` read (377 lines), shared
architecture/engineering/verification/UI and Desktop owner contracts reviewed.
Matching clipboard note read before legacy source inspection.

Legacy `D:\Code\NxTools\Controls\ClipboardHistoryListView.cs:79-99,249-299`:
constructor attaches one pointer/wheel owner; selection/hover affect row brush,
favorite edge/icon and pin icon remain independent item-flag projections.
DrawRow has no inline management toolbar; R2 intentionally preserves Next's richer
existing toolbar but rejects selected-only reveal.
`ClipboardHistoryPopupWindow.axaml.cs:61-167,206-259` constructs once, restores search
focus on Open/Activated/Prepare and hides menu -> preview -> public base Hide.
`ClipboardPreviewWindow.cs:29-95,200-237` is one passive topmost tool window and owns
WS_EX_NOACTIVATE/SWP_NOACTIVATE. Avalonia native base source is unavailable.

Preserve: 300x440/default resize and existing 60+4 text row/variable image height;
selection independent from hover; item-flag pin/favorite identities visible in idle,
selected, hover and keyboard states; no source/action overlap or shifting hit bounds;
original copy/paste/modifier/menu callbacks; one Root Tab/release owner; original
keyed row/action focus handles; one passive preview and all generation/cancellation,
placement/noactivate/failure-retry/deferred entity boundaries. No new operation or
logging stream is introduced; never log pointer/frame or clipboard payloads.

## Exact released component selection
Published source inspected at
`C:\Users\Administrator\.pi-desktop\scratch\b54c35cc-1a3b-45d3-ba44-64d6f226efc4\gpui-kit-071-reference\`.
Crate-relative paths below are exact locked releases, not upstream/latest/private API.

| Official candidate | Released public API / inspected render seam | Decision |
| --- | --- | --- |
| [List](https://gpui-kit.com/zh-CN/component/list) | gpui-component0.7.1 `src/list/list.rs:67-118`, ListState::new(delegate,window,cx), internal query/selection/scroll; documented same-height rows | Retain existing variable-height GPUI ListState/query worker/controller; no controller migration in R2 |
| ListItem | `src/list/list_item.rs:44-139,170-276`: new(id), selected(bool), accessibility_label, children, suffix(builder), public InteractiveElement; render projects selection and group hover | Retain adopted stateless styled row/public children, adjust only presentation after red |
| [Toolbar](https://gpui-kit.com/zh-CN/component/toolbar) | `src/toolbar.rs:166-259`: new(id), content/contents(non-sized), child/children(Sizable), disabled, Sizable; render -> BaseToolbar with original roving focus | Retain existing XSmall/public content and stable ID; no second Tab/activation owner |
| [Button](https://gpui-kit.com/zh-CN/component/button) | `src/button/button.rs:241,371-480,592-881`: new(id), ghost, xsmall, selected, icon, accessibility_label, tooltip, on_click, tab_stop; render uses public keyed FocusHandle -> Base Button; XSmall icon20px; pointer avoids focus, single release-click | Retain existing action callbacks/focus and20px tier; no manual ordinary control paint |
| [Icon](https://gpui-kit.com/zh-CN/component/icon) | `src/icon.rs:104-139,175-225`: new(Into<Icon>), path/data, Styled size/text_color; render -> Svg path/data | Retain actual Material Keep/Star asset paths; no font ligature or replacement SVG |

GPUI-pre0.3.8 `src/elements/div.rs:2524-2537` stops painting descendants when
visibility is Hidden; debug bounds are recorded BEFORE that check, so bounds alone
cannot prove visibility. `src/elements/svg.rs:149-185` -> `Window::paint_svg`
(`src/window.rs:4849-4908`) -> production asset loader/SVG alpha rasterizer -> scene.
Test-only zero-ink absolute Canvas descendants observe that real paint boundary;
they do not inject a visibility predicate, render replacement icons, affect flow
layout, input ownership or product builds. A separate actual GPUI SvgRenderer test
rasterizes both production NxToolsAssets and asserts nonempty distinct alpha masks.
No public scene-sprite getter was found; no upstream/private changes are authorized.

## Root seam and expected red
Current history.rs metadata slot hides for `selected || actions_focused` and group
hover (286-291); pin/favorite are children of that slot (296-309). Action rail hides
only for `!selected && !actions_focused` (now327-330 after test-only insertion).
Thus default selected first row paints actions, while selected/hovered/focused rows
lose independent status icons. ui/mod.rs:2440-2486 supplies the existing public keyed
focus projection, including the row focus handle; do not rewrite that caller/owner.

Parent filter: `cargo +1.96.0 test -p nxtools-desktop ui::history::feedback_tests -- --test-threads=1`
with required jobs1/incremental0/codegen-units256,21600s,one Cargo coordination.

New tests (Rust built-in tests create a public GPUI TestApp with NxToolsAssets,
NoopTextSystem and one actual Base Root; no native platform/window):

1. `r2_selected_first_row_without_hover_or_keyboard_focus_does_not_paint_actions`
   — expected RED: selected=true, real focus=None, pointer outside; action descendant
   reaches paint. Selection=false negative control also checks idle hiding.
2. `r2_pin_and_favorite_paint_independently_in_idle_selected_hover_and_focus_states`
   — expected RED: flag matrix vs selection/hover/real keyboard focus; status paint
   disappears on hover/selection/focus. Requires fresh draw each observation.
3. `r2_pointer_hover_reveals_then_retracts_actions_without_selection_or_dispatch`
   — compatibility: actual group hover/exit, unchanged20px delete hit bounds, no callbacks.
4. `r2_root_tab_and_key_release_keep_one_row_and_management_owner`
   — compatibility: real Root Tab -> row, Enter down0/up1; Tab -> Edit, Space down0/up1,
   management never pastes, blur retracts. No injected actions_focused bool.
5. `r2_production_pin_and_favorite_assets_rasterize_distinct_nonempty_ink`
   — production asset lookup and real GPUI rasterizer at compact scale; independent
   icon artwork/ink, not source-string/asset-existence-only assertion.

Compiled red/counts and green remain pending parent; tests may need ordinary compile
or harness adjustment based on that first run. No fix has been applied. Existing
history_menu_tests/history_kit_migration_tests/polish_tests remain unchanged.

## Verification
- Cargo/tests/fmt: deliberately deferred to parent red barrier.
- Static/spec gates: PASS `python tests/architecture/spec-contracts.py` (87 documents,
  12 owners); PASS scoped `git diff --check`. Neither establishes product behavior.
- Native/installed/physical/DPI/reader/visual acceptance: not executed or certified.

## Parent repair and integrated verification superseding checkpoint
Selected-only action paint failed on actual boundary; parent fixed hover/focus-only visibility and moved flags outside hidden metadata. Test fixture render-only keyed-state misuse corrected; public focus_next starts empty-focus TestApp, then real KeyDown/KeyUp and Root Tab verify one row/management release. Sixhistory regressions nowpass, including actual flag paint/assets and240/300/512 no-overlap; full674Desktop passes. Original history callbacks/worker/window/input owners unchanged. Test-only zero-ink Canvas nowrecords full clipped bounds, not injected visibility, product unaffected. Exact logs/intermediate errors/installed limits in outcomes.md. No actual screenshots/native clipboard acceptance.
