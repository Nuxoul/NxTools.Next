use super::*;
use gpui_kit::base::Root;
use gpui_kit::test::TestWindowExt;
use gpui_kit::{Context, Render, TestAppContext, size};
use std::cell::RefCell;

struct HistoryMenuFixture {
    item: ClipboardItem,
    actions: Rc<RefCell<Vec<ClipboardAction>>>,
    management: Rc<RefCell<Vec<ClipboardItemAction>>>,
}
impl Render for HistoryMenuFixture {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let actions = self.actions.clone();
        let management = self.management.clone();
        history_row(
            Theme::zed_dark(),
            self.item.clone(),
            0,
            true,
            246.0,
            ClipboardModifierActions::default(),
            CategorySourceMenu {
                options: vec![
                    CategorySourceOption {
                        category_id: "included".into(),
                        name: "已关联分类".into(),
                        included: true,
                    },
                    CategorySourceOption {
                        category_id: "new".into(),
                        name: "未关联分类".into(),
                        included: false,
                    },
                ],
                unavailable_hint: None,
            },
            HistoryRowHandlers {
                action: Rc::new(move |action, _, _, _| actions.borrow_mut().push(action)),
                management: Rc::new(move |action, _, _, _| management.borrow_mut().push(action)),
                open: Rc::new(|_, _, _, _| {}),
                preview: Rc::new(|_, _, _, _, _, _| {}),
                list_image: None,
                source_icon_color: None,
                text_character_count: None,
                open_targets: None,
                extension_feedback: None,
                extension_actions: vec![crate::extensions::manual_actions::ManualAction {
                    package_id: "fixture.tools".into(),
                    handler_id: "unique".into(),
                    title: "按行去重".into(),
                }],
            },
            true,
        )
    }
}
fn item() -> ClipboardItem {
    ClipboardItem {
        id: 47,
        kind: ClipboardKind::Text,
        text: "合成菜单测试".into(),
        image_path: String::new(),
        image_width: 0,
        image_height: 0,
        file_paths: Vec::new(),
        source: nxtools_clipboard::CaptureSource {
            window_handle: 0,
            process_id: 0,
            process_name: "Fixture.exe".into(),
            process_icon_path: String::new(),
            window_title: String::new(),
        },
        captured_at_unix_ms: 1,
        last_used_at_unix_ms: 1,
        copy_count: 1,
        is_favorite: false,
        is_pinned: false,
        deleted_at_unix_ms: None,
    }
}
#[gpui_kit::test]
fn production_history_menu_preserves_all_actions_categories_and_disabled_capabilities(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    let actions = Rc::new(RefCell::new(Vec::new()));
    let management = Rc::new(RefCell::new(Vec::new()));
    let fixture = cx.new({
        let actions = actions.clone();
        let management = management.clone();
        move |_| HistoryMenuFixture {
            item: item(),
            actions,
            management,
        }
    });
    let content = fixture.clone();
    let (_, surface) = cx.add_window_view(move |window, cx| {
        window.resize(size(px(300.0), px(440.0)));
        Root::new(content, window, cx)
    });
    surface.simulate_resize(size(px(300.0), px(440.0)));
    let open = |surface: &mut gpui_kit::VisualTestContext| {
        let row = surface
            .debug_bounds("history-row-slot-0")
            .expect("real history row");
        surface.simulate_mouse_down(
            row.center(),
            gpui_kit::MouseButton::Right,
            Default::default(),
        );
        surface.simulate_mouse_up(
            row.center(),
            gpui_kit::MouseButton::Right,
            Default::default(),
        );
        surface.refresh().expect("production menu");
    };
    for (id, expected) in [
        ("clipboard-menu-paste", ClipboardAction::OriginalPaste),
        ("clipboard-menu-copy", ClipboardAction::CopyOnly),
        ("clipboard-menu-plain-paste", ClipboardAction::PlainText),
        (
            "clipboard-menu-without-breaks",
            ClipboardAction::WithoutLineBreaks,
        ),
    ] {
        open(surface);
        let bounds = surface.debug_bounds(id).expect("preserved menu action");
        surface.simulate_click(bounds.center(), Default::default());
        assert_eq!(actions.borrow().last(), Some(&expected));
        assert_eq!(management.borrow().len(), 0);
    }
    for (id, expected) in [
        ("clipboard-menu-edit", ClipboardItemAction::Edit),
        ("clipboard-menu-pin", ClipboardItemAction::TogglePinned),
        (
            "clipboard-menu-favorite",
            ClipboardItemAction::ToggleFavorite,
        ),
        ("clipboard-menu-delete", ClipboardItemAction::Delete),
    ] {
        open(surface);
        let bounds = surface
            .debug_bounds(id)
            .expect("preserved management action");
        surface.simulate_click(bounds.center(), Default::default());
        assert_eq!(management.borrow().last(), Some(&expected));
    }
    for (id, remove) in [("included", true), ("new", false)] {
        open(surface);
        let category = surface
            .debug_bounds("clipboard-menu-categories")
            .expect("source categories");
        surface.simulate_mouse_move(category.center(), None, Default::default());
        surface.refresh().expect("production category submenu");
        surface.update(|window, _| {
            assert_eq!(
                window
                    .find(format!("clipboard-menu-category-{id}"))
                    .checked(),
                Some(remove)
            )
        });
        let bounds = surface
            .debug_bounds(if remove {
                "clipboard-menu-category-included"
            } else {
                "clipboard-menu-category-new"
            })
            .expect("actual category row");
        assert!(bounds.right() <= px(300.0));
        surface.simulate_click(bounds.center(), Default::default());
        assert_eq!(
            management.borrow().last(),
            Some(&ClipboardItemAction::ToggleCategorySource {
                category_id: id.into(),
                remove
            })
        );
    }
    open(surface);
    let extension = surface
        .debug_bounds("clipboard-menu-extension")
        .expect("extension submenu");
    surface.simulate_mouse_move(extension.center(), None, Default::default());
    surface.refresh().expect("real direct action submenu");
    let direct = surface
        .debug_bounds("clipboard-extension-fixture.tools-unique")
        .expect("actual action without chooser");
    surface.simulate_click(direct.center(), Default::default());
    assert!(
        matches!(management.borrow().last(),Some(ClipboardItemAction::RunExtension {action}) if action.handler_id=="unique")
    );
    management.borrow_mut().pop();

    assert_eq!(actions.borrow().len(), 4);
    assert_eq!(management.borrow().len(), 6);
    fixture.update(surface, |fixture, cx| {
        fixture.item.kind = ClipboardKind::Files;
        fixture.item.file_paths = vec!["C:\\Synthetic\\missing.txt".into()];
        cx.notify();
    });
    surface.refresh().expect("nontext capabilities");
    open(surface);
    surface.update(|window, _| {
        // Kit0.8.0 explicitly exposes disabled state through its accessibility bridge.
        assert_eq!(window.find("clipboard-menu-edit").disabled(), Some(true));
        assert_eq!(
            window.find("clipboard-menu-without-breaks").disabled(),
            Some(true)
        );
    });
    let disabled = surface
        .debug_bounds("clipboard-menu-edit")
        .expect("disabled edit");
    surface.simulate_click(disabled.center(), Default::default());
    assert_eq!(
        management.borrow().len(),
        6,
        "disabled edit must not dispatch"
    );
    let disabled = surface
        .debug_bounds("clipboard-menu-without-breaks")
        .expect("disabled transform");
    surface.simulate_click(disabled.center(), Default::default());
    assert_eq!(
        actions.borrow().len(),
        4,
        "disabled transform must not dispatch"
    );
}
