use super::*;
use gpui_kit::base::Root;
use gpui_kit::component::{
    Sizable,
    button::Button,
    group_box::{GroupBox, GroupBoxVariant, GroupBoxVariants},
    stepper::{Stepper, StepperItem},
};
use gpui_kit::{WindowBounds, WindowKind, WindowOptions, px, size};
use nxtools_core::AppConfig;
use tracing::{info, warn};

use crate::shell::MainModule;
use crate::theme::Theme;
use crate::ui::components::settings::{SettingsShellIds, dialog_footer, settings_shell};
use crate::ui::components::titlebar::client_titlebar;

const INTEGRATION_SETUP_WIDTH: f32 = 760.0;
const INTEGRATION_SETUP_HEIGHT: f32 = 540.0;
const INTEGRATION_SETUP_STEPS: [&str; 4] = ["欢迎", "浏览器下载", "压缩包关联", "完成"];
const INTEGRATION_SETUP_PAGE_COUNT: usize = INTEGRATION_SETUP_STEPS.len();
const ARCHIVE_PAGE: usize = 2;

struct IntegrationSetupView {
    main: gpui_kit::WeakEntity<MainWindow>,
    main_window: gpui_kit::AnyWindowHandle,
    downloads: gpui_kit::WeakEntity<DownloadApplication>,
    archive_settings: Option<gpui_kit::Entity<archive_page::ArchiveSettingsView>>,
    page: usize,
    _archive_observer: Option<gpui_kit::Subscription>,
}

impl IntegrationSetupView {
    fn new(
        main: gpui_kit::WeakEntity<MainWindow>,
        main_window: gpui_kit::AnyWindowHandle,
        downloads: gpui_kit::WeakEntity<DownloadApplication>,
        archive_settings: Option<gpui_kit::Entity<archive_page::ArchiveSettingsView>>,
        page: usize,
        cx: &mut Context<Self>,
    ) -> Self {
        // The archive page shows the native observation live; re-render when it arrives.
        let archive_observer = archive_settings
            .as_ref()
            .map(|settings| cx.observe(settings, |_, _, cx| cx.notify()));
        Self {
            main,
            main_window,
            downloads,
            archive_settings,
            page,
            _archive_observer: archive_observer,
        }
    }

    fn finish(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        info!(
            event = "desktop.integration_setup.finished",
            page = self.page,
            completed = self.page + 1 == INTEGRATION_SETUP_PAGE_COUNT,
            "integration setup closed"
        );
        let owner = self.main.clone();
        let main_window = self.main_window;
        window.remove_window();
        cx.defer(move |cx| {
            let _ = owner.update(cx, |main, _| main.dismiss_integration_setup());
            let _ = main_window.update(cx, |_, window, _| window.activate_window());
        });
    }

    fn next(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.page + 1 < INTEGRATION_SETUP_PAGE_COUNT {
            self.go_to(self.page + 1, cx);
        } else {
            self.finish(window, cx);
        }
    }

    fn back(&mut self, cx: &mut Context<Self>) {
        if self.page > 0 {
            self.go_to(self.page - 1, cx);
        }
    }

    fn go_to(&mut self, page: usize, cx: &mut Context<Self>) {
        if page < INTEGRATION_SETUP_PAGE_COUNT && page != self.page {
            self.page = page;
            cx.notify();
        }
    }

