# Markdown Reader compact settings

## Item 4 component selection

Target: retain the existing460×320 feature window, standard feature caption and first-body enable row. Reading preferences precede Windows file association, followed by the existing preview action. The shared feature shell remains the only vertical scroll owner; preferences retain immediate saves.

Reference authority: `docs/reference-code/markdown-reader.md`, especially Tool configuration and Explorer reference, and the owning task PRD. No independent reader/preferences owner exists in the recorded legacy NxTools reference. Read-only inspection of `D:\Code\NxTools\ViewModels\MainWindowImageViewModel.cs:3591–3605` confirms that its Markdown action only copies image syntax. The existing Reader settings section transaction, command UUID, source window, deferred notification ingress and per-extension observations remain authoritative.

Exact current sources reviewed:

- `vendor/gpui-component-080/src/setting/{settings,item}.rs`: full Settings is a searchable, resizable multi-page compound with a sidebar, keyed selection/search state and a ≤480-DIP stacked-field breakpoint. This is suitable for application settings, not a second workspace inside this single compact feature shell. Its public custom render slot does not remove those parent owners.
- `vendor/gpui-component-080/src/group_box.rs`: public `GroupBox::new().id(...).title(...).child(...)`, default Normal variant, no painted frame or content inset. Public `Styled::gap_2` narrows title/content spacing to 8 DIP. A named GPUI `Role::Group`/`aria_label` layout parent supplies semantics because this release's GroupBox itself only supplies styled grouping.
- `vendor/gpui-component-080/src/form/{form,field}.rs`: `Form::horizontal().small().label_width(px(264.))`, official `Field::new().items_center().gap_0().label_fn(...).child(...)`. The private field composition follows `main_window/tools_page.rs::compact_tool_field` without modifying it: a minimum 32-DIP row and a right-aligned elastic control lane. At 460 DIP, shared 12-DIP body padding leaves 436 DIP; 264-DIP labels + official 4-DIP horizontal field gap leave 168 DIP for controls. The shared `aligned_setting_row` was evaluated; it is designed for SettingItem custom slots, so the analogous native Field composition avoids nesting another title/control row inside a Form field.
- `vendor/gpui-component-080/src/button/button.rs`: Small outlined buttons retain official 24-DIP height, padding, focus, disabled semantics and single click owner. Both font actions, all four association actions and preview stay direct Buttons, outside `Toolbar.child` (which would force ghost/compact). Association actions use natural wrapping, not fixed widths.
- `vendor/gpui-component-080/src/switch.rs`: default Medium 36×20-DIP switches, centered in the same 32-DIP row slots, with explicit accessible names. Form size controls field spacing, not child sizes. Existing requested-value/deferred callbacks are retained.
- `rust/apps/nxtools-desktop/src/ui/components/settings.rs::feature_settings_window`: original caption, enable row and `overflow_y_scroll` body remain unchanged. No nested Settings/Scrollable/TextView owner is added.

## Invariants

- Enable remains the first body row. Caption controls and close behavior are untouched; close is presentation-only. Existing constructor/activation queries stay silent and deferred.
- Font −/+, wide body and default pinned outline retain immediate durable section saves, bounds and last-good preferences on failure.
- Register, Default Apps, Unregister and Refresh retain original availability rules, the optional platform bridge, one pending command and original UUID/source-window routing. Default Apps launch does not imply default selection. `.md` and `.markdown` observations remain separately visible.
- Busy text occupies the association title line rather than inserting a new row above preferences. Pending saves/commands disable the same preference/association actions; preview retains its original enabled-only policy.
- Preview opens through the existing deferred Reader owner. No new focus, placement, cancellation, native window or persistence owner is introduced.

## Regression and verification

`settings_layout_tests.rs` mounts the actual SettingsView through the production Base Root. It checks real group/field/action bounds, horizontal alignment/order, per-extension status mapping, official control states, busy dispatch, font limits and scroll reachability. Existing settings_notification_tests preserves immediate durable saves and source-qualified feedback. Base control snapshots do not include ordinary GPUI Group containers; their full native accessibility remains separate.

Commands:

```powershell
$env:CARGO_BUILD_JOBS='1'
$env:CARGO_INCREMENTAL='0'
$env:RUSTFLAGS='-C codegen-units=256'
cargo test -p nxtools-desktop tools::markdown_reader::settings::layout_tests -- --test-threads=1
cargo test -p nxtools-desktop tools::markdown_reader::settings::notification_tests -- --test-threads=1
```

The layout tests check misordered/vertical fields, misaligned or out-of-bounds official controls, per-extension observed state, unreachable preview/actions in the460×320 viewport and changed availability/busy dispatch. The notification tests cover durable immediate saves, invocation-specific feedback, stale/duplicate command handling, default-app readback and disable refusal.

After compiled gates, inspect actual Light/Dark 460×320 rendering, font action boundaries, status legibility and natural action wrapping. Exercise pointer and Tab/Shift+Tab/Space/Enter, busy and platform-unavailable states, per-extension partial registration/default state, end-of-scroll preview, close/reopen and return from Default Apps only under separate authorization for live/native operations. Verify 100/125/150% and mixed DPI independently. Synthetic semantic snapshots are not Windows provider or screen-reader certification.

## Verification status

The settings group passed10/10:3layout and7notification regressions, including Light/Dark460×320 geometry, pointer availability/busy/no-dispatch and scroll reachability. Rustfmt completed. Actual GPUI screenshots, Windows provider/reader, physical input, native Default Apps/association, DPI and user visual acceptance remain separate. No product launch, live settings/Registry/association mutation, package/install or commit was performed.
