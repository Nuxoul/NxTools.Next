use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Instant;

use gpui_kit::base::Root;
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::component::input::{Input, InputState, NumberInput, NumberInputEvent, StepAction};
use gpui_kit::component::menu::{DropdownMenu, PopupMenuItem};
use gpui_kit::component::scroll::ScrollableElement;
use gpui_kit::component::{Disableable, Sizable};
use gpui_kit::{
    AnyWindowHandle, Context, Entity, IntoElement, Render, WeakEntity, Window,
    WindowBackgroundAppearance, WindowBounds, WindowDecorations, WindowKind, WindowOptions, div,
    prelude::*, px, size,
};
use nxtools_core::{
    AppConfig, NetworkProxyMode, NetworkProxySettings, NetworkSettings, ProcessBlacklistRule,
    ProcessBlacklistSettings, SystemSettings,
};
use nxtools_ipc::{PlatformBridge, PlatformNotification};
use nxtools_network::{NetworkPolicy, proxy_mode_name};
use nxtools_protocol::{ConfigureDesktopShellCommand, PlatformCommand};
use tracing::{info, warn};

use crate::notifications::NotificationCenter;
use crate::theme::Theme;
use crate::ui::components::action::{SegmentedOption, action_button, primary_action_button};
use crate::ui::components::field::inline_error;
use crate::ui::components::settings::{SettingsShellIds, dialog_footer, settings_shell};
use crate::ui::components::titlebar::client_titlebar;
use gpui_kit::component::form::{Field, Form};
use gpui_kit::component::group_box::GroupBoxVariant;
use gpui_kit::component::setting::{SelectIndex, SettingGroup, SettingItem, SettingPage, Settings};
use gpui_kit::component::switch::Switch;

mod blacklist;
mod data_directory;
mod font_settings;
#[cfg(test)]
#[path = "network_settings/kit_migration_tests.rs"]
mod kit_migration_tests;
#[cfg(test)]
#[path = "network_settings/notification_tests.rs"]
mod notification_tests;
mod system_proxy;
#[path = "network_settings/theme_chooser.rs"]
mod theme_chooser;
#[cfg(debug_assertions)]
pub(crate) mod visual_preview;
mod window_picker;
mod window_picker_overlay;
#[cfg(test)]
mod window_picker_tests;

#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum GlobalSettingsTab {
    #[default]
    General,
    Network,
    Blacklist,
}

const GLOBAL_SETTINGS_WINDOW_WIDTH: f32 = 720.0;
const GLOBAL_SETTINGS_WINDOW_HEIGHT: f32 = 480.0;

pub(crate) struct NetworkSettingsApplication {
    policy: NetworkPolicy,
    config: AppConfig,
    config_path: PathBuf,
    notifications: Entity<NotificationCenter>,
    platform: Option<Rc<PlatformBridge>>,
    system_settings: Rc<RefCell<SystemSettings>>,
    window: Option<AnyWindowHandle>,
    draft_busy: Rc<std::cell::Cell<bool>>,
    clipboard: Option<WeakEntity<crate::clipboard::ClipboardApplication>>,
    quick_launch: Option<WeakEntity<crate::quick_launch::QuickLaunchApplication>>,
    #[cfg(test)]
    settings_view: Option<Entity<NetworkSettingsView>>,
    pending_apply: Option<PendingGlobalSettingsApply>,
    hidden_startup: bool,
    pending_proxy_snapshot: Option<system_proxy::PendingProxySnapshot>,
    data_paths: Option<nxtools_core::AppPaths>,
    pending_data_directory: Option<data_directory::PendingDataPicker>,
    pending_process_window_pick: Option<window_picker::PendingWindowPick>,
}

struct PendingGlobalSettingsApply {
    command_id: uuid::Uuid,
    correlation_id: uuid::Uuid,
    mode: &'static str,
    started_at: Instant,
    desired_system: SystemSettings,
    view: Option<WeakEntity<NetworkSettingsView>>,
    source_window: Option<AnyWindowHandle>,
    user_operation: bool,
}

fn global_shell_failure_message(failure_kind: &str, stage: Option<&str>) -> &'static str {
    match (failure_kind, stage) {
        (_, Some("desktop_executable_unavailable")) => {
            "设置已保存，但无法校验当前 NxTools 程序，启动项和托盘设置未应用。请关闭旧版本并从当前安装目录重新启动后重试。"
        }
        (_, Some("startup_registration_failed")) => {
            "设置已保存，但 Windows 启动项未能写入当前用户注册表。请检查系统策略或安全软件后重试。"
        }
        (_, Some("tray_icon_failed")) => {
            "设置已保存，但 Windows 未能创建或显示 NxTools 托盘图标。请检查通知区域设置后重试。"
        }
        (_, Some("windows_shell_enqueue_failed")) => {
            "设置已保存，但 Windows 未能安排系统设置更新。请稍后重试；问题持续时请重启 NxTools。"
        }
        (_, Some("windows_shell_apply_failed")) => {
            "Windows 未能应用启动项或托盘设置。设置已保存；请检查系统权限后重试。"
        }
        ("platform_timeout", _) => {
            "Windows 平台组件未及时响应。设置已保存；请检查 NxTools 后重试。"
        }
        ("platform_disconnected", _) => {
            "Windows 平台组件已断开。设置已保存；重新启动 NxTools 后重试。"
        }
        ("platform_enqueue", _) => "Windows 平台组件当前不可用。设置已保存；请稍后重试。",
        _ => "Windows 系统设置未能应用。设置已保存；请检查系统权限后重试。",
    }
}

impl NetworkSettingsApplication {
    #[cfg(test)]
    pub(crate) fn window_is_open(&self) -> bool {
        self.window.is_some()
    }

    pub(crate) fn new(
        policy: NetworkPolicy,
        config: AppConfig,
        config_path: PathBuf,
        notifications: Entity<NotificationCenter>,
        platform: Option<Rc<PlatformBridge>>,
        system_settings: Rc<RefCell<SystemSettings>>,
        hidden_startup: bool,
    ) -> Self {
        let data_paths = nxtools_core::AppPaths::default_locations()
            .ok()
            .map(|defaults| {
                if config_path.parent() == Some(defaults.config_dir.as_path()) {
                    defaults
                } else {
                    defaults.at_root(
                        config_path
                            .parent()
                            .and_then(|path| path.parent())
                            .unwrap_or(&defaults.config_dir),
                    )
                }
            });
        Self {
            policy,
            config,
            config_path,
            notifications,
            platform,
            system_settings,
            window: None,
            draft_busy: Rc::new(std::cell::Cell::new(false)),
            clipboard: None,
            quick_launch: None,
            #[cfg(test)]
            settings_view: None,
            pending_apply: None,
            pending_proxy_snapshot: None,
            hidden_startup,
            data_paths,
            pending_data_directory: None,
            pending_process_window_pick: None,
        }
    }

    pub(crate) fn bind_clipboard(
        &mut self,
        clipboard: WeakEntity<crate::clipboard::ClipboardApplication>,
    ) {
        self.clipboard = Some(clipboard);
    }

    pub(crate) fn bind_quick_launch(
        &mut self,
        quick_launch: WeakEntity<crate::quick_launch::QuickLaunchApplication>,
    ) {
        self.quick_launch = Some(quick_launch);
    }

    fn publish_live_blacklist(&self, correlation_id: uuid::Uuid, cx: &mut Context<Self>) {
        let clipboard = self.clipboard.clone();
        let quick_launch = self.quick_launch.clone();
        let settings = self.config.process_blacklist.clone();
        cx.defer(move |cx| {
            if let Some(clipboard) = clipboard {
                let _ = clipboard.update(cx, |owner, cx| {
                    owner.apply_live_blacklist(settings.clone(), correlation_id, cx)
                });
            }
            if let Some(quick_launch) = quick_launch {
                let _ = quick_launch.update(cx, |owner, cx| {
                    owner.apply_live_blacklist(settings, correlation_id, cx)
                });
            }
        });
    }

    pub(crate) fn open_window(&mut self, cx: &mut Context<Self>) {
        if let Some(window) = self.window {
            if window
                .update(cx, |_, window, _| {
                    window.activate_window();
                    window.refresh();
                })
                .is_ok()
            {
                return;
            }
            self.window = None;
        }

        match AppConfig::load_from(&self.config_path) {
            Ok(latest) => {
                self.config.network = latest.network;
                self.config.system = latest.system;
                self.config.process_blacklist = latest.process_blacklist;
            }
            Err(_) => {
                self.notifications.update(cx, |notifications, cx| {
                    notifications.show_error(
                        format!("global-settings-load-{}", uuid::Uuid::new_v4()),
                        "无法读取全局设置",
                        "请检查配置文件是否存在且可读取，然后重试。",
                        cx,
                    );
                });
                return;
            }
        }
        let blacklist = self.config.process_blacklist.clone();
        let draft_busy = Rc::clone(&self.draft_busy);
        let application = cx.entity();
        let settings = self.config.network.clone();
        let system = self.config.system.clone();
        #[cfg(test)]
        let view_slot = Rc::new(RefCell::new(None));
        #[cfg(test)]
        let opened_view_slot = Rc::clone(&view_slot);
        let window_size = size(
            px(GLOBAL_SETTINGS_WINDOW_WIDTH),
            px(GLOBAL_SETTINGS_WINDOW_HEIGHT),
        );
        let window_bounds = WindowBounds::centered(window_size, cx);
        let opened = cx.open_window(
            WindowOptions {
                window_bounds: Some(window_bounds),
                titlebar: None,
                focus: true,
                show: true,
                kind: WindowKind::Normal,
                is_movable: true,
                is_resizable: false,
                is_minimizable: false,
                display_id: None,
                window_background: WindowBackgroundAppearance::Opaque,
                app_id: Some("NxTools.NetworkSettings".into()),
                window_min_size: Some(window_size),
                window_decorations: Some(WindowDecorations::Client),
                app_owns_titlebar_drag: false,
                inactive_frame_interval: None,
                icon: None,
                tabbing_identifier: None,
            },
            move |window, cx| {
                let close_application = application.clone();
                window.on_window_should_close(cx, move |_, cx| {
                    close_application.update(cx, |application, cx| {
                        if application.pending_apply.is_some() || application.draft_busy.get() {
                            false
                        } else {
                            if let Some(id) = application
                                .pending_process_window_pick
                                .as_ref()
                                .map(|pending| pending.id)
                            {
                                application.cancel_process_window_pick(id, "closed", cx);
                            }
                            application.window = None;
                            #[cfg(test)]
                            {
                                application.settings_view = None;
                            }
                            true
                        }
                    })
                });
                let view = cx.new(|cx| {
                    NetworkSettingsView::new(
                        application,
                        settings,
                        system,
                        blacklist,
                        draft_busy,
                        window,
                        cx,
                    )
                });
                #[cfg(test)]
                opened_view_slot.borrow_mut().replace(view.clone());
                cx.new(|cx| Root::new(view, window, cx))
            },
        );
        match opened {
            Ok(window) => {
                self.window = Some(window.into());
                #[cfg(test)]
                {
                    self.settings_view = view_slot.borrow_mut().take();
                }
            }
            Err(error) => self.notifications.update(cx, |notifications, cx| {
                notifications.show_error(
                    format!("network-settings-window-{}", uuid::Uuid::new_v4()),
                    "无法打开全局设置",
                    error.to_string(),
                    cx,
                );
            }),
        }
    }

    fn close_window(&mut self, cx: &mut Context<Self>) {
        if self.pending_apply.is_some() || self.draft_busy.get() {
            return;
        }
        if let Some(id) = self
            .pending_process_window_pick
            .as_ref()
            .map(|pending| pending.id)
        {
            self.cancel_process_window_pick(id, "closed", cx);
        }
        let Some(window) = self.window.take() else {
            return;
        };
        #[cfg(test)]
        {
            self.settings_view = None;
        }
        let _ = window.update(cx, |_, window, _| window.remove_window());
    }

