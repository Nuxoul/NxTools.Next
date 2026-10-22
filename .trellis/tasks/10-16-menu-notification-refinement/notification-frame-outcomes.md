# Notification frame containment correction

## User rejection and root cause
User explicitly rejects the previous captured notification: View Result/Details and the close control appear outside the message border. The visible border belonged only to an inner Alert; the outer notification layout rectangle had opaque background but no border. Earlier assertions verified containment in that invisible rectangle and missed the user's exact visual symptom. Those screenshots and prior AI inspection are not visual acceptance.

## Smallest rendering-only fix
The original shared presentation now uses official Kit0.7.1 GroupBox::outline with public content_style/child slots as the complete notification surface. Message, details, both official Small24 action buttons and XSmall close are within that surface. GroupBox.footer is deliberately not used because the released footer is below/outside the frame. Alert retains official semantic icon/text but has a transparent, borderless, zero-padding public style to avoid a second message-only frame. Outer surface has opaque existing theme backing and8px vertical/12px horizontal content adaptation, message right reserve for close, bounded62px text area with actions or90px without. The112px total card budget and336px/adaptive width remain; controls are inset in the painted frame.

No queue, settings, six-position, target, generation, expiry, deferred action, extension truth/navigation/runtime/menu, C#/IPC/native or dependency changes. No new diagnostics/payload/frame logs or state manager.

## Actual regressions and checks
- Before product change, `cargo test -p nxtools-desktop painted_notification_frame_encloses_both_actions_and_close_in_every_anchor -- --test-threads=1` failed: no actual nontransparent positive-border quad matched the336x87 complete card at(12,60). Log notification-frame-red.log. This is behavioral paint evidence, not a source-only assertion.
- The final regression identifies the actual painted full-frame quad, compares it with current card bounds, and requires both actions plus close to be >=4px inside every edge. Covers Light/Dark x920/300width x6anchors xProgress/Success/Error-with-long-message. All pass; existing long-copy scroll, action/dismiss/replacement/keyboard and no-paste tests remain.
- Focused22notification tests PASS (notification-frame-green.log); exact final full Desktop **767/767**,0failed/ignored,92.76s (notification-frame-desktop-final.log).
- Initial full run766pass/1fail due old source-shape check looking for `let card = div()`; changed it to require official GroupBox/outline/content/opaque backing, retaining the stronger actual-paint regression. Final full run above is green.
- Strict Desktop tests Clippy -D warnings, workspace --tests check, fmt --all --check, Debug build PASS: notification-frame-{strict,workspace,fmt,preview}.log. Existing unused ClipboardApplication::extension_adapter production warning remains unsuppressed.
- All Cargo serialized Rust1.96/jobs1/incremental0/RUSTFLAGS=-C codegen-units=256.
- UI-state/preview-isolation/motion/Desktop-shell4source gates PASS (notification-frame-gates.log). Spec/catalog/context/scoped integrity and screenshot hashes/dimensions checked at handoff; source/document gates do not certify installed visuals.

## Current actual captures and limit
[Current4-image gallery](../../../artifacts/notification-frame-ui/index.html): dark/light normal720x480 success and narrow300x440 long-error/double-actions. Actual owned Debug GPUI rendering, synthetic notifications,96DPI, PID/HWND/client verified,33–42sample colors, hash/dimensions evidence.json; notification-frame-capture.log. Previous20-image gallery retained as historical; its notification message-only frame is user-rejected and superseded.

BrowserPreview opened the new page but screenshot failed with "Browser view did not become visible for the calling session. Switch to that session and retry." Therefore new captures are generated/identity checked, **not directly visually inspected or user accepted**. The actual-paint tests directly protect this boundary but are not a substitute for user aesthetics, installed/native/physical/mixed-DPI/reader acceptance.

No packaging/install/ordinary product launch/user data/plugin/credential operation, service mutation, commit/push/archive. Sequence11 installer remains unchanged and does not include this source correction. New captures and code are ready for review, not user visual approval.
