use super::*;
use gpui_kit::base::{Root, test_support};
use gpui_kit::{AppContext, Context, Render, TestAppContext, size};
use std::cell::RefCell;

struct RowFixture {
    item: ClipboardItem,
    actions: Rc<RefCell<Vec<ClipboardAction>>>,
    management: Rc<RefCell<Vec<ClipboardItemAction>>>,
}
impl Render for RowFixture {
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
                options: vec![],
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
                extension_actions: Vec::new(),
                extension_feedback: None,
            },
            true,
        )
    }
}
fn item() -> ClipboardItem {
    ClipboardItem {
        id: 808,
        kind: ClipboardKind::Text,
        text: "**literal** <b>中文</b> [label](https://synthetic.invalid)".into(),
        image_path: String::new(),
        image_width: 0,
        image_height: 0,
        file_paths: vec![],
        source: nxtools_clipboard::CaptureSource {
            window_handle: 0,
            process_id: 0,
            process_name: "合成长来源Editor.exe".into(),
            process_icon_path: String::new(),
            window_title: String::new(),
        },
        captured_at_unix_ms: 1,
        last_used_at_unix_ms: 1,
        copy_count: 3,
        is_favorite: true,
        is_pinned: true,
        deleted_at_unix_ms: None,
    }
}
#[gpui_kit::test]
fn queue08_history_mounts_real_listitem_and_styled_action_tier(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    let (_, surface) = cx.add_window_view(|window, cx| {
        let view = cx.new(|_| RowFixture {
            item: item(),
            actions: Rc::new(RefCell::new(vec![])),
            management: Rc::new(RefCell::new(vec![])),
        });
        Root::new(view, window, cx)
    });
    surface.simulate_resize(size(px(300.0), px(440.0)));
    surface.refresh().expect("real history presentation");
    let nodes = surface.update(|window, _| test_support::snapshots(window));
    assert!(
        nodes
            .iter()
            .any(|node| node.label() == Some("剪贴板记录 808")),
        "actual stateless ListItem"
    );
    let delete = surface
        .debug_bounds("clipboard-item-action-0-delete")
        .expect("delete");
    assert_eq!(
        delete.size.height,
        px(20.0),
        "official XSmall action inside original24px rail"
    );
}

#[gpui_kit::test]
fn queue08_history_actions_modifiers_and_variable_content_keep_one_owner(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    let actions = Rc::new(RefCell::new(vec![]));
    let management = Rc::new(RefCell::new(vec![]));
    let held_actions = actions.clone();
    let held_management = management.clone();
    let slot = Rc::new(RefCell::new(None));
    let held_slot = slot.clone();
    let (_, surface) = cx.add_window_view(move |window, cx| {
        let view = cx.new(|_| RowFixture {
            item: item(),
            actions: held_actions,
            management: held_management,
        });
        *held_slot.borrow_mut() = Some(view.clone());
        Root::new(view, window, cx)
    });
    let view = slot.borrow().clone().expect("row");
    surface.simulate_resize(size(px(240.0), px(380.0)));
    surface.update(|window, _| window.activate_window());
    surface.refresh().expect("minimum row");
    let row = surface
        .debug_bounds("history-row-slot-0")
        .expect("stable row");
    assert!(row.size.height >= px(TEXT_ROW_HEIGHT + ROW_GAP));
    let rich = RENDERED_MARKDOWN.with_borrow(|rows| rows[&808].upgrade().expect("actual rich row"));
    rich.read_with(surface, |state, _| {
        assert!(
            state.bounds().bottom() <= row.bottom() - px(ROW_GAP),
            "rendered Markdown fits the content-driven row"
        );
    });
    for (key, expected) in [
        ("edit", ClipboardItemAction::Edit),
        ("pin", ClipboardItemAction::TogglePinned),
        ("favorite", ClipboardItemAction::ToggleFavorite),
        ("delete", ClipboardItemAction::Delete),
    ] {
        let selector = match key {
            "edit" => "clipboard-item-action-0-edit",
            "pin" => "clipboard-item-action-0-pin",
            "favorite" => "clipboard-item-action-0-favorite",
            _ => "clipboard-item-action-0-delete",
        };
        let bounds = surface.debug_bounds(selector).expect("ordinary action");
        assert!(
            bounds.left() >= row.left()
                && bounds.right() <= row.right()
                && bounds.bottom() <= row.bottom()
        );
        surface.simulate_click(bounds.center(), Default::default());
        assert_eq!(management.borrow().last(), Some(&expected));
        assert!(actions.borrow().is_empty(), "management never pastes row");
    }
    let content = surface
        .debug_bounds("clipboard-history-content")
        .expect("plain content");
    surface.simulate_click(
        content.center(),
        gpui_kit::Modifiers {
            alt: true,
            ..Default::default()
        },
    );
    assert_eq!(actions.borrow().last(), Some(&ClipboardAction::PlainText));
    surface.simulate_click(
        content.center(),
        gpui_kit::Modifiers {
            control: true,
            ..Default::default()
        },
    );
    assert_eq!(actions.borrow().last(), Some(&ClipboardAction::CopyOnly));
    let original = item().text;
    assert_eq!(
        view.read_with(surface, |view, _| view.item.text.clone()),
        original,
        "plain data unchanged"
    );
    surface.update(|_, cx| {
        view.update(cx, |view, cx| {
            view.item.kind = ClipboardKind::Files;
            view.item.file_paths = vec![
                r"X:\synthetic\长文件名.txt".into(),
                r"X:\synthetic\second.bin".into(),
            ];
            cx.notify();
        })
    });
    surface.refresh().expect("file content same controller");
    assert_eq!(
        surface
            .debug_bounds("history-row-slot-0")
            .expect("file row")
            .size
            .height,
        px(TEXT_ROW_HEIGHT + ROW_GAP)
    );
    surface.update(|_, cx| {
        view.update(cx, |view, cx| {
            view.item.kind = ClipboardKind::Image;
            view.item.image_width = 1000;
            view.item.image_height = 2000;
            view.item.image_path = "synthetic-missing-image.png".into();
            cx.notify();
        })
    });
    surface.refresh().expect("image fallback variable height");
    let image_row = surface
        .debug_bounds("history-row-slot-0")
        .expect("variable image row");
    assert_eq!(
        image_row.size.height,
        px(IMAGE_LIST_MAX_HEIGHT + IMAGE_ROW_CHROME_HEIGHT + ROW_GAP)
    );
    let nodes = surface.update(|window, _| test_support::snapshots(window));
    assert!(
        nodes
            .iter()
            .any(|node| node.label() == Some("剪贴板记录 808"))
    );
}

#[gpui_kit::test]
fn queue08_metadata_labels_use_released_natural_lines_within_original_rails(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    let (_, surface) = cx.add_window_view(|_, _| MetadataFixture);
    surface.refresh().expect("real metadata helpers");
    assert_eq!(
        surface
            .debug_bounds("queue08-time")
            .expect("time")
            .size
            .height,
        px(20.0)
    );
    assert_eq!(
        surface
            .debug_bounds("queue08-metric")
            .expect("metric")
            .size
            .height,
        px(20.0)
    );
}
struct MetadataFixture;
impl Render for MetadataFixture {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .items_start()
            .gap_1()
            .child(
                div()
                    .debug_selector(|| "queue08-time".into())
                    .child(time_label(Theme::zed_dark(), "30 天前".into())),
            )
            .child(
                div()
                    .debug_selector(|| "queue08-metric".into())
                    .child(metric_chip(Theme::zed_dark(), "12345 字符".into())),
            )
    }
}
