//! R2 regressions at the real history_row paint and Root focus boundaries.
//! The probes only observe whether GPUI reaches a descendant's paint callback.
//! They do not supply visibility, replace icons, draw pixels or dispatch actions.
use super::*;
use gpui_kit::base::{Root, test_support};
use gpui_kit::{
    AssetSource, Context, NoopTextSystem, Render, TestApp, TestAppWindow, canvas, size,
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

thread_local! {
    static PAINTS: RefCell<HashMap<&'static str, Bounds<Pixels>>> = RefCell::new(HashMap::new());
}

pub(super) fn paint_probe(key: &'static str) -> impl IntoElement {
    canvas(
        |_, _, _| (),
        move |bounds, (), window, _| {
            let clipped = bounds
                .intersect(&window.content_mask().bounds)
                .intersect(&Bounds::new(Default::default(), window.viewport_size()));
            if clipped.size.width > px(0.0) && clipped.size.height > px(0.0) {
                PAINTS.with(|paints| paints.borrow_mut().insert(key, clipped));
            }
        },
    )
    .absolute()
    .size_full()
}

fn painted(key: &'static str) -> bool {
    PAINTS.with(|paints| paints.borrow().contains_key(key))
}

struct FeedbackRow {
    item: ClipboardItem,
    selected: bool,
    actions: Rc<RefCell<Vec<ClipboardAction>>>,
    management: Rc<RefCell<Vec<ClipboardItemAction>>>,
    row_focus: Option<gpui_kit::FocusHandle>,
}

impl Render for FeedbackRow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Same public keyed handles as ClipboardPanel's production projection.
        // Never seed a synthetic actions_focused flag or add an input owner.
        let actions_focused = window.focused(cx).is_some();
        self.row_focus = window.focused(cx);
        let actions = self.actions.clone();
        let management = self.management.clone();
        history_row(
            Theme::zed_dark(),
            self.item.clone(),
            0,
            self.selected,
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
            actions_focused,
        )
    }
}

