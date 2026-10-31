# Implementation boundary and source-backed design

## Smallest gap and owners
Official Component0.8.0 Notification::render has a private absolute top_1/right_1 hover-only Close; no public adjacent slot. Add optional header_action(builder -> AnyElement), defaultNone, render inside the same h_flex toolbar before original Close and suppress optional action while exiting. Existing unconfigured callers retain their exact lifecycle/appearance. Exact released controls: component::button::Button.ghost().xsmall() with IconName::Copy/Check; no Toolbar.child transformation, custom painted controls or second card. NxTools owns title right reservation for this toolbar; no unrequested generic typography rewrite.
Validated gotcha: GPUI KeyDown suppresses group_hover, so the new header-action toolbar must also remain visible when its own non-tabstop focus scope contains focus. This minimal conditional applies only when header_action is configured; optional content is still omitted during Ending. ActualSpaceDown/Up red->green proves the fix; no second key dispatcher.

A small CopyButton entity observes the original NotificationCenter and renders one official button. The center owns optional PlatformBridge and bounded copy metadata, not a second display/lifetime manager. Local button callback stops propagation and defers to center. Pending installs before correlated caller-ID dispatch; current ID/generation is checked on the next turn. Native result drives feedback, not queue admission. No automatic UI notice or clipboard read. Existing copy-only WindowsPasteCoordinator returns before activation branch.

## Invariants
- Original Notification dismiss/entry/exit/hoverpause/remaining time/native passive preparation-readback-frame-show are unchanged.
- Header action is absent from official Ending render; stale button closures cannot dispatch to replaced/retired business ID/generation. Removal before a deferred dispatch rejects; admitted copy finishes through native reply/deadline.
- Payload title/message/details only, max8 same as presentation; no raw text/paths/URL in diagnostics. Accepted/routed/terminal use one correlation, elapsed and native error code only.
- Success uses Check until the same card is replaced or removed; no feedback timer or repush. Error keeps Copy available with fixed Tooltip. Disable while pending, at most3 retained UI states/one active platform copy.

## Fork
Fork main base627ba35a preserved upstream fully_visible_bounds; local release source and fork source match after normalizing only that known upstream change. Duration58dc3881, headera5b185c7 and provenancec00db47e were pushed normally to https://github.com/Nuxoul/gpui-kit/main; finalformatfollowup recorded in outcomes. External Windows policy is an explicit immutable-crate+matchonce+replacement JSON archive, verified to replay active code, not activated/backendsource in Kit. Root local dependency remains pinned/path-based.