    #[cfg(test)]
    fn apply(
        &mut self,
        settings: NetworkSettings,
        system: SystemSettings,
        correlation_id: uuid::Uuid,
        completion: Option<(WeakEntity<NetworkSettingsView>, AnyWindowHandle)>,
        cx: &mut Context<Self>,
    ) -> Result<bool, String> {
        self.apply_global(
            settings,
            system,
            self.config.process_blacklist.clone(),
            correlation_id,
            completion,
            cx,
        )
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "one atomic global draft plus its operation completion"
    )]
    fn apply_global(
        &mut self,
        settings: NetworkSettings,
        system: SystemSettings,
        blacklist: ProcessBlacklistSettings,
        correlation_id: uuid::Uuid,
        completion: Option<(WeakEntity<NetworkSettingsView>, AnyWindowHandle)>,
        cx: &mut Context<Self>,
    ) -> Result<bool, String> {
        let started_at = Instant::now();
        let mode = proxy_mode_name(settings.proxy.mode);
        info!(
            event = "network.settings.apply_started",
            %correlation_id,
            mode,
            rule_count = blacklist.entries().len(),
            "product network settings apply started"
        );
        if self.pending_apply.is_some() {
            warn!(
                event = "network.settings.apply_failed",
                %correlation_id,
                mode,
                failure_kind = "operation_in_progress",
                elapsed_ms = started_at.elapsed().as_millis(),
                "another product settings apply is still awaiting Windows confirmation"
            );
            return Err("上一项全局设置仍在等待 Windows 确认，请稍候".into());
        }

        // Persist first. Until the canonical AppConfig write succeeds every newly starting
        // operation continues to snapshot the old policy, so a rejected save can never leak into
        // an active browser, scheduled, clipboard, quick-launch, or download operation.
        let next_config = match AppConfig::update_global_settings_with_blacklist(
            &self.config_path,
            settings.clone(),
            system.clone(),
            blacklist,
        ) {
            Ok(config) => config,
            Err(error) => {
                let native_error_code = error
                    .chain()
                    .find_map(|cause| cause.downcast_ref::<std::io::Error>())
                    .and_then(std::io::Error::raw_os_error);
                warn!(
                    event = "network.settings.apply_failed",
                    %correlation_id,
                    mode,
                    failure_kind = "persistence",
                    native_error_code,
                    elapsed_ms = started_at.elapsed().as_millis(),
                    "product network settings could not be persisted"
                );
                return Err(
                    "无法保存全局设置。请检查配置文件及目录权限后重试；草稿已保留。".into(),
                );
            }
        };
        let font_changed = self.config.system.ui_font_family != system.ui_font_family;
        info!(event = "global.settings.persisted", %correlation_id, rule_count = next_config.process_blacklist.entries().len(), elapsed_ms = started_at.elapsed().as_millis(), "global settings durably saved");
        let shell_changed = self.config.system.start_on_login_silently
            != system.start_on_login_silently
            || self.config.system.close_to_tray != system.close_to_tray
            || self.system_settings.borrow().start_on_login_silently
                != system.start_on_login_silently
            || self.system_settings.borrow().close_to_tray != system.close_to_tray;
        let next_theme = system.theme;
        cx.defer(move |cx| crate::theme::set_theme_preference(next_theme, cx));
        let next_font = system.ui_font_family.clone();
        cx.defer(move |cx| {
            let font_available = crate::theme::set_ui_font_family(&next_font, cx);
            info!(event = "global.settings.presentation.applied", %correlation_id, font_changed, font_available,
                elapsed_ms = started_at.elapsed().as_millis(), "saved appearance applied to existing windows");
        });
        let notification_center = self.notifications.clone();
        let notification_position = system.notification_position;
        let notification_duration = system.notification_duration_seconds;
        cx.defer(move |cx| {
            notification_center.update(cx, |center, cx| {
                center.set_position(notification_position, cx);
                center.set_duration_seconds(notification_duration, cx);
            })
        });
        self.config = next_config.clone();
        self.publish_live_blacklist(correlation_id, cx);
        if let Err(error) = self.policy.update(settings) {
            warn!(
                event = "network.settings.apply_failed",
                %correlation_id,
                mode,
                failure_kind = "runtime_policy",
                elapsed_ms = started_at.elapsed().as_millis(),
                "persisted product network settings require a process restart"
            );
            self.config = next_config;
            return Err(format!(
                "设置已保存，但当前进程无法应用；重启 NxTools 后生效：{error}"
            ));
        }

        self.config = next_config;
        if shell_changed && let Some(platform) = self.platform.as_ref() {
            let command_id = match platform.send_command_correlated(
                PlatformCommand::ConfigureDesktopShell(ConfigureDesktopShellCommand {
                    start_on_login_silently: system.start_on_login_silently,
                    tray_icon_visible: system.close_to_tray || system.start_on_login_silently,
                }),
                correlation_id,
            ) {
                Ok(command_id) => command_id,
                Err(error) => {
                    self.fail_close_live_shell_policy();
                    warn!(
                        event = "network.settings.apply_failed",
                        %correlation_id,
                        mode,
                        failure_kind = "platform_enqueue",
                        elapsed_ms = started_at.elapsed().as_millis(),
                        "persisted product shell settings could not be queued"
                    );
                    return Err(format!(
                        "设置已保存，但 Windows 系统设置暂时无法应用：{error}"
                    ));
                }
            };
            if let Some((view, source_window)) = completion {
                self.pending_apply = Some(PendingGlobalSettingsApply {
                    command_id,
                    correlation_id,
                    mode,
                    started_at,
                    desired_system: system,
                    view: Some(view),
                    source_window: Some(source_window),
                    user_operation: true,
                });
                Self::schedule_apply_timeout(command_id, cx);
                return Ok(false);
            }
        }
        *self.system_settings.borrow_mut() = system;
        self.finish_apply_success(correlation_id, mode, started_at, cx);
        Ok(true)
    }

    fn finish_apply_success(
        &mut self,
        correlation_id: uuid::Uuid,
        mode: &'static str,
        started_at: Instant,
        cx: &mut Context<Self>,
    ) {
        self.draft_busy.set(false);
        self.window = None;
        #[cfg(test)]
        {
            self.settings_view = None;
        }
        self.notifications.update(cx, |notifications, cx| {
            notifications.show_success(
                format!("network-settings-saved-{correlation_id}"),
                "全局设置已保存",
                "应用规则已更新；后续联网操作将使用保存的代理策略。",
                cx,
            );
        });
        info!(
            event = "network.settings.apply_succeeded",
            %correlation_id,
            mode,
            elapsed_ms = started_at.elapsed().as_millis(),
            "product network settings applied"
        );
    }

    pub(crate) fn handle_platform_notification(
        &mut self,
        notification: PlatformNotification,
        cx: &mut Context<Self>,
    ) {
        if self.handle_process_window_pick(&notification, cx) {
            return;
        }
        if self.handle_data_directory_notification(&notification, cx) {
            return;
        }
        match &notification {
            PlatformNotification::CommandResult {
                correlation_id,
                result,
            } if self
                .pending_proxy_snapshot
                .as_ref()
                .is_some_and(|pending| pending.id == *correlation_id) =>
            {
                let snapshot = result
                    .success
                    .then(|| {
                        result
                            .data
                            .as_deref()
                            .filter(|data| data.len() <= 8192)
                            .and_then(|data| serde_json::from_str(data).ok())
                    })
                    .flatten();
                self.complete_system_proxy(*correlation_id, snapshot, cx);
                return;
            }
            PlatformNotification::Disconnected { .. } => {
                if let Some(id) = self
                    .pending_proxy_snapshot
                    .as_ref()
                    .map(|pending| pending.id)
                {
                    self.complete_system_proxy(id, None, cx);
                }
            }
            _ => {}
        }
        match notification {
            PlatformNotification::Connected => {
                let settings = self.config.system.clone();
                let command =
                    PlatformCommand::ConfigureDesktopShell(ConfigureDesktopShellCommand {
                        start_on_login_silently: settings.start_on_login_silently,
                        tray_icon_visible: settings.close_to_tray
                            || settings.start_on_login_silently,
                    });
                if let Some(pending) = self.pending_apply.as_mut() {
                    let queued = self
                        .platform
                        .as_ref()
                        .expect("connected platform")
                        .send_command_correlated(command, pending.correlation_id);
                    match queued {
                        Ok(command_id) => {
                            pending.command_id = command_id;
                            Self::schedule_apply_timeout(command_id, cx);
                        }
                        Err(_) => {
                            let pending = self
                                .pending_apply
                                .take()
                                .expect("pending apply checked above");
                            self.fail_close_live_shell_policy();
                            self.finish_platform_apply_failure(
                                pending,
                                "platform_enqueue",
                                None,
                                None,
                                cx,
                            );
                        }
                    }
                } else if let Some(platform) = self.platform.as_ref() {
                    let correlation_id = uuid::Uuid::new_v4();
                    let started_at = Instant::now();
                    if let Ok(command_id) =
                        platform.send_command_correlated(command, correlation_id)
                    {
                        info!(
                            event = "product.shell.reconcile_started",
                            %correlation_id,
                            "persisted product shell settings reconciliation started"
                        );
                        self.pending_apply = Some(PendingGlobalSettingsApply {
                            command_id,
                            correlation_id,
                            mode: proxy_mode_name(self.config.network.proxy.mode),
                            started_at,
                            desired_system: settings,
                            view: None,
                            source_window: None,
                            user_operation: false,
                        });
                        Self::schedule_apply_timeout(command_id, cx);
                    } else {
                        self.fail_close_live_shell_policy();
                        warn!(
                            event = "product.shell.reconcile_failed",
                            %correlation_id,
                            failure_kind = "platform_enqueue",
                            elapsed_ms = started_at.elapsed().as_millis(),
                            "persisted product shell settings could not be queued after reconnect"
                        );
                        if self.hidden_startup {
                            cx.quit();
                        }
                    }
                }
            }
            PlatformNotification::Disconnected {
                generation,
                pending_command_ids,
            } => {
                if self
                    .pending_apply
                    .as_ref()
                    .is_some_and(|pending| pending_command_ids.contains(&pending.command_id))
                {
                    let pending = self
                        .pending_apply
                        .take()
                        .expect("pending apply checked above");
                    self.fail_close_live_shell_policy();
                    warn!(
                        event = "product.shell.reconcile_failed",
                        generation,
                        correlation_id = %pending.correlation_id,
                        failure_kind = "platform_disconnected",
                        elapsed_ms = pending.started_at.elapsed().as_millis(),
                        "platform disconnected before Windows confirmed shell settings"
                    );
                    self.finish_platform_apply_failure(
                        pending,
                        "platform_disconnected",
                        None,
                        None,
                        cx,
                    );
                }
            }
            PlatformNotification::CommandResult {
                correlation_id,
                result,
            } if self
                .pending_apply
                .as_ref()
                .is_some_and(|pending| pending.command_id == correlation_id) =>
            {
                let pending = self
                    .pending_apply
                    .take()
                    .expect("pending apply checked above");
                if result.success {
                    *self.system_settings.borrow_mut() = pending.desired_system;
                    if pending.user_operation {
                        if let Some(source_window) = pending.source_window {
                            let _ = source_window.update(cx, |_, window, _| window.remove_window());
                        }
                        self.finish_apply_success(
                            pending.correlation_id,
                            pending.mode,
                            pending.started_at,
                            cx,
                        );
                    } else {
                        info!(
                            event = "product.shell.reconcile_completed",
                            correlation_id = %pending.correlation_id,
                            elapsed_ms = pending.started_at.elapsed().as_millis(),
                            "persisted product shell settings were reconciled"
                        );
                    }
                } else {
                    self.fail_close_live_shell_policy();
                    self.finish_platform_apply_failure(
                        pending,
                        "platform_result",
                        result.native_error_code,
                        result.data.as_deref(),
                        cx,
                    );
                }
            }
            _ => {}
        }
    }

    fn timeout_pending_apply(&mut self, command_id: uuid::Uuid, cx: &mut Context<Self>) {
        if self
            .pending_apply
            .as_ref()
            .is_none_or(|pending| pending.command_id != command_id)
        {
            return;
        }
        let pending = self
            .pending_apply
            .take()
            .expect("pending apply checked above");
        self.fail_close_live_shell_policy();
        self.finish_platform_apply_failure(pending, "platform_timeout", None, None, cx);
    }

    fn fail_close_live_shell_policy(&self) {
        self.system_settings.borrow_mut().close_to_tray = false;
    }

    fn finish_platform_apply_failure(
        &mut self,
        pending: PendingGlobalSettingsApply,
        failure_kind: &'static str,
        native_error_code: Option<i32>,
        failure_stage: Option<&str>,
        cx: &mut Context<Self>,
    ) {
        // Platform result data is untrusted diagnostic text; only fixed stages are log-safe.
        let failure_stage = match failure_stage {
            Some(
                stage @ ("desktop_executable_unavailable"
                | "startup_registration_failed"
                | "tray_icon_failed"
                | "windows_shell_enqueue_failed"
                | "windows_shell_apply_failed"),
            ) => Some(stage),
            _ => None,
        };
        if pending.user_operation {
            warn!(
                event = "network.settings.apply_failed",
                correlation_id = %pending.correlation_id,
                mode = pending.mode,
                failure_kind,
                failure_stage = failure_stage.unwrap_or("unknown"),
                native_error_code,
                elapsed_ms = pending.started_at.elapsed().as_millis(),
                "Windows did not confirm persisted product shell settings"
            );
        } else {
            warn!(
                event = "product.shell.reconcile_failed",
                correlation_id = %pending.correlation_id,
                failure_kind,
                failure_stage = failure_stage.unwrap_or("unknown"),
                native_error_code,
                elapsed_ms = pending.started_at.elapsed().as_millis(),
                "Windows did not confirm product shell settings reconciliation"
            );
        }
        let center = self.notifications.clone();
        let message = global_shell_failure_message(failure_kind, failure_stage).to_owned();
        let operation_key = format!("network-settings-saved-{}", pending.correlation_id);
        cx.defer(move |cx| {
            center.update(cx, |center, cx| {
                center.show_error(operation_key, "全局设置未能应用", message, cx)
            })
        });
        self.draft_busy.set(false);
        if let Some(view) = pending.view {
            let _ = view.update(cx, |view, cx| {
                view.saving = false;
                view.error = None;
                cx.notify();
            });
        }
        if !pending.user_operation && self.hidden_startup {
            cx.quit();
        }
    }

    fn schedule_apply_timeout(command_id: uuid::Uuid, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(std::time::Duration::from_secs(5))
                .await;
            let _ = this.update(cx, |application, cx| {
                application.timeout_pending_apply(command_id, cx);
            });
        })
        .detach();
    }
}

