//! Real production result/action updates and repeated Root/Window composition.
use super::*;
use gpui_kit::{AppContext, SharedString, TestAppContext};

thread_local! {
    static TEXT: RefCell<Option<HashMap<String, Vec<SharedString>>>> = const { RefCell::new(None) };
}
pub(super) fn observe(key: String, value: &SharedString) {
    TEXT.with(|text| {
        if let Some(text) = text.borrow_mut().as_mut() {
            text.entry(key).or_default().push(value.clone());
        }
    });
}
struct Observation;
impl Observation {
    fn start() -> Self {
        TEXT.with(|text| *text.borrow_mut() = Some(HashMap::new()));
        Self
    }
    fn assert_shared(&self, expected: &[&str]) {
        TEXT.with(|text| {
            let text = text.borrow();
            let text = text.as_ref().expect("armed renderer observation");
            for key in expected {
                assert!(text.contains_key(*key), "actual production text slot composed: {key}");
            }
            for (key,values) in text {
                if values[0].len()<64 {continue;}
                assert!(values.len()>=3,"repeat real draws for {key}");
                let allocations=values.iter().map(|value|value.as_ptr() as usize).collect::<HashSet<_>>().len();
                assert!(values.iter().all(|value|value==&values[0]),"no truncation or content changes for {key}");
                println!("audit13 text key={key} retained_handles={} distinct_backing_allocations={allocations} text_bytes={} repeated_text_bytes={}",values.len(),values[0].len(),allocations.saturating_sub(1)*values[0].len());
                assert_eq!(allocations,1,"unchanged retained display text must clone the original owner's handle: {key}");
            }
        });
    }
}
impl Drop for Observation {
    fn drop(&mut self) {
        TEXT.with(|text| *text.borrow_mut() = None);
    }
}
fn result(id: &str) -> SearchResult {
    SearchResult {
        entry: nxtools_finder::ApplicationEntry {
            id: id.into(),
            name: format!("{id} {}", "完整中文标题LongTitle".repeat(20)),
            target: format!(r"C:\synthetic\{}\{id}.txt", "长路径".repeat(20)),
            source: ApplicationSource::IndexedFile,
        },
        action: SearchResultAction::CopyText {
            text: format!("FULL-EXECUTION-PAYLOAD-{id}-{}", "不可截断".repeat(256)),
        },
        subtitle: Some(format!("{id} {}", "完整副标题metadata".repeat(20))),
        score: 3,
        activation_count: 1,
        last_activated_at_unix_ms: None,
        query_activation_count: 2,
        last_query_activation_at_unix_ms: None,
    }
}
fn fixture(cx: &mut TestAppContext) -> (Entity<QuickLaunchPanel>, gpui_kit::AnyWindowHandle) {
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    cx.executor().allow_parking();
    let retained = Rc::new(RefCell::new(None));
    let output = retained.clone();
    let (_, surface) = cx.add_window_view(move |window, cx| {
        let panel = cx.new(|cx| {
            let mut panel = QuickLaunchPanel::new(
                vec![result("initial")],
                nxtools_protocol::ForegroundContext {
                    window_handle: 1,
                    focus_window_handle: 1,
                    process_id: 1,
                    process_name: "synthetic".into(),
                    window_title: "synthetic".into(),
                },
                None,
                Vec::new(),
                WeakEntity::new_invalid(),
                window,
                cx,
            );
            panel.results_visible = true;
            panel
        });
        *output.borrow_mut() = Some(panel.clone());
        Root::new(panel, window, cx)
    });
    let handle = surface.update(|window, _| window.window_handle());
    (
        retained.borrow_mut().take().expect("original panel"),
        handle,
    )
}
#[gpui_kit::test]
fn audit13_real_result_update_and_repeated_render_share_full_display_text(cx: &mut TestAppContext) {
    let (panel, handle) = fixture(cx);
    let mut surface = gpui_kit::VisualTestContext::from_window(handle, cx);
    surface.simulate_resize(size(px(PANEL_WIDTH), px(PANEL_HEIGHT)));
    let expected = vec![result("first"), result("second")];
    panel.update(&mut surface, |panel, cx| {
        panel.replace_results(expected.clone(), cx)
    });
    let input = panel.read_with(&surface, |panel, _| panel.search_input.entity_id());
    let observation = Observation::start();
    for _ in 0..3 {
        panel.update(&mut surface, |_, cx| cx.notify());
        surface.update(|window, cx| window.draw(cx).clear(cx));
    }
    panel.read_with(&surface, |panel, _| {
        assert_eq!(panel.results.as_slice(), expected.as_slice());
        assert_eq!(panel.search_input.entity_id(), input);
        assert_eq!(panel.results[0].copy_text(), expected[0].copy_text());
    });
    observation.assert_shared(&[
        "result-title-0",
        "result-title-1",
        "result-subtitle-0",
        "result-subtitle-1",
    ]);
    surface.update(|window, _| window.remove_window());
}
#[gpui_kit::test]
fn audit13_real_action_update_and_repeated_render_share_labels_and_context(
    cx: &mut TestAppContext,
) {
    let (panel, handle) = fixture(cx);
    let mut surface = gpui_kit::VisualTestContext::from_window(handle, cx);
    surface.simulate_resize(size(px(PANEL_WIDTH), px(PANEL_HEIGHT)));
    panel.update_in(&mut surface, |panel, window, cx| {
        panel.enter_action_mode(0, window, cx);
        let mode = panel.action_mode.as_ref().expect("original mode");
        let (mode_id, level_id, id) = (
            mode.id(),
            mode.current_level_id(),
            mode.target().entry.id.clone(),
        );
        panel.apply_shell_actions(
            mode_id,
            level_id,
            &id,
            Some(ShellActionMenu {
                session_id: "synthetic-session".into(),
                navigation_directory: None,
                items: vec![ShellActionItem {
                    title: "完整动态操作label".repeat(24),
                    command_id: Some(71),
                    submenu_id: None,
                    enabled: true,
                    separator: false,
                    icon_path: None,
                }],
            }),
            cx,
        );
    });
    let observation = Observation::start();
    for _ in 0..3 {
        panel.update(&mut surface, |_, cx| cx.notify());
        surface.update(|window, cx| window.draw(cx).clear(cx));
    }
    observation.assert_shared(&[
        "action-label-1",
        "action-target-title",
        "action-level-title",
    ]);
    panel.read_with(&surface, |panel, _| {
        let mode = panel.action_mode.as_ref().expect("mode");
        assert_eq!(mode.target().copy_text(), result("initial").copy_text());
        assert!(
            mode.visible_actions()
                .iter()
                .any(|item| item.label.as_ref() == "完整动态操作label".repeat(24))
        );
    });
    surface.update(|window, _| window.remove_window());
}

