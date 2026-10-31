# GPUI Kit0.8.0 current release and migration index

## Authority and scope
User explicitly authorized the product upgrade2026-10-21. Current root pins `gpui-kit = "=0.8.0"`, coherent Base/Component/Assets0.8.0, default gpui-pre0.3.9. No GPUI Fast/new Markdown editor/JS Shell/WebView feature is enabled by this upgrade. Historical0.7.1 API index and2026-10-15website0.7.3catalog/page hashes remain historical evidence; they are not rewritten as a fresh79page audit. Read this release delta before those tables, then exact current released source for each implementation.

## Verified publication
Official https://gpui-kit.com/zh-CN/releases/ v0.8.0 published2026-10-19; crates.io lateststable0.8.0/notyanked at review. SHA256-verified published archives (not Gitmain):

| Crate | SHA256 |
| --- | --- |
| gpui-kit0.8.0 | 3cdcb78553bef8785669e572e79759b0eab47fdd9d318fe0cce1def798189432 |
| gpui-base0.8.0 | d5cd7de73d5b2d3d18b94a928abf343922091fa77bc472f77b7d2e7e8b1823a7 |
| gpui-component0.8.0 | 4097e161cc81e86323b439664be0cdd4f7c56148e867ed6f67fd72b966b6c2d1 |
| gpui-pre0.3.9 | 34f61c1cfd36fc9c436af09308c963822ae27a45a84feb21a77ba5c9e051f1c0 |
| gpui-pre-windows0.3.9 | 513d4f0fc7d596d3bdef2a73a524ffb83dabc95bf484f2855a698e7f431290a9 |

The currentWindows target `cargo tree -p nxtools-desktop --locked --target x86_64-pc-windows-msvc --prefix none` confirms pre0.3.9/noFast active. Cargo.lock may retain upstream optionalFast packages; that is not feature activation. Keep exact dependency graph verification separate from download/status lines.

