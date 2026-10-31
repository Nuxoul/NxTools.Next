# Audit #3 only — accessible names on actual Buttons

## Scope and canonical phase
Foundation shared action contract correction, with direct consumers in Downloads / Quick Find /
Global settings + supporting surfaces; no phase reordering or renumbering. Only original audit #3.
Current dirty baselines saved before changes; preserve accepted #1/#9/#10 behavior and unrelated dirty work.
No recursive delegation, commit/push, package/install, product/host/service, real clipboard/picker/network,
user configuration/database or registry operations. Synthetic temporary/in-memory fixtures only.

## Gap and ownership
Component0.8.0 Button::render uses explicit accessibility_label.or_else(label). Arbitrary children,
icon and Tooltip do not supply its accessible name. Current primary_action_button uses child content;
icon_action_button uses icon only. path_list delete has only tooltip. Fix actual Button nodes, not outer div.
Low-level shared helpers must require a semantic name while retaining original IntoElement content,
style/size/parent adaptation, disabled/readable paint and sole original release-click/focus owner.
Existing icon_action passes its current tooltip text once as its Button name, without duplicate owners.

Expected paths: ui/components/action.rs, path_list.rs; direct helper callsites in Downloads settings/new/
task/schedule/window_support and network_settings.rs; necessary real semantic tests, reference append and
task evidence. No Menu/Clipboard native/provider/protocol/C#/vendor/dependencies/theme/font changes.

## Reference and invariants
Read current0.8.0 release/API, component table/index, applicable specs and download/system-shell/Quick Find
notes first. ABDM LocationTextField has action-specific contentDescription and original picker callback;
MainActionButtons uses named Add/Download/Cancel. Legacy SettingsWindow ctor->WindowAnimationService.Attach,
OnOpened/ApplySettingsCommand/Close and FFmpeg chooser retain original owners; native Avalonia base unavailable.
Old NxTools has no Quick Find exclusion UI; SwiftList ExclusionsTab -> Templates RuleItemTemplate uses one
trailing remove command. This task only fixes semantic names, not reference style or behavior.

Names distinguish action + object (default directory, FFmpeg, config/log/database paths, retry probe,
excluded path), never technical IDs. Primary semantic name remains stable across disabled/loading status
copy; visible text/ellipsis/paint/content remain unchanged. Keep existing IDs, callbacks, scheduling,
picker generations/identity, original keyboard submission routing, Clipboard stable ancestry/focus scope,
geometry/borders/hover/Tooltips and cross-owner deferred lifecycle byte-equivalent outside naming spans.

## Red-green and acceptance
Before production fix: actual rendered helper icon and primary Button role/name tests must fail;
actual Global Apply/Downloads Apply or picker/refresh/path delete snapshots verify production nodes.
After fix: ordinary and custom canvas content keep semantic name in normal/disabled/loading, no disabled
pointer/key dispatch, original one release dispatch and fixed bounds. Full direct-callsite list audited.
Run focused red/green, action/path/global/Downloads/#1/#9/#10 regressions, strictDesktopClippy --tests
-Dwarnings/workspacecheck/fmt and relevant source gates. Complete Desktop at most once; preserve known
Archive Unlock failure and any other failures, no unrelated fix/green hunting. All Rust shells jobs1,
incremental0, codegen256, serial full timeout/natural idle only. Source/synthetic snapshots are not actual
UIA/reader, physical input/IME/DPI/visual/native/user acceptance. Deliver exact dirty-baseline own-delta,
manifest and evidence in parent-readable workspace, then stop for parent review before any fifth item.
