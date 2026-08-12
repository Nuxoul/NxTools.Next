use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Instant;

use gpui::{
    AnyWindowHandle, Context, Entity, FontWeight, IntoElement, MouseButton, Render, WeakEntity,
    Window, WindowBackgroundAppearance, WindowBounds, WindowControlArea, WindowDecorations,
    WindowKind, WindowOptions, div, prelude::*, px, size,
};
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::input::{Input, InputState};
use gpui_component::{Disableable, Icon, Root, Sizable};
use nxtools_core::{
    AppConfig, NetworkProxyMode, NetworkProxySettings, NetworkSettings, SystemSettings,
};
use nxtools_ipc::{PlatformBridge, PlatformNotification};
use nxtools_network::{NetworkPolicy, proxy_mode_name};
use nxtools_protocol::{ConfigureDesktopShellCommand, PlatformCommand};
use tracing::{info, warn};

use crate::downloads::{
    download_primary_button_variant, download_settings_row, download_settings_section,
};
use crate::notifications::NotificationCenter;
use crate::theme::Theme;

pub(crate) struct NetworkSettingsApplication {
    policy: NetworkPolicy,
    config: AppConfig,
    config_path: PathBuf,
    notifications: Entity<NotificationCenter>,
    platform: Option<Rc<PlatformBridge>>,
    system_settings: Rc<RefCell<SystemSettings>>,
    window: Option<AnyWindowHandle>,
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
        let window_size = size(px(560.0), px(430.0));
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
                            true
                        }
                    })
                });
                let view = cx
                    .new(|cx| NetworkSettingsView::new(application, settings, system, window, cx));
                cx.new(|cx| Root::new(view, window, cx))
            },
        );
        match opened {
            Ok(window) => self.window = Some(window.into()),
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

fn network_settings_titlebar(
    theme: Theme,
    application: Entity<NetworkSettingsApplication>,
) -> impl IntoElement {
    div()
        .h(px(34.0))
        .pl(px(14.0))
        .border_b_1()
        .border_color(theme.border_subtle)
        .bg(theme.surface)
        .window_control_area(WindowControlArea::Drag)
        .flex()
        .items_center()
        .child(
            div()
                .flex_1()
                .text_size(px(12.0))
                .font_weight(FontWeight::SEMIBOLD)
                .child("全局设置"),
        )
        .child(
            div()
                .id("network-settings-close")
                .w(px(38.0))
                .h_full()
                .cursor_pointer()
                .flex()
                .items_center()
                .justify_center()
                .hover(move |style| style.bg(theme.danger).text_color(gpui::white()))
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .on_click(move |_, _, cx| {
                    let application = application.clone();
                    cx.defer(move |cx| {
                        application.update(cx, |application, cx| application.close_window(cx));
                    });
                })
                .child(Icon::new(crate::icons::MaterialIcon::Close).size(px(15.0))),
        )
}

impl Render for NetworkSettingsView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let manual = self.mode == NetworkProxyMode::ManualHttp;
        let mode_buttons = [
            (NetworkProxyMode::Direct, "直连"),
            (NetworkProxyMode::System, "系统代理"),
            (NetworkProxyMode::ManualHttp, "手动 HTTP"),
        ]
        .into_iter()
        .fold(div().flex().gap(px(6.0)), |row, (mode, label)| {
            row.child(
                Button::new(gpui::SharedString::from(format!("network-mode-{mode:?}")))
                    .label(label)
                    .small()
                    .compact()
                    .when(self.mode == mode, |button| {
                        button.custom(download_primary_button_variant(theme, cx))
                    })
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.mode = mode;
                        this.error = None;
                        cx.notify();
                    })),
            )
        });
        let error = self.error.clone();
        let start_on_login_silently = self.start_on_login_silently;
        let close_to_tray = self.close_to_tray;
        div()
            .size_full()
            .bg(theme.canvas)
            .text_color(theme.text)
            .flex()
            .flex_col()
            .child(network_settings_titlebar(theme, self.application.clone()))
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .p(px(14.0))
                    .flex()
                    .flex_col()
                    .gap(px(10.0))
                    .child(download_settings_section(
                        theme,
                        "常规",
                        vec![
                            download_settings_row(
                                theme,
                                "开机静默启动",
                                "登录 Windows 后在后台启动，不主动弹出主窗口",
                                global_settings_toggle(
                                    theme,
                                    "system-start-on-login",
                                    start_on_login_silently,
                                    cx.listener(|this, _, _, cx| {
                                        this.start_on_login_silently =
                                            !this.start_on_login_silently;
                                        cx.notify();
                                    }),
                                ),
                            ),
                            download_settings_row(
                                theme,
                                "关闭到托盘",
                                "关闭主窗口时继续在后台运行；托盘左键恢复，右键可退出",
                                global_settings_toggle(
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
                    .child(download_settings_section(
                        theme,
                        "网络与代理",
                        vec![
                            download_settings_row(
                                theme,
                                "代理模式",
                                "下载、剪贴板和速启中的实际联网操作共享；本地 IPC 与索引不使用",
                                mode_buttons.into_any_element(),
                            ),
                            download_settings_row(
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
                                    .gap(px(6.0))
                                    .child(
                                        div().flex_1().child(
                                            Input::new(&self.host)
                                                .appearance(false)
                                                .bordered(true)
                                                .disabled(!manual)
                                                .small(),
                                        ),
                                    )
                                    .child(
                                        div().w(px(82.0)).child(
                                            Input::new(&self.port)
                                                .appearance(false)
                                                .bordered(true)
                                                .disabled(!manual)
                                                .small(),
                                        ),
                                    )
                                    .into_any_element(),
                            ),
                        ],
                    ))
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(theme.text_muted)
                            .child(
                                "当前切片不提供 PAC 或代理密码输入；不会把敏感凭据明文写入配置。",
                            ),
                    )
                    .when_some(error, |panel, error| {
                        panel.child(
                            div()
                                .rounded(px(4.0))
                                .bg(theme.danger_soft)
                                .px(px(9.0))
                                .py(px(7.0))
                                .text_size(px(10.0))
                                .text_color(theme.danger)
                                .child(error),
                        )
                    }),
            )
            .child(
                div()
                    .h(px(48.0))
                    .px(px(14.0))
                    .border_t_1()
                    .border_color(theme.border_subtle)
                    .bg(theme.surface)
                    .flex()
                    .items_center()
                    .justify_end()
                    .gap(px(8.0))
                    .child(
                        Button::new("network-settings-cancel")
                            .label("取消")
                            .small()
                            .ghost()
                            .on_click({
                                let application = self.application.clone();
                                move |_, _, cx| {
                                    let application = application.clone();
                                    cx.defer(move |cx| {
                                        application.update(cx, |application, cx| {
                                            application.close_window(cx);
                                        });
                                    });
                                }
                            }),
                    )
                    .child(
                        div()
                            .debug_selector(|| "network-settings-save".into())
                            .child(
                                Button::new("network-settings-save-button")
                                    .label(if self.saving {
                                        "正在应用…"
                                    } else {
                                        "应用"
                                    })
                                    .small()
                                    .disabled(self.saving)
                                    .custom(download_primary_button_variant(theme, cx))
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.save(window, cx);
                                    })),
                            ),
                    ),
            )
    }
}

