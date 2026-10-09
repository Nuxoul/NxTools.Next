use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Instant;

use gpui::{
    AnyWindowHandle, Context, Entity, IntoElement, Render, WeakEntity, Window,
    WindowBackgroundAppearance, WindowBounds, WindowDecorations, WindowKind, WindowOptions, div,
    prelude::*, px, size,
};
use gpui_component::button::ButtonVariants;
use gpui_component::input::{Input, InputState};
use gpui_component::scroll::ScrollableElement;
use gpui_component::{Disableable, Root, Sizable};
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
use crate::ui::components::action::{
    SegmentedOption, action_button, primary_button_variant, segmented_control,
};
use crate::ui::components::field::{field_shell, inline_error};
use crate::ui::components::settings::{
    SettingsShellIds, dialog_footer, settings_row, settings_section, settings_shell,
    settings_toggle,
};
use crate::ui::components::titlebar::client_titlebar;
use crate::ui::metrics::SPACING_8;
use crate::ui::typography::{LINE_HEIGHT_BODY, TEXT_BODY, TEXT_LABEL};

mod blacklist;
#[cfg(debug_assertions)]
pub(crate) mod visual_preview;

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
            "找不到通过校验的 NxTools 程序。请修复或重新安装 NxTools 后重试。"
        }
        (_, Some("startup_registration_failed")) => {
            "Windows 启动项未能写入当前用户注册表。请检查系统策略或安全软件后重试。"
        }
        (_, Some("tray_icon_failed")) => {
            "Windows 未能创建或显示 NxTools 托盘图标。请检查通知区域设置后重试。"
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
            hidden_startup,
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
                        "global-settings-load",
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
                tabbing_identifier: None,
            },
            move |window, cx| {
                let close_application = application.clone();
                window.on_window_should_close(cx, move |_, cx| {
                    close_application.update(cx, |application, _| {
                        if application.pending_apply.is_some() || application.draft_busy.get() {
                            false
                        } else {
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
                    "network-settings-window",
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
        info!(event = "global.settings.persisted", %correlation_id, rule_count = next_config.process_blacklist.entries().len(), elapsed_ms = started_at.elapsed().as_millis(), "global settings durably saved");
        let shell_changed =
            self.config.system != system || *self.system_settings.borrow() != system;
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
                "network-settings-saved",
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
        self.draft_busy.set(false);
        if let Some(view) = pending.view {
            let _ = view.update(cx, |view, cx| {
                view.saving = false;
                view.error = Some(global_shell_failure_message(failure_kind, failure_stage).into());
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
    host: Entity<InputState>,
    port: Entity<InputState>,
    tab: GlobalSettingsTab,
    rules: Vec<ProcessBlacklistRule>,
    process_input: Entity<InputState>,
    editing_rule: Option<usize>,
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
        Self {
            theme: Theme::zed_dark(),
            application,
            mode: settings.proxy.mode,
            start_on_login_silently: system.start_on_login_silently,
            close_to_tray: system.close_to_tray,
            host,
            port,
            tab: GlobalSettingsTab::General,
            rules: blacklist.entries(),
            process_input,
            editing_rule: None,
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
        if !self.process_input.read(cx).value().trim().is_empty() || self.editing_rule.is_some() {
            self.error = Some("请先添加或更新正在编辑的应用规则，再应用设置。".into());
            cx.notify();
            return;
        }
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
        let blacklist = ProcessBlacklistSettings::from_entries(self.rules.clone());
        self.saving = true;
        self.draft_busy.set(true);
        self.error = None;
        let application = self.application.clone();
        let system = SystemSettings {
            start_on_login_silently: self.start_on_login_silently,
            close_to_tray: self.close_to_tray,
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
                        view.error = Some(error);
                        cx.notify();
                    });
                }
            }
        });
        cx.notify();
    }
}

impl Render for NetworkSettingsView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let manual = self.mode == NetworkProxyMode::ManualHttp;
        let selected_mode_id = match self.mode {
            NetworkProxyMode::Direct => "network-settings-proxy-mode-direct",
            NetworkProxyMode::System => "network-settings-proxy-mode-system",
            NetworkProxyMode::ManualHttp => "network-settings-proxy-mode-manual",
        };
        let mode_buttons = segmented_control(
            theme,
            "network-settings-proxy-mode",
            selected_mode_id,
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
                            this.mode = mode;
                            this.error = None;
                        }
                        cx.notify();
                    }),
                )
            })
            .collect(),
            cx,
        );
        let error = self.error.clone();
        let start_on_login_silently = self.start_on_login_silently;
        let close_to_tray = self.close_to_tray;
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
        let navigation = self.render_navigation(cx);
        let blacklist = self.render_blacklist(cx);
        let content = div()
            .flex_1()
            .h_full()
            .min_w_0()
            .min_h_0()
            .overflow_y_scrollbar()
            .p(px(20.0))
            .text_size(px(TEXT_BODY))
            .line_height(px(LINE_HEIGHT_BODY))
            .flex()
            .flex_col()
            .gap(px(16.0))
            .when(self.tab == GlobalSettingsTab::General, |content| {
                content.child(settings_section(
                    theme,
                    "network-settings-general-section",
                    "常规",
                    vec![
                        settings_row(
                            theme,
                            "开机静默启动",
                            "登录 Windows 后在后台启动，不主动弹出主窗口",
                            settings_toggle(
                                theme,
                                "system-start-on-login",
                                start_on_login_silently,
                                cx.listener(|this, _, _, cx| {
                                    if !this.saving {
                                        this.start_on_login_silently =
                                            !this.start_on_login_silently;
                                    }
                                    cx.notify();
                                }),
                            ),
                        ),
                        settings_row(
                            theme,
                            "关闭到托盘",
                            "关闭主窗口时继续在后台运行；托盘左键恢复，右键可退出",
                            settings_toggle(
                                theme,
                                "system-close-to-tray",
                                close_to_tray,
                                cx.listener(|this, _, _, cx| {
                                    if !this.saving {
                                        this.close_to_tray = !this.close_to_tray;
                                    }
                                    cx.notify();
                                }),
                            ),
                        ),
                    ],
                ))
            })
            .when(self.tab == GlobalSettingsTab::Network, |content| {
                content.child(settings_section(
                    theme,
                    "network-settings-network-section",
                    "网络与代理",
                    vec![
                        settings_row(
                            theme,
                            "代理模式",
                            "下载、剪贴板和速启中的实际联网操作共享；本地 IPC 与索引不使用",
                            mode_buttons,
                        ),
                        settings_row(
                            theme,
                            "代理服务器",
                            if manual {
                                "仅支持 HTTP 代理；修改只影响新开始或重新继续的操作"
                            } else {
                                "选择手动 HTTP 后填写地址和端口"
                            },
                            div()
                                .w(px(280.0))
                                .flex()
                                .items_center()
                                .gap(px(SPACING_8))
                                .child(
                                    div().flex_1().min_w_0().child(field_shell(
                                        theme,
                                        "network-settings-proxy-host-field",
                                        Input::new(&self.host)
                                            .appearance(false)
                                            .bordered(true)
                                            .disabled(!manual || self.saving)
                                            .small(),
                                    )),
                                )
                                .child(
                                    div().w(px(84.0)).child(field_shell(
                                        theme,
                                        "network-settings-proxy-port-field",
                                        Input::new(&self.port)
                                            .appearance(false)
                                            .bordered(true)
                                            .disabled(!manual || self.saving)
                                            .small(),
                                    )),
                                )
                                .into_any_element(),
                        ),
                    ],
                ))
            })
            .when(self.tab == GlobalSettingsTab::Blacklist, |content| {
                content.child(blacklist)
            })
            .child(if let Some(error) = error {
                inline_error(theme, "network-settings-inline-error", error)
            } else if self.tab == GlobalSettingsTab::Network {
                div()
                    .debug_selector(|| "network-settings-policy-note".into())
                    .text_size(px(TEXT_LABEL))
                    .text_color(theme.text_muted)
                    .child("不支持 PAC 或代理密码；敏感凭据不会写入配置。")
                    .into_any_element()
            } else {
                div().into_any_element()
            })
            .into_any_element();
        let content = div()
            .size_full()
            .flex()
            .min_h_0()
            .child(navigation)
            .child(content)
            .into_any_element();
        let footer_application = self.application.clone();
        let footer_actions = div()
            .flex()
            .items_center()
            .gap(px(SPACING_8))
            .child(
                div()
                    .debug_selector(|| "network-settings-cancel".into())
                    .child(
                        action_button("network-settings-cancel-button", "取消")
                            .h(px(32.0))
                            .w(px(72.0))
                            .text_size(px(TEXT_BODY))
                            .ghost()
                            .disabled(self.saving)
                            .on_click(move |_, _, cx| {
                                let application = footer_application.clone();
                                cx.defer(move |cx| {
                                    application.update(cx, |application, cx| {
                                        application.close_window(cx);
                                    });
                                });
                            }),
                    ),
            )
            .child(
                div()
                    .debug_selector(|| "network-settings-save".into())
                    .child(
                        action_button(
                            "network-settings-save-button",
                            if self.saving {
                                "正在应用…"
                            } else {
                                "应用"
                            },
                        )
                        .h(px(32.0))
                        .min_w(px(72.0))
                        .text_size(px(TEXT_BODY))
                        .disabled(self.saving)
                        .custom(primary_button_variant(theme, cx))
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
    use gpui::{AppContext, Modifiers, TestAppContext, VisualTestContext};

    use super::*;

    fn application_fixture(
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
                    })),
                    false,
                )
            })
        });
        (application, directory, policy)
    }

    #[gpui::test]
    fn blacklist_table_local_actions_cancel_failure_and_single_save(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
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
        let tab = window
            .debug_bounds("global-settings-tab-blacklist")
            .expect("blacklist tab");
        window.simulate_click(tab.center(), Modifiers::default());
        window.run_until_parked();
        window.update(|window, cx| {
            view.update(cx, |view, cx| {
                view.process_input.update(cx, |input, cx| {
                    input.set_value("C:\\Apps\\Editor.EXE", window, cx)
                });
            })
        });
        window.refresh().expect("render add");
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
        assert_eq!(add.size.height, field.size.height);
        assert_eq!(add.center().y, field.center().y);
        let rail = window
            .debug_bounds("global-settings-navigation")
            .expect("settings rail");
        assert_eq!(rail.size.width, px(128.0));
        assert!(field.left() >= rail.right() + px(20.0));
        let toggle = window
            .debug_bounds("global-blacklist-clipboard-cell-0")
            .expect("checkbox");
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
        for (header_id, marker_id, editor_id) in [
            (
                "global-blacklist-header-clipboard",
                "global-blacklist-clipboard-0-mark",
                "global-blacklist-new-clipboard-mark",
            ),
            (
                "global-blacklist-header-quick",
                "global-blacklist-quick-0-mark",
                "global-blacklist-new-quick-mark",
            ),
        ] {
            let header = window.debug_bounds(header_id).expect("column header");
            let marker = window
                .debug_bounds(marker_id)
                .expect("centered checkbox marker");
            let editor_marker = window
                .debug_bounds(editor_id)
                .expect("editor checkbox marker");
            assert!((header.center().x - marker.center().x).abs() <= px(1.0));
            assert!((header.center().x - editor_marker.center().x).abs() <= px(1.0));
            assert_eq!(marker.center().y, label.center().y);
            assert_eq!(editor_marker.center().y, field.center().y);
        }
        window.simulate_click(toggle.center(), Modifiers::default());
        window.run_until_parked();
        assert!(!window.read(|cx| view.read(cx).rules[0].disable_clipboard));
        window.simulate_mouse_down(
            toggle.center(),
            gpui::MouseButton::Left,
            Modifiers::default(),
        );
        window
            .refresh()
            .expect("focus actual clipboard scope action");
        window.simulate_keystrokes("space");
        window.simulate_event(gpui::KeyUpEvent {
            keystroke: gpui::Keystroke::parse("space").expect("Space"),
        });
        window.run_until_parked();
        assert!(
            window.read(|cx| view.read(cx).rules[0].disable_clipboard),
            "one keyboard toggle"
        );
        window.simulate_keystrokes("tab");
        window.refresh().expect("Tab to quick launch scope action");
        window.simulate_keystrokes("space");
        window.simulate_event(gpui::KeyUpEvent {
            keystroke: gpui::Keystroke::parse("space").expect("Space"),
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
            assert!(view.error.is_some());
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
        assert!(cx.read(|cx| application.read(cx).window.is_none()));
        let _ = std::fs::remove_dir_all(directory);
    }

    #[gpui::test]
    fn global_rules_publish_both_bound_consumers_only_after_durable_success(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_component::init);
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
        let rules = ProcessBlacklistSettings::from_entries(vec![
            ProcessBlacklistRule::new("Editor.EXE", true, false).expect("editor"),
            ProcessBlacklistRule::new("Game.EXE", false, true).expect("game"),
        ]);
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
        assert_eq!(command.excluded_processes, ["editor"]);
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

    #[gpui::test]
    fn blacklist_only_apply_skips_shell_when_desired_is_already_applied(cx: &mut TestAppContext) {
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

    #[gpui::test]
    fn save_persists_before_publishing_the_shared_policy(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
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
        assert_eq!(titlebar.size.height, px(36.0));
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
            .debug_bounds("network-settings-general-section")
            .expect("shared settings section");
        assert!(general_section.bottom() <= footer.top());
        let start_toggle = window
            .debug_bounds("system-start-on-login")
            .expect("shared settings toggle");
        assert_eq!(
            start_toggle.size,
            size(px(40.0), px(crate::ui::metrics::CONTROL_HEIGHT_DEFAULT))
        );
        window.simulate_click(start_toggle.center(), Modifiers::default());
        window.run_until_parked();
        window.update(|_, cx| {
            assert!(
                view.read(cx).start_on_login_silently,
                "the production shared toggle must mutate the settings draft"
            );
        });
        window.refresh().expect("render navigation");
        let network_tab = window
            .debug_bounds("global-settings-tab-network")
            .expect("network tab");
        window.simulate_click(network_tab.center(), Modifiers::default());
        window.run_until_parked();
        window.refresh().expect("render network page");
        let network_section = window
            .debug_bounds("network-settings-network-section")
            .expect("network section");
        assert_eq!(general_section.size.width, network_section.size.width);
        assert!(network_section.bottom() <= footer.top());
        let mode_group = window
            .debug_bounds("network-settings-proxy-mode")
            .expect("shared segmented proxy mode");
        assert_eq!(mode_group.size.height, px(28.0));
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

    #[gpui::test]
    fn invalid_proxy_uses_inline_error_above_the_fixed_footer(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
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

    #[gpui::test]
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

    #[gpui::test]
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

    #[gpui::test]
    fn close_to_tray_becomes_active_only_after_windows_confirms_it(cx: &mut TestAppContext) {
        let (application, directory, _policy) = application_fixture(cx, "confirmed-shell");
        let desired = SystemSettings {
            start_on_login_silently: true,
            close_to_tray: true,
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

    #[gpui::test]
    fn failed_shell_result_keeps_saved_intent_and_settings_window_open(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
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
            assert_eq!(
                view.error.as_deref(),
                Some("Windows 未能创建或显示 NxTools 托盘图标。请检查通知区域设置后重试。")
            );
        });
        let _ = std::fs::remove_dir_all(directory);
    }

    #[gpui::test]
    fn stale_shell_result_does_not_apply_or_close_the_current_window(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
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
            "找不到通过校验的 NxTools 程序。请修复或重新安装 NxTools 后重试。"
        );
        assert_eq!(
            global_shell_failure_message("platform_result", Some("startup_registration_failed")),
            "Windows 启动项未能写入当前用户注册表。请检查系统策略或安全软件后重试。"
        );
        assert_eq!(
            global_shell_failure_message("platform_result", Some("tray_icon_failed")),
            "Windows 未能创建或显示 NxTools 托盘图标。请检查通知区域设置后重试。"
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

    #[gpui::test]
    fn cancel_and_titlebar_close_release_the_owned_production_window(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
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
        assert_eq!(cancel.size.height, px(32.0));
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
        assert_eq!(close.size, size(px(28.0), px(28.0)));
        window.simulate_click(close.center(), Modifiers::default());
        window.run_until_parked();
        assert!(cx.read(|cx| application.read(cx).window.is_none()));
        assert!(
            cx.update(|cx| reopened.update(cx, |_, _, _| ())).is_err(),
            "the titlebar close must destroy the owned window, not only forget it"
        );
        let _ = std::fs::remove_dir_all(directory);
    }

    #[gpui::test]
    fn global_rule_layout_stays_aligned_for_long_many_empty_and_busy_states(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_component::init);
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
            assert!(table.bottom() < field.top() && field.bottom() < footer.top());
            assert!(
                field.size.width >= px(200.0),
                "input remains usable without window inflation"
            );
            assert!(table.right() <= px(700.0));
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
                view.saving = true;
                cx.notify();
            })
        });
        surface.refresh().expect("busy actions");
        let toggle = surface
            .debug_bounds("global-blacklist-new-quick")
            .expect("busy scope");
        surface.simulate_click(toggle.center(), Modifiers::default());
        let tab = surface
            .debug_bounds("global-settings-tab-general")
            .expect("busy navigation");
        surface.simulate_click(tab.center(), Modifiers::default());
        surface.run_until_parked();
        surface.read(|cx| {
            let view = view.read(cx);
            assert!(!view.new_disable_quick_launch);
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
}