struct NetworkSettingsView {
    theme: Theme,
    application: Entity<NetworkSettingsApplication>,
    mode: NetworkProxyMode,
    start_on_login_silently: bool,
    close_to_tray: bool,
    theme_preference: nxtools_core::ThemePreference,
    ui_font: font_settings::FontSettingsDraft,
    notification_position: nxtools_core::NotificationPosition,
    notification_duration: Entity<InputState>,
    _duration_step: gpui_kit::Subscription,
    host: Entity<InputState>,
    port: Entity<InputState>,
    proxy_snapshot: Option<nxtools_protocol::system_proxy::SystemProxySnapshot>,
    proxy_rows: Vec<(String, Entity<InputState>, Entity<InputState>)>,
    proxy_generation: u64,
    proxy_loading: bool,
    proxy_read_failed: bool,
    tab: GlobalSettingsTab,
    rules: Vec<ProcessBlacklistRule>,
    suppress_in_fullscreen: bool,
    process_input: Entity<InputState>,
    editing_rule: Option<usize>,
    process_window_pick: Option<uuid::Uuid>,
    new_disable_clipboard: bool,
    new_disable_quick_launch: bool,
    saving: bool,
    draft_busy: Rc<std::cell::Cell<bool>>,
    error: Option<String>,
}

impl NetworkSettingsView {
    #[allow(
        clippy::too_many_arguments,
        reason = "global draft initialization and shared busy close guard"
    )]
    fn new(
        application: Entity<NetworkSettingsApplication>,
        settings: NetworkSettings,
        system: SystemSettings,
        blacklist: ProcessBlacklistSettings,
        draft_busy: Rc<std::cell::Cell<bool>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let host = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(settings.proxy.host)
                .placeholder("proxy.example.com")
        });
        let port = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(settings.proxy.port.to_string())
                .placeholder("8080")
        });
        let process_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("应用进程名，例如 notepad.exe"));
        let notification_duration = cx.new(|cx| {
            let mut state = InputState::new(window, cx)
                .default_value(system.notification_duration_seconds.to_string());
            state.set_step(None, window, cx);
            state
        });
        let duration_step = cx.subscribe_in(
            &notification_duration,
            window,
            |_, _, event: &NumberInputEvent, window, cx| {
                let NumberInputEvent::Step(action) = *event;
                cx.defer_in(window, move |this, window, cx| {
                    if this.saving {
                        return;
                    }
                    let current = this
                        .notification_duration
                        .read(cx)
                        .value()
                        .parse::<u32>()
                        .unwrap_or(nxtools_core::default_notification_duration());
                    let next = match action {
                        StepAction::Increment => current.saturating_add(1).min(60),
                        StepAction::Decrement => current.saturating_sub(1).max(1),
                    };
                    this.notification_duration.update(cx, |input, cx| {
                        input.set_value(next.to_string(), window, cx)
                    });
                });
            },
        );
        if settings.proxy.mode == NetworkProxyMode::System {
            let application = application.clone();
            let view = cx.entity().downgrade();
            cx.defer(move |cx| {
                application.update(cx, |owner, cx| owner.request_system_proxy(view, 0, cx))
            });
        }
        let ui_font = font_settings::FontSettingsDraft::new(&system.ui_font_family, window, cx);
        Self {
            theme: Theme::zed_dark(),
            application,
            mode: settings.proxy.mode,
            start_on_login_silently: system.start_on_login_silently,
            close_to_tray: system.close_to_tray,
            theme_preference: system.theme,
            ui_font,
            notification_position: system.notification_position,
            notification_duration,
            _duration_step: duration_step,
            host,
            port,
            proxy_snapshot: None,
            proxy_rows: Vec::new(),
            proxy_generation: 0,
            proxy_loading: settings.proxy.mode == NetworkProxyMode::System,
            proxy_read_failed: false,
            tab: GlobalSettingsTab::General,
            rules: blacklist.entries(),
            suppress_in_fullscreen: blacklist.suppress_in_fullscreen.unwrap_or(true),
            process_input,
            editing_rule: None,
            process_window_pick: None,
            new_disable_clipboard: true,
            new_disable_quick_launch: false,
            saving: false,
            draft_busy,
            error: None,
        }
    }

    fn save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.saving {
            return;
        }
        self.observe_picker_page(0, cx);
        let Ok(notification_duration_seconds) =
            self.notification_duration.read(cx).value().parse::<u32>()
        else {
            self.error = Some("通知停留时间请输入 1–60 秒".into());
            cx.notify();
            return;
        };
        if !(1..=60).contains(&notification_duration_seconds) {
            self.error = Some("通知停留时间请输入 1–60 秒".into());
            cx.notify();
            return;
        }
        if !self.process_input.read(cx).value().trim().is_empty() || self.editing_rule.is_some() {
            self.error = Some("请先添加或更新正在编辑的应用规则，再应用设置。".into());
            cx.notify();
            return;
        }
        let ui_font_family = match self.ui_font.validated_family(cx) {
            Ok(family) => family,
            Err(message) => {
                self.error = Some(message);
                cx.notify();
                return;
            }
        };
        let correlation_id = uuid::Uuid::new_v4();
        let started_at = Instant::now();
        let settings = match parse_network_settings(
            self.mode,
            &self.host.read(cx).value(),
            &self.port.read(cx).value(),
        ) {
            Ok(settings) => settings,
            Err(error) => {
                let mode = proxy_mode_name(self.mode);
                info!(
                    event = "network.settings.apply_started",
                    %correlation_id,
                    mode,
                    "product network settings apply started"
                );
                warn!(
                    event = "network.settings.apply_failed",
                    %correlation_id,
                    mode,
                    failure_kind = "validation",
                    elapsed_ms = started_at.elapsed().as_millis(),
                    "product network settings validation failed"
                );
                self.error = Some(error);
                cx.notify();
                return;
            }
        };
        let mut blacklist = ProcessBlacklistSettings::from_entries(self.rules.clone());
        blacklist.suppress_in_fullscreen = Some(self.suppress_in_fullscreen);
        self.saving = true;
        self.draft_busy.set(true);
        self.error = None;
        let application = self.application.clone();
        let system = SystemSettings {
            start_on_login_silently: self.start_on_login_silently,
            close_to_tray: self.close_to_tray,
            theme: self.theme_preference,
            ui_font_family,
            notification_position: self.notification_position,
            notification_duration_seconds,
        };
        let view = cx.entity().downgrade();
        let source_window = window.window_handle();
        cx.defer(move |cx| {
            let completion = (view.clone(), source_window);
            let result = application.update(cx, |application, cx| {
                application.apply_global(
                    settings,
                    system,
                    blacklist,
                    correlation_id,
                    Some(completion),
                    cx,
                )
            });
            match result {
                Ok(true) => {
                    let _ = source_window.update(cx, |_, window, _| window.remove_window());
                }
                Ok(false) => {}
                Err(error) => {
                    let _ = view.update(cx, |view, cx| {
                        view.saving = false;
                        view.draft_busy.set(false);
                        view.error = None;
                        let application = view.application.clone();
                        cx.defer(move |cx| {
                            let center = application.read(cx).notifications.clone();
                            center.update(cx, |center, cx| {
                                center.show_error(
                                    format!("network-settings-saved-{correlation_id}"),
                                    "全局设置未能保存或应用",
                                    error,
                                    cx,
                                )
                            });
                        });
                        cx.notify();
                    });
                }
            }
        });
        cx.notify();
    }
}

