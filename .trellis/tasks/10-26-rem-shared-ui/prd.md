# Queue07 — Shared ordinary UI scales

## Authorized scope
Continue original finding #6 in this session, without delegation. Inventory shared production consumers and units before edits; obtain effective production Root 16/14/18 reds, then repair ordinary shared presentation and affected consumer composition. This delivery does not close original #6 or start Queue08. No complete Desktop run, native/user-data/Host/product execution, packaging/install, commits or data/thread refactors. Cursor and ellipsis/copy tasks are excluded.

## Authority and selection
Read AGENTS, shared component-selection/current UI authority, current Kit0.8.0/pre0.3.9 release delta, historical component table/index, Desktop owner and shared engineering/verification contracts. Exact released sources reviewed: Root prepare sets rem from Theme.font_size; Button Small/Medium use h_6/h_8 (icon slots size_6/size_8); Icon Small uses size_3p5; Input Medium uses input_h/h_8; Form/Field public children/footer and original label/control owners; Empty/EmptyContent public content; Alert XSmall literal message and title; DescriptionList/Tooltip/Scrollable. Released Form/Field retain internal pixel gaps (6/4); no vendor edits.

Requested shape remains existing Settings rows, actual Form/master-detail, official buttons/inputs/states and existing scroll viewports. Preserve visible variants, semantic sizes, parent transformations and single original callback. No Toolbar.child conversion (it would force ghost+compact). Shared layout does not add controllers, global input dispatch, persistence or diagnostics. Existing operation diagnostics remain owned by the original operations.

## Consumer and unit inventory (before production edits)
Detailed symbol/reference and unit captures are in evidence/components.consumers.json, metrics.rs.consumers.json and typography.rs.consumers.json. These include import references and source coordinates; they are navigation evidence, not compiled coverage. Generic method-name matches were removed. The following production call chains were inspected directly.

| Shared module/API | Actual production consumers | Current units / decision |
| --- | --- | --- |
| action: action_button/primary/icon/segmented | Downloads forms/task/settings; main shell/Downloads/Quick/Clipboard/Tools; Global; Quick panel; Clipboard history | Actual styled Small/Medium Button and ButtonGroup already semantic/rem, equal cells relative ratio. Ordinary explicit vector icon14px -> official Small. focused_action radius3 and focus border2 remain physical/state geometry. No cursor edits. |
| field: field_shell/path_picker_shell/inline_error | NewDownload original media compatibility path, settings/global Input shells | Medium Input already rem; path outer32px/gap8px mismatches -> h_8/gap_2. Alert XSmall stays official. Accepted local NewDownload ordinary shell remains behaviorally equivalent; do not modify submission/picker. |
| settings: aligned_setting_row | Download Settings 440 role (currently rem->Pixels->f32); Quick280; Clipboard300 including recorder; Global332 | min32px -> min_h_8; width f32 forced px -> actual Length. Callers explicitly choose ordinary rem, bounded by existing65% lane. Original nested controls/owners retained. |
| settings: feature_settings_window/dialog_footer/settings_shell | Tool link/currency/assistant settings; Download Settings/Schedule/delete/shutdown; Global | Title48 and footer48 are fixed native budgets. Ordinary body12/8/4 spacing and enable32 minimum -> official scale. Footer ordinary padding12/gap8 -> scale; footer allocation remains48px. |
| titlebar: ModuleHeader/client_titlebar | Main Downloads/Quick/Clipboard/Archive/Tools/shell; Download/Global auxiliary titles | Module48/title16/22/meta11/16 and ordinary12/8 spacing -> rem/official scale. Native client title36/close slot36 remain fixed; title ordinary12 -> rem. |
| state: empty/progress/error/interactive | Main Download/Quick/Clipboard; Quick results/preview/file-dialog; task fallback | ordinary minimum72px/icon18px -> rem; existing Label text_sm, padding/gaps and Spinner already official scale. Error remains distinct and natural-height, original owner scrolls long content. |
| custom_command_editor | Main Quick settings Commands (one original draft, five InputState entities, master selection, explicit Save) | ordinary master224/rows40/header40/detail48/type13/20+12/18/spacing/action32 -> rem/scale. Keep360px bounded editor viewport and original detail/list scrolls. |
| path_list | Main Quick settings Index exclusions | ordinary type13/20+12/18/insets/rails16/icon14/add32 -> rem/scale; preserve40px delete hit target and use minimum row40px so content can grow without clipping. Existing parent keyboard/identity/remove/add owners unchanged. |
| volume_scope | Main Quick Index | ordinary rows48/40/type/metadata196/spacing -> rem; retain physical border/radius. Existing one row callback/display Checkbox non-tab-stop unchanged. |
| quick_launch_diagnostics | Main Quick Diagnostics | ordinary48 rows, type13/20+12/18, label/value lanes and32 action reservations -> rem/scale; original memory snapshot, clear/log callback owners unchanged. |
| hotkey | Main Quick General | ordinary260 width and4/8 gaps -> rem/scale with max_w_full/min_w_0 to fit parent. Marker6 is ordinary status glyph; shape remains circular. Actual outlined default Button/Label/recording controller unchanged. |
| tooltip | Download/NewDownload/Schedule/task/main actions and Quick/Tools/shared help | fixed344x136 content and360x160 surface maximum are bounded overlay viewport, not font roles. Ordinary11/16 -> rem. XSmall help Button already semantic; max width guard remains fixed. |
| popover list_box | NewDownload category original ScrollHandle | flex/percent/min0 only; no absolute ordinary sizes, no change. |
| shortcut key/hint | Shared hints exposed to callers through qualified module references | actual Kbd/Label/text_xs/gap_1 already official/rem, no geometry override. |
| menu/styled_menu_surface | Download official PopupMenu; Clipboard independent menu viewport | bounded viewport widths192..240/224..280, placement margin/readback remain fixed; styled row/icon/Label sizes official. Content minimum160 is ordinary role; do not scale placement/native coordinates. |
| dialog_shell | Download Schedule/delete/shutdown | fixed caption36/footer48 with elastic clipped body; no ordinary numeric override to convert. |
| project_menu/native_context_menu | Clipboard original source-session/independent popup lifecycle, Download fallback | production native/device coordinates and placement remain fixed. Handpainted legacy rows are cfg(test), not production remediation evidence. No lifecycle edits. |
| list::compact_list_row | None (entire module cfg(test)) | historical fixed geometry only; no production consumer, keep unchanged rather than claim adoption. |

