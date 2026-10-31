//! Queue07 actual MainWindow consumers of shared ordinary presentation.
use super::*;
use gpui_kit::component::Theme as ComponentTheme;
use gpui_kit::{AppContext, Modifiers, SharedString, TestAppContext, VisualTestContext, size};
use nxtools_core::{AppConfig, CustomCommandSettings};
use nxtools_finder::FinderCatalog;

fn draw(surface: &mut VisualTestContext) {
    surface.run_until_parked();
    surface.update(|window, cx| {
        window.refresh();
        window.draw(cx).clear(cx);
    });
}
fn base(surface: &mut VisualTestContext, value: f32) {
    surface.update(|_, cx| ComponentTheme::update(cx, |theme| theme.font_size = px(value)));
    draw(surface);
    assert_eq!(surface.update(|window, _| window.rem_size()), px(value));
}
fn click(surface: &mut VisualTestContext, selector: &'static str) {
    let hit = surface
        .debug_bounds(selector)
        .unwrap_or_else(|| panic!("actual {selector}"));
    surface.simulate_click(hit.center(), Modifiers::default());
    draw(surface);
}
fn page(surface: &mut VisualTestContext, index: usize) {
    let id = gpui_kit::ElementId::from(SharedString::from(format!("0-{index}")));
    let hit = surface.update(|window, _| {
        gpui_kit::base::test_support::snapshots(window)
            .into_iter()
            .find(|node| node.path().last() == Some(&id))
            .expect("official Settings navigation")
            .bounds()
    });
    surface.simulate_click(hit.center(), Modifiers::default());
    draw(surface);
}
fn wheel(surface: &mut VisualTestContext, hit: gpui_kit::Point<gpui_kit::Pixels>, amount: f32) {
    surface.simulate_event(gpui_kit::ScrollWheelEvent {
        position: hit,
        delta: gpui_kit::ScrollDelta::Pixels(gpui_kit::point(px(0.0), px(amount))),
        ..Default::default()
    });
    draw(surface);
}
fn main_fixture(cx: &mut TestAppContext) -> (Entity<MainWindow>, VisualTestContext) {
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    let notifications = cx.new(|_| NotificationCenter::default());
    let retained = Rc::new(RefCell::new(None));
    let slot = retained.clone();
    let (_, surface) = cx.add_window_view(move |window, cx| {
        let main = cx.new(|cx| {
            MainWindow::new(
                gpui_kit::WeakEntity::new_invalid(),
                gpui_kit::WeakEntity::new_invalid(),
                gpui_kit::WeakEntity::new_invalid(),
                MainWindowShellContext {
                    settings: None,
                    platform: None,
                    system_settings: Rc::new(RefCell::new(SystemSettings::default())),
                },
                notifications,
                window,
                cx,
            )
        });
        *slot.borrow_mut() = Some(main.clone());
        gpui_kit::base::Root::new(main, window, cx)
    });
    let main = retained.borrow().clone().expect("retained production main");
    let handle = surface.update(|window, _| window.window_handle());
    (main, VisualTestContext::from_window(handle, cx))
}
#[gpui_kit::test]
fn audit06_shared_actual_main_headers_and_tools_keep_minimum_and_fixed_tile(
    cx: &mut TestAppContext,
) {
    let (main, mut surface) = main_fixture(cx);
    surface.simulate_resize(size(px(920.0), px(480.0)));
    let original = surface.read(|cx| ComponentTheme::global(cx).clone());
    for value in [16.0, 14.0, 18.0] {
        main.update(&mut surface, |_, cx| cx.notify());
        base(&mut surface, value);
        for (entry, header) in [
            ("main-module-下载", "download-module-header"),
            ("main-module-速启", "quick-launch-module-header"),
            ("main-module-剪贴板", "clipboard-module-header"),
            ("main-module-其他工具", "tools-module-header"),
        ] {
            click(&mut surface, entry);
            let rail = surface.debug_bounds(header).expect("actual module header");
            assert!((rail.size.height - px(48.0 * value / 16.0)).abs() <= px(0.5));
            assert!(rail.left() >= px(176.0) && rail.right() <= px(920.0));
            assert_eq!(
                surface
                    .debug_bounds("main-titlebar")
                    .expect("native caption")
                    .size
                    .height,
                px(36.0)
            );
        }
        let tile = surface
            .debug_bounds("tool-link-detector-card")
            .expect("approved tile");
        assert_eq!(
            tile.size,
            size(px(180.0), px(160.0)),
            "shared rem must not scale Tools tile"
        );
        click(&mut surface, "tool-link-detector-configure");
        let handle = main.read_with(&surface, |main, _| {
            *main
                .tool_settings_windows
                .get("tool-link-detector")
                .expect("original keyed settings window")
        });
        let mut tool = VisualTestContext::from_window(handle, &surface);
        tool.simulate_resize(size(px(460.0), px(320.0)));
        draw(&mut tool);
        let root = tool
            .debug_bounds("tool-settings-window")
            .expect("actual feature shell");
        assert_eq!(root.size, size(px(460.0), px(320.0)));
        for selector in [
            "tool-dwell-control",
            "tool-position-control",
            "tool-settings-toggle",
            "tool-settings-close",
        ] {
            let hit = tool.debug_bounds(selector).expect("actual Tool control");
            assert!(
                hit.left() >= root.left()
                    && hit.right() <= root.right()
                    && hit.top() >= root.top()
                    && hit.bottom() <= root.bottom(),
                "{selector}: {hit:?}"
            );
        }
        click(&mut tool, "tool-settings-toggle");
        assert!(tool.debug_bounds("tool-settings-form").is_some());
        let close = tool
            .debug_bounds("tool-settings-close")
            .expect("original close");
        tool.simulate_click(close.center(), Modifiers::default());
        surface.run_until_parked();
        assert!(surface.read(|cx| !cx.windows().contains(&handle)));
    }
    surface.update(|_, cx| ComponentTheme::update(cx, |theme| *theme = original));
}
#[gpui_kit::test]
fn audit06_shared_actual_quick_form_retains_inputs_and_original_save_at_minimum(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    let directory = tempfile::tempdir().expect("isolated Quick rem config");
    let config_path = directory.path().join("config.json");
    let mut config = AppConfig::default();
    config.quick_launch.custom_commands = vec![CustomCommandSettings {
        enabled: false,
        keyword: "synthetic".into(),
        title: "合成命令".into(),
        program: "synthetic.exe".into(),
        ..Default::default()
    }];
    config.save_to(&config_path).expect("synthetic config");
    let catalog =
        FinderCatalog::open(directory.path().join("history.db"), &[]).expect("no catalog roots");
    let (platform, _endpoint) = nxtools_ipc::platform_bridge(16);
    let (indexer, _indexer_endpoint) = crate::runtime::indexer_bridge(8);
    let notifications = cx.new(|_| NotificationCenter::default());
    let quick = cx.new(|_| {
        QuickLaunchApplication::new(
            catalog,
            Rc::new(platform),
            Rc::new(indexer),
            config,
            config_path.clone(),
            directory.path().join("logs"),
            notifications.clone(),
        )
    });
    let owner = quick.downgrade();
    let retained = Rc::new(RefCell::new(None));
    let slot = retained.clone();
    let (_, surface) = cx.add_window_view(move |window, cx| {
        let main = cx.new(|cx| {
            let mut main = MainWindow::new(
                gpui_kit::WeakEntity::new_invalid(),
                owner,
                gpui_kit::WeakEntity::new_invalid(),
                MainWindowShellContext {
                    settings: None,
                    platform: None,
                    system_settings: Rc::new(RefCell::new(SystemSettings::default())),
                },
                notifications,
                window,
                cx,
            );
            main.shell.activate(MainModule::QuickFind);
            main
        });
        *slot.borrow_mut() = Some(main.clone());
        gpui_kit::base::Root::new(main, window, cx)
    });
    surface.simulate_resize(size(px(920.0), px(480.0)));
    let original = surface.read(|cx| ComponentTheme::global(cx).clone());
    let main = retained.borrow().clone().expect("production MainWindow");
    let identity = main.read_with(surface, |main, _| main.custom_command_title.entity_id());
    for value in [16.0, 14.0, 18.0] {
        main.update(surface, |_, cx| cx.notify());
        base(surface, value);
        page(surface, 3);
        click(surface, "quick-launch-command-0");
        assert_eq!(
            main.read_with(surface, |main, _| main.selected_custom_command),
            Some(0)
        );
        let editor = surface
            .debug_bounds("quick-launch-custom-command-master-detail")
            .expect("production bounded editor");
        assert_eq!(editor.size.height, px(360.0));
        let input = main.read_with(surface, |main, _| main.custom_command_title.clone());
        assert_eq!(input.entity_id(), identity);
        surface.update(|window, cx| input.update(cx, |input, cx| input.focus(window, cx)));
        surface.simulate_keystrokes("ctrl-a");
        surface.simulate_input(&format!("合成已编辑{value}"));
        draw(surface);
        assert_eq!(
            input.read_with(surface, |input, _| input.value().to_string()),
            format!("合成已编辑{value}")
        );
        let scroll = surface
            .debug_bounds("quick-launch-command-form-scroll")
            .expect("original detail scroll");
        for _ in 0..5 {
            surface.simulate_event(gpui_kit::ScrollWheelEvent {
                position: gpui_kit::point(scroll.center().x, scroll.top() + px(80.0)),
                delta: gpui_kit::ScrollDelta::Lines(gpui_kit::point(0.0, -20.0)),
                ..Default::default()
            });
            draw(surface);
        }
        let save = surface
            .debug_bounds("quick-launch-save-custom-command")
            .expect("original Save");
        let viewport_bottom = editor.bottom().min(px(480.0));
        if save.bottom() > viewport_bottom {
            let content = surface
                .debug_bounds("quick-launch-kit-settings")
                .expect("original Settings viewport");
            wheel(
                surface,
                gpui_kit::point(content.right() - px(8.0), content.bottom() - px(20.0)),
                -1000.0,
            );
        }
        let save = surface
            .debug_bounds("quick-launch-save-custom-command")
            .expect("reachable original Save");
        let actual_scroll = surface
            .debug_bounds("quick-launch-command-form-scroll")
            .expect("current detail viewport");
        let actual_button = surface.update(|window, _| {
            gpui_kit::base::test_support::snapshots(window)
                .into_iter()
                .find(|node| {
                    node.path().last()
                        == Some(&gpui_kit::ElementId::from(
                            "quick-launch-save-custom-command",
                        ))
                })
                .expect("actual Save Button")
                .bounds()
        });
        assert!(
            save.top() >= px(36.0) && save.bottom() <= px(480.0),
            "Save {save:?}"
        );
        assert!(
            actual_button.left() >= actual_scroll.left()
                && actual_button.right() <= actual_scroll.right()
                && actual_button.top() >= actual_scroll.top()
                && actual_button.bottom() <= actual_scroll.bottom().min(px(480.0)),
            "actual Save must be inside clipped detail viewport: {actual_button:?} / {actual_scroll:?}"
        );
        surface.simulate_click(save.center(), Modifiers::default());
        draw(surface);
        let persisted: AppConfig =
            serde_json::from_slice(&std::fs::read(&config_path).expect("read owned config"))
                .expect("owned config JSON");
        assert_eq!(
            persisted.quick_launch.custom_commands[0].title,
            format!("合成已编辑{value}")
        );
        assert_eq!(
            quick.read_with(surface, |quick, _| quick.settings().custom_commands[0]
                .title
                .clone()),
            format!("合成已编辑{value}")
        );
        page(surface, 4);
        let log = surface
            .debug_bounds("quick-launch-diagnostics-log-row")
            .expect("actual mounted diagnostic");
        assert!((log.size.height - px(48.0 * value / 16.0)).abs() <= px(0.5));
        page(surface, 0);
        let hotkey = surface
            .debug_bounds("quick-launch-hotkey-field")
            .expect("actual recorder");
        assert!(hotkey.left() >= px(176.0) && hotkey.right() <= px(920.0));
    }
    surface.update(|_, cx| ComponentTheme::update(cx, |theme| *theme = original));
}
