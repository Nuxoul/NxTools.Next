//! Material admission into the existing download presentation owners.
use super::*;
use nxtools_protocol::window_material::MaterialSurface;

pub(super) const DOWNLOAD_MATERIAL_TIMEOUT: Duration = Duration::from_secs(3);

impl DownloadApplication {
    pub(super) fn after_download_material(
        &self,
        window: AnyWindowHandle,
        surface: MaterialSurface,
        timeout: Duration,
        completion: impl FnOnce(&mut Self, Result<(), &'static str>, &mut Context<Self>) + 'static,
        cx: &mut Context<Self>,
    ) {
        let application = cx.entity().downgrade();
        let platform = Rc::clone(&self.platform);
        cx.defer(move |cx| {
            if !cx.windows().contains(&window) {
                return;
            }
            if surface == MaterialSurface::Backdrop
                && crate::ui::material::runtime::is_waiting(window, cx)
            {
                return;
            }
            crate::ui::material::runtime::when_ready(
                window,
                platform,
                surface,
                timeout,
                move |result, cx| {
                    let _ = application.update(cx, |application, cx| {
                        completion(application, result, cx);
                    });
                },
                cx,
            );
        });
    }

    pub(super) fn prepare_settings_material(
        &self,
        window: AnyWindowHandle,
        first_show: bool,
        cx: &mut Context<Self>,
    ) {
        self.after_download_material(
            window,
            MaterialSurface::Backdrop,
            DOWNLOAD_MATERIAL_TIMEOUT,
            move |app, result, cx| {
                if app.settings_window != Some(window) || !cx.windows().contains(&window) {
                    return;
                }
                if let Err(reason) = result {
                    if !first_show {
                        app.download_material_failed("settings", reason, cx);
                        return;
                    }
                    let _ = window.update(cx, |_, window, _| window.remove_window());
                    app.handle_settings_window_closed(cx);
                    app.download_material_failed("settings", reason, cx);
                    return;
                }
                let _ = window.update(cx, |_, window, _| {
                    window.activate_window();
                    window.refresh();
                });
            },
            cx,
        );
    }

    pub(super) fn prepare_schedule_material(
        &self,
        window: AnyWindowHandle,
        first_show: bool,
        cx: &mut Context<Self>,
    ) {
        self.after_download_material(
            window,
            MaterialSurface::Backdrop,
            DOWNLOAD_MATERIAL_TIMEOUT,
            move |app, result, cx| {
                if app.schedule_window != Some(window) || !cx.windows().contains(&window) {
                    return;
                }
                if let Err(reason) = result {
                    if !first_show {
                        app.download_material_failed("schedule", reason, cx);
                        return;
                    }
                    app.close_schedule_window(cx);
                    app.download_material_failed("schedule", reason, cx);
                    return;
                }
                let _ = window.update(cx, |_, window, _| {
                    window.activate_window();
                    window.refresh();
                });
            },
            cx,
        );
    }

    pub(super) fn prepare_delete_material(
        &self,
        task_id: DownloadTaskId,
        window: AnyWindowHandle,
        first_show: bool,
        cx: &mut Context<Self>,
    ) {
        self.after_download_material(
            window,
            MaterialSurface::Backdrop,
            DOWNLOAD_MATERIAL_TIMEOUT,
            move |app, result, cx| {
                if app.delete_confirmation_windows.get(&task_id) != Some(&window)
                    || !cx.windows().contains(&window)
                {
                    return;
                }
                if let Err(reason) = result {
                    if !first_show {
                        app.download_material_failed("delete_confirmation", reason, cx);
                        return;
                    }
                    app.close_delete_confirmation(task_id, cx);
                    app.download_material_failed("delete_confirmation", reason, cx);
                    return;
                }
                let _ = window.update(cx, |_, window, _| window.activate_window());
            },
            cx,
        );
    }
    pub(super) fn prepare_taskbar_material(&self, window: AnyWindowHandle, cx: &mut Context<Self>) {
        self.after_download_material(window, MaterialSurface::Transparent, DOWNLOAD_MATERIAL_TIMEOUT, move |app, result, cx| {
            if app.taskbar_overlay_window != Some(window) || !cx.windows().contains(&window) { return; }
            if let Err(reason) = result {
                warn!(event = "download.taskbar_overlay.material_failed", reason);
                return;
            }
            let correlation_id = app.taskbar_overlay_diagnostic_correlation_id();
            if app.apply_taskbar_overlay_placement(correlation_id).is_err() {
                warn!(event = "download.taskbar_overlay.material_placement_failed", %correlation_id);
                return;
            }
            app.taskbar_overlay_material_ready = true;
            // The original view owns visibility transitions and animation generations.
            cx.notify();
        }, cx);
    }

    pub(super) fn prepare_new_download_material(
        &self,
        window: AnyWindowHandle,
        browser_context: Option<(uuid::Uuid, uuid::Uuid)>,
        intake_correlation: Option<uuid::Uuid>,
        auto_probe: bool,
        first_show: bool,
        cx: &mut Context<Self>,
    ) {
        self.after_download_material(window, MaterialSurface::Backdrop, DOWNLOAD_MATERIAL_TIMEOUT, move |app, result, cx| {
            if app.new_download_window != Some(window) || !cx.windows().contains(&window) { return; }
            let view = window.update(cx, |root, _, cx| {
                root.downcast::<Root>().ok().and_then(|root| root.read(cx).view().clone().downcast::<NewDownloadView>().ok())
            }).ok().flatten();
            if let Err(reason) = result {
                if !first_show {
                    app.download_material_failed("new_download", reason, cx);
                    return;
                }
                if let Some(view) = view {
                    if let Some(context) = view.read(cx).controlled_handoff.clone()
                        && context.claim_abort_for_window(window) {
                        app.abort_controlled_browser_handoff(&context, "material_failed", cx);
                    }
                    if let Some(context) = view.read(cx).external_blob.clone() {
                        app.discard_browser_blob(&context);
                    }
                }
                app.retire_new_download_clipboard("material_failed");
                app.new_download_window = None;
                #[cfg(test)]
                { app.new_download_view = None; }
                let _ = window.update(cx, |_, window, _| window.remove_window());
                app.download_material_failed("new_download", reason, cx);
                return;
            }
            let native = window.update(cx, |_, window, _| {
                window.activate_window();
                native_window_handle(window).unwrap_or_default()
            }).unwrap_or_default();
            let correlation_id = browser_context.map(|(id, _)| id).or(intake_correlation).unwrap_or_else(uuid::Uuid::new_v4);
            let topmost = app.platform.send_command_correlated(PlatformCommand::SetWindowTopmost(SetWindowTopmostCommand { window_handle: native, enabled: true }), correlation_id);
            let activation = app.platform.send_command_correlated(PlatformCommand::ActivateWindow(ActivateWindowCommand { window_handle: native }), correlation_id);
            info!(event = "download.browser_dialog.foreground_requested", %correlation_id, window_handle = native,
                topmost_accepted = topmost.is_ok(), activation_accepted = activation.is_ok());
            let application = cx.entity().downgrade();
            cx.defer(move |cx| {
                if application.upgrade().is_none_or(|app| app.read(cx).new_download_window != Some(window)) { return; }
                if let Some(view) = view {
                    let _ = window.update(cx, |_, window, cx| view.update(cx, |view, cx| {
                        view.focus_url_input(window, cx);
                        if auto_probe { view.begin_probe(window, cx); }
                    }));
                }
            });
        }, cx);
    }

    pub(super) fn download_material_failed(
        &self,
        surface: &'static str,
        reason: &'static str,
        cx: &mut Context<Self>,
    ) {
        warn!(event = "download.window.material_failed", surface, reason);
        self.notifications.update(cx, |notifications, cx| {
            notifications.show_error(
                "download-window-material",
                "无法准备下载窗口",
                "窗口外观准备失败，请重试",
                cx,
            );
        });
    }
}
