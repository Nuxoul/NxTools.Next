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
    AppConfig, NetworkProxyMode, NetworkProxySettings, NetworkSettings, SystemSettings,
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
use crate::ui::metrics::{SPACING_6, SPACING_8, SPACING_12};
use crate::ui::typography::TEXT_LABEL;

const GLOBAL_SETTINGS_WINDOW_WIDTH: f32 = 720.0;
const GLOBAL_SETTINGS_WINDOW_HEIGHT: f32 = 368.0;

pub(crate) struct NetworkSettingsApplication {
    policy: NetworkPolicy,
    config: AppConfig,
    config_path: PathBuf,
    notifications: Entity<NotificationCenter>,
    platform: Option<Rc<PlatformBridge>>,
    system_settings: Rc<RefCell<SystemSettings>>,
    window: Option<AnyWindowHandle>,
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
            #[cfg(test)]
            settings_view: None,
            pending_apply: None,
            hidden_startup,
        }
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
                        if application.pending_apply.is_some() {
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
                let view = cx
                    .new(|cx| NetworkSettingsView::new(application, settings, system, window, cx));
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
        if self.pending_apply.is_some() {
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

    fn apply(
        &mut self,
        settings: NetworkSettings,
        system: SystemSettings,
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
        let next_config = match AppConfig::update_global_settings(
            &self.config_path,
            settings.clone(),
            system.clone(),
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
                return Err(format!("无法保存全局代理设置：{error}"));
            }
        };
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
        if let Some(platform) = self.platform.as_ref() {
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
        self.window = None;
        self.notifications.update(cx, |notifications, cx| {
            notifications.show_success(
                "network-settings-saved",
                "全局设置已保存",
                "系统壳层选项已应用；后续联网操作将使用更新后的代理策略。",
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
        self.finish_platform_apply_failure(pending, "platform_timeout", None, cx);
    }

    fn fail_close_live_shell_policy(&self) {
        self.system_settings.borrow_mut().close_to_tray = false;
    }

    fn finish_platform_apply_failure(
        &mut self,
        pending: PendingGlobalSettingsApply,
        failure_kind: &'static str,
        native_error_code: Option<i32>,
        cx: &mut Context<Self>,
    ) {
        if pending.user_operation {
            warn!(
                event = "network.settings.apply_failed",
                correlation_id = %pending.correlation_id,
                mode = pending.mode,
                failure_kind,
                native_error_code,
                elapsed_ms = pending.started_at.elapsed().as_millis(),
                "Windows did not confirm persisted product shell settings"
            );
        } else {
            warn!(
                event = "product.shell.reconcile_failed",
                correlation_id = %pending.correlation_id,
                failure_kind,
                native_error_code,
                elapsed_ms = pending.started_at.elapsed().as_millis(),
                "Windows did not confirm product shell settings reconciliation"
            );
        }
        if let Some(view) = pending.view {
            let _ = view.update(cx, |view, cx| {
                view.saving = false;
                view.error = Some("设置已保存，但 Windows 启动项或托盘设置应用失败".into());
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
    saving: bool,
    error: Option<String>,
}

impl NetworkSettingsView {
    fn new(
        application: Entity<NetworkSettingsApplication>,
        settings: NetworkSettings,
        system: SystemSettings,
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
        Self {
            theme: Theme::zed_dark(),
            application,
            mode: settings.proxy.mode,
            start_on_login_silently: system.start_on_login_silently,
            close_to_tray: system.close_to_tray,
            host,
            port,
            saving: false,
            error: None,
        }
    }

    fn save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.saving {
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
        self.saving = true;
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
                application.apply(settings, system, correlation_id, Some(completion), cx)
            });
            match result {
                Ok(true) => {
                    let _ = source_window.update(cx, |_, window, _| window.remove_window());
                }
                Ok(false) => {}
                Err(error) => {
                    let _ = view.update(cx, |view, cx| {
                        view.saving = false;
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
                        this.mode = mode;
                        this.error = None;
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
        let content = div()
            .size_full()
            .min_h_0()
            .overflow_y_scrollbar()
            .w_full()
            .p(px(SPACING_12))
            .flex()
            .flex_col()
            .gap(px(SPACING_8))
            .child(settings_section(
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
                                this.start_on_login_silently = !this.start_on_login_silently;
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
                                this.close_to_tray = !this.close_to_tray;
                                cx.notify();
                            }),
                        ),
                    ),
                ],
            ))
            .child(settings_section(
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
                            .w(px(274.0))
                            .flex()
                            .items_center()
                            .gap(px(SPACING_6))
                            .child(
                                div().flex_1().min_w_0().child(field_shell(
                                    theme,
                                    "network-settings-proxy-host-field",
                                    Input::new(&self.host)
                                        .appearance(false)
                                        .bordered(true)
                                        .disabled(!manual)
                                        .small(),
                                )),
                            )
                            .child(
                                div().w(px(82.0)).child(field_shell(
                                    theme,
                                    "network-settings-proxy-port-field",
                                    Input::new(&self.port)
                                        .appearance(false)
                                        .bordered(true)
                                        .disabled(!manual)
                                        .small(),
                                )),
                            )
                            .into_any_element(),
                    ),
                ],
            ))
            .child(if let Some(error) = error {
                inline_error(theme, "network-settings-inline-error", error)
            } else {
                div()
                    .debug_selector(|| "network-settings-policy-note".into())
                    .text_size(px(TEXT_LABEL))
                    .text_color(theme.text_muted)
                    .child("当前切片不提供 PAC 或代理密码输入；不会把敏感凭据明文写入配置。")
                    .into_any_element()
            })
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
                            .ghost()
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
        assert_eq!(actual_window_size, size(px(720.0), px(368.0)));
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
        let network_section = window
            .debug_bounds("network-settings-network-section")
            .expect("shared network section");
        assert_eq!(general_section.size.width, network_section.size.width);
        let policy_note = window
            .debug_bounds("network-settings-policy-note")
            .expect("bounded network policy note");
        let trailing_inset = footer.top() - policy_note.bottom();
        assert!(trailing_inset >= px(SPACING_12));
        assert!(trailing_inset <= px(SPACING_12 + 2.0));
        let start_toggle = window
            .debug_bounds("system-start-on-login")
            .expect("shared settings toggle");
        assert_eq!(start_toggle.size, size(px(40.0), px(22.0)));
        window.simulate_click(start_toggle.center(), Modifiers::default());
        window.run_until_parked();
        window.update(|_, cx| {
            assert!(
                view.read(cx).start_on_login_silently,
                "the production shared toggle must mutate the settings draft"
            );
        });
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
        assert_eq!(cancel.size.height, px(28.0));
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
}
