//! Shared extension management; feature applications consume committed snapshots.
pub(crate) mod committed_events;
mod directory;
pub(crate) mod download_resolver;
pub(crate) mod feature_adapters;
pub(crate) mod manual_actions;
mod manual_notifications;
pub(crate) mod manual_runs;
mod runtime_environment;
pub(crate) mod service;
pub(crate) mod ui_host;
use gpui_kit::{Context, Entity, Global, WeakEntity};
use nxtools_core::AppConfig;
use nxtools_extensions::{ImportPreview, PackageRegistry};
use nxtools_ipc::{PlatformBridge, PlatformNotification};
use nxtools_protocol::{PickExtensionManifestCommand, PlatformCommand};
use runtime_environment::PlatformRequestOperation;
use std::{path::PathBuf, rc::Rc, time::Instant};
use tracing::{info, warn};

use crate::notifications::{AppNotification, NotificationLevel};
use crate::quick_launch::QuickLaunchApplication;

pub(crate) struct ExtensionApplication {
    config_path: PathBuf,
    pub(crate) packages: PackageRegistry,
    platform: Option<Rc<PlatformBridge>>,
    service: Option<service::ExtensionService>,
    event_control: Option<committed_events::EventControl>,
    clipboard: Option<WeakEntity<crate::clipboard::ClipboardApplication>>,
    notifications: Option<Entity<crate::notifications::NotificationCenter>>,
    pending_host:
        std::collections::HashMap<uuid::Uuid, async_channel::Sender<nxtools_extensions::HostReply>>,
    quick_launch: WeakEntity<QuickLaunchApplication>,
    pending_picker: Option<uuid::Uuid>,
    pending_runtime: std::collections::HashMap<
        uuid::Uuid,
        (
            Instant,
            async_channel::Sender<nxtools_protocol::PlatformCommandResult>,
            PlatformRequestOperation,
        ),
    >,
    pending_tool: Option<(uuid::Uuid, Instant)>,
    pending_directory: Option<(uuid::Uuid, Instant)>,
    pending_manual:
        std::collections::HashMap<uuid::Uuid, (String, tokio::sync::watch::Sender<bool>)>,
    manual_runs: manual_runs::ManualRuns,
    generation: u64,
    pending_import_validation: Option<u64>,
    pub(crate) import_failure: Option<String>,
    pub(crate) trust_failure: Option<String>,
    pub(crate) trust_revision: u64,
    pub(crate) busy: bool,
    pub(crate) preview: Option<(ImportPreview, String)>,
    // Regression-only legacy injection: production has no page status string.
    #[cfg(test)]
    pub(crate) status: Option<String>,
    /// Last management result (action, durably saved, runtime applied), never presentation text.
    pub(crate) last_commit: Option<(&'static str, bool, bool)>,
    pub(crate) commit_revision: u64,
}

pub(crate) struct ExtensionApplicationGlobal(pub(crate) WeakEntity<ExtensionApplication>);
impl Global for ExtensionApplicationGlobal {}

pub(crate) type ImportCandidateIdentity = (u64, String, String, String);
impl ExtensionApplication {
    pub(crate) fn new(
        config: AppConfig,
        config_path: PathBuf,
        platform: Option<Rc<PlatformBridge>>,
        quick_launch: WeakEntity<QuickLaunchApplication>,
    ) -> Self {
        Self {
            config_path,
            packages: config.extensions,
            platform,
            service: None,
            event_control: None,
            clipboard: None,
            notifications: None,
            pending_host: Default::default(),
            quick_launch,
            pending_picker: None,
            pending_runtime: Default::default(),
            pending_tool: None,
            pending_directory: None,
            pending_manual: Default::default(),
            manual_runs: Default::default(),
            generation: 0,
            pending_import_validation: None,
            import_failure: None,
            trust_revision: 0,
            trust_failure: None,
            busy: false,
            preview: None,
            #[cfg(test)]
            status: None,
            last_commit: None,
            commit_revision: 0,
        }
    }
    fn notify_operation(
        &self,
        key: impl Into<gpui_kit::SharedString>,
        level: NotificationLevel,
        message: impl Into<gpui_kit::SharedString>,
        cx: &mut Context<Self>,
    ) {
        let Some(center) = self
            .notifications
            .clone()
            .or_else(|| crate::notifications::current(cx))
        else {
            return;
        };
        let notification = AppNotification::new(key, level, "扩展", message);
        // The screen observer can write back into application views: release this lease first.
        cx.defer(move |cx| center.update(cx, |center, cx| center.push(notification, cx)));
    }
    pub(crate) fn import_candidate_identity(&self) -> Option<ImportCandidateIdentity> {
        self.preview.as_ref().map(|(preview, source)| {
            (
                self.generation,
                preview.manifest.id.clone(),
                preview.content_hash.clone(),
                source.clone(),
            )
        })
    }
    pub(crate) fn cancel_import_current(
        &mut self,
        identity: &ImportCandidateIdentity,
        cx: &mut Context<Self>,
    ) {
        if self.busy || self.import_candidate_identity().as_ref() != Some(identity) {
            return;
        }
        self.generation = self.generation.wrapping_add(1);
        self.preview = None;
        cx.notify();
    }
    pub(crate) fn retire_import_preview(&mut self, cx: &mut Context<Self>) {
        self.generation = self.generation.wrapping_add(1);
        self.pending_picker = None;
        self.preview = None;
        self.import_failure = None;
        cx.notify();
    }

