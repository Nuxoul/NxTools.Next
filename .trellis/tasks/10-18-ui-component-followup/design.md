# Component decision and implementation boundary

Use locked Kit0.7.1 public styled APIs, no framework/vendor changes. Existing workbench/native clients unchanged: Main1060x650/min920x480/176rail, Clipboard300x440, download compact176 and current content-sized confirmation246, settings984x650. Keep entity IDs and original save/native/cancel/generation owners; all cross-owner callbacks remain deferred.

- Trust: original Dialog stays. Toolbar.content(div actual outlined Medium32 Buttons) bypasses prepare_for_toolbar -> ghost().compact(); no hand-painted border or new activation handler. Verify actual positive-border quads under current light/dark theme.
- Activity: Empty/Header/Title/Description small composition inside existing bounded scroll; original session copy retained, no fake action.
- Shortcut descriptions: shared render-only helper composes released Kbd::new(Keystroke) and Label, optional layout div wrapping. Known static keys parsed explicitly; no binding registration, input ownership or native hotkey changes. Clipboard narrow footer may wrap or arrange hints without pushing Save/Cancel offscreen; do not just truncate all useful keys.
- Technical details: original SettingItem render uses released DescriptionList for named metadata and GroupBox/ListItem where appropriate, full content remains reachable. No trust declarations reinterpreted as authorization.
- Download detail: DescriptionList and Marker.Content project original models/order; text wraps/scrolls within old viewport and no event/logging changes. Probe uses Spinner only while probing+motion_allowed, static otherwise. FFmpeg Progress uses existing true percent when known; no invented progress for unknown bytes/other phases.

Not doing InputGroup composition, Dialog controller migration, new Table/DataTable/Command/calendar selection owner or blanket status restyling. Those need dedicated compatibility/visual evidence and are not part of this low-risk slice. No diagnostics changes needed: purely rendering existing correlated state, no new operation.
