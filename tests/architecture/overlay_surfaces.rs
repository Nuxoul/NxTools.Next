//! Actual official Dialog/Sheet outer paint commands through their public Styled slot.
//! TestPlatform evidence only; no HWND, native Acrylic or screenshot acceptance.
use gpui_kit::component::WindowExt as _;
use gpui_kit::{
    App, AppContext as _, Background, Context, IntoElement, Render, Styled as _, TestAppContext,
    VisualTestContext, Window, div, px, size,
};
use nxtools_core::{MaterialStyle, ThemePreference};
use std::time::Duration;

struct Fixture;
impl Render for Fixture {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

fn settle(surface: &mut VisualTestContext) {
    surface.update(|window, cx| window.draw(cx).clear(cx));
    surface.executor().advance_clock(Duration::from_millis(250));
    surface.run_until_parked();
    surface.update(|window, cx| {
        window.simulate_next_frame(cx);
        window.refresh();
        window.draw(cx).clear(cx);
    });
}

fn assert_outer_background(surface: &mut VisualTestContext, sheet: bool, expected: Background) {
    let bounds = if sheet {
        surface.update(|window, _| {
            gpui_kit::base::test_support::find(window, &[], &"sheet-content".into())
                .expect("official Sheet outer surface")
                .bounds()
        })
    } else {
        surface
            .debug_bounds("dialog-0")
            .expect("official Dialog outer surface")
    };
    assert!(bounds.size.width > px(100.) && bounds.size.height > px(50.));
    let expected = expected
        .as_solid()
        .expect("product solid-color overlay token");
    surface.update(|window, _| {
        let scale = window.scale_factor();
        let found = window.painted_quads().iter().any(|quad| {
            let same_bounds = [
                (
                    quad.bounds.origin.x.as_f32() / scale,
                    bounds.origin.x.as_f32(),
                ),
                (
                    quad.bounds.origin.y.as_f32() / scale,
                    bounds.origin.y.as_f32(),
                ),
                (
                    quad.bounds.size.width.as_f32() / scale,
                    bounds.size.width.as_f32(),
                ),
                (
                    quad.bounds.size.height.as_f32() / scale,
                    bounds.size.height.as_f32(),
                ),
            ]
            .into_iter()
            .all(|(actual, expected)| (actual - expected).abs() < 0.1);
            same_bounds
                && quad.background.as_solid().is_some_and(|color| {
                    [
                        (color.h, expected.h),
                        (color.s, expected.s),
                        (color.l, expected.l),
                        (color.a, expected.a),
                    ]
                    .into_iter()
                    .all(|(actual, expected)| (actual - expected).abs() < 0.00001)
                })
        });
        assert!(
            found,
            "official outer background must paint the selected overlay role"
        );
    });
}

fn verify_outer_slot(cx: &mut TestAppContext, sheet: bool) {
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    cx.update(|cx| cx.set_reduce_motion(true));
    cx.executor().allow_parking();
    let (_, surface) = cx.add_window_view(|window, cx| {
        let view = cx.new(|_| Fixture);
        gpui_kit::base::Root::new(view, window, cx)
    });
    surface.simulate_resize(size(px(900.), px(700.)));
    surface.update(|window, cx| {
        if sheet {
            window.open_sheet(cx, |sheet, _, cx| {
                sheet
                    .bg(crate::theme::overlay_background(cx))
                    .title("Sheet fixture")
            });
        } else {
            window.open_dialog(cx, |dialog, _, cx| {
                dialog
                    .bg(crate::theme::overlay_background(cx))
                    .title("Dialog fixture")
            });
        }
    });
    for preference in [ThemePreference::Light, ThemePreference::Dark] {
        for style in [
            MaterialStyle::Solid,
            MaterialStyle::Acrylic,
            MaterialStyle::Solid,
        ] {
            surface.update(|_, cx| crate::theme::set_appearance_preferences(preference, style, cx));
            settle(surface);
            let expected = surface.read(|cx: &App| {
                let theme = gpui_kit::component::Theme::global(cx);
                let expected = if style.is_frosted() {
                    assert_eq!(theme.colors.background.a, 0.0);
                    // Menus and dialogs are opaque Fluent flyouts on frosted styles.
                    assert_eq!(theme.colors.popover.a, 1.0);
                    theme.tokens.popover.background
                } else {
                    theme.tokens.background.background
                };
                assert_eq!(crate::theme::overlay_background(cx), expected);
                expected
            });
            assert_outer_background(surface, sheet, expected);
        }
    }
}

#[gpui_kit::test]
fn overlay_surfaces_dialog_public_slot_paints_outer_role_across_theme_switches(
    cx: &mut TestAppContext,
) {
    verify_outer_slot(cx, false);
}

#[gpui_kit::test]
fn overlay_surfaces_sheet_public_slot_paints_outer_role_across_theme_switches(
    cx: &mut TestAppContext,
) {
    verify_outer_slot(cx, true);
}