impl NetworkSettingsView {
    fn render_setting_control(
        &mut self,
        slot: usize,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> gpui_kit::AnyElement {
        match slot {
            0 => Switch::new("system-start-on-login")
                .checked(self.start_on_login_silently)
                .disabled(self.saving)
                .accessibility_label("开机静默启动")
                .on_change(cx.listener(|this, requested: &bool, _, cx| {
                    if !this.saving {
                        this.start_on_login_silently = *requested;
                        cx.notify();
                    }
                }))
                .into_any_element(),
            1 => Switch::new("system-close-to-tray")
                .checked(self.close_to_tray)
                .disabled(self.saving)
                .accessibility_label("关闭到托盘")
                .on_change(cx.listener(|this, requested: &bool, _, cx| {
                    if !this.saving {
                        this.close_to_tray = *requested;
                        cx.notify();
                    }
                }))
                .into_any_element(),
            2 => {
                let selected = match self.mode {
                    NetworkProxyMode::Direct => "network-settings-proxy-mode-direct",
                    NetworkProxyMode::System => "network-settings-proxy-mode-system",
                    NetworkProxyMode::ManualHttp => "network-settings-proxy-mode-manual",
                };
                crate::ui::components::action::input_segmented_control(
                    self.theme,
                    "network-settings-proxy-mode",
                    selected,
                    [
                        (
                            "network-settings-proxy-mode-direct",
                            NetworkProxyMode::Direct,
                            "直连",
                        ),
                        (
                            "network-settings-proxy-mode-system",
                            NetworkProxyMode::System,
                            "系统代理",
                        ),
                        (
                            "network-settings-proxy-mode-manual",
                            NetworkProxyMode::ManualHttp,
                            "手动 HTTP",
                        ),
                    ]
                    .into_iter()
                    .map(|(id, mode, label)| {
                        SegmentedOption::new(
                            id,
                            label,
                            cx.listener(move |this, _, _, cx| {
                                if !this.saving {
                                    this.select_proxy_mode(mode, cx);
                                }
                            }),
                        )
                    })
                    .collect(),
                    cx,
                )
                .into_any_element()
            }
            3 => {
                if self.mode == NetworkProxyMode::System {
                    return self.render_system_proxy();
                }
                let disabled = self.mode != NetworkProxyMode::ManualHttp || self.saving;
                Form::vertical()
                    .small()
                    .columns(4)
                    .child(
                        Field::new().col_span(3).label("代理地址").child(
                            div()
                                .debug_selector(|| "network-settings-proxy-host-field".into())
                                .w_full()
                                .child(
                                    Input::new(&self.host)
                                        .aria_label("代理地址")
                                        .disabled(disabled),
                                ),
                        ),
                    )
                    .child(
                        Field::new().label("端口").child(
                            div()
                                .debug_selector(|| "network-settings-proxy-port-field".into())
                                .w_full()
                                .child(
                                    Input::new(&self.port)
                                        .aria_label("代理端口")
                                        .disabled(disabled),
                                ),
                        ),
                    )
                    .into_any_element()
            }
            4 => self.render_blacklist_table(window, cx),
            5 => self.render_blacklist_editor(cx),
            6 => self.render_theme_chooser(cx),
            7 => {
                let view = cx.entity().downgrade();
                let selected = self.notification_position;
                action_button("global-notification-position", selected.label())
                    .outline()
                    .with_size(gpui_kit::component::Size::Medium)
                    .disabled(self.saving)
                    .dropdown_menu(move |menu, window, _| {
                        nxtools_core::NotificationPosition::ALL.into_iter().fold(
                            menu.min_w(
                                crate::ui::metrics::ordinary_rem(160.0)
                                    .to_pixels(window.rem_size()),
                            ),
                            |menu, position| {
                                let view = view.clone();
                                menu.item(
                                    PopupMenuItem::new(position.label())
                                        .checked(position == selected)
                                        .on_click(move |_, _, cx| {
                                            let view = view.clone();
                                            cx.defer(move |cx| {
                                                let _ = view.update(cx, |this, cx| {
                                                    if !this.saving {
                                                        this.notification_position = position;
                                                        cx.notify();
                                                    }
                                                });
                                            });
                                        }),
                                )
                            },
                        )
                    })
                    .into_any_element()
            }
            8 => div()
                .id("global-notification-duration")
                .debug_selector(|| "global-notification-duration".into())
                .w(crate::ui::metrics::ordinary_rem(168.0))
                .child(
                    NumberInput::new(&self.notification_duration)
                        .w_full()
                        .suffix("秒")
                        .disabled(self.saving),
                )
                .into_any_element(),
            9 => self.render_data_directory(cx),
            10 => self.ui_font.render(self.saving),
            _ => div().into_any_element(),
        }
    }
}

impl Render for NetworkSettingsView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        crate::theme::observe_theme(_window, cx);
        self.theme = Theme::active(cx);
        let theme = self.theme;
        let owner = cx.entity().downgrade();
        let ordinary = |slot: usize, title: &'static str, help: &'static str| {
            let owner = owner.clone();
            SettingItem::render(move |options, window, app| {
                let control = owner
                    .update(app, |view, cx| {
                        view.observe_picker_page(options.page_ix(), cx);
                        view.render_setting_control(slot, window, cx)
                    })
                    .unwrap_or_else(|_| div().into_any_element());
                crate::ui::components::settings::aligned_setting_row(
                    format!("global-setting-{slot}"),
                    title,
                    help,
                    format!("global-setting-{slot}-help"),
                    crate::ui::metrics::ordinary_rem(332.0),
                    control,
                )
            })
            .keywords([title, help])
        };
        let complex = |slot: usize, keywords: &'static str| {
            let owner = owner.clone();
            SettingItem::render(move |options, window, app| {
                let control = owner
                    .update(app, |view, cx| {
                        view.observe_picker_page(options.page_ix(), cx);
                        view.render_setting_control(slot, window, cx)
                    })
                    .unwrap_or_else(|_| div().into_any_element());
                div()
                    .w_full()
                    .min_w_0()
                    .when(slot != 3, |body| body.px_3())
                    .child(control)
            })
            .keywords([keywords])
        };
        let pages = vec![
            SettingPage::new("常规").resettable(false).group(SettingGroup::new().items([
                ordinary(0, "开机静默启动", "登录 Windows 后在后台启动，不主动弹出主窗口"),
                ordinary(1, "关闭到托盘", "关闭主窗口时继续在后台运行；托盘左键恢复，右键可退出"),
                ordinary(7, "通知位置", "通知相对操作所在屏幕的工作区显示在左上、上中、右上、左下、下中或右下；保存后生效"),
                ordinary(8, "通知停留时间", "1–60 秒，默认 6 秒；仅影响新通知。鼠标悬停任一通知时暂停整个通知列表的剩余时间，移开后继续倒计时"),
            ])),
            SettingPage::new("网络").resettable(false).group(SettingGroup::new().items([
                ordinary(2, "代理模式", "下载、剪贴板和速启中的实际联网操作共享；本地 IPC 与索引不使用"),
                complex(3, "代理服务器 地址 端口 HTTP"),
                SettingItem::render(|_, _, _| div().debug_selector(|| "network-settings-policy-note".into())
                    .text_sm().child("仅支持 HTTP 代理；修改只影响新开始或重新继续的操作。选择手动 HTTP 后填写地址和端口。\n不支持 PAC 或代理密码；敏感凭据不会写入配置。"))
                    .keywords(["支持 HTTP PAC 密码"]),
            ])),
            SettingPage::new("应用黑名单").resettable(false).group(SettingGroup::new().items([
                complex(5, "添加应用 编辑应用 进程名 更新"),
                complex(4, "应用黑名单 进程 禁用剪贴板 禁用速启 应用规则 全屏 独占全屏 无边框全屏 呼出"),
            ])),
            SettingPage::new("外观").resettable(false).group(SettingGroup::new().items([
                complex(6, "主题 浅色 深色 Latte Frappe Catppuccin Gruvbox Solarized 跟随系统"),
                ordinary(10, "界面字体", "搜索并选择已安装的中文或英文字体；应用后生效。不支持的字形使用系统回退，等宽诊断字体不变。"),
            ])),
            SettingPage::new("数据").resettable(false).group(SettingGroup::new().items([
                complex(9, "软件数据目录 存储位置 迁移 重启"),
            ])),
        ];
        let application = self.application.clone();
        let titlebar = client_titlebar(
            theme,
            "network-settings-titlebar",
            "network-settings-close",
            "全局设置",
            move |_, _, cx| {
                let application = application.clone();
                cx.defer(move |cx| {
                    application.update(cx, |application, cx| application.close_window(cx));
                });
            },
        );
        let page_seed = [
            GlobalSettingsTab::General,
            GlobalSettingsTab::Network,
            GlobalSettingsTab::Blacklist,
        ]
        .iter()
        .position(|tab| *tab == self.tab)
        .unwrap_or(0);
        let settings = Settings::new("global-official-settings")
            .small()
            .sidebar_width(px(128.0))
            .sidebar_size_range(px(128.0)..px(200.0))
            .with_group_variant(GroupBoxVariant::Normal)
            .default_selected_index(SelectIndex {
                page_ix: page_seed,
                ..Default::default()
            })
            .pages(pages);
        let content =
            div()
                .debug_selector(|| "global-settings-workspace".into())
                .size_full()
                .min_h_0()
                .flex()
                .flex_col()
                .child(div().flex_1().min_h_0().relative().child(settings).when(
                    self.saving,
                    |area| {
                        area.child(
                            div()
                                .id("global-settings-busy-occluder")
                                .absolute()
                                .inset_0()
                                .occlude()
                                .on_mouse_down(gpui_kit::MouseButton::Left, |_, _, cx| {
                                    cx.stop_propagation()
                                })
                                .on_click(|_, _, cx| cx.stop_propagation()),
                        )
                    },
                ))
                .when_some(self.error.clone(), |content, error| {
                    content.child(
                        div()
                            .id("global-settings-error-scroll")
                            .max_h(px(80.0))
                            .overflow_y_scroll()
                            .child(inline_error(theme, "network-settings-inline-error", error)),
                    )
                })
                .into_any_element();
        let footer_application = self.application.clone();
        let footer_actions = div()
            .flex()
            .items_center()
            .gap_2()
            .child(
                div()
                    .debug_selector(|| "network-settings-cancel".into())
                    .child(
                        action_button("network-settings-cancel-button", "取消")
                            .disabled(self.saving)
                            .on_click(move |_, _, cx| {
                                let application = footer_application.clone();
                                cx.defer(move |cx| {
                                    application
                                        .update(cx, |application, cx| application.close_window(cx));
                                });
                            }),
                    ),
            )
            .child(
                div()
                    .debug_selector(|| "network-settings-save".into())
                    .child(
                        primary_action_button(
                            "network-settings-save-button",
                            "应用全局设置",
                            if self.saving {
                                "正在应用…"
                            } else {
                                "应用"
                            },
                            theme,
                            self.saving,
                            cx,
                        )
                        .on_click(cx.listener(|this, _, window, cx| this.save(window, cx))),
                    ),
            )
            .into_any_element();
        settings_shell(
            theme,
            SettingsShellIds {
                root: "network-settings-shell",
                content: "network-settings-content",
                footer: "network-settings-footer",
            },
            titlebar,
            content,
            dialog_footer(footer_actions),
        )
    }
}

fn parse_network_settings(
    mode: NetworkProxyMode,
    host: &str,
    port: &str,
) -> Result<NetworkSettings, String> {
    let port = if mode == NetworkProxyMode::ManualHttp {
        port.trim()
            .parse::<u16>()
            .map_err(|_| "手动代理端口必须在 1–65535 之间".to_owned())?
    } else {
        8080
    };
    let mut settings = NetworkSettings {
        proxy: NetworkProxySettings {
            mode,
            host: host.into(),
            port,
        },
    };
    settings.normalize();
    settings.validate()?;
    Ok(settings)
}

#[cfg(test)]
mod tests {
    use gpui_kit::{AppContext, Modifiers, TestAppContext, VisualTestContext};

    use super::*;

