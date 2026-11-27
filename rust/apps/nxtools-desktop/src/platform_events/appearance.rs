use super::*;
use crate::ui::material::UiMaterialCapabilities;

impl PlatformEventDispatcher {
    pub(super) fn forward_appearance_to_panels(
        &self,
        notification: PlatformNotification,
        cx: &mut Context<Self>,
    ) {
        let clipboard = self.clipboard.clone();
        let quick_launch = self.quick_launch.downgrade();
        cx.defer(move |cx| {
            let _ = clipboard.update(cx, |application, cx| {
                application.handle_platform_notification(notification.clone(), cx);
            });
            let _ = quick_launch.update(cx, |application, cx| {
                application.handle_platform_notification(notification, cx);
            });
        });
    }

    // Native policy must pass startup buffering: first-show material needs these facts.
    pub(super) fn apply_appearance_notification(
        &self,
        notification: &PlatformNotification,
        cx: &mut Context<Self>,
    ) -> bool {
        match notification {
            PlatformNotification::Connected | PlatformNotification::Disconnected { .. } => {
                let mut policy = cx
                    .try_global::<UiMaterialCapabilities>()
                    .copied()
                    .unwrap_or_default();
                if matches!(notification, PlatformNotification::Connected) {
                    policy.connected();
                } else {
                    policy.disconnected();
                }
                cx.set_global(policy);
                false
            }
            PlatformNotification::Event(PlatformEvent::SystemMaterialCapabilitiesChanged(
                facts,
            )) => {
                let mut policy = cx
                    .try_global::<UiMaterialCapabilities>()
                    .copied()
                    .unwrap_or_default();
                if policy.apply(*facts) {
                    cx.set_global(policy);
                    cx.refresh_windows();
                    info!(event="system.material.capabilities_received", correlation_id=%facts.correlation_id,
                        session_id=%facts.session_id, revision=facts.revision, supported=facts.supported,
                        transparency_enabled=facts.transparency_enabled, high_contrast=facts.high_contrast,
                        energy_saver=facts.energy_saver, remote_session=facts.remote_session,
                        native_error_code=facts.native_error_code);
                }
                true
            }
            PlatformNotification::Event(PlatformEvent::SystemThemePreferenceChanged(
                preference,
            )) => {
                crate::theme::apply_system_theme(
                    preference.dark || preference.native_error_code != 0,
                    cx,
                );
                info!(event="system.theme.preference_applied", correlation_id=%preference.correlation_id,
                    dark=preference.dark, native_error_code=preference.native_error_code);
                true
            }
            PlatformNotification::Event(PlatformEvent::SystemMotionPreferenceChanged(
                preference,
            )) => {
                let mut policy = cx
                    .try_global::<UiMotionPreference>()
                    .copied()
                    .unwrap_or_default();
                cx.set_reduce_motion(
                    preference.reduced_motion || preference.native_error_code != 0,
                );
                if policy.apply(preference) {
                    cx.set_global(policy);
                    cx.refresh_windows();
                }
                info!(event="system.motion.preference_applied", correlation_id=%preference.correlation_id,
                    reduced_motion=policy.reduced_motion(), native_error_code=preference.native_error_code);
                true
            }
            _ => false,
        }
    }
}
