use super::titlebar::{CaptionCapabilities, controlled_titlebar};
use crate::theme::Theme;
use crate::ui::metrics::SETTINGS_FOOTER_HEIGHT;
use gpui_kit::component::Sizable;
use gpui_kit::component::switch::Switch;
use gpui_kit::{
    AnyElement, FontWeight, IntoElement, Length, SharedString, Window, div, prelude::*, px,
};

/// Shared public SettingItem::render content: title-adjacent help and one
/// right-aligned control lane. Controls keep their own Kit geometry/owner.
pub(crate) fn aligned_setting_row(
    selector: impl Into<SharedString>,
    title: &'static str,
    help: &'static str,
    help_selector: impl Into<SharedString>,
    control_width: impl Into<Length>,
    control: AnyElement,
) -> AnyElement {
    let control_width: Length = control_width.into();
    let selector = selector.into();
    let title_selector = format!("{selector}-title");
    div()
        .debug_selector(move || selector.to_string())
        .w_full()
        .min_w_0()
        .min_h_8()
        .flex()
        .items_center()
        .gap_3()
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .items_center()
                .gap_1()
                .child(
                    div()
                        .debug_selector(move || title_selector.clone())
                        .text_sm()
                        .child(title),
                )
                .when(!help.is_empty(), |label| {
                    label.child(super::tooltip::description_help(help_selector, help))
                }),
        )
        .child(
            div()
                .w(control_width)
                .max_w(gpui_kit::relative(0.65))
                .min_w_0()
                .flex_none()
                .flex()
                .items_center()
                .justify_end()
                .child(control),
        )
        .into_any_element()
}
pub(crate) struct SettingsShellIds {
    pub root: &'static str,
    pub content: &'static str,
    pub footer: &'static str,
}

pub(crate) fn settings_shell(
    theme: Theme,
    ids: SettingsShellIds,
    titlebar: AnyElement,
    content: AnyElement,
    footer: AnyElement,
) -> AnyElement {
    div()
        .debug_selector(move || ids.root.into())
        .size_full()
        .bg(theme.canvas)
        .text_color(theme.text)
        .flex()
        .flex_col()
        .child(titlebar)
        .child(
            div()
                .debug_selector(move || ids.content.into())
                .w_full()
                .flex_1()
                .min_h_0()
                .overflow_hidden()
                .child(content),
        )
        .child(
            div()
                .debug_selector(move || ids.footer.into())
                .h(px(SETTINGS_FOOTER_HEIGHT))
                .w_full()
                .flex_none()
                .border_t_1()
                .border_color(theme.border_subtle)
                .bg(theme.surface)
                .child(footer),
        )
        .into_any_element()
}

pub(crate) struct FeatureSettingsWindowIds {
    pub root: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub status: &'static str,
    pub enabled_toggle: &'static str,
    pub close: &'static str,
    pub content: &'static str,
}

pub(crate) struct FeatureSettingsWindowSpec {
    pub ids: FeatureSettingsWindowIds,
    pub title: SharedString,
    pub description: SharedString,
    pub enabled_label: SharedString,
    pub enabled: bool,
}

pub(crate) fn feature_settings_window(
    theme: Theme,
    spec: FeatureSettingsWindowSpec,
    on_toggle: impl Fn(&bool, &mut Window, &mut gpui_kit::App) + 'static,
    on_close: impl Fn(&gpui_kit::ClickEvent, &mut Window, &mut gpui_kit::App) + 'static,
    content: AnyElement,
) -> AnyElement {
    let FeatureSettingsWindowSpec {
        ids,
        title,
        description,
        enabled_label,
        enabled,
    } = spec;
    let status = if enabled { "已启用" } else { "已停用" };
    div()
        .id(ids.root)
        .debug_selector(move || ids.root.into())
        .size_full()
        .bg(theme.canvas)
        .text_color(theme.text)
        .flex()
        .flex_col()
        .child(controlled_titlebar(
            ids.title,
            ids.close,
            title,
            CaptionCapabilities::MINIMIZE,
            on_close,
        ))
        .child(
            div()
                .id(ids.content)
                .debug_selector(move || ids.content.into())
                .w_full()
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .p_3()
                .child(
                    div()
                        .w_full()
                        .flex()
                        .flex_col()
                        .gap_3()
                        .child(
                            div()
                                .id(gpui_kit::SharedString::from(format!(
                                    "{}-enable-row",
                                    ids.root
                                )))
                                .debug_selector(move || format!("{}-enable-row", ids.root))
                                .w_full()
                                .min_h_8()
                                .flex_none()
                                .flex()
                                .items_center()
                                .gap_3()
                                .child(
                                    div()
                                        .debug_selector(move || {
                                            format!("{}-enable-label", ids.root)
                                        })
                                        .flex_1()
                                        .min_w_0()
                                        .flex()
                                        .items_center()
                                        .gap_1()
                                        .text_sm()
                                        .font_weight(FontWeight::MEDIUM)
                                        .child(enabled_label.clone())
                                        .child(super::tooltip::description_help(
                                            ids.description,
                                            description,
                                        )),
                                )
                                .child(
                                    div()
                                        .flex_none()
                                        .flex()
                                        .items_center()
                                        .gap_2()
                                        .child(
                                            div()
                                                .id(ids.status)
                                                .debug_selector(move || ids.status.into())
                                                .text_xs()
                                                .text_color(if enabled {
                                                    theme.success
                                                } else {
                                                    theme.text_secondary
                                                })
                                                .child(status),
                                        )
                                        .child(
                                            div()
                                                .debug_selector(move || ids.enabled_toggle.into())
                                                .flex_none()
                                                .child(
                                                    Switch::new(ids.enabled_toggle)
                                                        .checked(enabled)
                                                        .accessibility_label(enabled_label)
                                                        .on_change(on_toggle),
                                                ),
                                        ),
                                ),
                        )
                        .child(content),
                ),
        )
        .into_any_element()
}