    fn page_header(theme: Theme, title: &'static str, description: &'static str) -> AnyElement {
        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .text_size(px(20.0))
                    .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                    .child(title),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(theme.text_secondary)
                    .child(description),
            )
            .into_any_element()
    }

    fn numbered_step(
        theme: Theme,
        number: &'static str,
        title: &'static str,
        detail: &'static str,
    ) -> AnyElement {
        div()
            .flex()
            .gap_3()
            .items_start()
            .child(
                div()
                    .size(px(28.0))
                    .flex_none()
                    .rounded(px(14.0))
                    .bg(theme.surface)
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(number),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(title)
                    .child(
                        div()
                            .min_w_0()
                            .text_sm()
                            .text_color(theme.text_secondary)
                            .child(detail),
                    ),
            )
            .into_any_element()
    }

    /// One effect of an integration choice: what changes and where the user will see it.
    fn effect_row(theme: Theme, title: &'static str, detail: &'static str) -> AnyElement {
        div()
            .flex()
            .gap_2()
            .items_start()
            .child(div().flex_none().text_color(theme.success).child("✓"))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .child(title)
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.text_secondary)
                            .child(detail),
                    ),
            )
            .into_any_element()
    }

    fn stepper(&self, cx: &mut Context<Self>) -> AnyElement {
        let owner = cx.entity().downgrade();
        div()
            .id("integration-setup-stepper")
            .debug_selector(|| "integration-setup-stepper".into())
            .w_full()
            .px_6()
            .pt_4()
            .pb_2()
            .child(
                Stepper::new("integration-setup-steps")
                    .selected_index(self.page)
                    .items(
                        INTEGRATION_SETUP_STEPS
                            .iter()
                            .map(|label| StepperItem::new().child(*label)),
                    )
                    .on_click(move |step, _, cx| {
                        let step = *step;
                        let _ = owner.update(cx, |view, cx| view.go_to(step, cx));
                    }),
            )
            .into_any_element()
    }

    fn welcome_page(&self, theme: Theme) -> AnyElement {
        let tiles = [
            MainModule::Downloads,
            MainModule::QuickFind,
            MainModule::Clipboard,
            MainModule::Archive,
        ]
        .into_iter()
        .fold(div().w_full().flex().flex_wrap().gap_3(), |grid, module| {
            grid.child(
                div().w(px(326.0)).flex_none().child(
                    GroupBox::new()
                        .title(module.label())
                        .with_variant(GroupBoxVariant::Outline)
                        .child(
                            div()
                                .text_sm()
                                .text_color(theme.text_secondary)
                                .child(module.description()),
                        ),
                ),
            )
        });
        div()
            .w_full()
            .flex()
            .flex_col()
            .gap_4()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_4()
                    .child(
                        img("brand/nxtools.png")
                            .size(px(56.0))
                            .flex_none()
                            .object_fit(ObjectFit::Contain),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(
                                div()
                                    .text_size(px(22.0))
                                    .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                                    .child("欢迎使用 NxTools"),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(theme.text_secondary)
                                    .child("花一分钟完成两项系统集成，之后在浏览器和资源管理器里就能直接用上 NxTools。"),
                            ),
                    ),
            )
            .child(tiles)
            .child(
                div()
                    .text_sm()
                    .text_color(theme.text_secondary)
                    .child("接下来的每一步都可以跳过；之后可在各功能的设置页调整集成选项。"),
            )
            .into_any_element()
    }

    fn browser_page(&self, theme: Theme) -> AnyElement {
        let downloads_for_settings = self.downloads.clone();
        let downloads_for_registration = self.downloads.clone();
        let downloads_for_chrome = self.downloads.clone();
        let downloads_for_firefox = self.downloads.clone();

        let steps = GroupBox::new()
            .title("安装步骤")
            .with_variant(GroupBoxVariant::Outline)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(Self::numbered_step(
                        theme,
                        "1",
                        "安装扩展",
                        "从右侧打开 Chrome / Edge 或 Firefox 商店并添加 NxTools 扩展",
                    ))
                    .child(Self::numbered_step(
                        theme,
                        "2",
                        "配置连接组件",
                        "商店版通常自动连接；受限环境点击“配置连接组件”，或在下载设置中导出本地包",
                    ))
                    .child(Self::numbered_step(
                        theme,
                        "3",
                        "测试连接",
                        "在扩展弹窗中点击“测试连接”，成功后浏览器下载会自动交给 NxTools",
                    )),
            );
        let actions = GroupBox::new()
            .title("开始安装")
            .with_variant(GroupBoxVariant::Outline)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        Button::new("integration-browser-chrome")
                            .label("打开 Chrome / Edge 商店")
                            .primary()
                            .with_size(gpui_kit::component::Size::Medium)
                            .on_click(move |_, _, cx| {
                                let downloads = downloads_for_chrome.clone();
                                cx.defer(move |cx| {
                                    let _ = downloads.update(cx, |downloads, cx| {
                                        downloads.open_chrome_store(cx);
                                    });
                                });
                            }),
                    )
                    .child(
                        Button::new("integration-browser-firefox")
                            .label("打开 Firefox 商店")
                            .outline()
                            .with_size(gpui_kit::component::Size::Medium)
                            .on_click(move |_, _, cx| {
                                let downloads = downloads_for_firefox.clone();
                                cx.defer(move |cx| {
                                    let _ = downloads.update(cx, |downloads, cx| {
                                        downloads.open_firefox_store(cx);
                                    });
                                });
                            }),
                    )
                    .child(
                        Button::new("integration-browser-register")
                            .label("配置连接组件")
                            .outline()
                            .with_size(gpui_kit::component::Size::Medium)
                            .on_click(move |_, _, cx| {
                                let downloads = downloads_for_registration.clone();
                                cx.defer(move |cx| {
                                    let _ = downloads.update(cx, |downloads, cx| {
                                        downloads
                                            .request_browser_registration(uuid::Uuid::new_v4(), cx);
                                    });
                                });
                            }),
                    )
                    .child(
                        Button::new("integration-browser-settings")
                            .label("打开下载设置")
                            .outline()
                            .with_size(gpui_kit::component::Size::Medium)
                            .on_click(move |_, _, cx| {
                                let downloads = downloads_for_settings.clone();
                                cx.defer(move |cx| {
                                    let _ = downloads.update(cx, |downloads, cx| {
                                        downloads.open_settings_window(cx);
                                    });
                                });
                            }),
                    ),
            );

        div()
            .w_full()
            .flex()
            .flex_col()
            .gap_4()
            .child(Self::page_header(
                theme,
                "接管浏览器下载",
                "安装扩展后，浏览器里的普通下载和需要登录的内容都会交给 NxTools 多线程下载。",
            ))
            .child(
                div()
                    .flex()
                    .gap_4()
                    .child(div().flex_1().min_w_0().child(steps))
                    .child(div().w(px(260.0)).flex_none().child(actions)),
            )
            .into_any_element()
    }

    fn archive_page(&self, theme: Theme, cx: &mut Context<Self>) -> AnyElement {
        let Some(settings) = self.archive_settings.clone() else {
            return div()
                .w_full()
                .flex()
                .flex_col()
                .gap_3()
                .child(Self::page_header(
                    theme,
                    "关联压缩包",
                    "压缩设置当前不可用：配置所有者尚未初始化。",
                ))
                .into_any_element();
        };
        let view = settings.read(cx);
        let pending = view.integration_pending;
        let status = view.integration_status_alert("integration-archive-status");
        let inline_settings = view.inline_integration_controls(settings.downgrade());
        let apply_target = settings.downgrade();

        let recommended = GroupBox::new()
            .title("推荐方案")
            .with_variant(GroupBoxVariant::Outline)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(Self::effect_row(
                        theme,
                        "默认程序",
                        "压缩包显示 NxTools 图标，双击直接用 NxTools 打开",
                    ))
                    .child(Self::effect_row(
                        theme,
                        "右键菜单",
                        "右键压缩包可解压、测试；右键文件或文件夹可直接压缩",
                    ))
                    .child(Self::effect_row(
                        theme,
                        "打开方式",
                        "保留在“打开方式”列表中，随时可以换回其他程序",
                    ))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .child(
                                Button::new("integration-archive-apply")
                                    .debug_selector(|| "integration-archive-apply".into())
                                    .label("一键关联（推荐）")
                                    .primary()
                                    .with_size(gpui_kit::component::Size::Medium)
                                    .loading(pending)
                                    .disabled(pending)
                                    .on_click(move |_, _, cx| {
                                        let target = apply_target.clone();
                                        cx.defer(move |cx| {
                                            let _ = target.update(cx, |settings, cx| {
                                                settings.apply_recommended_integration(cx);
                                            });
                                        });
                                    }),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .text_sm()
                                    .text_color(theme.text_secondary)
                                    .child("应用后资源管理器立即刷新，无需重启；可随时撤销。"),
                            ),
                    )
                    .child(status),
            );

        div()
            .w_full()
            .min_w_0()
            .flex()
            .flex_col()
            .gap_3()
            .child(Self::page_header(
                theme,
                "关联压缩包",
                "让资源管理器直接用 NxTools 打开、解压和创建压缩包。",
            ))
            .child(recommended)
            .child(
                div()
                    .text_sm()
                    .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                    .child("自定义"),
            )
            .child(
                div()
                    .id("integration-archive-inline-settings")
                    .debug_selector(|| "integration-archive-inline-settings".into())
                    .w_full()
                    .min_w_0()
                    .child(inline_settings),
            )
            .into_any_element()
    }

    fn complete_page(&self, theme: Theme, cx: &mut Context<Self>) -> AnyElement {
        let archive_status = self.archive_settings.as_ref().and_then(|settings| {
            let view = settings.read(cx);
            view.integration
                .is_some()
                .then(|| view.integration_status_alert("integration-complete-archive-status"))
        });
        let next_steps = GroupBox::new()
            .title("接下来可以")
            .with_variant(GroupBoxVariant::Outline)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(Self::numbered_step(
                        theme,
                        "1",
                        "在资源管理器中右键",
                        "压缩包可解压或测试，普通文件和文件夹可压缩为 .zip / .7z",
                    ))
                    .child(Self::numbered_step(
                        theme,
                        "2",
                        "在浏览器中直接下载",
                        "安装扩展后，下载会自动出现在 NxTools 的下载列表里",
                    ))
                    .child(Self::numbered_step(
                        theme,
                        "3",
                        "随时调整",
                        "可在下载设置中管理浏览器扩展，在压缩设置的“集成”页调整或撤销文件关联。",
                    )),
            );
        div()
            .w_full()
            .flex()
            .flex_col()
            .gap_4()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(
                        div()
                            .size(px(48.0))
                            .flex_none()
                            .rounded(px(24.0))
                            .bg(theme.success)
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_color(theme.canvas)
                            .child("✓"),
                    )
                    .child(Self::page_header(
                        theme,
                        "一切就绪",
                        "NxTools 会在后台待命，关闭主窗口后仍可从托盘使用。",
                    )),
            )
            .when_some(archive_status, |page, status| page.child(status))
            .child(next_steps)
            .into_any_element()
    }

    fn page_content(&self, theme: Theme, cx: &mut Context<Self>) -> AnyElement {
        match self.page {
            0 => self.welcome_page(theme),
            1 => self.browser_page(theme),
            ARCHIVE_PAGE => self.archive_page(theme, cx),
            _ => self.complete_page(theme, cx),
        }
    }

    fn navigation(&self, cx: &mut Context<Self>) -> AnyElement {
        let owner = cx.entity().downgrade();
        let first = self.page == 0;
        let last = self.page + 1 == INTEGRATION_SETUP_PAGE_COUNT;
        let mut footer = div().w_full().flex().items_center().gap_2();
        if first {
            let skip = owner.clone();
            footer = footer.child(
                Button::new("integration-setup-skip")
                    .label("跳过引导")
                    .ghost()
                    .with_size(gpui_kit::component::Size::Medium)
                    .on_click(move |_, window, cx| {
                        let _ = skip.update(cx, |view, cx| view.finish(window, cx));
                    }),
            );
        } else {
            let back = owner.clone();
            footer = footer.child(
                Button::new("integration-setup-back")
                    .label("上一步")
                    .outline()
                    .with_size(gpui_kit::component::Size::Medium)
                    .on_click(move |_, _, cx| {
                        let _ = back.update(cx, |view, cx| view.back(cx));
                    }),
            );
        }
        footer = footer.child(div().flex_1().min_w_0());
        if !first && !last {
            let later = owner.clone();
            footer = footer.child(
                Button::new("integration-setup-later")
                    .label("稍后设置")
                    .ghost()
                    .with_size(gpui_kit::component::Size::Medium)
                    .on_click(move |_, window, cx| {
                        let _ = later.update(cx, |view, cx| view.next(window, cx));
                    }),
            );
        }
        let next = owner.clone();
        footer = footer.child(
            Button::new(if last {
                "integration-setup-start"
            } else {
                "integration-setup-next"
            })
            .label(match self.page {
                0 => "开始设置",
                _ if last => "开始使用",
                _ => "下一步",
            })
            .primary()
            .with_size(gpui_kit::component::Size::Medium)
            .on_click(move |_, window, cx| {
                let _ = next.update(cx, |view, cx| view.next(window, cx));
            }),
        );
        dialog_footer(footer.into_any_element())
    }
}