## Exact API / behavior delta relevant to this product
| Surface | Released0.8.0 source/API | Preservation requirement |
| --- | --- | --- |
| Existing controls/compound | Original79catalog coordinates all still have release files; Component list/list_item.rs,toolbar.rs,button/button.rs,setting/*,select.rs | Existing ListItem/Settings/InputGroup/Select/Root/menu/Notification owners and variants stay. Public Toolbar.child still prepare_for_toolbar -> ghost().compact(); outlined commands retain direct/content composition. Availability is not adoption. |
| Markdown display | Component text/compat.rs -> Base text; defaulttypography/paragraphgap0.75rem, table/selection changes | Natural content layout plus user textcard210DIP/image190/file62/externalgap8. Fullsafe400charsummary/64KiBpreview resources/payload/native staging unchanged. Tests use actual geometry, not old style assumptions. |
| External Markdown editors | Base text/editor_host.rs exposes BlockNode/RenderContext/parse_markdown/collect_leaf_geometry | Public API is now present; no thirdpartyeditor is added automatically. Its model/focus/input still requires separate approved integration. |
| Diff | `gpui_kit::component::diff::{Diff, DiffState, DiffFile, DiffMode}`; Component diff/mod.rs/state.rs/document.rs | Diff is available in0.8.0; historicalcatalog unavailable applies only0.7.1. Do not treat availability as permission to add a new product feature. |
| Settings accessible names | Component setting/fields/dropdown.rs accessiblelabel `title: selectedlabel`; bool/number/string inherit title | Tests target semantic setting+value; visiblelabel and callbacks unchanged. |
| Disabled controls | Base a11y_disabled.rs marks original accessible node without extra layout/focus | Disabled snapshots nowSome(true), notNone; retain actual disabled pointer/no-dispatch tests. No claim real reader/provider acceptance from fixture snapshots. |
| Scrolling | Kitrelease nested gesture ownership, Textarea/Editor/scrollableTextView | One existing scrollowner per viewport; verify longcontent/end/footer and no unintended parent gesture behavior. Windows does not adopt macOS bounce default. |
| Native / input | gpui-pre0.3.9 Window::handle_input/intercept_keystrokes/observe_window_activation; InputState.input_bounds/markedrange retained | OriginalClipboardtype-anywhere bridge, nofocusCtrl-digit, IME/editor/dialog/menu, samewindow and original actioncorrelation survive. Primarypanel activates; passive previews do not. |

## Exactly two carried source exceptions
- `repo:vendor/gpui-component-080/NXTOOLS-PATCH.md`: optional notification duration forwards into official ToastOptions.timeout. User2026-10-23 also authorizes additive `Notification::header_action` in the same notification.rs, for a compact Copy button left of original Close; optional/defaultNone/officialEnding omitted, no extra clock/globalCtrlC/clipboard transport. Unconfigured5s, product1–60/default6 and original lifecycle remain. Fork preservation: https://github.com/Nuxoul/gpui-kit; current task `.trellis/tasks/10-23-notification-copy-button/` records commits/limits.
- `repo:vendor/gpui-pre-windows-039/NXTOOLS-PATCH.md`: `WM_MOUSEACTIVATE` reads live `WS_EX_NOACTIVATE` -> `MA_NOACTIVATE`, otherwise `MA_ACTIVATE`. C# owns the style; original Rust staging/GPUI rendering is preserved. Published 0.3.9 did not contain this fix.
- Perfile upstream hashes in eachNXTOOLS-UPSTREAM.json; oldvendor0.7.1/0.5.1/Windows0.3.8keptinactive/unmodified. No registry sources edited. Only approvednotification.rs/events.rs implementation deltas.

## Verification and limitations
`python tests/architecture/gpui-release-contracts.py` verifies lock/source/provenance; optional --upstream compares independently extractedarchive hashes and --active-tree checks actual enabledtree. `python tests/architecture/gpui-component-catalog.py` retains historicalpage/table/viewer equality and now checks currentrelease authority separately. Neither validates nativeactivation, screenshots or useracceptance.

Compile/fulltests/strictlint/workspace/lifecycle/fmt and source/native evidence levels are recorded in `.trellis/tasks/10-21-gpui-kit-080-upgrade/outcomes.md` when settled. This upgrade itself does not authorize package/install/product/service launch, physicalpointer/keyboard/WindowsIME/DPI/reader or releaseacceptance. Preserve original reference lifecycle invariants and deferredentitycallbacks across framework upgrades.

## Separately approved2026-10-22 integration
The completed upgrade itself did not adopt an editor or recolor defaults. A later explicit user-approved six-item plan now adds [pinned unified Markdown](markdown-editor.md) and [project Graphite defaults](graphite-theme.md), retaining the same0.8.0/pre0.3.9 graph. This does not expand the two framework patches or relabel historical catalog audits. Editor57-file provenance has its own gate: passive element,3cull lowbits,and separately approved single equivalent serializer expression with two exact format fragments. Defaultsyntect/alllanguages/Fast remain disabled.

## Audit #3 actual Button accessible-name prerequisite (2026-10-25)
Verified exact Component0.8.0 button/button.rs::Button::render: explicit accessibility_label
falls back only to Button.label, not icon, arbitrary child content or Tooltip. Public
Button::accessibility_label supplies the original Base Button semantic node without new layout,
focus or activation owner. Shared primary custom-content and icon helpers therefore need an
explicit semantic-name contract; outer Tooltip names do not repair their inner Button.
Reference-only recheck: ABDM LocationTextField.kt::LocationTextField uses download_location
contentDescription and original directory picker callback; AddDownloadPage.kt::MainActionButtons
keeps named Add/Download/Cancel. Legacy SettingsWindow constructor -> WindowAnimationService.Attach,
OnOpened -> ConfigureForCurrentScope, SaveButton_Click -> ApplySettingsCommand -> Close, and
VideoSettingsWindow.axaml BrowseFfmpegExecutableCommand retain action purposes and owners.
Avalonia native base unavailable. Quick Find note correctly records no legacy exclusion surface;
SwiftList ExclusionsTab.xaml -> Templates.xaml::RuleItemTemplate preserves one trailing remove
command. Names-only correction must keep original visible copy/IntoElement paint, component
variants/sizes/parent adaptations, IDs, single pointer/keyboard click, disabled/loading, picker/
submission/focus generations and native/deferred boundaries. Full provider/reader acceptance is
not implied by semantic fixture snapshots; task .trellis/tasks/10-25-button-accessible-names/.
