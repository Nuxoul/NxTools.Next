# Runtime interaction feedback

## Goal and approval
User approved implementation of `.pi/plan/runtime-interaction-feedback-20261024.md` with “开始实施”. Correct the ten reported interaction/settings gaps using existing owners. Pin remains visible after paste and applies only to the current opening; explicit hide/reopen resets it without persistence.

## Requirements and observable acceptance
| ID | Requirement | Acceptance |
|---|---|---|
| R1 | No Clipboard preview while context menu is open | Cancel pending/visible preview; reject delayed/cache-hit reveal; retain deliberate movement/800ms after dismissal. |
| R2 | Click Clipboard outside menu dismisses menu only | Panel stays; dismiss gesture cannot paste/activate background item; truly external behavior remains correct. |
| R3 | Session-only pin left of Recording | Official control, default off; loss of focus/external click/paste do not auto-hide; paste restores target focus, not panel focus; explicit close allowed; reopen resets. |
| R4 | Diagnose intermittent floating image picker failure | Causal regression before fix; cancel quiet, distinguish timeout/busy/source/native failures; stale requests never commit settings. |
| R5 | Diagnose oversized startup main window | Trace native logical/device/display/DPI/create/show; causal regression before fix; preserve user sizing and launch focus modes. |
| R6 | Align pictured download button | Identify screenshot before edit, assert actual final geometry at normal/minimum size. |
| R7 | Consistent Other Tools status | Real enabled state, same dot+text convention, no false success on save failure. |
| R8 | Display system proxy address/port | Bounded read-only native snapshot; keep System route and independent manual draft; truthful PAC/multiple/no proxy/failure; stale rejection. |
| R9 | Add application above blacklist | Reach add before long list; preserve draft/Cancel/save/row actions. |
| R10 | Firefox store link button | Open exact https://addons.mozilla.org/zh-CN/firefox/addon/nxtools-download-interceptor/ through existing Shell owner. No speculative Chrome URL. |

## Confirmed context
Clipboard native menu classifies any nonmenu HWND press as outside and closes panel; hover readiness lacks a menu guard. Picker Rust600s/native2min mismatch is evidence, not proved root cause. Main initial logical1060x650/display placement does not prove native DPI correctness. Download PNG downloaded but visual identity unverified due unavailable browser/blocked terminal viewer. Full research and source anchors remain in approved plan.

## Exclusions and limits
No delegation/peer-session contact, unrelated edits, framework/vendor upgrade, new controllers, browser privacy changes, package/install/service/formal launch/real clipboard/history/config/proxy operations, commit/push/archive. Preserve concurrent work. Native/physical/user visual/installed acceptance stays separate. Unreproduced rare failures remain explicitly unresolved, not silently declared repaired.
