use crate::icons::MaterialIcon;
use crate::theme::Theme;
use crate::ui::components::action::{focused_action, icon_action};
use crate::ui::metrics::{
    CONTROL_HEIGHT_ACTION, CONTROL_HEIGHT_DEFAULT, RADIUS_CONTROL, SETTINGS_FOOTER_HEIGHT,
    SETTINGS_ROW_MIN_HEIGHT, SPACING_4, SPACING_8, SPACING_12,
};
use crate::ui::typography::{
    LINE_HEIGHT_BODY, LINE_HEIGHT_LABEL, TEXT_BODY, TEXT_LABEL, UI_FONT_FAMILY,
};
use gpui::{
    AnyElement, FontWeight, IntoElement, MouseButton, SharedString, Window, WindowControlArea, div,
    prelude::*, px,
};
use gpui_component::button::ButtonVariants;
const TOGGLE_KNOB_SIZE: f32 = 16.0;
// Internal knob clearance, not a layout spacing token: 24 - two 2px borders - 16px knob.
const TOGGLE_KNOB_CLEARANCE: f32 = 2.0;

pub(crate) struct SettingsShellIds {
    pub root: &'static str,
    pub content: &'static str,
    pub footer: &'static str,
}

pub(crate) struct SettingsSplitSurfaceIds {
    pub surface: &'static str,
    pub primary: &'static str,
    pub divider: &'static str,
    pub secondary: &'static str,
}

pub(crate) fn settings_split_surface(
    theme: Theme,
    ids: SettingsSplitSurfaceIds,
    secondary_width: f32,
    primary: AnyElement,
    secondary: AnyElement,
) -> AnyElement {
    div()
        .id(ids.surface)
        .debug_selector(move || ids.surface.into())
        .w_full()
        .border_1()
        .border_color(theme.border_subtle)
        .bg(theme.surface)
        .flex()
        .child(
            div()
                .id(ids.primary)
                .debug_selector(move || ids.primary.into())
                .flex_1()
                .min_w_0()
                .px(px(SPACING_12))
                .child(primary),
        )
        .child(
            div()
                .id(ids.divider)
                .debug_selector(move || ids.divider.into())
                .w(px(1.0))
                .flex_none()
                .bg(theme.border_subtle),
        )
        .child(
            div()
                .id(ids.secondary)
                .debug_selector(move || ids.secondary.into())
                .w(px(secondary_width))
                .flex_none()
                .px(px(SPACING_12))
                .child(secondary),
        )
        .into_any_element()
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
        .font_family(UI_FONT_FAMILY)
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
    pub enabled: bool,
}

pub(crate) fn feature_settings_window(
    theme: Theme,
    spec: FeatureSettingsWindowSpec,
    on_toggle: impl Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static,
    on_close: impl Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static,
    content: AnyElement,
) -> AnyElement {
    let FeatureSettingsWindowSpec {
        ids,
        title,
        description,
        enabled,
    } = spec;
    let status = if enabled { "已启用" } else { "已停用" };
    div()
        .id(ids.root)
        .debug_selector(move || ids.root.into())
        .size_full()
        .bg(theme.canvas)
        .font_family(UI_FONT_FAMILY)
        .text_color(theme.text)
        .flex()
        .flex_col()
        .child(
            div()
                .h(px(68.0))
                .w_full()
                .flex_none()
                .px(px(SPACING_12))
                .border_b_1()
                .border_color(theme.border_subtle)
                .bg(theme.surface)
                .window_control_area(WindowControlArea::Drag)
                .flex()
                .items_center()
                .gap(px(SPACING_12))
                .child(
                    div()
                        .min_w_0()
                        .flex_1()
                        .flex()
                        .flex_col()
                        .gap(px(SPACING_4))
                        .child(
                            div()
                                .id(ids.title)
                                .debug_selector(move || ids.title.into())
                                .text_size(px(TEXT_BODY))
                                .line_height(px(LINE_HEIGHT_BODY))
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(title),
                        )
                        .child(
                            div()
                                .id(ids.description)
                                .debug_selector(move || ids.description.into())
                                .min_w_0()
                                .text_size(px(TEXT_LABEL))
                                .line_height(px(LINE_HEIGHT_LABEL))
                                .text_color(theme.text_muted)
                                .child(description),
                        ),
                )
                .child(
                    div()
                        .h_full()
                        .flex()
                        .items_center()
                        .gap(px(SPACING_12))
                        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                        .child(
                            div()
                                .id(ids.status)
                                .debug_selector(move || ids.status.into())
                                .flex_none()
                                .text_size(px(TEXT_LABEL))
                                .line_height(px(LINE_HEIGHT_LABEL))
                                .text_color(if enabled {
                                    theme.success
                                } else {
                                    theme.text_muted
                                })
                                .child(status),
                        )
                        .child(settings_toggle(
                            theme,
                            ids.enabled_toggle,
                            enabled,
                            on_toggle,
                        ))
                        .child(
                            icon_action(ids.close, MaterialIcon::Close, "关闭")
                                .ghost()
                                .on_click(on_close),
                        ),
                ),
        )
        .child(
            div()
                .id(ids.content)
                .debug_selector(move || ids.content.into())
                .w_full()
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .px(px(SPACING_12))
                .py(px(SPACING_12))
                .child(content),
        )
        .into_any_element()
}