pub(crate) fn settings_toggle(
    _theme: Theme,
    id: &'static str,
    enabled: bool,
    on_toggle: impl Fn(&bool, &mut Window, &mut gpui_kit::App) + 'static,
) -> AnyElement {
    div()
        .debug_selector(move || id.into())
        .flex_none()
        .w(px(36.0))
        .child(
            Switch::new(id)
                .checked(enabled)
                .with_size(gpui_kit::component::Size::Medium)
                .on_change(on_toggle),
        )
        .into_any_element()
}

pub(crate) fn dialog_footer(content: AnyElement) -> AnyElement {
    div()
        .size_full()
        .px_3()
        .flex()
        .items_center()
        .justify_end()
        .gap_2()
        .child(content)
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    use gpui_kit::base::Root;
    use gpui_kit::{Modifiers, Render, TestAppContext, size};

    use super::*;

    struct FeatureSettingsTestView {
        enabled: bool,
        close_requested: Rc<Cell<bool>>,
    }

    impl Render for FeatureSettingsTestView {
        fn render(
            &mut self,
            _window: &mut Window,
            cx: &mut gpui_kit::Context<Self>,
        ) -> impl IntoElement {
            let on_toggle = cx.listener(|this, _event, _window, cx| {
                this.enabled = !this.enabled;
                cx.notify();
            });
            let close_requested = Rc::clone(&self.close_requested);
            let on_close = cx.listener(move |_, _event, window, _cx| {
                close_requested.set(true);
                window.remove_window();
            });
            let content = div()
                .debug_selector(|| "feature-settings-test-body".into())
                .child("功能自己的设置")
                .into_any_element();
            feature_settings_window(
                Theme::zed_dark(),
                FeatureSettingsWindowSpec {
                    ids: FeatureSettingsWindowIds {
                        root: "feature-settings-test-root",
                        title: "feature-settings-test-title",
                        description: "feature-settings-test-description",
                        status: "feature-settings-test-status",
                        enabled_toggle: "feature-settings-test-toggle",
                        close: "feature-settings-test-close",
                        content: "feature-settings-test-content",
                    },
                    title: "测试工具".into(),
                    description: "功能说明".into(),
                    enabled_label: "启用测试工具".into(),
                    enabled: self.enabled,
                },
                on_toggle,
                on_close,
                content,
            )
        }
    }

    #[test]
    fn feature_settings_window_keeps_a_native_drag_region_outside_controls() {
        let source = include_str!("settings.rs");
        let feature_window = source
            .split("pub(crate) fn feature_settings_window(")
            .nth(1)
            .and_then(|source| source.split("pub(crate) fn settings_toggle(").next())
            .expect("feature settings shell source");
        assert!(feature_window.contains("controlled_titlebar("));
        assert!(feature_window.contains("CaptionCapabilities::MINIMIZE"));
    }

    #[gpui_kit::test]
    fn feature_settings_window_has_shared_status_toggle_close_and_content_slot(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_kit::init);
        let view_slot = Rc::new(RefCell::new(None));
        let close_requested = Rc::new(Cell::new(false));
        let view_slot_for_window = Rc::clone(&view_slot);
        let close_requested_for_window = Rc::clone(&close_requested);
        let (_root, window) = cx.add_window_view(move |window, cx| {
            window.resize(size(px(460.0), px(320.0)));
            let view = cx.new(|_| FeatureSettingsTestView {
                enabled: true,
                close_requested: close_requested_for_window,
            });
            view_slot_for_window.borrow_mut().replace(view.clone());
            Root::new(view, window, cx)
        });

        window.refresh().expect("render feature settings shell");
        let status = window
            .debug_bounds("feature-settings-test-status")
            .expect("enabled status label");
        let toggle = window
            .debug_bounds("feature-settings-test-toggle")
            .expect("shared enabled toggle");
        let close = window
            .debug_bounds("feature-settings-test-close")
            .expect("shared close action");
        assert!(status.origin.x < toggle.origin.x);
        assert!(
            toggle.top() > close.bottom(),
            "shared template puts feature state in body"
        );
        assert!(
            window
                .debug_bounds("feature-settings-test-root-enable-label")
                .is_some()
        );
        assert!(
            window.update(|window, _| gpui_kit::base::test_support::snapshots(window)
                .iter()
                .any(|snapshot| snapshot.label() == Some("启用测试工具"))),
            "shared template uses consumer-supplied enable label"
        );
        assert!(window.debug_bounds("feature-settings-test-body").is_some());

        window.simulate_click(toggle.center(), Modifiers::default());
        window.run_until_parked();
        assert!(!window.read(|cx| {
            view_slot
                .borrow()
                .as_ref()
                .expect("feature settings view")
                .read(cx)
                .enabled
        }));
        assert!(window.debug_bounds("feature-settings-test-body").is_some());

        window.refresh().expect("paint toggle keyboard focus");
        window.simulate_keystrokes("space");
        window.simulate_event(gpui_kit::KeyUpEvent {
            keystroke: gpui_kit::Keystroke::parse("space").expect("Space stroke"),
        });
        window.run_until_parked();
        assert!(
            window.read(|cx| view_slot
                .borrow()
                .as_ref()
                .expect("keyboard toggle view")
                .read(cx)
                .enabled),
            "Space release must toggle the same setting once"
        );
        assert_eq!(
            window.debug_bounds("feature-settings-test-toggle"),
            Some(toggle),
            "keyboard focus must not move or inflate the toggle"
        );

        window.simulate_click(close.center(), Modifiers::default());
        window.run_until_parked();
        assert!(close_requested.get());
    }
}