    pub(crate) fn event_paused(&self, package: &str, handler: &str) -> bool {
        self.event_control
            .as_ref()
            .is_some_and(|control| control.is_paused(package, handler))
    }
    pub(crate) fn retry_event(&mut self, package: String, handler: String, cx: &mut Context<Self>) {
        let declared = self.packages.packages.iter().any(|item| {
            item.manifest.id == package
                && item.manifest.handlers.iter().any(|item| {
                    item.id == handler
                        && matches!(
                            item.entry.as_str(),
                            "created" | "completed" | "failed" | "committed"
                        )
                })
        });
        let restored = declared
            && self
                .event_control
                .as_ref()
                .is_some_and(|control| control.retry(&package, &handler).is_ok());
        self.notify_operation(
            format!("extension-event-retry-{}", uuid::Uuid::new_v4()),
            if restored {
                NotificationLevel::Success
            } else {
                NotificationLevel::Error
            },
            if restored {
                "自动事件已恢复；等待下一次新提交事件，不重放旧事件"
            } else {
                "无法恢复自动事件，请重试"
            },
            cx,
        );
        cx.notify();
    }

    #[cfg(test)]
    pub(crate) fn bind_event_control_for_test(&mut self, control: committed_events::EventControl) {
        self.event_control = Some(control);
    }
    pub(crate) fn pick_manifest(&mut self, cx: &mut Context<Self>) {
        if self.busy || self.pending_picker.is_some() {
            return;
        }
        let id = uuid::Uuid::new_v4();
        let Some(platform) = self.platform.clone() else {
            self.notify_operation(
                format!("extension-picker-{id}"),
                NotificationLevel::Error,
                "Windows 文件选择器当前不可用；请在下方输入 ZIP 扩展包路径后检查",
                cx,
            );
            cx.notify();
            return;
        };
        self.pending_picker = Some(id);
        if platform
            .send_command_with_id(
                PlatformCommand::PickExtensionManifest(PickExtensionManifestCommand {
                    current_path: String::new(),
                }),
                id,
            )
            .is_err()
        {
            self.pending_picker = None;
            self.notify_operation(
                format!("extension-picker-{id}"),
                NotificationLevel::Error,
                "无法打开文件选择器，请重试",
                cx,
            );
        } else {
            cx.spawn(async move |this, cx| {
                cx.background_executor()
                    .timer(std::time::Duration::from_secs(300))
                    .await;
                let _ = this.update(cx, |app, cx| {
                    if app.pending_picker == Some(id) {
                        app.pending_picker = None;
                        app.notify_operation(
                            format!("extension-picker-{id}"),
                            NotificationLevel::Error,
                            "文件选择已超时，请重试",
                            cx,
                        );
                        cx.notify();
                    }
                });
            })
            .detach();
        }
        cx.notify();
    }

    pub(crate) fn handle_platform_notification(
        &mut self,
        notification: PlatformNotification,
        cx: &mut Context<Self>,
    ) {
        if let PlatformNotification::CommandResult {
            correlation_id,
            result,
        } = &notification
            && let Some((started, reply, operation)) = self.pending_runtime.remove(correlation_id)
        {
            info!(event=operation.terminal_event(),correlation_id=%correlation_id,success=result.success,native_error_code=?result.native_error_code,elapsed_ms=started.elapsed().as_millis(),"extension platform operation terminated");
            let _ = reply.try_send(result.clone());
            return;
        }
        if matches!(&notification, PlatformNotification::Disconnected { .. }) {
            if let Some((id, _)) = self.pending_directory {
                self.finish_open_directory(id, false, "Windows 适配器连接已断开，请重试", cx);
            }
            for (id, (started, reply, operation)) in self.pending_runtime.drain() {
                info!(event=operation.terminal_event(),correlation_id=%id,success=false,reason="disconnected",elapsed_ms=started.elapsed().as_millis(),"extension platform operation terminated");
                let _ = reply.try_send(nxtools_protocol::PlatformCommandResult {
                    success: false,
                    error: Some("Windows 适配器连接已断开".into()),
                    native_error_code: None,
                    data: None,
                });
            }
        }
        if let PlatformNotification::CommandResult {
            correlation_id,
            result,
        } = &notification
            && let Some(reply) = self.pending_host.remove(correlation_id)
        {
            let success = result.success;
            let native_error_code = result.native_error_code;
            cx.defer(move |_| {
                let _ = reply.try_send(nxtools_extensions::HostReply {
                    success,
                    value: serde_json::json!({"native_error_code":native_error_code}),
                });
            });
            return;
        }
        if matches!(&notification, PlatformNotification::Disconnected { .. }) {
            for (_, reply) in self.pending_host.drain() {
                cx.defer(move |_| {
                    let _ = reply.try_send(nxtools_extensions::HostReply::error("disconnected"));
                });
            }
        }
        match notification {
            PlatformNotification::Disconnected { .. } => {
                if let Some((id, _)) = self.pending_tool.take() {
                    self.notify_operation(
                        format!("extension-action-{id}"),
                        NotificationLevel::Error,
                        "扩展动作连接已断开，请重试",
                        cx,
                    );
                    cx.notify();
                }
                if let Some(id) = self.pending_picker.take() {
                    self.notify_operation(
                        format!("extension-picker-{id}"),
                        NotificationLevel::Error,
                        "文件选择连接已断开，请重试",
                        cx,
                    );
                    cx.notify();
                }
            }
            PlatformNotification::CommandResult {
                correlation_id,
                result,
            } if self
                .pending_tool
                .is_some_and(|(id, _)| id == correlation_id) =>
            {
                let (_, started) = self.pending_tool.take().expect("matched pending action");
                self.notify_operation(
                    format!("extension-action-{correlation_id}"),
                    if result.success {
                        NotificationLevel::Success
                    } else {
                        NotificationLevel::Error
                    },
                    if result.success {
                        "动作已完成"
                    } else {
                        "扩展动作失败，请重试"
                    },
                    cx,
                );
                info!(event="extension.action.completed", correlation_id=%correlation_id,
                    success=result.success, native_error_code=?result.native_error_code,
                    elapsed_ms=started.elapsed().as_millis(), "extension native action terminated");
                cx.notify();
            }
            PlatformNotification::CommandResult {
                correlation_id,
                result,
            } if self.pending_picker == Some(correlation_id) => {
                self.pending_picker = None;
                if result.success {
                    if let Some(path) = result.data.filter(|path| !path.trim().is_empty()) {
                        self.preview_path(PathBuf::from(path), cx);
                    }
                } else {
                    if result.native_error_code != Some(995) {
                        self.notify_operation(
                            format!("extension-picker-{correlation_id}"),
                            NotificationLevel::Error,
                            format!(
                                "无法选择文件{}；可重试或手动输入路径后预览",
                                result
                                    .native_error_code
                                    .map(|code| format!("（错误码 {code}）"))
                                    .unwrap_or_default()
                            ),
                            cx,
                        );
                    }
                    cx.notify();
                }
            }
            _ => {}
        }
    }

