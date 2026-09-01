use super::*;

impl MainWindow {
    pub(crate) fn module_icon(module: MainModule) -> MaterialIcon {
        match module {
            MainModule::Downloads => MaterialIcon::Download,
            MainModule::QuickFind => MaterialIcon::Search,
            MainModule::Clipboard => MaterialIcon::ContentPaste,
        }
    }

    pub(crate) fn render_titlebar(&self, window: &Window) -> AnyElement {
        let theme = self.theme;
        let network_settings = self.network_settings.clone();
        let global_settings_action = div()
            .id("global-network-settings")
            .debug_selector(|| "global-network-settings".into())
            .h(px(CONTROL_HEIGHT_ACTION))
            .child(
                labelled_icon_action(
                    "global-network-settings-button",
                    MaterialIcon::Settings,
                    "全局设置",
                )
                .ghost()
                .on_click(move |_, _, cx| {
                    let Some(network_settings) = network_settings.clone() else {
                        return;
                    };
                    cx.defer(move |cx| {
                        network_settings.update(cx, |application, cx| {
                            application.open_window(cx);
                        });
                    });
                }),
            );
        div()
            .id("main-titlebar")
            .debug_selector(|| "main-titlebar".into())
            .h(px(TITLEBAR_HEIGHT))
            .w_full()
            .flex()
            .items_center()
            .border_b_1()
            .border_color(theme.border_subtle)
            .bg(theme.canvas)
            .child(
                div()
                    .id("main-titlebar-drag-region")
                    .debug_selector(|| "main-titlebar-drag-region".into())
                    .h_full()
                    .flex_1()
                    .on_mouse_down(MouseButton::Left, {
                        let platform = self.platform.clone();
                        let window_handle = self.native_window_handle;
                        move |_, _, cx| {
                            if window_handle == 0 {
                                return;
                            }
                            if let Some(platform) = platform.as_ref() {
                                let _ = platform.send_command(PlatformCommand::BeginWindowMove(
                                    BeginWindowMoveCommand { window_handle },
                                ));
                            }
                            cx.stop_propagation();
                        }
                    })
                    .flex()
                    .items_center()
                    .gap(px(SPACING_8))
                    .pl(px(SPACING_12))
                    .child(
                        div()
                            .id("main-brand-icon")
                            .debug_selector(|| "main-brand-icon".into())
                            .size(px(18.0))
                            .rounded(px(RADIUS_CONTROL))
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(
                                img("brand/nxtools.png")
                                    .size(px(18.0))
                                    .object_fit(ObjectFit::Contain),
                            ),
                    )
                    .child(
                        div()
                            .debug_selector(|| "main-brand-title".into())
                            .h(px(LINE_HEIGHT_BODY))
                            .text_size(px(TEXT_BODY))
                            .line_height(px(LINE_HEIGHT_BODY))
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(theme.text_secondary)
                            .child("NxTools"),
                    )
                    .child(
                        div()
                            .debug_selector(|| "main-active-module".into())
                            .h(px(LINE_HEIGHT_LABEL))
                            .text_size(px(TEXT_LABEL))
                            .line_height(px(LINE_HEIGHT_LABEL))
                            .text_color(theme.text_muted)
                            .child(format!("· {}", self.shell.active_module().label())),
                    ),
            )
            .child(
                div()
                    .debug_selector(|| "main-titlebar-utilities".into())
                    .h_full()
                    .flex()
                    .items_center()
                    .gap(px(SPACING_4))
                    .child(global_settings_action)
                    .child(
                        div()
                            .debug_selector(|| "main-window-controls".into())
                            .h_full()
                            .flex()
                            .child(self.window_control(MainWindowControl::Minimize, theme))
                            .child(
                                self.window_control(maximize_control(window.is_maximized()), theme),
                            )
                            .child(self.window_control(MainWindowControl::Close, theme)),
                    ),
            )
            .into_any_element()
    }

    pub(crate) fn window_control(&self, control: MainWindowControl, theme: Theme) -> AnyElement {
        let platform = self.platform.clone();
        let window_handle = self.native_window_handle;
        let system_settings = Rc::clone(&self.system_settings);
        window_control(control, theme, move |_, window, cx| match control {
            MainWindowControl::Minimize if window_handle != 0 => {
                if let Some(platform) = platform.as_ref() {
                    let _ = platform.send_command(PlatformCommand::SetWindowMinimized(
                        SetWindowMinimizedCommand { window_handle },
                    ));
                }
            }
            MainWindowControl::Maximize | MainWindowControl::Restore if window_handle != 0 => {
                if let Some(platform) = platform.as_ref() {
                    let _ = platform.send_command(PlatformCommand::ToggleWindowMaximized(
                        ToggleWindowMaximizedCommand { window_handle },
                    ));
                }
            }
            MainWindowControl::Minimize => window.minimize_window(),
            MainWindowControl::Maximize | MainWindowControl::Restore => window.zoom_window(),
            MainWindowControl::Close
                if system_settings.borrow().close_to_tray && window_handle != 0 =>
            {
                if let Some(platform) = platform.as_ref() {
                    if platform
                        .send_command(PlatformCommand::SetWindowVisibility(
                            SetWindowVisibilityCommand {
                                window_handle,
                                visible: false,
                            },
                        ))
                        .is_err()
                    {
                        cx.quit();
                    }
                } else {
                    cx.quit();
                }
            }
            MainWindowControl::Close => cx.quit(),
        })
        .into_any_element()
    }