pub(crate) fn settings_section(
    theme: Theme,
    selector: &'static str,
    title: &'static str,
    rows: Vec<AnyElement>,
) -> AnyElement {
    div()
        .debug_selector(move || selector.into())
        .w_full()
        .rounded(px(RADIUS_CONTROL))
        .border_1()
        .border_color(theme.border_subtle)
        .bg(theme.surface)
        .flex()
        .flex_col()
        .child(
            div()
                .h(px(CONTROL_HEIGHT_ACTION))
                .px(px(SPACING_8))
                .border_b_1()
                .border_color(theme.border_subtle)
                .flex()
                .items_center()
                .text_size(px(TEXT_LABEL))
                .line_height(px(LINE_HEIGHT_LABEL))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme.text_secondary)
                .child(title),
        )
        .children(rows)
        .into_any_element()
}

pub(crate) fn settings_row(
    theme: Theme,
    title: &'static str,
    description: impl Into<gpui::SharedString>,
    control: AnyElement,
) -> AnyElement {
    div()
        .min_h(px(SETTINGS_ROW_MIN_HEIGHT))
        .px(px(SPACING_8))
        .border_b_1()
        .border_color(theme.border_subtle)
        .flex()
        .items_center()
        .gap(px(SPACING_12))
        .child(
            div()
                .min_w_0()
                .flex_1()
                .flex()
                .flex_col()
                .gap(px(SPACING_4))
                .child(
                    div()
                        .text_size(px(TEXT_BODY))
                        .line_height(px(LINE_HEIGHT_BODY))
                        .child(title),
                )
                .child(
                    div()
                        .text_size(px(TEXT_LABEL))
                        .line_height(px(LINE_HEIGHT_LABEL))
                        .text_color(theme.text_muted)
                        .child(description.into()),
                ),
        )
        .child(control)
        .into_any_element()
}

pub(crate) fn settings_toggle(
    theme: Theme,
    id: &'static str,
    enabled: bool,
    on_toggle: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> AnyElement {
    SettingsToggle {
        theme,
        id,
        enabled,
        on_toggle: Box::new(on_toggle),
    }
    .into_any_element()
}

#[derive(gpui::IntoElement)]
struct SettingsToggle {
    theme: Theme,
    id: &'static str,
    enabled: bool,
    on_toggle: crate::ui::components::action::ClickHandler,
}

impl gpui::RenderOnce for SettingsToggle {
    fn render(self, window: &mut Window, cx: &mut gpui::App) -> impl IntoElement {
        let Self {
            theme,
            id,
            enabled,
            on_toggle,
        } = self;
        let focus = window
            .use_keyed_state(
                gpui::SharedString::from(format!("{id}-focus")),
                cx,
                |_, cx| cx.focus_handle().tab_stop(true),
            )
            .read(cx)
            .clone();
        focused_action(id, &focus, theme, false, window, on_toggle)
            .debug_selector(move || id.into())
            .w(px(40.0))
            .h(px(CONTROL_HEIGHT_DEFAULT))
            .p(px(TOGGLE_KNOB_CLEARANCE))
            .rounded(px(CONTROL_HEIGHT_DEFAULT / 2.0))
            .bg(if enabled {
                theme.accent_soft
            } else {
                theme.surface
            })
            .hover(move |style| {
                style.bg(if enabled {
                    theme.element_selected
                } else {
                    theme.surface_hover
                })
            })
            .active(move |style| style.bg(theme.surface_pressed))
            .flex()
            .items_center()
            .when(enabled, |toggle| toggle.justify_end())
            .child(
                div()
                    .size(px(TOGGLE_KNOB_SIZE))
                    .rounded(px(TOGGLE_KNOB_SIZE / 2.0))
                    .bg(if enabled {
                        theme.accent_text
                    } else {
                        theme.text_secondary
                    }),
            )
    }
}

pub(crate) fn dialog_footer(content: AnyElement) -> AnyElement {
    div()
        .size_full()
        .px(px(SPACING_12))
        .flex()
        .items_center()
        .justify_end()
        .gap(px(SPACING_8))
        .child(content)
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    use gpui::{Modifiers, Render, TestAppContext, size};
    use gpui_component::Root;

    use super::*;

    struct FeatureSettingsTestView {
        enabled: bool,
        close_requested: Rc<Cell<bool>>,
    }

    impl Render for FeatureSettingsTestView {
        fn render(
            &mut self,
            _window: &mut Window,
            cx: &mut gpui::Context<Self>,
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
            .and_then(|source| source.split("pub(crate) fn settings_section(").next())
            .expect("feature settings shell source");
        assert!(feature_window.contains(".window_control_area(WindowControlArea::Drag)"));
        assert!(
            feature_window
                .contains("on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())")
        );
    }

    #[gpui::test]
    fn feature_settings_window_has_shared_status_toggle_close_and_content_slot(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_component::init);
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
        assert!(toggle.origin.x < close.origin.x);
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
        window.simulate_event(gpui::KeyUpEvent {
            keystroke: gpui::Keystroke::parse("space").expect("Space stroke"),
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