    pub(super) fn application_fixture(
        cx: &mut TestAppContext,
        label: &str,
    ) -> (Entity<NetworkSettingsApplication>, PathBuf, NetworkPolicy) {
        let directory = std::env::temp_dir().join(format!(
            "nxtools-network-settings-{label}-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&directory).expect("create settings test directory");
        let config = AppConfig::default();
        let config_path = directory.join("config.json");
        config.save_to(&config_path).expect("save config fixture");
        let policy = NetworkPolicy::new(config.network.clone()).expect("network policy");
        let notifications = cx.update(|cx| cx.new(|_| NotificationCenter::default()));
        let application = cx.update(|cx| {
            cx.new(|_| {
                NetworkSettingsApplication::new(
                    policy.clone(),
                    config,
                    config_path,
                    notifications,
                    None,
                    Rc::new(RefCell::new(SystemSettings {
                        start_on_login_silently: false,
                        close_to_tray: false,
                        theme: nxtools_core::ThemePreference::Dark,
                        ui_font_family: nxtools_core::default_ui_font_family(),
                        notification_position: Default::default(),
                        notification_duration_seconds: nxtools_core::default_notification_duration(
                        ),
                    })),
                    false,
                )
            })
        });
        (application, directory, policy)
    }

    #[gpui_kit::test]
    fn settings_navigation_rows_fill_one_continuous_equal_width_rail(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        let (application, directory, _) = application_fixture(cx, "navigation-continuity");
        cx.update(|cx| application.update(cx, |owner, cx| owner.open_window(cx)));
        cx.run_until_parked();
        let handle = cx.read(|cx| application.read(cx).window.expect("settings window"));
        let mut surface = VisualTestContext::from_window(handle, cx);
        surface.simulate_resize(size(px(720.0), px(480.0)));
        for active in 0..3 {
            surface.refresh().expect("actual official navigation");
            super::kit_migration_tests::select_page(&mut surface, active);
            let nodes = surface.update(|window, _| gpui_kit::base::test_support::snapshots(window));
            let rows = (0..3)
                .map(|index| {
                    nodes
                        .iter()
                        .find(|node| node.path().last() == Some(&format!("0-{index}").into()))
                        .expect("all original topics")
                        .bounds()
                })
                .collect::<Vec<_>>();
            assert!((rows[0].size.width - rows[1].size.width).abs() <= px(1.0));
            assert!((rows[1].size.width - rows[2].size.width).abs() <= px(1.0));
            assert_eq!(rows[0].left(), rows[1].left());
            assert_eq!(rows[1].left(), rows[2].left());
            assert!(
                rows.iter()
                    .all(|row| row.size.width > px(0.0) && row.right() <= px(720.0))
            );
        }
        cx.update(|cx| application.update(cx, |owner, cx| owner.close_window(cx)));
        let _ = std::fs::remove_dir_all(directory);
    }

    #[gpui_kit::test]
    fn blacklist_table_local_actions_cancel_failure_and_single_save(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        let (application, directory, _) = application_fixture(cx, "blacklist-draft");
        let path = directory.join("config.json");
        let before = std::fs::read(&path).expect("before");
        cx.update(|cx| application.update(cx, |owner, cx| owner.open_window(cx)));
        cx.run_until_parked();
        let (handle, view) = cx.read(|cx| {
            let owner = application.read(cx);
            (
                owner.window.expect("window"),
                owner.settings_view.clone().expect("view"),
            )
        });
        let mut window = VisualTestContext::from_window(handle, cx);
        window.run_until_parked();
        window.refresh().expect("render nav");
        super::kit_migration_tests::select_page(&mut window, 2);
        window.update(|window, cx| {
            view.update(cx, |view, cx| {
                view.process_input.update(cx, |input, cx| {
                    input.set_value("C:\\Apps\\Editor.EXE", window, cx)
                });
            })
        });
        window.refresh().expect("render add");
        let fullscreen = window
            .debug_bounds("global-setting-9")
            .expect("global fullscreen summon setting row");
        assert!(fullscreen.size.height >= px(24.0));
        assert!(window.read(|cx| view.read(cx).suppress_in_fullscreen));
        let switch = super::kit_migration_tests::control_bounds(
            &mut window,
            "global-fullscreen-suppression",
        );
        window.simulate_click(switch.center(), Modifiers::default());
        window.run_until_parked();
        assert!(!window.read(|cx| view.read(cx).suppress_in_fullscreen));
        assert_eq!(
            std::fs::read(&path).expect("fullscreen draft has no save"),
            before
        );
        super::kit_migration_tests::focus_control(&mut window, "global-fullscreen-suppression");
        window.simulate_keystrokes("space");
        window.simulate_event(gpui_kit::KeyUpEvent {
            keystroke: gpui_kit::Keystroke::parse("space").expect("Space"),
        });
        window.run_until_parked();
        assert!(window.read(|cx| view.read(cx).suppress_in_fullscreen));
        let add = window
            .debug_bounds("global-blacklist-add-action")
            .expect("add");
        let count = window.read(|cx| view.read(cx).rules.len());
        window.simulate_click(add.center(), Modifiers::default());
        window.run_until_parked();
        assert_eq!(window.read(|cx| view.read(cx).rules.len()), count + 1);
        window.update(|window, cx| {
            view.update(cx, |view, cx| {
                view.process_input
                    .update(cx, |input, cx| input.set_value("editor.exe", window, cx));
                view.new_disable_quick_launch = true;
                view.add_rule(window, cx);
                assert_eq!(
                    view.rules
                        .iter()
                        .filter(|rule| rule.process == "editor")
                        .count(),
                    1
                );
                let editor_index = view
                    .rules
                    .iter()
                    .position(|rule| rule.process == "editor")
                    .expect("editor row");
                view.editing_rule = Some(editor_index);
                view.new_disable_clipboard = false;
                view.new_disable_quick_launch = true;
                view.process_input
                    .update(cx, |input, cx| input.set_value("EDITOR.EXE", window, cx));
                view.add_rule(window, cx);
                assert!(
                    !view
                        .rules
                        .iter()
                        .find(|rule| rule.process == "editor")
                        .expect("edited")
                        .disable_clipboard
                );
                let unchanged = view.rules.clone();
                view.saving = true;
                view.add_rule(window, cx);
                assert_eq!(view.rules, unchanged, "pending edits cannot dispatch");
                view.saving = false;
                view.add_rule(window, cx);
                assert!(view.error.is_some(), "blank input is invalid");
            })
        });
        assert_eq!(std::fs::read(&path).expect("draft has no save"), before);
        window.refresh().expect("render draft");
        assert!(
            window
                .debug_bounds("network-settings-policy-note")
                .is_none(),
            "blacklist page must not show unrelated proxy guidance"
        );
        let footer = window
            .debug_bounds("network-settings-footer")
            .expect("footer");
        let table = window
            .debug_bounds("global-blacklist-table")
            .expect("continuous table");
        assert!(table.bottom() <= footer.top());
        let new_row = window
            .debug_bounds("global-blacklist-new-row")
            .expect("new rule row");
        let add = window
            .debug_bounds("global-blacklist-add-action")
            .expect("add rule action");
        assert!(new_row.bottom() <= footer.top());
        assert!(add.right() <= new_row.right() && add.size.width > px(0.0));
        let field = window
            .debug_bounds("global-blacklist-process-field")
            .expect("application input must use an explicit 32px field shell");
        assert_eq!(field.size.height, px(32.0));
        assert_eq!(add.size.height, px(24.0));
        assert_eq!(add.center().y, field.center().y);
        assert!(field.left() >= px(128.0));
        let toggle = window
            .debug_bounds("global-blacklist-clipboard-cell-0")
            .expect("checkbox");
        assert!(toggle.right() <= table.right() && toggle.bottom() <= table.bottom());
        let label = window
            .debug_bounds("global-blacklist-label-0")
            .expect("application label");
        let app_header = window
            .debug_bounds("global-blacklist-header-app")
            .expect("application header");
        assert_eq!(
            label.left(),
            app_header.left(),
            "left-aligned application names"
        );
        let nodes = window.update(|window, _| gpui_kit::base::test_support::snapshots(window));
        for (header_id, id) in [
            (
                "global-blacklist-header-clipboard",
                "global-blacklist-clipboard-0",
            ),
            ("global-blacklist-header-quick", "global-blacklist-quick-0"),
        ] {
            let header = window.debug_bounds(header_id).expect("column header");
            let checkbox = nodes
                .iter()
                .find(|node| node.path().last() == Some(&id.into()))
                .expect("actual Checkbox");
            assert!((header.center().x - checkbox.bounds().center().x).abs() <= px(1.0));
            assert!((checkbox.bounds().center().y - label.center().y).abs() <= px(1.0));
        }
        let toggle_bounds =
            super::kit_migration_tests::control_bounds(&mut window, "global-blacklist-clipboard-0");
        window.simulate_click(toggle_bounds.center(), Modifiers::default());
        window.run_until_parked();
        assert!(!window.read(|cx| view.read(cx).rules[0].disable_clipboard));
        super::kit_migration_tests::focus_control(&mut window, "global-blacklist-clipboard-0");
        window.simulate_keystrokes("space");
        window.simulate_event(gpui_kit::KeyUpEvent {
            keystroke: gpui_kit::Keystroke::parse("space").expect("Space"),
        });
        window.run_until_parked();
        assert!(
            window.read(|cx| view.read(cx).rules[0].disable_clipboard),
            "one keyboard toggle"
        );
        window.simulate_keystrokes("tab");
        window.refresh().expect("Tab to quick launch scope action");
        window.simulate_keystrokes("space");
        window.simulate_event(gpui_kit::KeyUpEvent {
            keystroke: gpui_kit::Keystroke::parse("space").expect("Space"),
        });
        window.run_until_parked();
        assert!(
            window.read(|cx| {
                let rule = &view.read(cx).rules[0];
                rule.disable_clipboard && rule.disable_quick_launch
            }),
            "keyboard scopes are independent"
        );
        window.refresh().expect("render remove");
        let remove = window
            .debug_bounds("global-blacklist-remove-cell-0")
            .expect("remove");
        window.simulate_click(remove.center(), Modifiers::default());
        window.run_until_parked();
        assert_eq!(window.read(|cx| view.read(cx).rules.len()), count);
        std::fs::remove_file(&path).expect("force persistence failure");
        window.update(|window, cx| {
            view.update(cx, |view, cx| {
                view.save(window, cx);
                view.save(window, cx);
                assert!(view.saving);
            })
        });
        window.run_until_parked();
        window.read(|cx| {
            let view = view.read(cx);
            assert!(!view.saving);
            assert!(view.error.is_none(), "persistence failure is center-only");
            assert!(
                application
                    .read(cx)
                    .notifications
                    .read(cx)
                    .items()
                    .iter()
                    .any(|notice| notice.level == crate::notifications::NotificationLevel::Error)
            );
            assert!(view.rules.iter().any(|rule| rule.process == "editor"));
        });
        std::fs::write(&path, &before).expect("restore");
        window.refresh().expect("render cancel");
        let cancel = window
            .debug_bounds("network-settings-cancel")
            .expect("cancel");
        window.simulate_click(cancel.center(), Modifiers::default());
        window.run_until_parked();
        assert_eq!(std::fs::read(&path).expect("cancel no publish"), before);
        assert!(cx.read(|cx| application.read(cx).window.is_none()));
        cx.update(|cx| application.update(cx, |owner, cx| owner.open_window(cx)));
        cx.run_until_parked();
        let (handle, view) = cx.read(|cx| {
            let owner = application.read(cx);
            (
                owner.window.expect("reopened"),
                owner.settings_view.clone().expect("view"),
            )
        });
        let mut window = VisualTestContext::from_window(handle, cx);
        window.run_until_parked();
        window.update(|window, cx| {
            view.update(cx, |view, cx| {
                assert!(
                    view.suppress_in_fullscreen,
                    "cancel retained durable fullscreen policy"
                );
                view.suppress_in_fullscreen = false;
                view.rules.clear();
                view.save(window, cx);
                view.save(window, cx);
            })
        });
        window.run_until_parked();
        assert!(
            AppConfig::load_from(&path)
                .expect("empty persisted")
                .process_blacklist
                .entries()
                .is_empty()
        );
        assert!(
            !AppConfig::load_from(&path)
                .expect("saved fullscreen policy")
                .suppress_panel_in_fullscreen()
        );
        assert!(cx.read(|cx| application.read(cx).window.is_none()));
        let _ = std::fs::remove_dir_all(directory);
    }

    #[gpui_kit::test]
    fn global_rules_publish_both_bound_consumers_only_after_durable_success(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_kit::init);
        let (application, directory, _) = application_fixture(cx, "bound-consumers");
        let path = directory.join("config.json");
        let config = AppConfig::load_from(&path).expect("fixture config");
        let notifications = cx.update(|cx| cx.new(|_| NotificationCenter::default()));
        let worker = crate::clipboard_worker::ClipboardCaptureWorkerHandle::spawn(
            directory.join("clipboard.db"),
            directory.join("images"),
            nxtools_clipboard::ClipboardRetentionPolicy {
                max_items: 100,
                retention_days: 30,
            },
            1000,
        )
        .expect("synthetic clipboard worker");
        let worker_shutdown = worker.clone();
        let (bridge, endpoint) = nxtools_ipc::platform_bridge(8);
        endpoint.set_connected_for_contract_test(true);
        let bridge = Rc::new(bridge);
        let clipboard = cx.update(|cx| {
            cx.new(|cx| {
                crate::clipboard::ClipboardApplication::new(
                    worker,
                    bridge.clone(),
                    config.clone(),
                    path.clone(),
                    notifications.clone(),
                    cx,
                )
            })
        });
        let catalog = nxtools_finder::FinderCatalog::open(directory.join("finder.db"), &[])
            .expect("synthetic catalog");
        let (indexer, _) = crate::runtime::indexer_bridge(8);
        let quick = cx.update(|cx| {
            cx.new(|_| {
                crate::quick_launch::QuickLaunchApplication::new(
                    catalog,
                    bridge,
                    Rc::new(indexer),
                    config.clone(),
                    path.clone(),
                    directory.join("logs"),
                    notifications,
                )
            })
        });
        let correlation_id = uuid::Uuid::new_v4();
        let mut rules = ProcessBlacklistSettings::from_entries(vec![
            ProcessBlacklistRule::new("Editor.EXE", true, false).expect("editor"),
            ProcessBlacklistRule::new("Game.EXE", false, true).expect("game"),
        ]);
        rules.suppress_in_fullscreen = Some(false);
        cx.update(|cx| {
            application.update(cx, |owner, cx| {
                owner.bind_clipboard(clipboard.downgrade());
                owner.bind_quick_launch(quick.downgrade());
                assert!(
                    owner
                        .apply_global(
                            config.network.clone(),
                            config.system.clone(),
                            rules,
                            correlation_id,
                            None,
                            cx
                        )
                        .expect("save")
                );
                assert!(
                    !clipboard.read(cx).is_process_excluded("editor"),
                    "consumer update must be deferred"
                );
                assert!(!quick.read(cx).is_process_excluded("game"));
            })
        });
        cx.run_until_parked();
        cx.read(|cx| {
            assert!(
                clipboard
                    .read(cx)
                    .is_process_excluded("C:\\Apps\\EDITOR.EXE")
            );
            assert!(!clipboard.read(cx).is_process_excluded("game"));
            assert!(!quick.read(cx).is_process_excluded("editor"));
            assert!(quick.read(cx).is_process_excluded("GAME.EXE"));
        });
        let envelope = endpoint
            .try_receive_command_for_contract_test()
            .expect("one native clipboard configuration");
        assert_eq!(envelope.correlation_id, Some(correlation_id));
        let nxtools_protocol::Message::PlatformCommand(PlatformCommand::ConfigureClipboard(
            command,
        )) = envelope.message
        else {
            panic!("clipboard command")
        };
        assert!(!command.suppress_in_fullscreen);
        let envelope = endpoint
            .try_receive_command_for_contract_test()
            .expect("one native quick-launch configuration");
        assert_eq!(envelope.correlation_id, Some(correlation_id));
        let nxtools_protocol::Message::PlatformCommand(PlatformCommand::ConfigureQuickLaunch(
            command,
        )) = envelope.message
        else {
            panic!("quick-launch command")
        };
        assert!(!command.suppress_in_exclusive_fullscreen);
        assert!(
            endpoint.try_receive_command_for_contract_test().is_err(),
            "no duplicate save/native dispatch"
        );
        let saved = std::fs::read(&path).expect("durable bytes");
        std::fs::remove_file(&path).expect("force save failure");
        cx.update(|cx| {
            application.update(cx, |owner, cx| {
                assert!(
                    owner
                        .apply_global(
                            config.network.clone(),
                            config.system.clone(),
                            ProcessBlacklistSettings::from_entries(vec![]),
                            uuid::Uuid::new_v4(),
                            None,
                            cx
                        )
                        .is_err()
                );
            })
        });
        cx.run_until_parked();
        assert!(cx.read(|cx| clipboard.read(cx).is_process_excluded("editor")));
        assert!(cx.read(|cx| quick.read(cx).is_process_excluded("game")));
        assert!(!path.exists(), "consumers cannot write settings");
        std::fs::write(&path, saved).expect("restore synthetic config");
        worker_shutdown.shutdown().expect("stop synthetic worker");
        let _ = std::fs::remove_dir_all(directory);
    }

    #[gpui_kit::test]
    fn blacklist_only_apply_skips_shell_when_desired_is_already_applied(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        cx.update(crate::theme::apply_component_theme);
        let (application, directory, _) = application_fixture(cx, "blacklist-only");
        let (bridge, endpoint) = nxtools_ipc::platform_bridge(8);
        endpoint.set_connected_for_contract_test(true);
        cx.update(|cx| {
            application.update(cx, |owner, cx| {
                owner.platform = Some(Rc::new(bridge));
                *owner.system_settings.borrow_mut() = owner.config.system.clone();
                assert!(
                    owner
                        .apply_global(
                            owner.config.network.clone(),
                            owner.config.system.clone(),
                            ProcessBlacklistSettings::from_entries(vec![]),
                            uuid::Uuid::new_v4(),
                            None,
                            cx
                        )
                        .expect("rules saved")
                );
            })
        });
        cx.run_until_parked();
        assert!(
            endpoint.try_receive_command_for_contract_test().is_err(),
            "no startup/tray mutation"
        );
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn blacklist_single_writer_and_independent_real_consumption() {
        let mut config = AppConfig::default();
        config.set_process_blacklist_settings(ProcessBlacklistSettings::from_entries(vec![
            nxtools_core::ProcessBlacklistRule::new("Editor.exe", true, false)
                .expect("clipboard rule"),
            nxtools_core::ProcessBlacklistRule::new("Game.exe", false, true).expect("quick rule"),
        ]));
        assert!(config.process_is_blacklisted(
            "EDITOR.EXE",
            nxtools_core::ProcessBlacklistFeature::Clipboard
        ));
        assert!(!crate::quick_launch::quick_launch_process_is_blacklisted(
            &config,
            "editor.exe"
        ));
        assert!(crate::quick_launch::quick_launch_process_is_blacklisted(
            &config, "GAME.EXE"
        ));
        assert!(
            !config.process_is_blacklisted(
                "game.exe",
                nxtools_core::ProcessBlacklistFeature::Clipboard
            )
        );
        assert_eq!(config.clipboard.excluded_processes, ["editor"]);
        assert!(!include_str!("clipboard.rs").contains("AppConfig::update_process_blacklist"));
        assert!(!include_str!("quick_launch.rs").contains("AppConfig::update_process_blacklist"));
    }

    #[test]
    fn parser_rejects_invalid_manual_proxy() {
        assert!(parse_network_settings(NetworkProxyMode::Direct, "", "").is_ok());
        assert!(parse_network_settings(NetworkProxyMode::System, "", "").is_ok());
        assert!(parse_network_settings(NetworkProxyMode::ManualHttp, "", "8080").is_err());
        assert!(
            parse_network_settings(NetworkProxyMode::ManualHttp, "proxy.example.test", "0")
                .is_err()
        );
    }

    #[gpui_kit::test]
    fn save_persists_before_publishing_the_shared_policy(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        let (application, directory, policy) = application_fixture(cx, "save");
        cx.update(|cx| {
            application.update(cx, |application, cx| application.open_window(cx));
        });
        cx.run_until_parked();
        let (window_handle, view) = cx.read(|cx| {
            let application = application.read(cx);
            (
                application.window.expect("production settings window"),
                application
                    .settings_view
                    .clone()
                    .expect("production settings view"),
            )
        });
        let mut window = VisualTestContext::from_window(window_handle, cx);
        window.run_until_parked();
        let actual_window_size = window_handle
            .update(cx, |_, window, _| window.bounds().size)
            .expect("read production global-settings bounds");
        assert_eq!(actual_window_size, size(px(720.0), px(480.0)));
        window.update(|window, cx| {
            view.update(cx, |view, cx| {
                view.host.update(cx, |input, cx| {
                    input.set_value("127.0.0.1", window, cx);
                });
                view.port.update(cx, |input, cx| {
                    input.set_value("3128", window, cx);
                });
            });
        });
        window.refresh().expect("render network settings");
        let shell = window
            .debug_bounds("network-settings-shell")
            .expect("shared settings shell");
        let titlebar = window
            .debug_bounds("network-settings-titlebar")
            .expect("shared settings titlebar");
        assert_eq!(titlebar.size.height, px(34.0));
        let content = window
            .debug_bounds("network-settings-content")
            .expect("shared settings content");
        let footer = window
            .debug_bounds("network-settings-footer")
            .expect("shared settings footer");
        assert_eq!(shell.size.width, content.size.width);
        assert_eq!(shell.size.width, footer.size.width);
        assert!(
            content.bottom() <= footer.top(),
            "content={content:?}, footer={footer:?}"
        );
        let general_section = window
            .debug_bounds("global-settings-workspace")
            .expect("official settings workspace");
        assert!(general_section.bottom() <= footer.top());
        let start_toggle =
            super::kit_migration_tests::control_bounds(&mut window, "system-start-on-login");
        window.simulate_click(start_toggle.center(), Modifiers::default());
        window.run_until_parked();
        window.update(|_, cx| {
            assert!(
                view.read(cx).start_on_login_silently,
                "the production shared toggle must mutate the settings draft"
            );
        });
        window.refresh().expect("render navigation");
        super::kit_migration_tests::select_page(&mut window, 1);
        window.run_until_parked();
        window.refresh().expect("render network page");
        let network_section = window
            .debug_bounds("global-settings-workspace")
            .expect("official workspace");
        assert_eq!(general_section.size.width, network_section.size.width);
        assert!(network_section.bottom() <= footer.top());
        let mode_group = window
            .debug_bounds("network-settings-proxy-mode")
            .expect("shared segmented proxy mode");
        assert_eq!(mode_group.size.height, px(32.0));
        let direct_mode = window
            .debug_bounds("network-settings-proxy-mode-direct")
            .expect("shared segmented direct option");
        let system_mode = window
            .debug_bounds("network-settings-proxy-mode-system")
            .expect("shared segmented system option");
        let manual_mode = window
            .debug_bounds("network-settings-proxy-mode-manual")
            .expect("shared segmented manual proxy option");
        assert_eq!(direct_mode.size.height, system_mode.size.height);
        assert_eq!(system_mode.size.height, manual_mode.size.height);
        assert!((direct_mode.size.width - system_mode.size.width).abs() <= px(1.0));
        assert!((system_mode.size.width - manual_mode.size.width).abs() <= px(1.0));
        assert!(manual_mode.top() >= mode_group.top());
        assert!(manual_mode.bottom() <= mode_group.bottom());
        assert!(manual_mode.left() >= mode_group.left());
        assert!(manual_mode.right() <= mode_group.right());
        window.simulate_click(manual_mode.center(), Modifiers::default());
        window.run_until_parked();
        window.update(|_, cx| {
            assert_eq!(view.read(cx).mode, NetworkProxyMode::ManualHttp);
        });
        let host_field = window
            .debug_bounds("network-settings-proxy-host-field")
            .expect("shared proxy host field");
        let port_field = window
            .debug_bounds("network-settings-proxy-port-field")
            .expect("shared proxy port field");
        assert_eq!(host_field.size.height, px(32.0));
        assert_eq!(port_field.size.height, px(32.0));
        let save = window
            .debug_bounds("network-settings-save")
            .expect("network settings save button");
        window.simulate_click(save.center(), Modifiers::default());
        window.run_until_parked();

        let persisted =
            AppConfig::load_from(&directory.join("config.json")).expect("load persisted settings");
        assert_eq!(persisted.network.proxy.mode, NetworkProxyMode::ManualHttp);
        assert_eq!(
            policy.snapshot().expect("live snapshot").mode(),
            NetworkProxyMode::ManualHttp
        );
        let _ = std::fs::remove_dir_all(directory);
    }

    #[gpui_kit::test]
    fn invalid_proxy_uses_inline_error_above_the_fixed_footer(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        let (application, directory, _policy) = application_fixture(cx, "inline-error");
        cx.update(|cx| {
            application.update(cx, |application, cx| application.open_window(cx));
        });
        cx.run_until_parked();
        let (window_handle, view) = cx.read(|cx| {
            let application = application.read(cx);
            (
                application.window.expect("production settings window"),
                application
                    .settings_view
                    .clone()
                    .expect("production settings view"),
            )
        });
        let mut window = VisualTestContext::from_window(window_handle, cx);
        window.run_until_parked();
        window.update(|window, cx| {
            view.update(cx, |view, cx| {
                view.mode = NetworkProxyMode::ManualHttp;
                view.host.update(cx, |input, cx| {
                    input.set_value("", window, cx);
                });
                view.port.update(cx, |input, cx| {
                    input.set_value("0", window, cx);
                });
            });
        });
        window.refresh().expect("render invalid settings draft");
        let save = window
            .debug_bounds("network-settings-save")
            .expect("settings save action");
        window.simulate_click(save.center(), Modifiers::default());
        window.run_until_parked();
        window.refresh().expect("render inline validation error");

        let content = window
            .debug_bounds("network-settings-content")
            .expect("settings content");
        let inline_error = window
            .debug_bounds("network-settings-inline-error")
            .expect("shared inline error");
        let footer = window
            .debug_bounds("network-settings-footer")
            .expect("fixed settings footer");
        assert!(inline_error.left() >= content.left());
        assert!(inline_error.right() <= content.right());
        assert!(
            inline_error.bottom() <= footer.top(),
            "inline_error={inline_error:?}, footer={footer:?}"
        );
        window.update(|_, cx| assert!(view.read(cx).error.is_some()));
        let _ = std::fs::remove_dir_all(directory);
    }

    #[gpui_kit::test]
    fn persistence_failure_never_publishes_a_tentative_policy(cx: &mut TestAppContext) {
        let (application, directory, policy) = application_fixture(cx, "failed-save");
        std::fs::remove_file(directory.join("config.json")).expect("remove config fixture");
        let result = cx.update(|cx| {
            application.update(cx, |application, cx| {
                application.apply(
                    NetworkSettings {
                        proxy: NetworkProxySettings {
                            mode: NetworkProxyMode::ManualHttp,
                            host: "127.0.0.1".into(),
                            port: 3128,
                        },
                    },
                    SystemSettings::default(),
                    uuid::Uuid::new_v4(),
                    None,
                    cx,
                )
            })
        });
        assert!(result.is_err());
        assert_eq!(
            policy.snapshot().expect("unchanged policy").mode(),
            NetworkProxyMode::Direct
        );
        let _ = std::fs::remove_dir_all(directory);
    }

    #[gpui_kit::test]
    fn a_pending_windows_apply_cannot_be_overwritten_by_another_save(cx: &mut TestAppContext) {
        let (application, directory, _policy) = application_fixture(cx, "pending-apply");
        let first_correlation = uuid::Uuid::new_v4();
        cx.update(|cx| {
            application.update(cx, |application, _| {
                application.pending_apply = Some(PendingGlobalSettingsApply {
                    command_id: uuid::Uuid::new_v4(),
                    correlation_id: first_correlation,
                    mode: "direct",
                    started_at: Instant::now(),
                    desired_system: SystemSettings::default(),
                    view: None,
                    source_window: None,
                    user_operation: false,
                });
            });
        });

        let result = cx.update(|cx| {
            application.update(cx, |application, cx| {
                application.apply(
                    NetworkSettings::default(),
                    SystemSettings::default(),
                    uuid::Uuid::new_v4(),
                    None,
                    cx,
                )
            })
        });
        assert!(result.is_err());
        assert_eq!(
            cx.read(|cx| {
                application
                    .read(cx)
                    .pending_apply
                    .as_ref()
                    .map(|pending| pending.correlation_id)
            }),
            Some(first_correlation)
        );
        let _ = std::fs::remove_dir_all(directory);
    }

    #[gpui_kit::test]
    fn close_to_tray_becomes_active_only_after_windows_confirms_it(cx: &mut TestAppContext) {
        let (application, directory, _policy) = application_fixture(cx, "confirmed-shell");
        let desired = SystemSettings {
            start_on_login_silently: true,
            close_to_tray: true,
            theme: nxtools_core::ThemePreference::Dark,
            ui_font_family: nxtools_core::default_ui_font_family(),
            notification_position: Default::default(),
            notification_duration_seconds: nxtools_core::default_notification_duration(),
        };
        let complete = |application: &mut NetworkSettingsApplication,
                        command_id,
                        desired_system,
                        success,
                        cx: &mut Context<NetworkSettingsApplication>| {
            application.pending_apply = Some(PendingGlobalSettingsApply {
                command_id,
                correlation_id: uuid::Uuid::new_v4(),
                mode: "direct",
                started_at: Instant::now(),
                desired_system,
                view: None,
                source_window: None,
                user_operation: false,
            });
            application.handle_platform_notification(
                PlatformNotification::CommandResult {
                    correlation_id: command_id,
                    result: nxtools_protocol::PlatformCommandResult {
                        success,
                        error: (!success).then(|| "fixture".into()),
                        native_error_code: (!success).then_some(5),
                        data: None,
                    },
                },
                cx,
            );
        };

        cx.update(|cx| {
            application.update(cx, |application, cx| {
                complete(
                    application,
                    uuid::Uuid::new_v4(),
                    desired.clone(),
                    false,
                    cx,
                );
            });
        });
        assert!(!cx.read(|cx| { application.read(cx).system_settings.borrow().close_to_tray }));

        cx.update(|cx| {
            application.update(cx, |application, cx| {
                complete(application, uuid::Uuid::new_v4(), desired, true, cx);
            });
        });
        assert!(cx.read(|cx| { application.read(cx).system_settings.borrow().close_to_tray }));

        cx.update(|cx| {
            application.update(cx, |application, cx| {
                complete(
                    application,
                    uuid::Uuid::new_v4(),
                    SystemSettings {
                        start_on_login_silently: true,
                        close_to_tray: true,
                        theme: nxtools_core::ThemePreference::Dark,
                        ui_font_family: nxtools_core::default_ui_font_family(),
                        notification_position: Default::default(),
                        notification_duration_seconds: nxtools_core::default_notification_duration(
                        ),
                    },
                    false,
                    cx,
                );
            });
        });
        assert!(
            !cx.read(|cx| { application.read(cx).system_settings.borrow().close_to_tray }),
            "a failed tray reconciliation must fail closed even after a prior success"
        );
        let _ = std::fs::remove_dir_all(directory);
    }

    #[gpui_kit::test]
    fn failed_shell_result_keeps_saved_intent_and_settings_window_open(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        let (application, directory, _policy) = application_fixture(cx, "shell-failure");
        cx.update(|cx| {
            application.update(cx, |application, cx| application.open_window(cx));
        });
        cx.run_until_parked();
        let (window_handle, view) = cx.read(|cx| {
            let application = application.read(cx);
            (
                application.window.expect("production settings window"),
                application
                    .settings_view
                    .clone()
                    .expect("production settings view"),
            )
        });
        let desired_system = SystemSettings {
            start_on_login_silently: true,
            close_to_tray: true,
            theme: nxtools_core::ThemePreference::Dark,
            ui_font_family: nxtools_core::default_ui_font_family(),
            notification_position: Default::default(),
            notification_duration_seconds: nxtools_core::default_notification_duration(),
        };
        let config_path = directory.join("config.json");
        let mut persisted = AppConfig::load_from(&config_path).expect("load config fixture");
        persisted.system = desired_system.clone();
        let command_id = uuid::Uuid::new_v4();
        persisted
            .save_to(&config_path)
            .expect("persist desired settings");
        cx.update(|cx| {
            application.update(cx, |application, cx| {
                view.update(cx, |view, cx| {
                    view.saving = true;
                    cx.notify();
                });
                application.pending_apply = Some(PendingGlobalSettingsApply {
                    command_id,
                    correlation_id: command_id,
                    mode: "direct",
                    started_at: Instant::now(),
                    desired_system,
                    view: Some(view.downgrade()),
                    source_window: Some(window_handle),
                    user_operation: true,
                });
                application.handle_platform_notification(
                    PlatformNotification::CommandResult {
                        correlation_id: command_id,
                        result: nxtools_protocol::PlatformCommandResult {
                            success: false,
                            error: Some("safe fixture failure".into()),
                            native_error_code: Some(5),
                            data: Some("tray_icon_failed".into()),
                        },
                    },
                    cx,
                );
            });
        });

        let persisted = AppConfig::load_from(&config_path).expect("reload persisted settings");
        assert!(persisted.system.start_on_login_silently);
        assert!(persisted.system.close_to_tray);
        cx.read(|cx| {
            let application = application.read(cx);
            assert!(application.window == Some(window_handle));
            assert!(!application.system_settings.borrow().close_to_tray);
            assert!(application.pending_apply.is_none());
            let view = view.read(cx);
            assert!(!view.saving);
            assert!(view.error.is_none(), "native failure is not form validation");
            assert!(application.notifications.read(cx).items().iter().any(|notice|
                notice.level == crate::notifications::NotificationLevel::Error
                    && notice.message.as_ref() == "设置已保存，但 Windows 未能创建或显示 NxTools 托盘图标。请检查通知区域设置后重试。"));
        });
        let _ = std::fs::remove_dir_all(directory);
    }

    #[gpui_kit::test]
    fn stale_shell_result_does_not_apply_or_close_the_current_window(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        let (application, directory, _policy) = application_fixture(cx, "stale-shell-result");
        cx.update(|cx| {
            application.update(cx, |application, cx| application.open_window(cx));
        });
        cx.run_until_parked();
        let (window_handle, view) = cx.read(|cx| {
            let application = application.read(cx);
            (
                application.window.expect("production settings window"),
                application
                    .settings_view
                    .clone()
                    .expect("production settings view"),
            )
        });
        let command_id = uuid::Uuid::new_v4();

        cx.update(|cx| {
            application.update(cx, |application, cx| {
                application.pending_apply = Some(PendingGlobalSettingsApply {
                    command_id,
                    correlation_id: command_id,
                    mode: "direct",
                    started_at: Instant::now(),
                    desired_system: SystemSettings {
                        start_on_login_silently: true,
                        close_to_tray: true,
                        theme: nxtools_core::ThemePreference::Dark,
                        ui_font_family: nxtools_core::default_ui_font_family(),
                        notification_position: Default::default(),
                        notification_duration_seconds: nxtools_core::default_notification_duration(
                        ),
                    },
                    view: Some(view.downgrade()),
                    source_window: Some(window_handle),
                    user_operation: true,
                });
                application.handle_platform_notification(
                    PlatformNotification::CommandResult {
                        correlation_id: uuid::Uuid::new_v4(),
                        result: nxtools_protocol::PlatformCommandResult {
                            success: true,
                            error: None,
                            native_error_code: None,
                            data: None,
                        },
                    },
                    cx,
                );
            });
        });

        cx.read(|cx| {
            let application = application.read(cx);
            assert!(application.window == Some(window_handle));
            assert!(
                application
                    .pending_apply
                    .as_ref()
                    .is_some_and(|pending| pending.command_id == command_id)
            );
            assert!(view.read(cx).error.is_none());
        });
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn windows_shell_failure_stages_have_actionable_payload_free_messages() {
        assert_eq!(
            global_shell_failure_message("platform_result", Some("windows_shell_apply_failed")),
            "Windows 未能应用启动项或托盘设置。设置已保存；请检查系统权限后重试。"
        );
        assert_eq!(
            global_shell_failure_message("platform_disconnected", None),
            "Windows 平台组件已断开。设置已保存；重新启动 NxTools 后重试。"
        );
        assert_eq!(
            global_shell_failure_message("platform_enqueue", None),
            "Windows 平台组件当前不可用。设置已保存；请稍后重试。"
        );
        assert_eq!(
            global_shell_failure_message("platform_result", Some("desktop_executable_unavailable")),
            "设置已保存，但无法校验当前 NxTools 程序，启动项和托盘设置未应用。请关闭旧版本并从当前安装目录重新启动后重试。"
        );
        assert_eq!(
            global_shell_failure_message("platform_result", Some("startup_registration_failed")),
            "设置已保存，但 Windows 启动项未能写入当前用户注册表。请检查系统策略或安全软件后重试。"
        );
        assert_eq!(
            global_shell_failure_message("platform_result", Some("tray_icon_failed")),
            "设置已保存，但 Windows 未能创建或显示 NxTools 托盘图标。请检查通知区域设置后重试。"
        );
        assert_eq!(
            global_shell_failure_message("platform_timeout", None),
            "Windows 平台组件未及时响应。设置已保存；请检查 NxTools 后重试。"
        );
        assert_eq!(
            global_shell_failure_message("platform_result", Some("windows_shell_enqueue_failed")),
            "设置已保存，但 Windows 未能安排系统设置更新。请稍后重试；问题持续时请重启 NxTools。"
        );
        assert_eq!(
            global_shell_failure_message("platform_result", Some("unknown")),
            "Windows 系统设置未能应用。设置已保存；请检查系统权限后重试。"
        );
    }

    #[gpui_kit::test]
    fn cancel_and_titlebar_close_release_the_owned_production_window(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        let (application, directory, _policy) = application_fixture(cx, "lifecycle");
        cx.update(|cx| application.update(cx, |application, cx| application.open_window(cx)));
        let first = cx
            .read(|cx| application.read(cx).window)
            .expect("first settings window");

        cx.update(|cx| application.update(cx, |application, cx| application.open_window(cx)));
        let second = cx
            .read(|cx| application.read(cx).window)
            .expect("reused settings window");
        assert!(first == second, "opening twice must reuse one owned window");

        let mut window = VisualTestContext::from_window(first, cx);
        window.run_until_parked();
        window.refresh().expect("render shared settings shell");
        let cancel = window
            .debug_bounds("network-settings-cancel")
            .expect("production Cancel action");
        assert_eq!(cancel.size.height, px(24.0));
        window.simulate_click(cancel.center(), Modifiers::default());
        window.run_until_parked();
        assert!(cx.read(|cx| application.read(cx).window.is_none()));
        assert!(
            cx.update(|cx| first.update(cx, |_, _, _| ())).is_err(),
            "Cancel must destroy the owned production window"
        );

        cx.update(|cx| application.update(cx, |application, cx| application.open_window(cx)));
        let reopened = cx
            .read(|cx| application.read(cx).window)
            .expect("reopened settings window");
        let mut window = VisualTestContext::from_window(reopened, cx);
        window.run_until_parked();
        window.refresh().expect("render shared client titlebar");
        let close = window
            .debug_bounds("network-settings-close")
            .expect("shared titlebar close action");
        assert_eq!(
            close.size,
            size(
                px(crate::ui::metrics::TITLEBAR_HEIGHT),
                px(crate::ui::metrics::TITLEBAR_HEIGHT - 1.0)
            )
        );
        window.simulate_click(close.center(), Modifiers::default());
        window.run_until_parked();
        assert!(cx.read(|cx| application.read(cx).window.is_none()));
        assert!(
            cx.update(|cx| reopened.update(cx, |_, _, _| ())).is_err(),
            "the titlebar close must destroy the owned window, not only forget it"
        );
        let _ = std::fs::remove_dir_all(directory);
    }

    #[gpui_kit::test]
    fn global_rule_layout_stays_aligned_for_long_many_empty_and_busy_states(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_kit::init);
        let (application, directory, _) = application_fixture(cx, "layout-states");
        cx.update(|cx| application.update(cx, |owner, cx| owner.open_window(cx)));
        cx.run_until_parked();
        let (handle, view) = cx.read(|cx| {
            let owner = application.read(cx);
            (
                owner.window.expect("window"),
                owner.settings_view.clone().expect("view"),
            )
        });
        let mut surface = VisualTestContext::from_window(handle, cx);
        surface.simulate_resize(size(px(720.0), px(480.0)));
        surface.refresh().expect("official initialization");
        super::kit_migration_tests::select_page(&mut surface, 2);
        for count in [12, 0] {
            surface.update(|_, cx| {
                view.update(cx, |view, cx| {
                    view.tab = GlobalSettingsTab::Blacklist;
                    view.rules = (0..count)
                        .map(|index| {
                            ProcessBlacklistRule::new(
                                &format!(
                                    "VeryLongApplicationProcessNameForAlignmentContract{index}.exe"
                                ),
                                true,
                                false,
                            )
                            .expect("synthetic rule")
                        })
                        .collect();
                    cx.notify();
                })
            });
            surface.refresh().expect("render layout state");
            let table = surface
                .debug_bounds("global-blacklist-table")
                .expect("table");
            let footer = surface
                .debug_bounds("network-settings-footer")
                .expect("footer");
            let field = surface
                .debug_bounds("global-blacklist-process-field")
                .expect("field");
            assert!(
                table.size.height <= px(202.0),
                "rows scroll inside the bounded table"
            );
            assert!(
                field.bottom() < table.top() && table.bottom() < footer.top(),
                "field={field:?} table={table:?} footer={footer:?}"
            );
            assert!(
                field.size.width >= px(200.0),
                "input remains usable without window inflation"
            );
            assert!(
                table.right() <= px(700.0),
                "table={table:?} field={field:?}"
            );
            if count > 0 {
                let label = surface
                    .debug_bounds("global-blacklist-label-0")
                    .expect("long label");
                let cell = surface
                    .debug_bounds("global-blacklist-clipboard-cell-0")
                    .expect("scope column");
                assert!(
                    label.right() <= cell.left(),
                    "long name cannot push fixed columns"
                );
            }
        }
        surface.update(|_, cx| {
            view.update(cx, |view, cx| {
                view.rules = vec![
                    ProcessBlacklistRule::new("BusyScopeFixture.exe", true, false)
                        .expect("busy rule"),
                ];
                view.saving = true;
                cx.notify();
            })
        });
        surface.refresh().expect("busy actions");
        let toggle =
            super::kit_migration_tests::control_bounds(&mut surface, "global-blacklist-quick-0");
        surface.update(|window, _| {
            let nodes = gpui_kit::base::test_support::snapshots(window);
            let scope = nodes
                .iter()
                .find(|node| {
                    node.path().last()
                        == Some(&gpui_kit::ElementId::from("global-blacklist-quick-0"))
                })
                .expect("actual busy row checkbox");
            assert_eq!(scope.disabled(), Some(true));
        });
        surface.simulate_click(toggle.center(), Modifiers::default());
        let general = super::kit_migration_tests::page_bounds(&mut surface, 0);
        surface.simulate_click(general.center(), Modifiers::default());
        surface.run_until_parked();
        surface.read(|cx| {
            let view = view.read(cx);
            assert!(!view.new_disable_quick_launch);
            assert!(!view.rules[0].disable_quick_launch);
            assert!(view.tab == GlobalSettingsTab::Blacklist);
        });
        surface.update(|_, cx| {
            view.update(cx, |view, cx| {
                view.saving = false;
                cx.notify();
            })
        });
        cx.update(|cx| application.update(cx, |owner, cx| owner.close_window(cx)));
        let _ = std::fs::remove_dir_all(directory);
    }
    #[gpui_kit::test]
    fn notification_position_draft_cancel_and_save_preserve_original_global_transaction(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_kit::init);
        let (application, directory, _) = application_fixture(cx, "notification-position");
        let path = directory.join("config.json");
        let before = std::fs::read(&path).expect("initial config");
        application.update(cx, |app, cx| app.open_window(cx));
        cx.run_until_parked();
        let (handle, view, center) = application.read_with(cx, |app, _| {
            (
                app.window.expect("settings"),
                app.settings_view.clone().expect("draft"),
                app.notifications.clone(),
            )
        });
        let mut surface = VisualTestContext::from_window(handle, cx);
        surface.simulate_resize(size(px(720.0), px(480.0)));
        surface.update(|window, cx| window.draw(cx).clear(cx));
        let chooser = surface.update(|window, _| {
            gpui_kit::base::test_support::snapshots(window)
                .into_iter()
                .find(|node| node.path().last() == Some(&"global-notification-position".into()))
                .expect("official notification-position Button")
                .bounds()
        });
        assert_eq!(chooser.size.height, px(32.0));
        view.update(&mut surface, |view, _| {
            view.notification_position = nxtools_core::NotificationPosition::BottomRight
        });
        application.update(&mut surface, |app, cx| app.close_window(cx));
        assert_eq!(std::fs::read(&path).expect("cancel has no write"), before);
        assert_eq!(
            center.read_with(&surface, |center, _| center.position()),
            nxtools_core::NotificationPosition::TopCenter
        );
        application.update(&mut surface, |app, cx| {
            let mut settings = app.config.system.clone();
            settings.notification_position = nxtools_core::NotificationPosition::BottomRight;
            app.apply_global(
                app.config.network.clone(),
                settings,
                app.config.process_blacklist.clone(),
                uuid::Uuid::new_v4(),
                None,
                cx,
            )
            .expect("save notification location");
        });
        surface.run_until_parked();
        assert_eq!(
            AppConfig::load_from(&path)
                .expect("saved location")
                .system
                .notification_position,
            nxtools_core::NotificationPosition::BottomRight
        );
        assert_eq!(
            center.read_with(&surface, |center, _| center.position()),
            nxtools_core::NotificationPosition::BottomRight
        );
        let good = std::fs::read(&path).expect("last good settings");
        std::fs::write(&path, b"corrupt fixture").expect("owned persistence failure");
        application.update(&mut surface, |app, cx| {
            let mut settings = app.config.system.clone();
            settings.notification_position = nxtools_core::NotificationPosition::BottomLeft;
            assert!(
                app.apply_global(
                    app.config.network.clone(),
                    settings,
                    app.config.process_blacklist.clone(),
                    uuid::Uuid::new_v4(),
                    None,
                    cx
                )
                .is_err()
            );
        });
        surface.run_until_parked();
        assert_eq!(
            center.read_with(&surface, |center, _| center.position()),
            nxtools_core::NotificationPosition::BottomRight,
            "failed persistence never publishes draft position"
        );
        std::fs::write(&path, good).expect("restore owned fixture");
        let _ = std::fs::remove_dir_all(directory);
    }
}

#[cfg(test)]
mod page_rem_tests;
