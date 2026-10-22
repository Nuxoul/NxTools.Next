# Verified browser UI ownership references

Source authority: docs/reference-code/download.md browser-extension sections, rechecked2026-10-12 before implementation.

- Legacy D:/Code/NxTools/browser_extension/nxtools-interceptor/popup.html/js: four views, site toggle, task/resource/selection, media actions, settings auto-save/explicit save. Legacy WebSocket and cancel-before-accept are superseded unsafe behavior, not copied.
- Next popup.js: collectSettings/render/setView/refresh and stable popup_* messages preserve settings schema, task8/resource24/batch32, accepted-item-only deselection, queue-vs-confirm close policy, frame/index media target.
- Next content-script.js ensureOverlay: one28px entry, original source HWND-free browser context, CSSStyleSheet adopted in Shadow DOM, original choice candidate/keyboard/Escape/outside/focus-return owner. Original computeDropTargetRect and configured96x48..360x220 unchanged.
- Manifest/background/page-hook/capture/handoff/batch/Blob policies remain byte-identical to captured dirty baseline. No extension ID/permissions/CSP widening or Native Messaging lifetime change. Passive popup snapshot cannot launch Desktop.
- New presentation: independent browser neutral workbench, shared popup/Shadow tokens, visible status, accessible resource preview/filter/feature state, original action-path failures/busy feedback. Current real DOM suite uses mockedChrome in isolated profile with all network blocked; screenshots/DOM do not certify installed browser/data/physical/provider behavior.
