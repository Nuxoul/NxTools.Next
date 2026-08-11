use std::path::PathBuf;
use std::time::Instant;

use gpui::{
    AnyWindowHandle, Context, Entity, FontWeight, IntoElement, MouseButton, Render, Window,
    WindowBackgroundAppearance, WindowBounds, WindowControlArea, WindowDecorations, WindowKind,
    WindowOptions, div, prelude::*, px, size,
};
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::input::{Input, InputState};
use gpui_component::{Disableable, Icon, Root, Sizable};
use nxtools_core::{AppConfig, NetworkProxyMode, NetworkProxySettings, NetworkSettings};
use nxtools_network::{NetworkPolicy, proxy_mode_name};
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
    window: Option<AnyWindowHandle>,
}

impl NetworkSettingsApplication {
    pub(crate) fn new(
        policy: NetworkPolicy,
        config: AppConfig,
        config_path: PathBuf,
        notifications: Entity<NotificationCenter>,
    ) -> Self {
        Self {
            policy,
            config,
            config_path,
            notifications,
            window: None,
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
        let window_size = size(px(500.0), px(330.0));
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
                        application.window = None;
                    });
                    true
                });
                let view = cx.new(|cx| NetworkSettingsView::new(application, settings, window, cx));
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
        let Some(window) = self.window.take() else {
            return;
        };
        let _ = window.update(cx, |_, window, _| window.remove_window());
    }

    fn apply(
        &mut self,
        settings: NetworkSettings,
        correlation_id: uuid::Uuid,
        cx: &mut Context<Self>,
    ) -> Result<(), String> {
        let started_at = Instant::now();
        let mode = proxy_mode_name(settings.proxy.mode);
        info!(
            event = "network.settings.apply_started",
            %correlation_id,
            mode,
            "product network settings apply started"
        );

        // Persist first. Until the canonical AppConfig write succeeds every newly starting
        // operation continues to snapshot the old policy, so a rejected save can never leak into
        // an active browser, scheduled, clipboard, quick-launch, or download operation.
        let next_config =
            match AppConfig::update_network_settings(&self.config_path, settings.clone()) {
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
        self.window = None;
        self.notifications.update(cx, |notifications, cx| {
            notifications.show_success(
                "network-settings-saved",
                "全局网络设置已保存",
                "新开始或重新继续的联网操作将使用更新后的代理策略。",
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
        Ok(())
    }
}

struct NetworkSettingsView {
    theme: Theme,
    application: Entity<NetworkSettingsApplication>,
    mode: NetworkProxyMode,
    host: Entity<InputState>,
    port: Entity<InputState>,
    saving: bool,
    error: Option<String>,
}

impl NetworkSettingsView {
    fn new(
        application: Entity<NetworkSettingsApplication>,
        settings: NetworkSettings,
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
        let view = cx.entity().downgrade();
        let source_window = window.window_handle();
        cx.defer(move |cx| {
            let result = application.update(cx, |application, cx| {
                application.apply(settings, correlation_id, cx)
            });
            match result {
                Ok(()) => {
                    let _ = source_window.update(cx, |_, window, _| window.remove_window());
                }
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
                NetworkSettingsApplication::new(policy.clone(), config, config_path, notifications)
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
                    uuid::Uuid::new_v4(),
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
