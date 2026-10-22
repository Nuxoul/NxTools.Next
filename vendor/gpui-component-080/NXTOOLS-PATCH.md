# NxTools notification duration patch on Component0.8.0

User-authorized2026-10-21 upgrade carries the original approved2026-10-20patch to published gpui-component0.8.0. Archive SHA2564097e161cc81e86323b439664be0cdd4f7c56148e867ed6f67fd72b966b6c2d1; Apache-2.0 LICENSE-APACHE retained. NXTOOLS-UPSTREAM.json records every shipped file hash. Fresh release source, not an old vendor transplant; old0.7.1/0.5.1snapshots remain inactive historical evidence.

Only src/notification.rs differs: optional duration field, public duration(Duration)builder, admission snapshot default5seconds forwarded to original ToastOptions.timeout when autohide=true. Rawunconfigured callers5s/autohidefalsepersistent/systemdeliveryplatformowned unchanged. NxTools1-60/default6 remains owned setting; officialNotificationList/ToastManager sole countdown/hover/focus/pause/stack/close/animation owners. No new clock/card/palette/APIbeyond originalduration.

Run python tests/architecture/gpui-release-contracts.py, plus --upstream with checksum-verified extractedsources for independent source comparison. Desktop notification expiry/presentation/screen/settings tests provide compiled behavior, not physical/native/installed/user visual acceptance. Current graph Kit/Base/Component/Assets0.8.0 + defaultpre0.3.9. No registry edits.
