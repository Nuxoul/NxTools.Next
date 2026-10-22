# Official notification latest release check — 2026-10-20

User interrupted implementation to inspect0.7.3/latest first. Implementation remains paused: no official source patch or dependency upgrade performed. Earlier1–60/default6 local-patch approval is recorded in PRD but not applied pending this check/handoff.

Downloaded official crates.io version metadata and released gpui-component/gpui-kit0.7.3 and0.8.0 archives to session scratch notification-073-capability; SHA256 verified against registry checksums. Registry max_stable/max_version is0.8.0 (published2026-10-19);0.7.3 was published2026-10-12. Product stays exactKit0.7.1.

## Result
-0.7.3 notification.rs is byte-for-byte identical to0.7.1: SHA25609010c94024ee6e2580ecf557d97dcf276512a52be8cf298e0936088df0afeca.
-0.7.3 public Notification::autohide(bool) only toggles automatic dismissal; Notification/NotificationSettings expose no duration/timeout builder or setting. NotificationList::push still forwards ToastOptions timeout autohide.then_some(Duration::from_secs(5)).
-0.8.0 has the same fixed5s statement and no public duration/timeout setter. Its Kit/Component require gpui-pre exact0.3.9 plusBase/Component/Assets0.8.0. Merely upgrading to it does not enable configurabledwell; fullframeworkmigration is not authorized or justified by this capability.
-This is source/API verification,not a compiled/runtime compatibility test of0.7.3/0.8.0. No product Cargo build or lockfile resolution after the research downloads.

## Current work state
The two added production multiline/nativehost tests were actually run against original0.7.1 before anyfix:0pass/2fail; both observed54pxactualcontentheight, matching user screenshot. Noheightfix yet. New vendor0.7.1 snapshot was only copied unmodified; guarded hashes verify all259files before moving it back into scratch. Remove temporary rootgpui-componentpatch line, retaining originalWindowsbackendpatch. Existingdirtywork/priorcurrencyandEnterimplementation retained. No installed/service/clipboard operation/delegation/commit/package.

## Resume after check
User then says “既然不支持那就继续吧”, explicitly resuming the previously approved minimal0.7.1patch and natural-height repair. The pause/work-state paragraphs above describe the interrupted checkpoint, not final source. See design.md/outcomes.md for implementation and actual verification; no framework upgrade or registry cache edit.