    pub(crate) fn preview_path(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        self.generation = self.generation.wrapping_add(1);
        let generation = self.generation;
        self.busy = true;
        self.preview = None;
        self.pending_import_validation = Some(generation);
        self.import_failure = None;
        let id = uuid::Uuid::new_v4();
        let operation_key = format!("extension-import-validation-{id}");
        let started = Instant::now();
        info!(event="extension.import.validation.accepted",correlation_id=%id,"extension ZIP validation accepted");
        self.notify_operation(
            operation_key.clone(),
            NotificationLevel::Progress,
            "正在检查扩展包…",
            cx,
        );
        let background = cx.background_executor().clone();
        cx.spawn(async move |this, cx| {
            let outcome = background
                .spawn(async move {
                    if !path.extension().is_some_and(|extension|extension.eq_ignore_ascii_case("zip")) {
                        return Err(nxtools_extensions::PackageError::ArchiveInvalid);
                    }
                    let source = path.to_string_lossy().into_owned();
                    let preview = ImportPreview::from_archive(&path)?;
                    nxtools_finder::validate_package_commands(&preview.manifest).map_err(|_|nxtools_extensions::PackageError::InvalidManifest)?;
                    Ok((preview, source))
                })
                .await;
            let _ = this.update(cx, |app, cx| {
                if app.pending_import_validation==Some(generation) { app.pending_import_validation=None; app.busy=false; }
                if app.generation != generation {
                    info!(event="extension.import.validation.cancelled",correlation_id=%id,elapsed_ms=started.elapsed().as_millis(),"extension ZIP preview retired");
                    if let Some(center) = app.notifications.clone().or_else(|| crate::notifications::current(cx)) {
                        let notice_generation = center.read(cx).items().iter().find(|notice| notice.id.as_ref() == operation_key.as_str()).map(|notice| notice.generation());
                        if let Some(notice_generation) = notice_generation {
                            cx.defer(move |cx| center.update(cx, |center, cx| center.dismiss_generation(operation_key, notice_generation, cx)));
                        }
                    }
                    cx.notify();return;
                }
                app.busy = false;
                match outcome {
                    Ok(preview) => {
                        app.preview = Some(preview);
                        app.notify_operation(operation_key.clone(), NotificationLevel::Success, "扩展包检查完成，请确认来源和权限", cx);
                        info!(event="extension.import.validation.completed",correlation_id=%id,elapsed_ms=started.elapsed().as_millis(),"extension ZIP validated");
                    }
                    Err(error) => {
                        app.notify_operation(operation_key.clone(), NotificationLevel::Error, match error {
                            nxtools_extensions::PackageError::ArchiveUnsafe=>"无法导入：ZIP 包含危险路径、重复名称或链接，请重新打包。",
                            nxtools_extensions::PackageError::ArchiveTooLarge=>"无法导入：扩展包体积、解压后大小或文件数量超过限制。",
                            nxtools_extensions::PackageError::ArchiveInvalid=>"无法导入：请选择有效的 ZIP 扩展包；extension.json 应在根目录或唯一顶层文件夹中。",
                            nxtools_extensions::PackageError::CodeRequired=>"无法导入：扩展包缺少清单声明的脚本或文件，请重新打包。",
                            nxtools_extensions::PackageError::FileOperation=>"无法读取扩展包，请检查文件是否存在以及访问权限。",
                            _=>"无法导入：扩展清单、功能声明或文件不符合要求，请检查扩展包。",
                        }, cx);
                        warn!(event="extension.import.validation.failed",correlation_id=%id,reason="invalid_package",elapsed_ms=started.elapsed().as_millis(),"extension ZIP rejected");
                    }
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    pub(crate) fn confirm_import_current(
        &mut self,
        identity: ImportCandidateIdentity,
        expected_previous: Option<String>,
        cx: &mut Context<Self>,
    ) {
        if self.busy || self.import_candidate_identity().as_ref() != Some(&identity) {
            return;
        }
        let current = self
            .packages
            .packages
            .iter()
            .find(|package| package.manifest.id == identity.1)
            .map(|package| package.content_hash.clone());
        if current != expected_previous {
            self.import_failure = Some("已安装的扩展已变化，请取消并重新检查扩展包。".into());
            cx.notify();
            return;
        }
        let Some((preview, source)) = self.preview.clone() else {
            return;
        };
        self.commit(
            "import",
            move |path| {
                let packages_root = path
                    .parent()
                    .ok_or_else(|| anyhow::anyhow!("extension config directory unavailable"))?
                    .join("extensions");
                AppConfig::import_extension_preview_checked(
                    path,
                    preview,
                    &packages_root,
                    source,
                    expected_previous,
                )
            },
            cx,
        );
    }

    pub(crate) fn approve_runtime_checked(
        &mut self,
        id: String,
        expected_hash: String,
        path: PathBuf,
        expected_runtime_hash: Option<String>,
        expected_selected: Option<PathBuf>,
        cx: &mut Context<Self>,
    ) {
        if self.busy {
            return;
        }
        self.commit(
            "trust",
            move |config| {
                let runtime = nxtools_extensions::RuntimeIdentity::inspect(&path)?;
                if let Some(hash) = expected_runtime_hash {
                    anyhow::ensure!(
                        runtime.sha256.eq_ignore_ascii_case(&hash),
                        "detected runtime changed"
                    );
                }
                AppConfig::approve_extension_checked(
                    config,
                    &id,
                    &expected_hash,
                    runtime,
                    expected_selected,
                )
            },
            cx,
        );
    }
    pub(crate) fn revoke(&mut self, id: String, cx: &mut Context<Self>) {
        if let Some(service) = self.service.as_ref() {
            service.runtime.cancel_package(&id);
        }
        self.commit(
            "revoke",
            move |path| AppConfig::revoke_extension(path, &id),
            cx,
        );
    }
    pub(crate) fn set_event(
        &mut self,
        id: String,
        handler: String,
        enabled: bool,
        cx: &mut Context<Self>,
    ) {
        self.commit(
            "event",
            move |path| AppConfig::set_extension_event(path, &id, &handler, enabled),
            cx,
        );
    }
    pub(crate) fn set_provider(
        &mut self,
        id: String,
        handler: String,
        enabled: bool,
        cx: &mut Context<Self>,
    ) {
        self.commit(
            "provider",
            move |path| AppConfig::set_extension_provider(path, &id, &handler, enabled),
            cx,
        );
    }
    pub(crate) fn set_resolver_chain(
        &mut self,
        host: String,
        route: Vec<nxtools_extensions::HandlerReference>,
        cx: &mut Context<Self>,
    ) {
        self.commit(
            "resolver_chain",
            move |path| AppConfig::save_extension_resolver_chain(path, host, route),
            cx,
        );
    }
    pub(crate) fn save_settings(
        &mut self,
        id: String,
        values: std::collections::BTreeMap<String, serde_json::Value>,
        secrets: std::collections::BTreeMap<String, Option<String>>,
        cx: &mut Context<Self>,
    ) {
        if secrets.is_empty() {
            self.commit(
                "settings",
                move |path| AppConfig::save_extension_settings(path, &id, values),
                cx,
            );
            return;
        }
        if self.busy {
            return;
        }
        let Some(package) = self
            .packages
            .packages
            .iter()
            .find(|package| package.manifest.id == id)
            .cloned()
        else {
            return;
        };
        self.last_commit = None;
        let operation_key = format!("extension-management-{}", uuid::Uuid::new_v4());
        let Some(service) = self.service.clone() else {
            self.notify_operation(
                operation_key,
                NotificationLevel::Error,
                "凭据宿主不可用，未保存",
                cx,
            );
            cx.notify();
            return;
        };
        let path = self.config_path.clone();
        self.busy = true;
        self.last_commit = None;
        self.notify_operation(
            operation_key.clone(),
            NotificationLevel::Progress,
            "正在保存扩展设置…",
            cx,
        );
        let task = service.executor().spawn(async move {
            let store = service
                .runtime
                .secret_store()
                .map_err(|_| nxtools_extensions::SettingsSaveError::ReadFailed)?;
            nxtools_extensions::save_settings(
                &package,
                nxtools_extensions::SettingsDraft { values, secrets },
                &store,
                move |values| async move {
                    tokio::task::spawn_blocking(move || {
                        AppConfig::save_extension_settings(&path, &id, values)
                    })
                    .await
                    .map_err(|_| nxtools_extensions::PackageError::FileOperation)?
                    .map_err(|_| nxtools_extensions::PackageError::FileOperation)
                },
            )
            .await
        });
        cx.spawn(async move |this, cx| {
            let outcome = task.await;
            let _ = this.update(cx, |app, cx| {
                app.busy = false;
                app.commit_revision = app.commit_revision.wrapping_add(1);
                match outcome {
                    Ok(Ok(config)) => {
                        app.packages = config.extensions;
                        let applied = app.service.as_ref().is_none_or(|service| {
                            service.publish_registry(app.packages.clone()).is_ok()
                        });
                        app.last_commit = Some(("settings", true, applied));
                        app.notify_operation(
                            operation_key.clone(),
                            if applied {
                                NotificationLevel::Success
                            } else {
                                NotificationLevel::Error
                            },
                            if applied {
                                "已保存"
                            } else {
                                "已保存，但扩展运行时未能应用；请重试"
                            },
                            cx,
                        );
                        let quick = app.quick_launch.clone();
                        let snapshot = app.packages.clone();
                        cx.defer(move |cx| {
                            let _ = quick.update(cx, |quick, cx| {
                                quick.apply_extension_snapshot(snapshot, cx)
                            });
                        });
                    }
                    Ok(Err(nxtools_extensions::SettingsSaveError::CompensationFailed)) => {
                        app.last_commit = Some(("settings", false, false));
                        app.notify_operation(
                            operation_key.clone(),
                            NotificationLevel::Error,
                            "保存失败且凭据恢复不完整，请重试或清除凭据",
                            cx,
                        );
                    }
                    _ => {
                        app.last_commit = Some(("settings", false, false));
                        app.notify_operation(
                            operation_key.clone(),
                            NotificationLevel::Error,
                            "未保存；已尝试恢复原有凭据，请重试",
                            cx,
                        );
                    }
                };
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    pub(crate) fn set_aliases(
        &mut self,
        id: String,
        handler: String,
        aliases: Vec<String>,
        cx: &mut Context<Self>,
    ) {
        self.commit(
            "aliases",
            move |path| AppConfig::set_extension_aliases(path, &id, &handler, aliases),
            cx,
        );
    }

    pub(crate) fn execute_tool_action(
        &mut self,
        package_id: &str,
        handler_id: &str,
        item_id: Option<&str>,
        cx: &mut Context<Self>,
    ) {
        if self.busy || self.pending_tool.is_some() {
            return;
        }
        let id = uuid::Uuid::new_v4();
        let action = self
            .packages
            .packages
            .iter()
            .find(|package| package.enabled && package.manifest.id == package_id)
            .and_then(|package| {
                package
                    .manifest
                    .handlers
                    .iter()
                    .find(|handler| handler.id == handler_id && handler.feature == "tools")
            })
            .and_then(|handler| match (&handler.action, item_id) {
                (nxtools_extensions::DeclarativeAction::Items { items }, Some(id)) => items
                    .iter()
                    .find(|item| item.id == id)
                    .map(|item| item.action.as_ref()),
                (nxtools_extensions::DeclarativeAction::Items { .. }, None) => None,
                (action, None) => Some(action),
                _ => None,
            })
            .cloned();
        let Some(action) = action else {
            self.notify_operation(
                format!("extension-action-{id}"),
                NotificationLevel::Error,
                "扩展已停用、更新或动作不存在",
                cx,
            );
            cx.notify();
            return;
        };
        if matches!(action, nxtools_extensions::DeclarativeAction::Script { .. }) {
            self.invoke_manual(
                package_id.to_owned(),
                handler_id.to_owned(),
                "tools",
                serde_json::json!({}),
                cx,
            );
            return;
        }
        let prepared = nxtools_finder::prepare_package_action(&action, "");
        let Ok((target, action, _)) = prepared else {
            self.notify_operation(
                format!("extension-action-{id}"),
                NotificationLevel::Error,
                "扩展动作参数无效",
                cx,
            );
            cx.notify();
            return;
        };
        let command = match action {
            nxtools_finder::SearchResultAction::RunCommand { command } => {
                PlatformCommand::LaunchProcess(nxtools_protocol::LaunchProcessCommand {
                    executable: command.executable,
                    arguments: command.arguments,
                    working_directory: command.working_directory,
                    run_as_admin: false,
                    run_silently: command.run_silently,
                })
            }
            nxtools_finder::SearchResultAction::OpenTarget => {
                PlatformCommand::OpenShellTarget(nxtools_protocol::OpenShellTargetCommand {
                    target,
                    action: nxtools_protocol::ShellOpenAction::Open,
                })
            }
            _ => {
                self.notify_operation(
                    format!("extension-action-{id}"),
                    NotificationLevel::Error,
                    "扩展动作类型不可用",
                    cx,
                );
                cx.notify();
                return;
            }
        };
        self.pending_tool = Some((id, Instant::now()));
        let sent = self
            .platform
            .as_ref()
            .is_some_and(|platform| platform.send_command_with_id(command, id).is_ok());
        if !sent {
            self.pending_tool = None;
            self.notify_operation(
                format!("extension-action-{id}"),
                NotificationLevel::Error,
                "Windows 动作通道不可用，请重试",
                cx,
            );
        } else {
            self.notify_operation(
                format!("extension-action-{id}"),
                NotificationLevel::Progress,
                "动作已提交，等待 Windows 结果",
                cx,
            );
            info!(event="extension.action.accepted", correlation_id=%id, "extension action queued");
            cx.spawn(async move |this, cx| {
                cx.background_executor().timer(std::time::Duration::from_secs(15)).await;
                let _ = this.update(cx, |app, cx| {
                    if app.pending_tool.is_some_and(|(pending, _)| pending == id) {
                        app.pending_tool = None;
                        app.notify_operation(format!("extension-action-{id}"), NotificationLevel::Error, "扩展动作超时，请确认后重试", cx);
                        warn!(event="extension.action.failed", correlation_id=%id, reason="timeout", "extension action timed out");
                        cx.notify();
                    }
                });
            }).detach();
        }
        cx.notify();
    }

    pub(crate) fn set_enabled(&mut self, id: String, enabled: bool, cx: &mut Context<Self>) {
        if !enabled && let Some(service) = self.service.as_ref() {
            service.runtime.cancel_package(&id);
        }
        self.commit(
            if enabled { "enable" } else { "disable" },
            move |path| AppConfig::set_extension_enabled(path, &id, enabled),
            cx,
        );
    }

    pub(crate) fn remove(&mut self, id: String, cx: &mut Context<Self>) {
        if let Some(service) = self.service.as_ref() {
            service.runtime.cancel_package(&id);
        }
        self.commit(
            "remove",
            move |path| AppConfig::remove_extension(path, &id),
            cx,
        );
    }

    #[cfg(test)]
    pub(crate) fn has_pending_manual(&self) -> bool {
        !self.pending_manual.is_empty()
    }

    #[cfg(test)]
    pub(crate) fn cancel_manual(&mut self, cx: &mut Context<Self>) {
        let ids = self.pending_manual.keys().copied().collect::<Vec<_>>();
        for id in ids {
            self.cancel_manual_run(id, cx);
        }
    }
    pub(crate) fn invoke_manual(
        &mut self,
        package_id: String,
        handler_id: String,
        feature: &str,
        context: serde_json::Value,
        cx: &mut Context<Self>,
    ) {
        match self.start_manual(package_id, handler_id, feature, context, cx) {
            Ok(id) => self.notify_manual_run(id, cx),
            Err(message) => {
                self.notify_operation(
                    format!("extension-action-{}", uuid::Uuid::new_v4()),
                    NotificationLevel::Error,
                    message,
                    cx,
                );
                cx.notify();
            }
        }
    }
    pub(crate) fn start_manual(
        &mut self,
        package_id: String,
        handler_id: String,
        feature: &str,
        context: serde_json::Value,
        cx: &mut Context<Self>,
    ) -> Result<uuid::Uuid, &'static str> {
        if self.pending_manual.len() >= 4 {
            return Err("扩展调用繁忙，请稍后重试");
        }
        let Some(package) = self.packages.packages.iter().find(|package| {
            package.enabled && package.trust.is_some() && package.manifest.id == package_id
        }) else {
            return Err("扩展未启用或未信任；请到扩展管理检查");
        };
        let Some(handler) = package.manifest.handlers.iter().find(|handler| {
            handler.id == handler_id
                && handler.feature == feature
                && matches!(handler.entry.as_str(), "action" | "command")
        }) else {
            return Err("扩展动作已失效，请重新选择");
        };
        let package_name = package.manifest.name.clone();
        let action_name = handler.title.clone();
        let hash = package.content_hash.clone();
        let Some(service) = self.service.clone() else {
            return Err("扩展运行通道不可用，请检查后重试");
        };
        let id = uuid::Uuid::new_v4();
        let (cancel, cancellation) = tokio::sync::watch::channel(false);
        self.pending_manual.insert(id, (package_id.clone(), cancel));
        self.begin_manual_run(id, package_id.clone(), package_name, action_name);
        service
            .manual_effects
            .begin(id, &package_id, &handler_id, &hash);
        info!(event="extension.manual.accepted", correlation_id=%id, "manual extension invocation accepted");
        let task = service.invoke(nxtools_extensions::InvokeRequest {
            package_id,
            handler_id,
            correlation_id: id,
            generation: self.generation,
            context,
            causality: nxtools_extensions::Causality::root(id),
            cancellation,
            batches: None,
        });
        cx.spawn(async move |this, cx| {
            let outcome = task
                .await
                .unwrap_or(Err(nxtools_extensions::RuntimeError::NativeUnavailable));
            let _ = this.update(cx, |app, cx| app.finish_manual_run(id, outcome, cx));
        })
        .detach();
        cx.notify();
        Ok(id)
    }
    fn commit(
        &mut self,
        action: &'static str,
        operation: impl FnOnce(&std::path::Path) -> anyhow::Result<AppConfig> + Send + 'static,
        cx: &mut Context<Self>,
    ) {
        if self.busy {
            return;
        }
        self.busy = true;
        self.last_commit = None;
        let id = uuid::Uuid::new_v4();
        let operation_key = format!("extension-management-{id}");
        self.notify_operation(
            operation_key.clone(),
            NotificationLevel::Progress,
            "正在保存扩展更改…",
            cx,
        );
        if action == "import" {
            self.import_failure = None;
        }
        let config_path = self.config_path.clone();
        if action == "trust" {
            self.trust_failure = None;
        }
        let started = Instant::now();
        info!(event = "extension.management.accepted", correlation_id=%id, action, "extension change accepted");
        let background = cx.background_executor().clone();
        cx.spawn(async move |this, cx| {
            let outcome = background.spawn(async move { operation(&config_path) }).await;
            let _ = this.update(cx, |app, cx| {
                app.busy = false;
                if action=="trust" { app.trust_revision=app.trust_revision.wrapping_add(1); }
                app.commit_revision = app.commit_revision.wrapping_add(1);
                match outcome {
                    Ok(config) => {
                        app.packages = config.extensions;
                        app.preview = None;
                        let applied = app.service.as_ref().is_none_or(|service| service.publish_registry(app.packages.clone()).is_ok());
                        app.last_commit = Some((action, true, applied));
                        app.notify_operation(operation_key.clone(), if applied { NotificationLevel::Success } else { NotificationLevel::Error },
                            if applied { "已保存" } else { "已保存，但扩展运行时未能应用；请重试" }, cx);
                        let snapshot = app.packages.clone();
                        let quick = app.quick_launch.clone();
                        cx.defer(move |cx| { let _ = quick.update(cx, |quick, cx| quick.apply_extension_snapshot(snapshot, cx)); });
                        info!(event = "extension.management.completed", correlation_id=%id, action, elapsed_ms=started.elapsed().as_millis(), "extension change committed");
                    }
                    Err(error) => {
                        let message: String = match error.downcast_ref::<nxtools_extensions::PackageError>() {
                            Some(nxtools_extensions::PackageError::KeywordConflict { keyword }) => format!("关键词“{keyword}”已被使用，请修改关键词或停用冲突命令后重试"),
                            Some(nxtools_extensions::PackageError::AlreadyInstalled) => "此包已存在，请预览后明确确认替换".into(),
                            _ => "未保存：声明无效或配置不可写；现有配置未被替换，请检查后重试".into(),
                        };
                        app.last_commit = Some((action, false, false));
                        // Keep blocking identity/validation failures beside the risk decision;
                        // filesystem/runtime operation failures belong only to the shared center.
                        let contextual = error.downcast_ref::<nxtools_extensions::PackageError>().is_some_and(|error| !matches!(error, nxtools_extensions::PackageError::FileOperation));
                        if action=="import" {app.import_failure=Some(if contextual { message.clone() } else { "导入尚未完成；当前候选和原配置已保留，可重试或取消。".into() });}
                        if action=="trust" {app.trust_failure=Some(if contextual { message.clone() } else { "信任尚未保存；权限未变更，可重试或取消。".into() });}
                        app.notify_operation(operation_key.clone(), NotificationLevel::Error, message, cx);
                        warn!(event = "extension.management.failed", correlation_id=%id, action, reason="validation_or_persistence", elapsed_ms=started.elapsed().as_millis(), "extension change rejected");
                    }
                }
                cx.notify();
            });
        }).detach();
        cx.notify();
    }
}

pub(crate) fn register_global(application: &Entity<ExtensionApplication>, cx: &mut gpui_kit::App) {
    cx.set_global(ExtensionApplicationGlobal(application.downgrade()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::{AppContext, TestAppContext};

    #[gpui_kit::test]
    fn management_commits_keep_distinct_notices_and_preserve_last_good_on_failure(
        cx: &mut TestAppContext,
    ) {
        use crate::notifications::NotificationLevel;
        cx.update(gpui_kit::init);
        cx.executor().allow_parking();
        let temp = tempfile::tempdir().expect("owned management fixture");
        let path = temp.path().join("config.json");
        let mut config = AppConfig::default();
        config.extensions.install(ImportPreview::from_json(br#"{"schema_version":1,"id":"fixture.notice","name":"Notice","version":"1","handlers":[{"id":"open","title":"Open","feature":"tools","entry":"action","action":{"type":"website","url_template":"https://example.invalid/"}}]}"#).expect("manifest"), "owned".into(), false).expect("install");
        config.save_to(&path).expect("owned configuration");
        let center = cx.new(|_| crate::notifications::NotificationCenter::default());
        let app = cx.new(|_| {
            ExtensionApplication::new(config, path.clone(), None, WeakEntity::new_invalid())
        });
        app.update(cx, |app, _| app.notifications = Some(center.clone()));
        for (index, enabled) in [true, false].into_iter().enumerate() {
            app.update(cx, |app, cx| {
                app.set_enabled("fixture.notice".into(), enabled, cx);
                assert!(app.busy);
                assert!(app.status.is_none(), "no page feedback while committing");
            });
            let deadline = Instant::now() + std::time::Duration::from_secs(5);
            while app.read_with(cx, |app, _| app.busy) {
                cx.run_until_parked();
                assert!(Instant::now() < deadline, "owned commit deadline");
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
            cx.run_until_parked();
            app.read_with(cx, |app, _| {
                assert!(
                    app.status.is_none(),
                    "Saved belongs only to the shared center"
                );
                assert_eq!(app.packages.packages[0].enabled, enabled);
                assert!(
                    app.recent_manual_runs().is_empty(),
                    "enable is not execution"
                );
            });
            assert_eq!(
                AppConfig::load_from(&path)
                    .expect("durable config")
                    .extensions
                    .packages[0]
                    .enabled,
                enabled
            );
            center.read_with(cx, |center, _| {
                assert_eq!(
                    center.items().len(),
                    index + 1,
                    "each invocation keeps one progress-to-terminal notice",
                );
                assert_eq!(center.items()[0].level, NotificationLevel::Success);
                assert_eq!(center.items()[0].message, "已保存");
            });
        }
        let before = std::fs::read(&path).expect("last good bytes");
        app.update(cx, |app, cx| {
            app.set_enabled("missing.package".into(), true, cx)
        });
        let deadline = Instant::now() + std::time::Duration::from_secs(5);
        while app.read_with(cx, |app, _| app.busy) {
            cx.run_until_parked();
            assert!(Instant::now() < deadline, "owned failure deadline");
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        cx.run_until_parked();
        assert_eq!(std::fs::read(path).expect("unchanged config"), before);
        assert!(app.read_with(cx, |app, _| app.status.is_none()));
        center.read_with(cx, |center, _| {
            assert_eq!(center.items().len(), 3);
            assert_eq!(
                center
                    .items()
                    .iter()
                    .filter(|notice| notice.level == NotificationLevel::Success)
                    .count(),
                2
            );
            assert_eq!(center.items()[0].level, NotificationLevel::Error);
        });
    }

    #[gpui_kit::test]
    fn manual_cancel_keeps_pending_until_runtime_terminal(cx: &mut TestAppContext) {
        let owner = cx.new(|_| {
            ExtensionApplication::new(
                AppConfig::default(),
                PathBuf::new(),
                None,
                WeakEntity::new_invalid(),
            )
        });
        let id = uuid::Uuid::new_v4();
        let (cancel, cancellation) = tokio::sync::watch::channel(false);
        owner.update(cx, |owner, cx| {
            owner
                .pending_manual
                .insert(id, ("fixture.manual".into(), cancel));
            owner.cancel_manual(cx);
            assert!(
                owner.has_pending_manual(),
                "cancel request is not runtime terminal/cleanup"
            );
        });
        assert!(
            *cancellation.borrow(),
            "the actual run receives cancellation"
        );
    }

    #[gpui_kit::test]
    fn w6_tool_action_dispatches_only_enabled_current_package_and_confirms_result(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_kit::init);
        let center = cx.new(|_| crate::notifications::NotificationCenter::default());
        let (platform, endpoint) = nxtools_ipc::platform_bridge(8);
        endpoint.set_connected_for_contract_test(true);
        let mut config = AppConfig::default();
        let manifest = br#"{"schema_version":1,"id":"example.tool","name":"Tool","version":"1.0.0","handlers":[{"id":"open","title":"Open","feature":"tools","entry":"action","keywords":[],"action":{"type":"website","url_template":"https://example.test/"}}]}"#;
        config
            .extensions
            .install(
                ImportPreview::from_json(manifest).expect("manifest"),
                "fixture".into(),
                false,
            )
            .expect("install");
        config
            .extensions
            .set_enabled("example.tool", true, &[])
            .expect("enable");
        let app = cx.new(|_| {
            ExtensionApplication::new(
                config,
                PathBuf::new(),
                Some(Rc::new(platform)),
                WeakEntity::new_invalid(),
            )
        });
        app.update(cx, |app, _| app.notifications = Some(center.clone()));
        app.update(cx, |app, cx| {
            app.execute_tool_action("example.tool", "open", None, cx)
        });
        let command = endpoint
            .try_receive_command_for_contract_test()
            .expect("tool action command");
        assert!(
            matches!(command.message, nxtools_protocol::Message::PlatformCommand(PlatformCommand::OpenShellTarget(open)) if open.target == "https://example.test/")
        );
        app.update(cx, |app, cx| {
            app.handle_platform_notification(
                PlatformNotification::CommandResult {
                    correlation_id: command.id,
                    result: nxtools_protocol::PlatformCommandResult {
                        success: true,
                        error: None,
                        native_error_code: None,
                        data: None,
                    },
                },
                cx,
            )
        });
        cx.run_until_parked();
        app.read_with(cx, |app, _| assert!(app.status.is_none()));
        center.read_with(cx, |center, _| {
            assert_eq!(center.items().len(), 1);
            assert_eq!(center.items()[0].message, "动作已完成");
            assert_eq!(center.items()[0].level, NotificationLevel::Success);
        });
        app.update(cx, |app, cx| {
            app.packages
                .set_enabled("example.tool", false, &[])
                .expect("disable");
            app.execute_tool_action("example.tool", "open", None, cx);
        });
        assert!(
            endpoint.try_receive_command_for_contract_test().is_err(),
            "retired tool no dispatch"
        );
    }

    #[gpui_kit::test]
    fn w6_picker_cancel_disconnect_and_late_result_never_import(cx: &mut TestAppContext) {
        let (platform, endpoint) = nxtools_ipc::platform_bridge(8);
        endpoint.set_connected_for_contract_test(true);
        let app = cx.new(|_| {
            ExtensionApplication::new(
                AppConfig::default(),
                PathBuf::new(),
                Some(Rc::new(platform)),
                WeakEntity::new_invalid(),
            )
        });
        for disconnected in [false, true] {
            app.update(cx, |app, cx| app.pick_manifest(cx));
            let command = endpoint
                .try_receive_command_for_contract_test()
                .expect("picker queued");
            app.read_with(cx, |app, _| {
                assert_eq!(
                    app.pending_picker,
                    Some(command.id),
                    "pending before command observation"
                )
            });
            app.update(cx, |app, cx| {
                if disconnected {
                    app.handle_platform_notification(
                        PlatformNotification::Disconnected {
                            generation: 1,
                            pending_command_ids: vec![command.id],
                        },
                        cx,
                    );
                }
                app.handle_platform_notification(
                    PlatformNotification::CommandResult {
                        correlation_id: command.id,
                        result: nxtools_protocol::PlatformCommandResult {
                            success: true,
                            error: None,
                            native_error_code: None,
                            data: Some(if disconnected {
                                "C:\\must-not-read.json".into()
                            } else {
                                String::new()
                            }),
                        },
                    },
                    cx,
                );
                assert!(app.pending_picker.is_none());
                assert!(app.preview.is_none());
                assert!(!app.busy);
                assert!(app.packages.packages.is_empty());
            });
        }
    }
}
