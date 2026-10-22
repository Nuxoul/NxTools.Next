use super::*;
use gpui_kit::base::Root;
use gpui_kit::{AppContext, TestAppContext};

#[gpui_kit::test]
fn clipboard_copy_completion_uses_shared_center_not_panel_status_badge(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.executor().allow_parking();
    let center = cx.new(|_| crate::notifications::NotificationCenter::default());
    cx.update(|cx| crate::notifications::register(&center, cx));
    let directory = tempfile::tempdir().expect("owned history");
    let worker = crate::clipboard_worker::ClipboardCaptureWorkerHandle::spawn(
        directory.path().join("history.db"),
        directory.path().join("images"),
        nxtools_clipboard::ClipboardRetentionPolicy {
            max_items: 1000,
            retention_days: 30,
        },
        1000,
    )
    .expect("owned worker");
    let worker_cleanup = worker.clone();
    let slot = Rc::new(RefCell::new(None));
    let mounted = slot.clone();
    let (_, surface) = cx.add_window_view(move |window, cx| {
        let panel = cx.new(|cx| {
            ClipboardPanel::new(
                ClipboardHistoryPage {
                    items: Vec::new(),
                    total: 0,
                    has_more: false,
                },
                vec![ClipboardCategory {
                    id: BUILTIN_ALL_CATEGORY_ID.into(),
                    name: "全部".into(),
                    kind: ClipboardCategoryKind::All,
                    sort_order: 0,
                }],
                HashMap::new(),
                Vec::new(),
                ClipboardModifierActions::default(),
                nxtools_protocol::ForegroundContext {
                    window_handle: 1,
                    focus_window_handle: 1,
                    process_id: 1,
                    process_name: "fixture".into(),
                    window_title: String::new(),
                },
                worker,
                WeakEntity::new_invalid(),
                uuid::Uuid::new_v4(),
                None,
                window,
                cx,
            )
        });
        *mounted.borrow_mut() = Some(panel.clone());
        Root::new(panel, window, cx)
    });
    surface.simulate_resize(size(px(300.0), px(440.0)));
    let panel = slot.borrow().clone().expect("panel");
    let mut completed_keys = Vec::new();
    for _ in 0..2 {
        let id = uuid::Uuid::new_v4();
        let expected_key = panel.read_with(surface, |panel, _| {
            format!("clipboard-operation-{}-{id}", panel.session_id)
        });
        panel.update_in(surface, |panel, _, cx| {
            panel.pending_action = Some((id, ClipboardAction::CopyOnly, 1));
            panel.pending_action_started_at = Some((id, Instant::now()));
            panel.set_operation_status_for(id, PanelStatus::Pending("正在复制…"), cx);
        });
        surface.run_until_parked();
        center.read_with(surface, |center, _| {
            let pending = center
                .items()
                .iter()
                .find(|notice| notice.id.as_ref() == expected_key.as_str())
                .expect("invocation progress");
            assert_eq!(
                pending.level,
                crate::notifications::NotificationLevel::Progress
            );
            assert_eq!(center.items().len(), completed_keys.len() + 1);
        });
        let result = PlatformCommandResult {
            success: true,
            error: None,
            native_error_code: None,
            data: None,
        };
        panel.update_in(surface, |panel, window, cx| {
            panel.handle_command_result(id, result.clone(), window, cx);
            panel.handle_command_result(id, result, window, cx);
        });
        surface.run_until_parked();
        completed_keys.push(expected_key);
        center.read_with(surface, |center, _| {
            assert_eq!(
                center.items().len(),
                completed_keys.len(),
                "same invocation replaces, duplicate terminal is ignored"
            );
            for key in &completed_keys {
                let notice = center
                    .items()
                    .iter()
                    .find(|notice| notice.id.as_ref() == key.as_str())
                    .expect("completed invocation retained");
                assert_eq!(notice.message, "已复制");
                assert_eq!(
                    notice.level,
                    crate::notifications::NotificationLevel::Success
                );
            }
        });
    }
    assert_ne!(completed_keys[0], completed_keys[1]);
    // Two concurrent worker callbacks in the same panel must not steal each other's key.
    panel.update(surface, |panel, cx| {
        panel.delete_category("missing-category-a", cx);
        panel.delete_category("missing-category-b", cx);
    });
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        surface.run_until_parked();
        if center.read_with(surface, |center, _| {
            center
                .items()
                .iter()
                .all(|notice| notice.level != crate::notifications::NotificationLevel::Progress)
        }) {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "owned category worker callbacks complete"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    center.read_with(surface, |center, _| {
        assert_eq!(
            center
                .items()
                .iter()
                .filter(|notice| notice.level == crate::notifications::NotificationLevel::Error)
                .count(),
            2,
            "different category failures retain distinct identities"
        );
    });
    surface.update(|window, cx| window.draw(cx).clear(cx));
    assert!(
        surface
            .debug_bounds("clipboard-operation-status-badge")
            .is_none(),
        "no custom transient status badge"
    );
    worker_cleanup.shutdown().expect("flush owned worker");
    surface.run_until_parked();
}