impl Render for IntegrationSetupView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        crate::theme::observe_theme(window, cx);
        let theme = Theme::active(cx);
        let stepper = self.stepper(cx);
        let content = self.page_content(theme, cx);
        settings_shell(
            theme,
            SettingsShellIds {
                root: "integration-setup-window",
                content: "integration-setup-content",
                footer: "integration-setup-footer",
            },
            client_titlebar(
                theme,
                "integration-setup-titlebar",
                "integration-setup-close",
                "NxTools 使用引导",
                cx.listener(|view, _, window, cx| view.finish(window, cx)),
            ),
            div()
                .size_full()
                .min_w_0()
                .min_h_0()
                .flex()
                .flex_col()
                .child(stepper)
                .child(
                    div()
                        .id("integration-setup-content-scroll")
                        .debug_selector(|| "integration-setup-content-scroll".into())
                        .flex_1()
                        .w_full()
                        .min_w_0()
                        .min_h_0()
                        .overflow_y_scroll()
                        .child(div().w_full().min_w_0().px_6().py_3().child(content)),
                )
                .into_any_element(),
            self.navigation(cx),
        )
    }
}

impl MainWindow {
    pub(crate) fn maybe_show_integration_setup(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(existing) = self.integration_setup_window {
            if existing
                .update(cx, |_, window, _| {
                    window.activate_window();
                    window.refresh();
                })
                .is_ok()
            {
                return;
            }
            self.integration_setup_window = None;
        }
        if self.integration_setup_offered {
            return;
        }
        self.integration_setup_offered = true;

        let config_path = nxtools_core::AppPaths::discover()
            .ok()
            .map(|paths| paths.config_file());
        let dismissed = config_path
            .as_ref()
            .and_then(|path| AppConfig::load_from(path).ok())
            .map(|config| config.integration_setup_dismissed)
            .unwrap_or(false);
        if dismissed {
            return;
        }

        let owner = cx.entity().downgrade();
        let main_window = _window.window_handle();
        let downloads = self.downloads.clone();
        let archive_settings = self.archive_settings.clone();
        let window_size = size(px(INTEGRATION_SETUP_WIDTH), px(INTEGRATION_SETUP_HEIGHT));
        let window_bounds = WindowBounds::centered(window_size, cx);
        let opened = cx.open_window(
            WindowOptions {
                window_bounds: Some(window_bounds),
                focus: true,
                show: true,
                kind: WindowKind::Normal,
                is_movable: true,
                is_resizable: false,
                is_minimizable: true,
                display_id: None,
                app_id: Some("NxTools.IntegrationSetup".into()),
                window_min_size: Some(window_size),
                ..crate::ui::components::titlebar::caption_window_options()
            },
            move |window, cx| {
                let close_owner = owner.clone();
                window.on_window_should_close(cx, move |_, cx| {
                    let close_owner = close_owner.clone();
                    cx.defer(move |cx| {
                        let _ = close_owner.update(cx, |main, _| main.dismiss_integration_setup());
                        let _ = main_window.update(cx, |_, window, _| window.activate_window());
                    });
                    true
                });
                let view = cx.new(|cx| {
                    IntegrationSetupView::new(
                        owner.clone(),
                        main_window,
                        downloads.clone(),
                        archive_settings.clone(),
                        0,
                        cx,
                    )
                });
                cx.new(|cx| Root::new(view, window, cx))
            },
        );
        match opened {
            Ok(window) => {
                info!(
                    event = "desktop.integration_setup.opened",
                    "integration setup opened"
                );
                self.integration_setup_window = Some(window.into());
            }
            Err(error) => warn!(
                event = "desktop.integration_setup.open_failed",
                "integration setup window could not be opened: {error:#}"
            ),
        }
    }