fn global_settings_toggle(
    theme: Theme,
    id: &'static str,
    enabled: bool,
    on_toggle: impl Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static,
) -> gpui::AnyElement {
    div()
        .id(id)
        .debug_selector(move || id.into())
        .w(px(46.0))
        .h(px(22.0))
        .p(px(2.0))
        .rounded(px(11.0))
        .border_1()
        .border_color(if enabled {
            theme.border_active
        } else {
            theme.border_subtle
        })
        .bg(if enabled {
            theme.accent_soft
        } else {
            theme.canvas
        })
        .cursor_pointer()
        .flex()
        .when(enabled, |toggle| toggle.justify_end())
        .on_click(on_toggle)
        .child(div().size(px(16.0)).rounded(px(8.0)).bg(if enabled {
            theme.accent
        } else {
            theme.text_muted
        }))
        .into_any_element()
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
    use gpui::{AppContext, Modifiers, TestAppContext};

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
        let application_for_window = application.clone();
        let (view, window) = cx.add_window_view(move |window, cx| {
            NetworkSettingsView::new(
                application_for_window,
                NetworkSettings::default(),
                SystemSettings::default(),
                window,
                cx,
            )
        });
        window.update(|window, cx| {
            view.update(cx, |view, cx| {
                view.mode = NetworkProxyMode::ManualHttp;
                view.host.update(cx, |input, cx| {
                    input.set_value("127.0.0.1", window, cx);
                });
                view.port.update(cx, |input, cx| {
                    input.set_value("3128", window, cx);
                });
            });
        });
        window.refresh().expect("render network settings");
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
    fn window_open_reuses_and_close_releases_the_owned_window(cx: &mut TestAppContext) {
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

        cx.update(|cx| application.update(cx, |application, cx| application.close_window(cx)));
        cx.run_until_parked();
        assert!(cx.read(|cx| application.read(cx).window.is_none()));
        assert!(
            cx.update(|cx| first.update(cx, |_, _, _| ())).is_err(),
            "the owned window must be destroyed, not only forgotten"
        );
        let _ = std::fs::remove_dir_all(directory);
    }
}