fn draw_repeated(panel: &Entity<QuickLaunchPanel>, surface: &mut gpui_kit::VisualTestContext) {
    for _ in 0..3 {
        panel.update(surface, |_, cx| cx.notify());
        surface.update(|window, cx| window.draw(cx).clear(cx));
    }
}
#[gpui_kit::test]
fn audit13_result_reorder_and_same_id_update_replace_presentation_without_mutating_old_payload(
    cx: &mut TestAppContext,
) {
    let (panel, handle) = fixture(cx);
    let mut surface = gpui_kit::VisualTestContext::from_window(handle, cx);
    surface.simulate_resize(size(px(PANEL_WIDTH), px(PANEL_HEIGHT)));
    let original = vec![result("first"), result("second")];
    panel.update(&mut surface, |panel, cx| {
        panel.replace_results(original.clone(), cx);
        panel.selected = 1;
    });
    let retained = panel.read_with(&surface, |panel, _| panel.results.clone());
    let old_title = retained.display(1).title.clone();
    let mut updated = original[1].clone();
    updated.entry.name = "UPDATED完整标题".repeat(40);
    updated.subtitle = Some("UPDATED完整副标题".repeat(40));
    let expected = vec![updated.clone(), original[0].clone()];
    panel.update(&mut surface, |panel, cx| {
        panel.replace_results(expected.clone(), cx)
    });
    let observation = Observation::start();
    draw_repeated(&panel, &mut surface);
    observation.assert_shared(&[
        "result-title-0",
        "result-title-1",
        "result-subtitle-0",
        "result-subtitle-1",
    ]);
    panel.read_with(&surface, |panel, _| {
        assert_eq!(
            panel.selected, 0,
            "selection follows domain identity across reorder"
        );
        assert_eq!(panel.results.as_slice(), expected.as_slice());
        assert_eq!(panel.results.display(0).title.as_ref(), updated.entry.name);
        assert_eq!(
            panel.results.display(0).subtitle.as_ref(),
            updated.subtitle.as_deref().expect("updated subtitle")
        );
        assert_eq!(
            panel.results[0].copy_text(),
            original[1].copy_text(),
            "display change never rewrites execution data"
        );
    });
    assert_eq!(
        retained.as_slice(),
        original.as_slice(),
        "retained preview snapshot keeps its complete original records"
    );
    assert_eq!(old_title.as_ref(), original[1].entry.name);
    surface.update(|window, _| window.remove_window());
}