    fn dismiss_integration_setup(&mut self) {
        self.integration_setup_offered = true;
        self.integration_setup_window = None;
        let Some(path) = nxtools_core::AppPaths::discover()
            .ok()
            .map(|paths| paths.config_file())
        else {
            warn!(
                event = "desktop.integration_setup.dismiss_failed",
                "application paths were unavailable"
            );
            return;
        };
        if let Err(error) = AppConfig::update_section(&path, |config| {
            config.integration_setup_dismissed = true;
        }) {
            warn!(
                event = "desktop.integration_setup.dismiss_failed",
                "failed to persist integration setup dismissal: {error:#}"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mount_setup(
        cx: &mut gpui_kit::TestAppContext,
        page: usize,
    ) -> (
        gpui_kit::Entity<IntegrationSetupView>,
        gpui_kit::Entity<archive_page::ArchiveSettingsView>,
        &mut gpui_kit::VisualTestContext,
        std::path::PathBuf,
    ) {
        cx.update(gpui_kit::init);
        cx.update(crate::theme::apply_component_theme);
        let config_path = std::env::temp_dir().join(format!(
            "nxtools-inline-archive-settings-{}.json",
            uuid::Uuid::new_v4()
        ));
        nxtools_core::AppConfig::default()
            .save_to(&config_path)
            .expect("seed inline archive settings");
        let config_path_for_window = config_path.clone();
        let mut handles = None;
        let (_root, window) = cx.add_window_view(|window, cx| {
            let archive_settings = cx.new(|cx| {
                archive_page::ArchiveSettingsView::new(
                    nxtools_core::archive::ArchiveSettings::default(),
                    config_path_for_window.clone(),
                    window,
                    cx,
                )
            });
            let main_window = window.window_handle();
            let settings_for_view = archive_settings.clone();
            let view = cx.new(|cx| {
                IntegrationSetupView::new(
                    gpui_kit::WeakEntity::<MainWindow>::new_invalid(),
                    main_window,
                    gpui_kit::WeakEntity::<DownloadApplication>::new_invalid(),
                    Some(settings_for_view),
                    page,
                    cx,
                )
            });
            handles = Some((view.clone(), archive_settings));
            gpui_kit::base::Root::new(view, window, cx)
        });
        let (view, settings) = handles.expect("mounted setup view");
        window.simulate_resize(size(
            px(INTEGRATION_SETUP_WIDTH),
            px(INTEGRATION_SETUP_HEIGHT),
        ));
        (view, settings, window, config_path)
    }

    #[gpui_kit::test]
    fn archive_page_renders_the_real_inline_settings_owner(cx: &mut gpui_kit::TestAppContext) {
        let (_view, _settings, window, config_path) = mount_setup(cx, ARCHIVE_PAGE);
        window
            .refresh()
            .expect("render inline archive integration settings");

        let viewport = window
            .debug_bounds("integration-setup-content-scroll")
            .expect("integration setup content viewport");
        let stepper = window
            .debug_bounds("integration-setup-stepper")
            .expect("official stepper above the scrolling content");
        let inline_settings = window
            .debug_bounds("integration-archive-inline-settings")
            .expect("inline archive settings panel");
        let cascade = window
            .debug_bounds("archive-shell-cascade")
            .expect("real archive integration switch");
        let set_default = window
            .debug_bounds("archive-shell-set-default")
            .expect("default program switch");
        let apply = window
            .debug_bounds("integration-archive-apply")
            .expect("recommended integration action");
        let status = window
            .debug_bounds("integration-archive-status")
            .expect("observed integration status");
        let last_command = window
            .debug_bounds("archive-menu-command-create")
            .expect("real archive menu command");
        assert!(stepper.bottom() <= viewport.top());
        assert!(inline_settings.size.width > px(600.0));
        assert!(inline_settings.right() <= viewport.right());
        assert!(cascade.left() >= inline_settings.left());
        assert!(set_default.right() <= inline_settings.right());
        assert!(apply.bottom() <= viewport.bottom());
        assert!(
            status.top() >= apply.bottom(),
            "the native result is shown right below the recommended action"
        );
        assert!(
            last_command.bottom() > viewport.bottom(),
            "long integration content must extend below the outer scroll viewport"
        );
        let _ = std::fs::remove_file(config_path);
    }

    #[gpui_kit::test]
    fn recommended_action_saves_default_claim_and_requests_apply(
        cx: &mut gpui_kit::TestAppContext,
    ) {
        let (_view, settings, window, config_path) = mount_setup(cx, ARCHIVE_PAGE);
        window.refresh().expect("render archive step");
        let apply = window
            .debug_bounds("integration-archive-apply")
            .expect("recommended integration action");
        window.simulate_click(apply.center(), gpui_kit::Modifiers::default());
        window.run_until_parked();
        let saved = nxtools_core::AppConfig::load_from(&config_path)
            .expect("saved recommended integration")
            .archive
            .shell_integration;
        assert!(saved.classic_cascade && saved.open_with && saved.set_default);
        assert_eq!(
            saved.extensions.len(),
            nxtools_core::archive::ARCHIVE_SHELL_EXTENSIONS.len()
        );
        // No archive owner is registered in this isolated mount: the explicit request must
        // surface a non-applied outcome instead of a fabricated success.
        assert!(settings.read_with(window, |view, _| matches!(
            view.integration,
            Some(crate::archive::shell_integration::IntegrationOutcome::NotDelivered(_))
        ) && !view.integration_pending));
        window.refresh().expect("render observed status");
        assert!(
            window.debug_bounds("integration-archive-status").is_some(),
            "the observed status re-renders inside the guide"
        );
        let _ = std::fs::remove_file(config_path);
    }

    #[gpui_kit::test]
    fn stepper_and_footer_navigate_between_pages(cx: &mut gpui_kit::TestAppContext) {
        let (view, _settings, window, config_path) = mount_setup(cx, 0);
        window.refresh().expect("render welcome step");
        view.update(window, |view, cx| view.go_to(ARCHIVE_PAGE, cx));
        assert_eq!(view.read_with(window, |view, _| view.page), ARCHIVE_PAGE);
        view.update(window, |view, cx| view.back(cx));
        assert_eq!(view.read_with(window, |view, _| view.page), 1);
        view.update(window, |view, cx| view.go_to(INTEGRATION_SETUP_PAGE_COUNT, cx));
        assert_eq!(
            view.read_with(window, |view, _| view.page),
            1,
            "out-of-range steps are ignored"
        );
        window.refresh().expect("render browser step");
        for selector in ["integration-setup-stepper", "integration-setup-content-scroll"] {
            assert!(window.debug_bounds(selector).is_some(), "{selector}");
        }
        let _ = std::fs::remove_file(config_path);
    }

    #[test]
    fn finishing_setup_closes_then_reactivates_the_main_window() {
        let source = include_str!("integration_setup.rs");
        let finish = source
            .split("fn finish(&mut self, window: &mut Window, cx: &mut Context<Self>)")
            .nth(1)
            .and_then(|body| body.split("fn next(").next())
            .expect("integration setup finish implementation");
        assert!(
            finish
                .find("window.remove_window()")
                .expect("finish closes the setup window")
                < finish
                    .find("cx.defer")
                    .expect("finish defers main-window work")
        );
        assert!(finish.contains("main_window.update"));
        assert!(finish.contains("window.activate_window()"));
    }
}
