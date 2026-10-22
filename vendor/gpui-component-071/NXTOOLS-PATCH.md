# NxTools notification duration patch

Upstream: gpui-component0.7.1, https://github.com/longbridge/gpui-kit, crates.io registry checksum bc8b7def8e74b0d822d3e96f6b28fb5a2c20eb7a83d30d22fa59ca8088d1159b. Published Apache-2.0 LICENSE-APACHE retained. Source/locales/tests/build.rs/manifests/README are copied from the release; no registry cache file is edited. Historical vendor/gpui-component0.5.1 is unrelated and untouched.

User2026-10-20 explicitly approves configurable1–60seconds/default6 in NxTools after checking official0.7.3 and latest0.8.0; notification.rs is identical in0.7.1/0.7.3/0.8.0 and all hardcode5seconds.

Only src/notification.rs is changed:
- private duration:Option<Duration>, initializedNone;
- public Notification::duration(Duration) builder;
- NotificationList::push snapshots requested duration or original5seconds and passes it to the existing ToastOptions.timeout when autohide=true.

Unconfigured upstream consumers retain5seconds; autohidefalse/action persistence still means no timeout,system delivery remains platform-owned. No change to official ToastManager/ToastStack/countdown/remaining-time hover/focus pause/close/animations/card/theme/native ownership. NxTools clamps persisted globalsetting1–60 and snapshots it at business admission; settings/position edits do not reset active cards.

Regression coverage lives in Desktop notifications expiry/presentation/screen and globalnotification settings tests. Dependency remains0.7.1 via root [patch.crates-io]; future upgrades should drop this patch when upstream supplies the public API. Do not broaden it to repair unrelated component behavior.
