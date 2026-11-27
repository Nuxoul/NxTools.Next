use super::*;
use gpui_kit::App;
use std::time::Duration;

impl NetworkSettingsApplication {
    pub(super) fn prepare_material_window(
        &self,
        handle: AnyWindowHandle,
        first_show: bool,
        cx: &mut Context<Self>,
    ) {
        let owner = cx.entity().downgrade();
        let platform = self.platform.clone();
        let id = uuid::Uuid::new_v4();
        let started = Instant::now();
        cx.defer(move |cx| {
            if !first_show && crate::ui::material::runtime::is_waiting(handle, cx) {
                return;
            }
            let completion = move |result: Result<(), &'static str>, cx: &mut App| {
                let Some(owner) = owner.upgrade() else { return; };
                if owner.read(cx).window != Some(handle) || !cx.windows().contains(&handle) {
                    return;
                }
                match result {
                    Ok(()) => {
                        let _ = handle.update(cx, |_, window, _| {
                            window.activate_window();
                            window.refresh();
                        });
                        tracing::info!(event = "desktop.settings.material.ready", correlation_id = %id,
                            elapsed_ms = started.elapsed().as_millis() as u64);
                    }
                    Err(reason) => {
                        let notifications = owner.read(cx).notifications.clone();
                        if first_show {
                            owner.update(cx, |owner, _| {
                                owner.window = None;
                                #[cfg(test)]
                                { owner.settings_view = None; }
                            });
                            let _ = handle.update(cx, |_, window, _| window.remove_window());
                        }
                        tracing::warn!(event = "desktop.settings.material.failed", correlation_id = %id,
                            reason, elapsed_ms = started.elapsed().as_millis() as u64);
                        notifications.update(cx, |notifications, cx| {
                            notifications.show_error(format!("global-settings-material-{id}"),
                                "无法准备全局设置窗口", "Windows 外观组件未能完成窗口准备，请稍后重试。", cx);
                        });
                    }
                }
            };
            if let Some(platform) = platform {
                crate::ui::material::runtime::when_ready(
                    handle, platform, nxtools_protocol::window_material::MaterialSurface::Backdrop,
                    Duration::from_secs(8), completion, cx,
                );
            } else {
                completion(Ok(()), cx);
            }
        });
    }
}
