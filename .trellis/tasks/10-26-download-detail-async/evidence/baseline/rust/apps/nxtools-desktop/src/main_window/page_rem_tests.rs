//! Queue08 actual shell and fixed-client Tools ordinary-role regressions.
use super::*;
use gpui_kit::component::Theme as ComponentTheme;
use gpui_kit::{AppContext, TestAppContext, VisualTestContext, size};
use nxtools_core::LinkPanelPosition;

fn draw(surface: &mut VisualTestContext) {
    surface.run_until_parked();
    surface.update(|window, cx| { window.refresh(); window.draw(cx).clear(cx); });
}
fn base(surface: &mut VisualTestContext, value: f32) {
    surface.update(|_, cx| ComponentTheme::update(cx, |theme| theme.font_size = px(value)));
    draw(surface);
    assert_eq!(surface.update(|window, _| window.rem_size()), px(value));
}
fn main_fixture(cx: &mut TestAppContext) -> (Entity<MainWindow>, VisualTestContext) {
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    let notifications = cx.new(|_| NotificationCenter::default());
    let retained = Rc::new(RefCell::new(None));
    let slot = retained.clone();
    let (_, surface) = cx.add_window_view(move |window, cx| {
        let main = cx.new(|cx| MainWindow::new(gpui_kit::WeakEntity::new_invalid(), gpui_kit::WeakEntity::new_invalid(), gpui_kit::WeakEntity::new_invalid(), MainWindowShellContext { settings: None, platform: None, system_settings: Rc::new(RefCell::new(SystemSettings::default())) }, notifications, window, cx));
        *slot.borrow_mut() = Some(main.clone());
        gpui_kit::base::Root::new(main, window, cx)
    });
    let main = retained.borrow().clone().expect("actual main");
    let handle = surface.update(|window, _| window.window_handle());
    (main, VisualTestContext::from_window(handle, cx))
}
#[gpui_kit::test]
fn audit06_pages_shell_ordinary_type_and_tools_insets_keep_fixed_contracts(cx: &mut TestAppContext) {
    let (main, mut surface) = main_fixture(cx);
    let original = surface.read(|cx| ComponentTheme::global(cx).clone());
    for viewport in [size(px(1060.0), px(650.0)), size(px(920.0), px(480.0))] {
        surface.simulate_resize(viewport);
        for value in [16.0, 14.0, 18.0] {
            main.update(&mut surface, |_, cx| cx.notify());
            base(&mut surface, value);
            let title = surface.debug_bounds("main-brand-title").expect("actual ordinary brand text");
            assert!((title.size.height - px(22.0 * value / 16.0)).abs() <= px(0.5), "ordinary line at{value}: {title:?}");
            assert_eq!(surface.debug_bounds("main-titlebar").expect("native caption").size.height, px(36.0));
            assert_eq!(surface.debug_bounds("main-brand-icon").expect("bitmap slot").size, size(px(18.0), px(18.0)));
            assert_eq!(surface.debug_bounds("main-sidebar").expect("approved rail").size.width, px(176.0));
            let tools = surface.debug_bounds("main-module-其他工具").expect("actual navigation Button");
            surface.simulate_click(tools.center(), Default::default());
            draw(&mut surface);
            let content = surface.debug_bounds("tools-content").expect("original workspace viewport");
            let tile = surface.debug_bounds("tool-link-detector-card").expect("original approved tile");
            assert_eq!(tile.size, size(px(180.0), px(160.0)));
            assert!((tile.left() - content.left() - px(16.0 * value / 16.0)).abs() <= px(0.5));
            assert!((tile.top() - content.top() - px(16.0 * value / 16.0)).abs() <= px(0.5));
        }
    }
    surface.update(|_, cx| ComponentTheme::update(cx, |theme| *theme = original));
}
#[gpui_kit::test]
fn audit06_pages_tool_form_coordinates_label_and_controls_inside_fixed_client(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    let retained = Rc::new(RefCell::new(None));
    let slot = retained.clone();
    let (_, surface) = cx.add_window_view(move |window, cx| {
        let body = cx.new(|_| tools_page::ToolSettingsWindowBody::foundation_fixture());
        *slot.borrow_mut() = Some(body.clone());
        gpui_kit::base::Root::new(body, window, cx)
    });
    surface.simulate_resize(size(px(460.0), px(320.0)));
    surface.update(|window, _| window.activate_window());
    let original = surface.read(|cx| ComponentTheme::global(cx).clone());
    let body = retained.borrow().clone().expect("original tool body");
    for value in [16.0, 14.0, 18.0] {
        body.update(surface, |_, cx| cx.notify());
        base(surface, value);
        let control = surface.debug_bounds("tool-dwell-control").expect("actual Form control");
        let root = surface.debug_bounds("tool-settings-window").expect("fixed original client");
        let inset = 12.0 * value / 16.0;
        let label = (264.0 * value / 16.0).min(460.0 - 2.0 * inset - 4.0 - 168.0 * value / 16.0);
        assert!((control.left() - px(inset + label + 4.0)).abs() <= px(0.5), "coordinated ordinary label rail at{value}: {control:?}");
        assert!(control.size.width >= px(168.0 * value / 16.0 - 0.5));
        assert!((control.size.height - px(32.0 * value / 16.0)).abs() <= px(0.5));
        assert!(control.right() <= root.right() && control.bottom() <= root.bottom());
        assert_eq!(root.size, size(px(460.0), px(320.0)));
    }
    let input = body.read_with(surface, |body, _| body.dwell_input());
    let identity = input.entity_id();
    surface.update(|window, cx| input.update(cx, |input, cx| input.focus(window, cx)));
    surface.simulate_keystrokes("ctrl-a");
    surface.simulate_input("14");
    draw(surface);
    assert_eq!(body.read_with(surface, |body, _| body.foundation_values().1), 14);
    assert_eq!(body.read_with(surface, |body, _| body.dwell_input().entity_id()), identity);
    surface.simulate_keystrokes("tab");
    draw(surface);
    surface.simulate_keystrokes("escape");
    assert_eq!(body.read_with(surface, |body, _| body.foundation_values().2), LinkPanelPosition::Center);
    surface.update(|_, cx| ComponentTheme::update(cx, |theme| *theme = original));
}
