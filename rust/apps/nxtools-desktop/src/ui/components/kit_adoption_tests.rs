use super::{action::action_button, settings::settings_toggle};
use crate::theme::{Theme, apply_component_theme};
use gpui_kit::base::Root;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::switch::Switch;
use gpui_kit::{Context, IntoElement, Render, TestAppContext, Window, div, prelude::*};

#[gpui_kit::test]
fn kit_adoption_theme_preserves_released_type_and_radius(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let defaults = cx.read(|cx| {
        let theme = gpui_kit::component::Theme::global(cx);
        (
            theme.font_size,
            theme.mono_font_size,
            theme.radius,
            theme.radius_lg,
        )
    });
    cx.update(apply_component_theme);
    cx.read(|cx| {
        let theme = gpui_kit::component::Theme::global(cx);
        assert_eq!(
            (
                theme.font_size,
                theme.mono_font_size,
                theme.radius,
                theme.radius_lg
            ),
            defaults,
            "neutral color adaptation must not rebuild old typography or corners"
        );
        assert_ne!(
            theme.colors.background.a, 0.0,
            "ordinary Kit controls need an opaque surface; native outer alpha is separate"
        );
    });
}

struct GeometryFixture;
impl Render for GeometryFixture {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_4()
            .child(
                div()
                    .flex()
                    .gap_4()
                    .child(action_button("kit-adopted-action", "保存设置"))
                    .child(
                        Button::new("kit-reference-action")
                            .secondary()
                            .small()
                            .label("保存设置")
                            .debug_selector(|| "kit-reference-action".into()),
                    ),
            )
            .child(
                div()
                    .flex()
                    .gap_4()
                    .child(settings_toggle(
                        Theme::zed_dark(),
                        "kit-adopted-switch",
                        true,
                        |_, _, _| {},
                    ))
                    .child(
                        div()
                            .debug_selector(|| "kit-reference-switch".into())
                            .w(gpui_kit::px(36.0))
                            .child(Switch::new("kit-reference-switch").checked(true)),
                    ),
            )
    }
}

#[gpui_kit::test]
fn kit_adoption_shared_button_matches_released_small_geometry(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(apply_component_theme);
    let (_, window) = cx.add_window_view(|window, cx| {
        let view = cx.new(|_| GeometryFixture);
        Root::new(view, window, cx)
    });
    window
        .refresh()
        .expect("paint real shared and reference buttons");
    let adopted = window
        .debug_bounds("kit-adopted-action")
        .expect("adopted action");
    let reference = window
        .debug_bounds("kit-reference-action")
        .expect("released action");
    assert_eq!(
        adopted.size, reference.size,
        "shared action must preserve official small geometry, not force28px"
    );
}

#[gpui_kit::test]
fn kit_adoption_shared_switch_matches_released_medium_geometry(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(apply_component_theme);
    let (_, window) = cx.add_window_view(|window, cx| {
        let view = cx.new(|_| GeometryFixture);
        Root::new(view, window, cx)
    });
    window
        .refresh()
        .expect("paint real shared and reference switches");
    let adopted = window
        .debug_bounds("kit-adopted-switch")
        .expect("adopted switch");
    let reference = window
        .debug_bounds("kit-reference-switch")
        .expect("released switch");
    assert_eq!(
        adopted.size, reference.size,
        "shared toggle must be a Kit Switch, not a painted40x24 button"
    );
}

#[test]
fn kit_adoption_field_shell_retains_component_chrome() {
    let source = include_str!("field.rs");
    let field = source
        .split("pub(crate) fn field_shell(")
        .nth(1)
        .and_then(|source| source.split("pub(crate) fn path_picker_shell(").next())
        .expect("production styled-field boundary");
    assert!(!field.contains("appearance(false)"));
    assert!(!field.contains("bordered(false)"));
    assert!(field.contains("appearance(true)") && field.contains("focus_bordered(true)"));
}

struct FieldFixture {
    input: gpui_kit::Entity<gpui_kit::component::input::InputState>,
    reference: gpui_kit::Entity<gpui_kit::component::input::InputState>,
}
impl Render for FieldFixture {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        use gpui_kit::component::input::Input;
        div()
            .flex()
            .gap_4()
            .child(
                div()
                    .w(gpui_kit::px(200.0))
                    .child(super::field::field_shell(
                        Theme::zed_dark(),
                        "kit-adopted-field",
                        Input::new(&self.input),
                    )),
            )
            .child(div().w(gpui_kit::px(200.0)).child(
                div().debug_selector(|| "kit-reference-field".into()).child(
                    Input::new(&self.reference).with_size(gpui_kit::component::Size::Medium),
                ),
            ))
    }
}

#[gpui_kit::test]
fn kit_adoption_input_keeps_component_geometry_and_user_edit(cx: &mut TestAppContext) {
    use gpui_kit::component::input::InputState;
    cx.update(gpui_kit::init);
    cx.update(apply_component_theme);
    let (view, window) = cx.add_window_view(|window, cx| FieldFixture {
        input: cx.new(|cx| InputState::new(window, cx)),
        reference: cx.new(|cx| InputState::new(window, cx)),
    });
    window.refresh().expect("paint real input boundaries");
    let adopted = window
        .debug_bounds("kit-adopted-field")
        .expect("adopted input");
    let reference = window
        .debug_bounds("kit-reference-field")
        .expect("released input");
    assert_eq!(
        adopted.size.height, reference.size.height,
        "ordinary field uses the official medium tier"
    );
    window.simulate_click(adopted.center(), gpui_kit::Modifiers::default());
    window.simulate_input("合成输入");
    window.run_until_parked();
    assert_eq!(
        window.read(|cx| view.read(cx).input.read(cx).value().to_string()),
        "合成输入"
    );
}

#[test]
fn kit_adoption_segments_use_the_released_button_group() {
    let source = include_str!("action.rs");
    let group = source
        .split("pub(crate) fn segmented_control(")
        .nth(1)
        .and_then(|source| source.split("#[cfg(test)]").next())
        .expect("production group");
    assert!(
        group.contains("ButtonGroup::new("),
        "Kit must own connected button borders and corners"
    );
    assert!(!group.contains("ButtonCustomVariant::new("));
}
