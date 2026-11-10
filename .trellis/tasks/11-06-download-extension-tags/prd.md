# Download category extension tags

## Goal

Present download-category extension rules as the supplied compact reference: category icon/name, wrapping outlined pills with individual removal, and a trailing Add entry. Retain the existing category editor and download policy.

## Scope

Only the extension-rule field changes. Name/directory/URL rules, category enabled state/order/removal, automatic-category switch, original explicit Save/Cancel and central future-task classification remain. No download engine, protocol, service, dependency, theme or native window-size change.

## Component selection

Released Kit/Component0.8.0: existing Settings/SettingPage/SettingGroup/SettingItem public field slot and Form/Field remain the compound owners. Tag::secondary().outline().rounded_full(), Medium typography, hosts an official XSmall ghost close Button in its public child slot. Public zero vertical padding accommodates the24DIP close target; no substitute tag drawing. Add is an official Small outlined pill Button outside Toolbar.child adaptations. New value uses the stable official default32DIP Input; Small Add/ghost Cancel retain original keyboard dispatch. Real official overflow_y_scrollbar owns bounded tags and form content; original footer remains outside the scroll body.

## Behavioral contracts

- The original extension InputState remains the unsaved serialized backing value; a separate stable InputState owns pending new text.
- Add/Enter strips leading dots, folds ASCII case and deduplicates while retaining compound suffixes such as tar.gz. Bulk input accepts comma/semicolon/whitespace separators.
- Individual removal acts on the extension value under the current category ID. Deferred callbacks reject saving, removed or no-longer-selected categories.
- Read/write of input entities occurs outside the settings-view lease; input-emitted Enter schedules the same deferred operation as the official Add button.
- Escape/Cancel discards only pending text and restores the Add trigger focus. The settings Save includes still-visible pending text; closing the settings draft writes nothing.
- Extension and URL rules retain their existing AND semantics. Rule changes affect only later accepted downloads; existing task destinations remain immutable.
- No extension text, URL, path or pending input is diagnostic data. Existing correlated settings validation/save events retain ownership; tag edits are local draft operations, not new IPC.

## Reference

ABDM Category.kt/NewCategoryPage.kt and legacy CategoryDialogViewModel.cs/CategoryWindow.axaml.cs were inspected through docs/reference-code/download.md. The screenshot intentionally changes the old comma-text presentation, not matching or save policy. Parent constructor/show/close references retain ordinary activating settings and explicit cancellation; no new auxiliary window is introduced.

## Verification

Production GPUI tests cover actual tag removal/no autosave, Add/Enter/dedup/compound suffix, Escape/pending cancellation, saving guards and genuine scroll reachability. Retain existing category master/name/directory/URL/action and settings persistence/cancel tests. Scoped formatting/lint and lifecycle/Kit/editor/spec gates apply. Compiled scene evidence is separate from physical/DPI, installed and user visual acceptance.
