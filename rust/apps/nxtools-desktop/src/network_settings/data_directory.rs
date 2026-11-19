use super::*;
use crate::data_directory::{DataMigration, MigrationSlot};

#[derive(Clone)]
pub(super) struct PendingDataPicker {
    id: uuid::Uuid,
    view: WeakEntity<NetworkSettingsView>,
    window: AnyWindowHandle,
    started_at: Instant,
    validating: bool,
}

impl NetworkSettingsView {
    pub(super) fn render_data_directory(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let paths = self.application.read(cx).data_paths.clone();
        let current = paths
            .as_ref()
            .map(|paths| paths.software_root().display().to_string())
            .unwrap_or_default();
        div().w_full().min_w_0().flex().flex_col().gap_2()
            .child(div().text_sm().child("软件数据目录"))
            .child(div().text_sm().child(current.trim_start_matches(r"\\?\").to_owned()))
            .child(div().text_sm().text_color(self.theme.text_secondary)
                .child("选择空的本地目录后立即迁移并重启；未应用的设置草稿不保存。下载文件、独立剪贴板目录和索引服务数据不移动，原目录保留为备份。"))
            .child(action_button("global-data-directory-pick", "选择目录并迁移…")
                .outline().with_size(gpui_kit::component::Size::Medium)
                .disabled(self.saving || paths.is_none())
                .on_click(cx.listener(|view, _, window, cx| {
                    if view.saving { return; }
                    let source_window_handle = crate::ui::native_window_handle(window).unwrap_or_default();
                    let source_window = window.window_handle();
                    let origin = cx.entity().downgrade();
                    let application = view.application.clone();
                    view.saving = true;
                    view.draft_busy.set(true);
                    view.error = None;
                    cx.defer(move |cx| application.update(cx, |app, cx| {
                        app.pick_data_directory(origin, source_window, source_window_handle, cx)
                    }));
                    cx.notify();
                })))
            .into_any_element()
    }
}

impl NetworkSettingsApplication {
    fn pick_data_directory(
        &mut self,
        view: WeakEntity<NetworkSettingsView>,
        window: AnyWindowHandle,
        source_window_handle: i64,
        cx: &mut Context<Self>,
    ) {
        if self.pending_data_directory.is_some() {
            return;
        }
        let id = uuid::Uuid::new_v4();
        let Some(paths) = &self.data_paths else {
            self.release_data_picker(view, Some("无法确定当前软件数据目录。".into()), cx);
            return;
        };
        let command = PlatformCommand::PickDirectory(nxtools_protocol::PickDirectoryCommand {
            source_window_handle,
            source_process_id: std::process::id(),
            current_directory: paths.software_root().to_string_lossy().into_owned(),
            title: "选择空目录，迁移软件数据并重启".into(),
        });
        self.pending_data_directory = Some(PendingDataPicker {
            id,
            view,
            window,
            started_at: Instant::now(),
            validating: false,
        });
        let queued = self.platform.as_ref().is_some_and(|platform| {
            platform
                .send_command_with_id_correlated(command, id, id)
                .is_ok()
        });
        info!(event = "data_directory.picker.started", correlation_id = %id, queued, "software data directory selection started");
        if !queued {
            self.finish_data_picker(
                id,
                None,
                Some("无法打开目录选择器，请检查平台组件后重试。".into()),
                cx,
            );
            return;
        }
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(std::time::Duration::from_secs(300))
                .await;
            let _ = this.update(cx, |app, cx| {
                app.finish_data_picker(id, None, Some("目录选择已超时，请重试。".into()), cx)
            });
        })
        .detach();
    }

    pub(super) fn handle_data_directory_notification(
        &mut self,
        notification: &PlatformNotification,
        cx: &mut Context<Self>,
    ) -> bool {
        match notification {
            PlatformNotification::CommandResult {
                correlation_id,
                result,
            } if self
                .pending_data_directory
                .as_ref()
                .is_some_and(|pending| pending.id == *correlation_id && !pending.validating) =>
            {
                let id = *correlation_id;
                let selected = result
                    .success
                    .then(|| result.data.clone())
                    .flatten()
                    .filter(|path| !path.is_empty());
                let error =
                    (!result.success).then(|| "无法选择目录，请检查系统权限后重试。".to_owned());
                info!(event = "data_directory.picker.result", correlation_id = %id, success = result.success, native_error_code = result.native_error_code, "software data picker returned");
                let owner = cx.entity().downgrade();
                cx.defer(move |cx| {
                    let _ = owner.update(cx, |app, cx| {
                        app.finish_data_picker(id, selected, error, cx)
                    });
                });
                true
            }
            PlatformNotification::Disconnected { .. } => {
                if let Some(id) = self
                    .pending_data_directory
                    .as_ref()
                    .map(|pending| pending.id)
                {
                    self.finish_data_picker(
                        id,
                        None,
                        Some("平台组件已断开，请重新启动后重试。".into()),
                        cx,
                    );
                }
                false
            }
            _ => false,
        }
    }

    fn release_data_picker(
        &self,
        view: WeakEntity<NetworkSettingsView>,
        error: Option<String>,
        cx: &mut Context<Self>,
    ) {
        cx.defer(move |cx| {
            let _ = view.update(cx, |view, cx| {
                view.saving = false;
                view.draft_busy.set(false);
                view.error = error;
                cx.notify();
            });
        });
    }

    fn finish_data_picker(
        &mut self,
        id: uuid::Uuid,
        selected: Option<String>,
        error: Option<String>,
        cx: &mut Context<Self>,
    ) {
        if self
            .pending_data_directory
            .as_ref()
            .is_none_or(|pending| pending.id != id || (pending.validating && selected.is_some()))
        {
            return;
        }
        let pending = self.pending_data_directory.take().expect("matched picker");
        let active = self.window == Some(pending.window) && pending.view.upgrade().is_some();
        info!(event = "data_directory.picker.completed", correlation_id = %id, active, selected = selected.is_some(), elapsed_ms = pending.started_at.elapsed().as_millis(), "software data directory selection completed");
        if !active || selected.is_none() {
            self.release_data_picker(pending.view, error, cx);
            return;
        }
        let paths = self.data_paths.clone().expect("admitted data paths");
        let selected = PathBuf::from(selected.expect("selected path"));
        let (sender, receiver) = async_channel::bounded(1);
        let spawned = std::thread::Builder::new()
            .name("data-directory-validation".into())
            .spawn(move || {
                let result = (|| {
                    let executable = std::env::current_exe()?;
                    DataMigration::prepare(
                        paths,
                        &selected,
                        executable
                            .parent()
                            .ok_or_else(|| anyhow::anyhow!("程序目录不可用。"))?,
                        id,
                    )
                })();
                let _ =
                    sender.send_blocking(result.map_err(|error: anyhow::Error| error.to_string()));
            });
        if spawned.is_err() {
            self.release_data_picker(pending.view, Some("无法启动迁移，请稍后重试。".into()), cx);
            return;
        }
        self.pending_data_directory = Some(PendingDataPicker {
            validating: true,
            ..pending.clone()
        });
        cx.spawn(async move |this, cx| {
            let result = receiver
                .recv()
                .await
                .unwrap_or_else(|_| Err("目录校验未能完成，请重试。".into()));
            let _ = this.update(cx, |app, cx| {
                app.complete_data_validation(pending, result, cx)
            });
        })
        .detach();
    }

    fn complete_data_validation(
        &mut self,
        pending: PendingDataPicker,
        result: Result<DataMigration, String>,
        cx: &mut Context<Self>,
    ) {
        if self
            .pending_data_directory
            .as_ref()
            .is_none_or(|current| current.id != pending.id || !current.validating)
        {
            return;
        }
        self.pending_data_directory = None;
        if self.window != Some(pending.window) || pending.view.upgrade().is_none() {
            return;
        }
        let id = pending.id;
        match result {
            Ok(migration) => {
                let Some(slot) = cx.try_global::<MigrationSlot>().cloned() else {
                    self.release_data_picker(pending.view, Some("迁移运行环境不可用。".into()), cx);
                    return;
                };
                let Ok(mut slot) = slot.0.lock() else {
                    self.release_data_picker(pending.view, Some("迁移状态不可用。".into()), cx);
                    return;
                };
                if slot.is_some() {
                    self.release_data_picker(
                        pending.view,
                        Some("已有数据迁移正在进行。".into()),
                        cx,
                    );
                    return;
                }
                info!(event = "data_directory.migration.accepted", correlation_id = %id, "software data migration admitted; normal shutdown requested");
                *slot = Some(migration);
                cx.defer(|cx| cx.quit());
            }
            Err(message) => {
                warn!(event = "data_directory.validation.failed", correlation_id = %id, "software data destination validation failed");
                self.release_data_picker(pending.view, Some(message), cx);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::{TestAppContext, VisualTestContext};

    #[gpui_kit::test]
    fn data_directory_picker_cancel_disconnect_and_old_result_preserve_current_data(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_kit::init);
        cx.update(crate::theme::apply_component_theme);
        let (app, root, _) = super::super::tests::application_fixture(cx, "data-directory-picker");
        let (bridge, endpoint) = nxtools_ipc::platform_bridge(16);
        endpoint.set_connected_for_contract_test(true);
        app.update(cx, |app, cx| {
            app.platform = Some(Rc::new(bridge));
            app.data_paths = Some(nxtools_core::AppPaths {
                config_dir: root.clone(),
                data_dir: root.join("data"),
                cache_dir: root.join("cache"),
                log_dir: root.join("logs"),
            });
            app.open_window(cx);
        });
        cx.run_until_parked();
        let (handle, view) = cx.read(|cx| {
            let app = app.read(cx);
            (
                app.window.expect("window"),
                app.settings_view.clone().expect("view"),
            )
        });
        let mut surface = VisualTestContext::from_window(handle, cx);
        super::super::kit_migration_tests::select_page(&mut surface, 4);
        let button = surface
            .debug_bounds("global-data-directory-pick")
            .expect("real migration button");
        surface.simulate_click(button.center(), Default::default());
        surface.run_until_parked();
        assert!(view.read_with(&surface, |view, _| view.saving && view.draft_busy.get()));
        let command = endpoint
            .try_receive_command_for_contract_test()
            .expect("picker command");
        assert!(matches!(
            command.message,
            nxtools_protocol::Message::PlatformCommand(PlatformCommand::PickDirectory(_))
        ));
        app.update(&mut surface, |app, cx| {
            app.handle_platform_notification(
                PlatformNotification::CommandResult {
                    correlation_id: command.id,
                    result: nxtools_protocol::PlatformCommandResult {
                        success: true,
                        data: None,
                        error: None,
                        native_error_code: None,
                    },
                },
                cx,
            )
        });
        surface.run_until_parked();
        assert!(!view.read_with(&surface, |view, _| view.saving || view.draft_busy.get()));
        surface.simulate_click(button.center(), Default::default());
        surface.run_until_parked();
        let second = endpoint
            .try_receive_command_for_contract_test()
            .expect("second picker");
        assert_ne!(command.id, second.id);
        app.update(&mut surface, |app, cx| {
            app.handle_platform_notification(
                PlatformNotification::CommandResult {
                    correlation_id: command.id,
                    result: nxtools_protocol::PlatformCommandResult {
                        success: true,
                        data: Some(root.to_string_lossy().into_owned()),
                        error: None,
                        native_error_code: None,
                    },
                },
                cx,
            )
        });
        surface.run_until_parked();
        assert!(view.read_with(&surface, |view, _| view.saving));
        let retired = app.update(&mut surface, |app, cx| {
            let pending = app
                .pending_data_directory
                .as_mut()
                .expect("current request");
            pending.validating = true;
            let retired = pending.clone();
            app.finish_data_picker(second.id, None, Some("平台组件已断开".into()), cx);
            retired
        });
        surface.run_until_parked();
        assert!(!view.read_with(&surface, |view, _| view.saving));
        app.update(&mut surface, |app, cx| {
            app.complete_data_validation(retired, Err("迟到结果".into()), cx)
        });
        surface.run_until_parked();
        assert_eq!(
            view.read_with(&surface, |view, _| view.error.clone())
                .as_deref(),
            Some("平台组件已断开")
        );
        assert!(root.join("config.json").is_file());
        assert!(!root.join("cache/data-location.json").exists());
        app.update(&mut surface, |app, cx| app.close_window(cx));
        std::fs::remove_dir_all(root).expect("cleanup");
    }
}
