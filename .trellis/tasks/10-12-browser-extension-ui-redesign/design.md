# Compact browser download workbench

## Visual direction
Neutral graphite (#18181b canvas, #1f1f23 surface, #27272a raised, #f4f4f5 text, #a1a1aa secondary, #bfc0c4 action). Browser token vocabulary shared by popup and injected Shadow DOM, independently adopting the approved neutral brand rather than changing desktop theme. Font Segoe UI Variable/YaHei UI,13/20 body,12/18 supporting copy,16px product title; radius6 ordinary/8 overlay. No gradients, decorative signal sweeps, index numerals, idle animation or equal card walls.

## Hierarchy
```
brand + honest existing-desktop status
current page/domain + original send page action
Tasks | Capture | Media | Settings
bounded continuous content / original controls
operation live feedback
```
Page context remains visible across tabs. Capture summary is static textual context in Capture, not a decorative live ticker. Resource filename first, kind/resolution/size supporting, full path/context reachable; preview has a real separate keyboard button and independent selection checkbox. Persistent batch action remains reachable in the bounded content, unsupported merge explicitly disabled.

Settings grouped by download policy/page integration/request context, labels and help together; original auto-save is explicitly explained and explicit Save retries. Media controls use clear grouped rows and accessible pressed state. Errors remain visible; no fabricated percentage for loading. Popup456px and420..445 workspace retained.

## Owners and UX changes
Presentation-only CSS/HTML rewrite; stable IDs and messages retained. popup.js adds actual filter aria-pressed, feature pressed, resource preview keyboard/Escape/focus restoration, refresh focus preservation and caught asynchronous action failures/busy suppression on the original action path. No second worker/controller/timer/native owner. Content media panel keeps original placement/candidates/keyboard/Escape owner, exposes operation status visibly in its existing entry instead of only hidden text; drop target styled for configured96x48..360x220 budgets.

## Evidence
Original reference extension popup/content constructors and Next manifest/content ensureOverlay/showResourceChoices/closeOverlayPanel/computeDropTargetRect reviewed; old WebSocket/cancel-before-accept are not copied. Production popup render script runs in a separate temporary Chrome profile with Chrome API fixture and network blocked. Preview files are generated project artifacts, fixture logic test-owned; never install extension or invoke real native host.