#[cfg(test)]
mod alignment_tests {
    use super::*;
    use gpui_kit::{Context, Render, TestAppContext, size};

    struct HeaderFixture {
        enabled: bool,
        long_description: bool,
    }
    impl Render for HeaderFixture {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            feature_settings_window(
                Theme::zed_dark(),
                FeatureSettingsWindowSpec {
                    ids: FeatureSettingsWindowIds {
                        root: "aligned-tool",
                        title: "aligned-title",
                        description: "aligned-description",
                        status: "aligned-status",
                        enabled_toggle: "aligned-switch",
                        close: "aligned-close",
                        content: "aligned-content",
                    },
                    title: "链接识别".into(),
                    description: if self.long_description {
                        "识别网页链接及现有文件路径；默认仅复制，不自动打开网页。合成超长说明用于验证布局而不读取用户内容。".repeat(3).into()
                    } else {
                        "识别网页链接及现有文件路径；默认仅复制，不自动打开网页".into()
                    },
                    enabled_label: "启用链接识别".into(),
                    enabled: self.enabled,
                },
                |_, _, _| {},
                |_, _, _| {},
                div().into_any_element(),
            )
        }
    }

    #[gpui_kit::test]
    fn foundation_alignment_header_has_one_title_action_centerline_and_separate_description(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_kit::init);
        cx.update(crate::theme::apply_component_theme);
        let (view, window) = cx.add_window_view(|_, _| HeaderFixture {
            enabled: false,
            long_description: false,
        });
        window.simulate_resize(size(px(460.0), px(320.0)));
        for (enabled, long_description) in [(false, false), (true, false), (false, true)] {
            view.update(window, |view, cx| {
                view.enabled = enabled;
                view.long_description = long_description;
                cx.notify();
            });
            window.refresh().expect("actual tool header frame");
            let title = window.debug_bounds("aligned-title").expect("title");
            let description = window
                .debug_bounds("aligned-description")
                .expect("description");
            let status = window.debug_bounds("aligned-status").expect("status");
            let toggle = window.debug_bounds("aligned-switch").expect("switch");
            let close = window.debug_bounds("aligned-close").expect("close");
            assert!((close.center().y - title.center().y).abs() <= px(1.0));
            assert!(
                description.size.width <= px(24.0),
                "description is a help trigger, not inline copy"
            );
            assert!(description.top() >= px(34.0));
            assert!((description.center().y - toggle.center().y).abs() <= px(1.0));
            assert!((status.center().y - toggle.center().y).abs() <= px(1.0));
            assert!(
                toggle.top() > close.bottom(),
                "feature switch is not a caption action"
            );
            assert!(status.right() < toggle.left());
            assert!(description.right() <= px(448.0));
            assert!(close.right() <= px(460.0));
        }
    }
}

#[cfg(test)]
mod consistency_tests {
    use super::*;
    use gpui_kit::{Context, Render, TestAppContext, size};
    struct ToggleFixture;
    impl Render for ToggleFixture {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child(settings_toggle(
                Theme::zed_dark(),
                "ux-setting-switch",
                true,
                |_, _, _| {},
            ))
        }
    }
    #[gpui_kit::test]
    fn ux_repair_settings_switch_uses_same_official_medium_size(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        let (_, surface) = cx.add_window_view(|window, cx| {
            gpui_kit::base::Root::new(cx.new(|_| ToggleFixture), window, cx)
        });
        surface.simulate_resize(size(px(300.0), px(100.0)));
        surface.refresh().expect("real settings switch");
        let nodes = surface.update(|window, _| gpui_kit::base::test_support::snapshots(window));
        let toggle = nodes
            .iter()
            .find(|node| node.role() == Some(gpui_kit::Role::Switch))
            .expect("actual switch semantic node")
            .bounds();
        assert_eq!(toggle.size, size(px(36.0), px(20.0)));
    }
}