fn fixture_item() -> ClipboardItem {
    ClipboardItem {
        id: 1302,
        kind: ClipboardKind::Text,
        text: "合成反馈行".into(),
        image_path: String::new(),
        image_width: 0,
        image_height: 0,
        file_paths: vec![],
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

fn fixture(selected: bool) -> (TestApp, TestAppWindow<Root>, gpui_kit::Entity<FeedbackRow>) {
    let mut app = TestApp::with_text_system_and_assets(
        Arc::new(NoopTextSystem),
        Arc::new(crate::assets::NxToolsAssets),
    );
    app.update(gpui_kit::init);
    app.update(crate::theme::apply_component_theme);
    let view = app.new_entity(|_| FeedbackRow {
        item: fixture_item(),
        selected,
        actions: Rc::new(RefCell::new(vec![])),
        management: Rc::new(RefCell::new(vec![])),
        row_focus: None,
    });
    let content = view.clone();
    let mut surface = app.open_window(move |window, cx| Root::new(content, window, cx));
    surface.simulate_resize(size(px(300.0), px(440.0)));
    surface.update(|_, window, _| window.activate_window());
    surface.simulate_mouse_move(point(px(290.0), px(400.0)));
    redraw(&mut app, &mut surface, &view);
    (app, surface, view)
}

fn redraw(
    app: &mut TestApp,
    surface: &mut TestAppWindow<Root>,
    view: &gpui_kit::Entity<FeedbackRow>,
) {
    // Force an actual fresh paint; stale cached snapshots cannot satisfy the test.
    PAINTS.with(|paints| paints.borrow_mut().clear());
    app.update_entity(view, |_, cx| cx.notify());
    surface.draw();
}

fn action_snapshot(
    surface: &mut TestAppWindow<Root>,
    key: &'static str,
) -> test_support::ElementSnapshot {
    let id = gpui_kit::ElementId::from(gpui_kit::SharedString::from(format!(
        "clipboard-item-action-0-{key}"
    )));
    surface.update(|_, window, _| {
        test_support::snapshots(window)
            .into_iter()
            .find(|node| node.path().last() == Some(&id))
            .expect("real mounted Kit action")
    })
}

#[test]
fn r2_selected_first_row_without_hover_or_keyboard_focus_does_not_paint_actions() {
    let (mut app, mut surface, view) = fixture(true);
    assert!(app.read_entity(&view, |view, _| view.selected));
    surface.update(|_, window, cx| assert!(window.focused(cx).is_none()));
    assert!(
        !painted("actions"),
        "selected first row alone must not reach the action rail's paint descendants"
    );
    app.update_entity(&view, |view, cx| {
        view.selected = false;
        cx.notify();
    });
    redraw(&mut app, &mut surface, &view);
    assert!(!painted("actions"), "unselected idle rail is also hidden");
}

#[test]
fn r2_pin_and_favorite_paint_independently_in_idle_selected_hover_and_focus_states() {
    let (mut app, mut surface, view) = fixture(false);
    for (pinned, favorite) in [(false, false), (true, false), (false, true), (true, true)] {
        app.update_entity(&view, |view, cx| {
            view.item.is_pinned = pinned;
            view.item.is_favorite = favorite;
            cx.notify();
        });
        for selected in [false, true] {
            app.update_entity(&view, |view, cx| {
                view.selected = selected;
                cx.notify();
            });
            for hovered in [false, true] {
                surface.simulate_mouse_move(if hovered {
                    point(px(20.0), px(40.0))
                } else {
                    point(px(290.0), px(400.0))
                });
                redraw(&mut app, &mut surface, &view);
                assert_eq!(
                    painted("已置顶"),
                    pinned,
                    "pin: selected={selected} hover={hovered}"
                );
                assert_eq!(
                    painted("已收藏"),
                    favorite,
                    "favorite: selected={selected} hover={hovered}"
                );
            }
            surface.simulate_mouse_move(point(px(290.0), px(400.0)));
            surface.update(|_, window, cx| window.focus_next(cx));
            redraw(&mut app, &mut surface, &view);
            assert_eq!(
                painted("已置顶"),
                pinned,
                "pin survives actual keyboard focus"
            );
            assert_eq!(
                painted("已收藏"),
                favorite,
                "favorite survives actual keyboard focus"
            );
            surface.update(|_, window, cx| window.blur(cx));
        }
    }
}

#[test]
fn r2_pointer_hover_reveals_then_retracts_actions_without_selection_or_dispatch() {
    let (mut app, mut surface, view) = fixture(false);
    assert!(!painted("actions"));
    surface.simulate_mouse_move(point(px(20.0), px(40.0)));
    redraw(&mut app, &mut surface, &view);
    assert!(
        painted("actions"),
        "real ListItem group hover reveals actions"
    );
    let before = action_snapshot(&mut surface, "delete").bounds();
    assert_eq!(before.size, size(px(20.0), px(20.0)));
    surface.simulate_mouse_move(point(px(290.0), px(400.0)));
    redraw(&mut app, &mut surface, &view);
    assert!(
        !painted("actions"),
        "exit retracts even without selection change"
    );
    surface.simulate_mouse_move(point(px(20.0), px(40.0)));
    redraw(&mut app, &mut surface, &view);
    assert_eq!(action_snapshot(&mut surface, "delete").bounds(), before);
    app.read_entity(&view, |view, _| {
        assert!(!view.selected);
        assert!(view.actions.borrow().is_empty());
        assert!(view.management.borrow().is_empty());
    });
}

#[test]
fn r2_root_tab_and_key_release_keep_one_row_and_management_owner() {
    let (mut app, mut surface, view) = fixture(false);
    surface.update(|_, window, cx| window.focus_next(cx));
    redraw(&mut app, &mut surface, &view);
    surface.update(|_, window, cx| {
        assert!(
            window.focused(cx).is_some(),
            "one Base Root Tab owner enters the actual row/action scope"
        );
    });
    assert!(
        painted("actions"),
        "row keyboard access makes the rail reachable"
    );
    surface.simulate_event(gpui_kit::KeyDownEvent {
        keystroke: gpui_kit::Keystroke::parse("enter").expect("Enter"),
        is_held: false,
        prefer_character_input: false,
    });
    app.read_entity(&view, |view, _| assert!(view.actions.borrow().is_empty()));
    surface.simulate_event(gpui_kit::KeyUpEvent {
        keystroke: gpui_kit::Keystroke::parse("enter").expect("Enter"),
    });
    app.read_entity(&view, |view, _| {
        assert_eq!(
            view.actions.borrow().as_slice(),
            &[ClipboardAction::OriginalPaste]
        );
    });
    surface.simulate_event(gpui_kit::KeyDownEvent {
        keystroke: gpui_kit::Keystroke::parse("tab").expect("Tab"),
        is_held: false,
        prefer_character_input: false,
    });
    redraw(&mut app, &mut surface, &view);
    assert_eq!(action_snapshot(&mut surface, "edit").focused(), Some(true));
    assert!(painted("actions"));
    surface.simulate_event(gpui_kit::KeyDownEvent {
        keystroke: gpui_kit::Keystroke::parse("space").expect("Space"),
        is_held: false,
        prefer_character_input: false,
    });
    app.read_entity(&view, |view, _| {
        assert!(view.management.borrow().is_empty())
    });
    surface.simulate_event(gpui_kit::KeyUpEvent {
        keystroke: gpui_kit::Keystroke::parse("space").expect("Space"),
    });
    app.read_entity(&view, |view, _| {
        assert_eq!(
            view.management.borrow().as_slice(),
            &[ClipboardItemAction::Edit]
        );
        assert_eq!(view.actions.borrow().len(), 1, "management does not paste");
    });
    surface.update(|_, window, cx| window.blur(cx));
    redraw(&mut app, &mut surface, &view);
    assert!(!painted("actions"), "focus departure retracts idle actions");
}

#[test]
fn r2_production_pin_and_favorite_assets_rasterize_distinct_nonempty_ink() {
    use gpui_kit::component::IconNamed;
    let renderer = gpui_kit::SvgRenderer::new(Arc::new(crate::assets::NxToolsAssets));
    let mut masks = vec![];
    for icon in [MaterialIcon::Keep, MaterialIcon::Star] {
        let bytes = crate::assets::NxToolsAssets
            .load(icon.path().as_ref())
            .expect("production asset lookup")
            .expect("embedded status SVG");
        let image = renderer
            .render_single_frame(&bytes, 0.5)
            .expect("same GPUI SVG rasterizer used by Icon paint");
        let mask = image
            .as_bytes(0)
            .expect("status frame")
            .chunks_exact(4)
            .map(|pixel| pixel[3])
            .collect::<Vec<_>>();
        assert!(
            mask.iter().any(|alpha| *alpha > 0),
            "status icon must paint ink"
        );
        assert!(mask.contains(&0), "status icon is not a solid box");
        masks.push(mask);
    }
    assert_ne!(
        masks[0], masks[1],
        "pin and favorite must remain identifiable"
    );
}

#[test]
fn r2_status_ink_never_overlaps_hover_action_rail_at_minimum_width() {
    let (mut app, mut surface, view) = fixture(true);
    app.update_entity(&view, |view, cx| {
        view.item.is_pinned = true;
        view.item.is_favorite = true;
        cx.notify();
    });
    for width in [240.0, 300.0, 512.0] {
        surface.simulate_resize(size(px(width), px(440.0)));
        surface.simulate_mouse_move(point(px(20.0), px(40.0)));
        redraw(&mut app, &mut surface, &view);
        PAINTS.with(|paints| {
            let paints = paints.borrow();
            let pin = paints.get("已置顶").expect("pin painted bounds");
            let favorite = paints.get("已收藏").expect("favorite painted bounds");
            let actions = paints.get("actions").expect("actions painted bounds");
            assert!(
                pin.right() <= actions.left() && favorite.right() <= actions.left(),
                "status cannot hide under hover actions"
            );
            assert!(pin.size.width >= px(16.0) && favorite.size.width >= px(16.0));
        });
    }
}
