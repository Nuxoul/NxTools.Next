//! Exact released Settings capability checks before production adoption.
use super::*;
use gpui_kit::component::group_box::GroupBoxVariant;
use gpui_kit::component::setting::{SelectIndex, SettingGroup, SettingItem, SettingPage, Settings};
use std::cell::Cell;

struct SettingsFixture {
    initial_page: usize,
    rendered_page: Rc<Cell<Option<usize>>>,
}

impl Render for SettingsFixture {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let pages = ["常规", "索引", "上下文", "命令", "诊断"]
            .into_iter()
            .enumerate()
            .map(|(index, title)| {
                let rendered = self.rendered_page.clone();
                SettingPage::new(title).resettable(false).group(
                    SettingGroup::new()
                        .variant(GroupBoxVariant::Normal)
                        .item(SettingItem::render(move |_, _, _| {
                            rendered.set(Some(index));
                            div().h(px(32.0)).child(title)
                        })),
                )
            });
        div().size_full().child(
            Settings::new("quick-settings-capability-fixture")
                .sidebar_width(px(160.0))
                .default_selected_index(SelectIndex {
                    page_ix: self.initial_page,
                    group_ix: None,
                })
                .pages(pages),
        )
    }
}

#[gpui_kit::test]
fn released_settings_always_mounts_a_search_input(cx: &mut gpui_kit::TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    let (_, surface) = cx.add_window_view(|window, cx| {
        let view = cx.new(|_| SettingsFixture {
            initial_page: 0,
            rendered_page: Rc::new(Cell::new(None)),
        });
        gpui_kit::base::Root::new(view, window, cx)
    });
    surface.simulate_resize(gpui_kit::size(px(744.0), px(390.0)));
    surface
        .refresh()
        .expect("render exact released Settings hierarchy");
    let inputs = surface.update(|window, _| {
        gpui_kit::base::test_support::snapshots(window)
            .into_iter()
            .filter(|snapshot| snapshot.role() == Some(gpui_kit::Role::TextInput))
            .collect::<Vec<_>>()
    });
    assert_eq!(
        inputs.len(),
        1,
        "Settings adds a search Input even when every item is custom content"
    );
    assert!(inputs[0].visible());
    assert!(inputs[0].bounds().size.width > px(0.0));
    assert!(inputs[0].bounds().size.height > px(0.0));
}

#[gpui_kit::test]
fn released_settings_default_selection_is_not_a_controlled_page_binding(
    cx: &mut gpui_kit::TestAppContext,
) {
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    let rendered = Rc::new(Cell::new(None));
    let observed = rendered.clone();
    let view_slot = Rc::new(RefCell::new(None));
    let retained = view_slot.clone();
    let (_, surface) = cx.add_window_view(move |window, cx| {
        let view = cx.new(|_| SettingsFixture {
            initial_page: 0,
            rendered_page: observed,
        });
        *retained.borrow_mut() = Some(view.clone());
        gpui_kit::base::Root::new(view, window, cx)
    });
    surface.simulate_resize(gpui_kit::size(px(744.0), px(390.0)));
    surface.refresh().expect("initial settings page");
    assert_eq!(rendered.get(), Some(0));
    let view = view_slot.borrow().as_ref().expect("fixture owner").clone();
    rendered.set(None);
    surface.update(|_, cx| {
        view.update(cx, |view, cx| {
            view.initial_page = 1;
            cx.notify();
        })
    });
    surface
        .refresh()
        .expect("same keyed Settings with changed initial page");
    assert_eq!(
        rendered.get(),
        Some(0),
        "changing default_selected_index cannot synchronize the existing page owner"
    );
}

#[gpui_kit::test]
fn quick_settings_production_mounts_official_search_and_hierarchy(
    cx: &mut gpui_kit::TestAppContext,
) {
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    let (_, surface) = cx.add_window_view(|window, cx| {
        let notifications = cx.new(|_| NotificationCenter::default());
        let view = cx.new(|cx| {
            let mut main = MainWindow::new(
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
            );
            main.shell.activate(MainModule::QuickFind);
            main
        });
        gpui_kit::base::Root::new(view, window, cx)
    });
    surface.simulate_resize(gpui_kit::size(px(1060.0), px(650.0)));
    surface.refresh().expect("production Quick Find Settings");
    for selector in [
        "kbd:up",
        "kbd:down",
        "kbd:enter",
        "kbd:ctrl-enter",
        "kbd:escape",
    ] {
        let key = surface
            .debug_bounds(selector)
            .expect("official shortcut on General page");
        assert!(key.left() >= px(176.0) && key.right() <= px(1060.0) && key.bottom() <= px(650.0));
    }
    let inputs = surface.update(|window, _| {
        gpui_kit::base::test_support::snapshots(window)
            .into_iter()
            .filter(|snapshot| snapshot.role() == Some(gpui_kit::Role::TextInput))
            .collect::<Vec<_>>()
    });
    assert_eq!(
        inputs.len(),
        1,
        "official Settings search must exist on the actual General page"
    );
    assert!(
        surface.debug_bounds("quick-launch-kit-settings").is_some(),
        "real Settings host, not old navigation"
    );
    surface.update(|window, cx| {
        window.activate_window();
        window.focus_next(cx);
    });
    surface.simulate_click(inputs[0].bounds().center(), gpui_kit::Modifiers::default());
    surface.simulate_input("关键词");
    surface.run_until_parked();
    surface
        .refresh()
        .expect("official search selects matching Commands");
    crate::main_window::tests::assert_official_quick_page(
        surface,
        QuickLaunchSettingsPage::Commands,
    );
    surface.simulate_keystrokes("ctrl-a");
    surface.simulate_input("没有任何匹配的合成设置");
    surface.run_until_parked();
    surface.refresh().expect("official empty search");
    let active = surface.update(|window, _| {
        gpui_kit::base::test_support::snapshots(window)
            .into_iter()
            .filter(|snapshot| {
                snapshot.role() == Some(gpui_kit::Role::Group)
                    && QuickLaunchSettingsPage::ALL
                        .iter()
                        .any(|page| snapshot.label() == Some(page.metadata().label))
            })
            .count()
    });
    assert_eq!(active, 0, "no match must unmount all business settings");
    surface.simulate_keystrokes("ctrl-a");
    surface.simulate_input("日志");
    surface.run_until_parked();
    surface.refresh().expect("diagnostics keyword");
    crate::main_window::tests::assert_official_quick_page(
        surface,
        QuickLaunchSettingsPage::Diagnostics,
    );
    surface.simulate_keystrokes("ctrl-a");
    surface.simulate_keystrokes("backspace");
    surface.run_until_parked();
    surface
        .refresh()
        .expect("clear official search keeps current page");
    crate::main_window::tests::assert_official_quick_page(
        surface,
        QuickLaunchSettingsPage::Diagnostics,
    );
}

#[test]
fn command_editor_uses_official_form_and_fields() {
    let source = include_str!("../../ui/components/custom_command_editor.rs");
    assert!(
        source.contains("Form::new()"),
        "command detail must adopt a real Form"
    );
    assert!(
        source.contains("Field::new()"),
        "command labels must use real Field composition"
    );
}