## Metrics and typography classification
Constants retain their values/types to avoid accidental cross-family scaling. ordinary_rem remains rems(design_pixels / 16), never divides by current rem. Typography keeps existing families and all16/14/12/11/10,22/20/18/16/14 and workbench13/20+12/18 role numbers. Consumption, not numeric roles, is repaired here. Remaining direct per-page pixel consumption belongs to Queue08–10, not an implicit global rewrite.

Fixed/native roles: TITLEBAR36, SETTINGS_FOOTER48, feature title48/client geometry, SIDEBAR176, pixel coordinates/readback/monitor placement, hit geometry, physical border/selection edges, bitmap/media source geometry, taskbar progress geometry, status glyph28 where used as a symbol. Module header48 is ordinary module content, not native caption. COMPACT_RESULT rows/rails must be reconciled with actual Quick result controller in Queue09; not globally converted from a shared constant. RADIUS and focus widths stay current component/native geometry. SPACING constants are design-base values; each use must classify its role, not globally change them to rem.

Approved fixed exclusions: Tools180x160 tiles; Clipboard persisted text-card cap120..320/default160, image190 (media144), file62, external gap8, source24, overflow feather14 and static shadow offset1/sigma1/alpha8%; preview/native caps/hit/payload/full-by-ID owners. Shared state/type/controls must not resize these cards or their caps. Fixed Tooltip/menu viewports and command editor360 remain bounded scrolling allocations, not a font-scaling policy.

## Reference and invariant review
ABDM LocationTextField and SettingsPage reviewed before legacy download compatibility: original field/action callback and bounded scroll remain. Legacy SettingsWindow constructor -> WindowAnimationService.Attach -> OnOpened/Configure -> Apply or Cancel/Close -> OnClosed and animation Closing cancellation/final Close reviewed. MainWindow constructor/Open/close-to-tray/final close reviewed. Avalonia/AWT native base implementations unavailable, not inferred. Zed ButtonLike/ListItem are presentation/one-action precedents, not numeric authority. SwiftList plugin Templates master-detail was inspected; no legacy Quick custom-command feature exists.

No lifecycle, cancellation, placement, native focus/show/hide, submission, persistence or operation-diagnostic changes. Shared callback bodies remain byte-preserved except layout parameters. Test actual pointer and key release through original controls, not direct business calls as positive evidence. Keep old first-seven and accepted #6 tests unchanged; legitimate new shared layout deltas are recorded separately from immutable older manifests.

## Verification plan
1. Live dirty snapshots/hashes before edits, then production Root16/14/18 effective layout reds.
2. Shared real Form/Input/Button/Empty/Alert and consumers; fixed footer/minimum windows; long content scroll-to-end, actual pointer/Tab/key release once, retained input entities.
3. Scoped affected regressions, first-seven names/identity/keyboard/display and accepted audit06 slices. No complete Desktop run.
4. Serial natural-idle Rust with jobs1/incremental0/codegen256; strict Desktop Clippy, workspace check, scoped fmt/diff and observed workspace fmt, source/catalog/release/editor/spec gates.
5. English outcomes, exact forward/reverse dirty deltas, source snapshots, immutable logs/hash manifest and read-only verifier. Compiled synthetic fonts16/14/18 are not Windows DPI/native/reader/visual/user/release acceptance.

## Verified integration addendum
Production Root18/920x480 exposed an original nested-scroll mismatch after ordinary roles scaled:
Save bottom475 exceeded detail viewport bottom461 and actual pointer Save did not persist.
Naked overflow_y_scroll did not receive wheel because the released outer Settings ScrollBounce
claims capture-stage ownership. Form measured bounds included Save; flex/nonshrinking attempts
were ineffective and removed. Public overflow_y_scrollbar on the original detail content joins
the released nested arbitration with one stable ID, retaining360viewport/Form/entities/callbacks.
The real wheel/current-hit/persistence regression now passes. This is an affected shared-surface
adaptation, not permission for a new dispatcher/controller or vendor change.

The final fixed-exception fixture keeps image190/file62/gap8/source24/feather14 exact; text160
is a maximum, not forced height (natural long-card159 at base14 remains <=160 and not rem140).
Original old tests are unchanged. Final production-only strict Clippy remains blocked by the
task-start-identical tools/floating_assistant/media.rs::DecodedMedia.animated dead-code field;
no suppression or out-of-scope repair is authorized. Test strict/workspace check/fmt are distinct.
