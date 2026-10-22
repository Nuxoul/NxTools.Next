use super::*;
use gpui_kit::component::sidebar::{Sidebar, SidebarGroup, SidebarItem};
use gpui_kit::component::{Collapsible, Selectable};
use gpui_kit::{ClickEvent, ElementId, RenderOnce};

type ModuleClickHandler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;

impl MainWindow {
    pub(crate) fn module_icon(module: MainModule) -> MaterialIcon {
        match module {
            MainModule::Downloads => MaterialIcon::Download,
            MainModule::QuickFind => MaterialIcon::Search,
            MainModule::Clipboard => MaterialIcon::ContentPaste,
            MainModule::Extensions => MaterialIcon::DataObject,
            MainModule::Archive => MaterialIcon::FolderZip,
            MainModule::Tools => MaterialIcon::Dashboard,
        }
    }

    pub(crate) fn render_extensions(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        if self.extension_manager.is_none()
            && let Some(application) = self.extension_application.clone()
        {
            self.extension_manager = Some(
                cx.new(|cx| extensions_page::ExtensionManagerView::new(application, window, cx)),
            );
        }
        let mut page = div()
            .id("extensions-workspace")
            .debug_selector(|| "extensions-workspace".into())
            .size_full()
            .min_w_0()
            .min_h_0()
            .flex()
            .flex_col()
            .bg(self.theme.canvas);
        if let Some(manager) = self.extension_manager.clone() {
            page = page.child(manager);
        } else {
            page = page.child(
                ModuleHeader::new(
                    "extensions-module-header",
                    "extensions-module-title",
                    "extensions-module-status",
                    "扩展",
                    "扩展管理当前不可用",
                )
                .render(self.theme),
            );
        }
        page.into_any_element()
    }

    pub(crate) fn render_titlebar(&self, window: &Window) -> AnyElement {
        let theme = self.theme;
        let network_settings = self.network_settings.clone();
        let global_settings_action = div()
            .id("global-network-settings")
            .debug_selector(|| "global-network-settings".into())
            .h(px(CONTROL_HEIGHT_ACTION))
            .flex()
            .items_center()
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
                            .h(px(LINE_HEIGHT_PAGE_TITLE))
                            .text_size(px(TEXT_PAGE_TITLE))
                            .line_height(px(LINE_HEIGHT_PAGE_TITLE))
                            .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                            .text_color(theme.text)
                            .child("NxTools"),
                    )
                    .child(
                        div()
                            .debug_selector(|| "main-active-module".into())
                            .h(px(LINE_HEIGHT_LABEL))
                            .text_size(px(TEXT_LABEL))
                            .line_height(px(LINE_HEIGHT_LABEL))
                            .text_color(theme.text_secondary)
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

    pub(crate) fn render_sidebar(
        &mut self,
        _window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let active = self.shell.active_module();
        let items = MainModule::ALL
            .into_iter()
            .enumerate()
            .map(|(index, module)| {
                let focus = self.module_focus[index].clone();
                let action_focus = focus.clone();
                FocusedModuleButton {
                    module,
                    focus,
                    selected: module == active,
                    on_click: Rc::new(cx.listener(move |this, _, window, cx| {
                        if this.shell.active_module() == MainModule::Extensions
                            && module != MainModule::Extensions
                        {
                            let owner = this
                                .extension_application
                                .as_ref()
                                .map(|app| app.downgrade());
                            cx.defer(move |cx| {
                                if let Some(owner) = owner {
                                    let _ =
                                        owner.update(cx, |app, cx| app.retire_import_preview(cx));
                                }
                            });
                        }
                        this.shell.activate(module);
                        // Unmounting a page must release its orphaned menu focus.
                        window.blur(cx);
                        action_focus.focus(window, cx);
                        cx.notify();
                        cx.stop_propagation();
                    })),
                }
            })
            .collect();
        // The wrapper is an observation/layout boundary only; Kit paints the rail
        // and its group label. The 20px label inset is the shared icon column.
        div()
            .debug_selector(|| "main-sidebar".into())
            .w(px(SIDEBAR_WIDTH))
            .h_full()
            .flex_none()
            .child(
                Sidebar::new("main-kit-sidebar")
                    .w_full()
                    .collapsible(false)
                    .child(SidebarGroup::new("工作区").child(FocusedModuleMenu { items })),
            )
            .into_any_element()
    }
}

// Kit 0.7.1 sidebar/menu.rs:94-110,286-390 exposes a click-only TreeItem,
// with no focus-handle/tab binding. Keep the real Sidebar and styled Buttons,
// not a second hand-painted row or a nested pointer/keyboard dispatcher.
#[derive(Clone)]
struct FocusedModuleMenu {
    items: Vec<FocusedModuleButton>,
}

impl Collapsible for FocusedModuleMenu {
    fn is_collapsed(&self) -> bool {
        false
    }
    fn collapsed(self, _collapsed: bool) -> Self {
        self
    }
}

impl SidebarItem for FocusedModuleMenu {
    fn render(
        self,
        _id: impl Into<ElementId>,
        _window: &mut Window,
        _cx: &mut App,
    ) -> impl IntoElement {
        div()
            .p(px(4.0))
            .flex()
            .flex_col()
            .gap_2()
            .children(self.items)
    }
}

#[derive(Clone, gpui_kit::IntoElement)]
struct FocusedModuleButton {
    module: MainModule,
    focus: FocusHandle,
    selected: bool,
    on_click: ModuleClickHandler,
}

impl RenderOnce for FocusedModuleButton {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = gpui_kit::SharedString::from(format!("main-module-{}", self.module.label()));
        // Component Button chooses a keyed FocusHandle internally (button.rs:629).
        // Seed that public seam in the same element scope before its render, so
        // Tab, pointer focus and page-unmount restoration share module_focus.
        let focus = self.focus;
        window.use_keyed_state(id.clone(), cx, move |_, _| focus);
        let debug_selector = id.clone();
        let icon_selector = format!("main-module-{}-icon-slot", self.module.label());
        let label_selector = format!("main-module-{}-label-slot", self.module.label());
        // Medium compact keeps the released 32px row and an 8px inset, the
        // same inset as Kit SidebarGroup/MenuItem labels.
        Button::new(id)
            .debug_selector(move || debug_selector.to_string())
            .ghost()
            .compact()
            .selected(self.selected)
            .accessibility_label(self.module.label())
            .child(
                div()
                    .w_full()
                    .min_w_0()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .debug_selector(move || icon_selector.clone())
                            .size_4()
                            .flex_none()
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(Icon::new(MainWindow::module_icon(self.module)).size_4()),
                    )
                    .child(
                        div()
                            .debug_selector(move || label_selector.clone())
                            .flex_1()
                            .min_w_0()
                            .text_left()
                            .truncate()
                            .child(self.module.label()),
                    ),
            )
            .tooltip(self.module.description())
            .w_full()
            .on_click(move |event, window, cx| (self.on_click)(event, window, cx))
            .render(window, cx)
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
    on_click: impl Fn(&gpui_kit::ClickEvent, &mut Window, &mut App) + 'static,
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
                style.bg(theme.danger).text_color(gpui_kit::white())
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
        .child(
            Button::new(id)
                .debug_selector(move || format!("{id}-keyboard-target"))
                .ghost()
                .w_full()
                .h_full()
                .rounded(px(crate::ui::metrics::RADIUS_NONE))
                .on_click(on_click)
                .child(
                    div()
                        .debug_selector(move || icon_selector.clone())
                        .size(px(12.0))
                        .child(icon),
                ),
        )
}