    pub(crate) fn render_sidebar(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme;
        let active = self.shell.active_module();
        let mut modules = div().flex().flex_col().gap(px(4.0));
        for module in MainModule::ALL {
            let selected = module == active;
            let selector = gpui::SharedString::from(format!("main-module-{}", module.label()));
            let debug_selector = selector.clone();
            let title_selector = format!("main-module-{}-title", module.label());
            let description_selector = format!("main-module-{}-description", module.label());
            modules = modules.child(
                div()
                    .id(selector)
                    .debug_selector(move || debug_selector.to_string())
                    .h(px(SIDEBAR_ROW_HEIGHT))
                    .px(px(10.0))
                    .rounded(px(RADIUS_CONTROL))
                    .border_l_2()
                    .border_color(if selected { theme.accent } else { theme.canvas })
                    .flex()
                    .items_center()
                    .gap(px(10.0))
                    .cursor_pointer()
                    .bg(if selected {
                        theme.surface_raised
                    } else {
                        theme.canvas
                    })
                    .hover(move |style| style.bg(theme.surface_hover))
                    .child(
                        Icon::new(Self::module_icon(module))
                            .size(px(18.0))
                            .text_color(if selected {
                                theme.accent
                            } else {
                                theme.text_secondary
                            }),
                    )
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .gap(px(1.0))
                            .child(
                                div()
                                    .debug_selector(move || title_selector.clone())
                                    .text_size(px(TEXT_BODY))
                                    .line_height(px(LINE_HEIGHT_BODY))
                                    .font_weight(if selected {
                                        gpui::FontWeight::MEDIUM
                                    } else {
                                        gpui::FontWeight::NORMAL
                                    })
                                    .text_color(theme.text)
                                    .child(module.label()),
                            )
                            .child(
                                div()
                                    .debug_selector(move || description_selector.clone())
                                    .text_size(px(TEXT_LABEL))
                                    .line_height(px(LINE_HEIGHT_LABEL))
                                    .text_color(theme.text_muted)
                                    .child(module.description()),
                            ),
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.shell.activate(module);
                        cx.notify();
                    })),
            );
        }

        div()
            .debug_selector(|| "main-sidebar".into())
            .w(px(SIDEBAR_WIDTH))
            .h_full()
            .p(px(10.0))
            .border_r_1()
            .border_color(theme.border_subtle)
            .bg(theme.canvas)
            .flex()
            .flex_col()
            .child(
                div().px(px(8.0)).pt(px(8.0)).pb(px(14.0)).child(
                    div()
                        .text_size(px(TEXT_LABEL))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(theme.text_muted)
                        .child("工作区"),
                ),
            )
            .child(modules)
            .child(div().flex_1())
            .into_any_element()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MainWindowControl {
    Minimize,
    Maximize,
    Restore,
    Close,
}

impl MainWindowControl {
    pub(crate) fn id(self) -> &'static str {
        match self {
            Self::Minimize => "window-control-minimize",
            Self::Maximize => "window-control-maximize",
            Self::Restore => "window-control-restore",
            Self::Close => "window-control-close",
        }
    }

    pub(crate) fn icon(self) -> CaptionIcon {
        match self {
            Self::Minimize => CaptionIcon::Minimize,
            Self::Maximize => CaptionIcon::Maximize,
            Self::Restore => CaptionIcon::Restore,
            Self::Close => CaptionIcon::Close,
        }
    }

    pub(crate) fn tooltip(self) -> &'static str {
        match self {
            Self::Minimize => "最小化",
            Self::Maximize => "最大化",
            Self::Restore => "还原",
            Self::Close => "关闭",
        }
    }
}

pub(crate) fn maximize_control(is_maximized: bool) -> MainWindowControl {
    if is_maximized {
        MainWindowControl::Restore
    } else {
        MainWindowControl::Maximize
    }
}

pub(crate) fn window_control(
    control: MainWindowControl,
    theme: Theme,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let close = matches!(control, MainWindowControl::Close);
    let id = control.id();
    let icon_selector = format!("{id}-icon");
    let tooltip_selector = format!("{id}-tooltip");
    let icon = Icon::new(control.icon()).size(px(12.0));
    let tooltip = control.tooltip();
    div()
        .id(id)
        .debug_selector(move || id.into())
        .w(px(TITLEBAR_HEIGHT))
        .h_full()
        .flex()
        .items_center()
        .justify_center()
        .text_color(theme.text_secondary)
        .hover(move |style| {
            if close {
                style.bg(gpui::rgb(0xc42b1c)).text_color(gpui::white())
            } else {
                style.bg(theme.surface_hover).text_color(theme.text)
            }
        })
        .tooltip(move |window, cx| {
            let tooltip_selector = tooltip_selector.clone();
            Tooltip::element(move |_, _| {
                let debug_selector = tooltip_selector.clone();
                div()
                    .debug_selector(move || debug_selector.clone())
                    .child(tooltip)
            })
            .build(window, cx)
        })
        .on_click(on_click)
        .child(
            div()
                .debug_selector(move || icon_selector.clone())
                .size(px(12.0))
                .child(icon),
        )
}