#[gpui_kit::test]
fn audit13_result_fallbacks_and_status_keep_full_shared_view_values(cx: &mut TestAppContext) {
    let (panel, handle) = fixture(cx);
    let mut surface = gpui_kit::VisualTestContext::from_window(handle, cx);
    surface.simulate_resize(size(px(PANEL_WIDTH), px(PANEL_HEIGHT)));
    let mut file = result("path-fallback");
    file.subtitle = None;
    file.action = SearchResultAction::OpenTarget;
    let mut copy = result("copy-fallback");
    copy.subtitle = None;
    let expected = vec![file.clone(), copy.clone()];
    let message = "完整查询错误status".repeat(30);
    panel.update(&mut surface, |panel, cx| {
        panel.replace_results(expected.clone(), cx);
        panel.set_error(message.clone(), cx);
    });
    let observation = Observation::start();
    draw_repeated(&panel, &mut surface);
    observation.assert_shared(&[
        "status",
        "result-title-0",
        "result-title-1",
        "result-subtitle-0",
        "result-subtitle-1",
    ]);
    panel.read_with(&surface, |panel, _| {
        assert_eq!(
            panel.results.display(0).subtitle.as_ref(),
            file.entry.target
        );
        assert_eq!(
            panel.results.display(1).subtitle.as_ref(),
            "按 Enter 复制结果"
        );
        assert_eq!(panel.status.as_deref(), Some(message.as_str()));
        assert_eq!(panel.results.as_slice(), expected.as_slice());
    });
    surface.update(|window, _| window.remove_window());
}

#[gpui_kit::test]
fn audit13_dynamic_submenu_title_is_shared_through_original_keyboard_stack(
    cx: &mut TestAppContext,
) {
    let (panel, handle) = fixture(cx);
    let mut surface = gpui_kit::VisualTestContext::from_window(handle, cx);
    surface.simulate_resize(size(px(PANEL_WIDTH), px(PANEL_HEIGHT)));
    let title = "完整动态子菜单标题".repeat(20);
    panel.update_in(&mut surface, |panel, window, cx| {
        panel.enter_action_mode(0, window, cx);
        let mode = panel.action_mode.as_ref().expect("action mode");
        let (mode_id, level_id, id) = (
            mode.id(),
            mode.current_level_id(),
            mode.target().entry.id.clone(),
        );
        panel.apply_shell_actions(
            mode_id,
            level_id,
            &id,
            Some(ShellActionMenu {
                session_id: "synthetic-only".into(),
                navigation_directory: None,
                items: vec![ShellActionItem {
                    command_id: None,
                    title: title.clone(),
                    enabled: true,
                    separator: false,
                    submenu_id: Some(7),
                    icon_path: None,
                }],
            }),
            cx,
        );
        panel.focus_search(window, cx);
    });
    surface.refresh().expect("actual dynamic action row");
    let row = surface
        .debug_bounds("quick-launch-action-row-1")
        .expect("actual submenu row");
    surface.simulate_mouse_move(row.center(), None, gpui_kit::Modifiers::default());
    surface.simulate_click(row.center(), gpui_kit::Modifiers::default());
    surface.run_until_parked();
    panel.read_with(&surface, |panel, _| {
        let mode = panel.action_mode.as_ref().expect("nested mode");
        assert_eq!(mode.depth(), 2);
        assert_eq!(mode.title().as_ref(), title);
        assert_eq!(mode.target().copy_text(), result("initial").copy_text());
    });
    let observation = Observation::start();
    draw_repeated(&panel, &mut surface);
    observation.assert_shared(&["action-level-title", "action-target-title"]);
    surface.simulate_keystrokes("escape");
    surface.run_until_parked();
    panel.update_in(&mut surface, |panel, window, cx| {
        assert_eq!(panel.action_mode.as_ref().expect("parent").depth(), 1);
        assert!(panel.search_input.focus_handle(cx).is_focused(window));
    });
    surface.update(|window, _| window.remove_window());
}
