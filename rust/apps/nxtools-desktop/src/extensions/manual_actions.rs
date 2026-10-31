use super::{ExtensionApplication, ExtensionApplicationGlobal};
use gpui_kit::component::WindowExt;
use gpui_kit::{App, AppContext, Entity, ParentElement, Styled, WeakEntity, Window, px};
#[cfg(test)]
use gpui_kit::{Context, IntoElement, Render, div};
#[path = "manual_feedback.rs"]
mod manual_feedback;
use manual_feedback::ManualFeedback;
use serde_json::Value;

#[derive(Clone)]
pub(crate) struct ClipboardOrigin {
    pub(crate) panel: WeakEntity<crate::ui::ClipboardPanel>,
    pub(crate) session_id: uuid::Uuid,
    pub(crate) window: gpui_kit::AnyWindowHandle,
    pub(crate) item_id: i64,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ManualAction {
    pub(crate) package_id: String,
    pub(crate) handler_id: String,
    pub(crate) title: String,
}
#[cfg(test)]
pub(crate) fn available(feature: &str, cx: &App) -> Vec<ManualAction> {
    available_for(feature, None, cx)
}
pub(crate) fn available_for(
    feature: &str,
    selection: Option<nxtools_extensions::InputSelection>,
    cx: &App,
) -> Vec<ManualAction> {
    let Some(owner) = cx
        .try_global::<ExtensionApplicationGlobal>()
        .and_then(|global| global.0.upgrade())
    else {
        return Vec::new();
    };
    owner
        .read(cx)
        .packages
        .packages
        .iter()
        .filter(|package| package.enabled && package.trust.is_some())
        .flat_map(|package| {
            package
                .manifest
                .handlers
                .iter()
                .filter(move |handler| {
                    handler.feature == feature
                        && handler.entry == "action"
                        && selection.is_none_or(|selection| {
                            handler
                                .accepts
                                .as_ref()
                                .is_none_or(|accepts| accepts.accepts(selection))
                        })
                })
                .map(move |handler| ManualAction {
                    package_id: package.manifest.id.clone(),
                    handler_id: handler.id.clone(),
                    title: handler.title.clone(),
                })
        })
        .collect()
}
fn publish_admitted_run(owner: Entity<ExtensionApplication>, id: uuid::Uuid, cx: &mut App) {
    owner.update(cx, |app, cx| app.notify_manual_run(id, cx));
}

pub(crate) fn invoke_selected(
    feature: &'static str,
    action: ManualAction,
    context: Value,
    origin: Option<ClipboardOrigin>,
    window: &mut Window,
    cx: &mut App,
) {
    let owner = cx
        .try_global::<ExtensionApplicationGlobal>()
        .and_then(|global| global.0.upgrade());
    let source = window.window_handle();
    cx.defer(move |cx| {
        let Some(owner) = owner else {
            return;
        };
        if let Some(origin) = &origin
            && !origin
                .panel
                .upgrade()
                .is_some_and(|panel| panel.read(cx).extension_origin_current(origin.session_id))
        {
            return;
        }
        let result = owner.update(cx, |app, cx| {
            let id =
                app.start_manual(action.package_id, action.handler_id, feature, context, cx)?;
            app.attach_manual_origin(id, origin.clone());
            Ok::<_, &'static str>(id)
        });
        let _ = source.update(cx, |_, _window, cx| match result {
            Ok(id) => publish_admitted_run(owner.clone(), id, cx),
            Err(message) => {
                if let Some(center) = crate::notifications::current(cx) {
                    let mut notice = crate::notifications::AppNotification::new(
                        format!("extension-start-{}", uuid::Uuid::new_v4()),
                        crate::notifications::NotificationLevel::Error,
                        "未能启动扩展动作",
                        message,
                    );
                    if let Some(origin) = origin.clone() {
                        notice = notice.in_window(origin.window);
                    }
                    cx.defer(move |cx| center.update(cx, |center, cx| center.push(notice, cx)));
                }
            }
        });
    });
}

pub(crate) fn show_run(
    owner: Entity<ExtensionApplication>,
    id: uuid::Uuid,
    origin: Option<ClipboardOrigin>,
    window: &mut Window,
    cx: &mut App,
) {
    let origin = origin.or_else(|| owner.read(cx).manual_run(id).and_then(|run| run.origin));
    // Recent activity may live in the manager; present feedback in the still-current
    // original panel window when available. Never create or show a native window.
    if let Some(origin) = &origin
        && origin.window != window.window_handle()
        && origin
            .panel
            .upgrade()
            .is_some_and(|panel| panel.read(cx).extension_origin_current(origin.session_id))
    {
        let source_window = origin.window;
        let origin = origin.clone();
        cx.defer(move |cx| {
            if !origin
                .panel
                .upgrade()
                .is_some_and(|panel| panel.read(cx).extension_origin_current(origin.session_id))
            {
                return;
            }
            let _ = source_window.update(cx, |_, window, cx| {
                show_run(owner, id, Some(origin), window, cx);
            });
        });
        return;
    }
    let feedback = cx.new(|cx| {
        let subscription = cx.observe(&owner, |_, _, cx| cx.notify());
        let origin_subscription = origin
            .as_ref()
            .and_then(|origin| origin.panel.upgrade())
            .map(|panel| cx.observe(&panel, |_, _, cx| cx.notify()));
        ManualFeedback {
            owner,
            id,
            origin,
            selected_result: 0,
            _subscription: subscription,
            _origin_subscription: origin_subscription,
        }
    });
    window.open_dialog(cx, move |dialog, window, _| {
        let feedback = feedback.clone();
        dialog
            .title("扩展动作")
            .margin_top(px(8.0))
            .w((window.viewport_size().width - px(24.0)).min(px(480.0)))
            .content(move |content, _, _| content.min_h_0().child(feedback.clone()))
    });
}
pub(crate) fn clipboard_context(item: &nxtools_clipboard::ClipboardItem) -> Option<Value> {
    let item = crate::clipboard::extension_adapter::ClipboardExtensionSnapshot::selected_item(item)
        .ok()?;
    let value = match item.content {
        crate::clipboard::extension_adapter::ClipboardExtensionSnapshotContent::Text(text) => {
            serde_json::json!({"item_id":item.item_id,"kind":"text","text":text})
        }
        crate::clipboard::extension_adapter::ClipboardExtensionSnapshotContent::Files(paths) => {
            serde_json::json!({"item_id":item.item_id,"kind":"files","paths":paths})
        }
        crate::clipboard::extension_adapter::ClipboardExtensionSnapshotContent::Image(image) => {
            serde_json::json!({"item_id":item.item_id,"kind":"image","width":image.width,"height":image.height,"image_read_available":false})
        }
    };
    Some(serde_json::json!({"selection":value}))
}
#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::{TestAppContext, size};

    #[gpui_kit::test]
    fn ux_refinement_short_feedback_does_not_force_large_empty_body(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        let owner = cx.new(|_| {
            ExtensionApplication::new(
                Default::default(),
                Default::default(),
                None,
                WeakEntity::new_invalid(),
            )
        });
        let id = uuid::Uuid::new_v4();
        owner.update(cx, |app, _| {
            app.begin_manual_run(id, "fixture".into(), "文本工具".into(), "整理文本".into());
            let run = app.manual_runs.front_mut().expect("fixture");
            run.state = crate::extensions::manual_runs::ManualState::NotHandled;
            run.message = Some("文本无需整理。".into());
        });
        let (_, window) = cx.add_window_view(|window, cx| {
            let content = cx.new(|_| Fixture);
            gpui_kit::base::Root::new(content, window, cx)
        });
        window.simulate_resize(size(px(1060.0), px(650.0)));
        window.update(|window, cx| show_run(owner, id, None, window, cx));
        window.update(|window, cx| window.draw(cx).clear(cx));
        let body = window
            .debug_bounds("extension-manual-feedback")
            .expect("actual feedback");
        assert!(
            body.size.height <= px(260.0),
            "short outcome must not reserve an empty report: {body:?}"
        );
    }

    #[gpui_kit::test]
    fn manual_feedback_real_cancel_is_scoped_and_close_does_not_cancel_other_run(
        cx: &mut TestAppContext,
    ) {
        cx.update(|cx| {
            gpui_kit::init(cx);
            cx.set_reduce_motion(true);
        });
        let owner = cx.new(|_| {
            ExtensionApplication::new(
                Default::default(),
                Default::default(),
                None,
                WeakEntity::new_invalid(),
            )
        });
        let id = uuid::Uuid::new_v4();
        let other = uuid::Uuid::new_v4();
        let (cancel, receiver) = tokio::sync::watch::channel(false);
        let (other_cancel, other_receiver) = tokio::sync::watch::channel(false);
        owner.update(cx, |app, _| {
            app.pending_manual.insert(id, ("fixture".into(), cancel));
            app.pending_manual
                .insert(other, ("other".into(), other_cancel));
            app.begin_manual_run(
                id,
                "fixture".into(),
                "剪贴板文本整理".into(),
                "整理文本".into(),
            );
            app.begin_manual_run(other, "other".into(), "其他插件".into(), "等待".into());
            app.manual_runs.front_mut().expect("fixture run").message =
                Some("演示插件提供的长说明，不是宿主保存证明。\n".repeat(60));
        });
        let retained = owner.clone();
        let (_, window) = cx.add_window_view(move |window, cx| {
            let view = cx.new(|_| Fixture);
            gpui_kit::base::Root::new(view, window, cx)
        });
        window.update(|window, cx| show_run(retained, id, None, window, cx));
        for (width, height) in [(300.0, 440.0), (280.0, 280.0)] {
            window.simulate_resize(size(px(width), px(height)));
            window.update(|window, cx| window.draw(cx).clear(cx));
            window.update(|window, cx| window.draw(cx).clear(cx));
            let footer = window
                .debug_bounds("extension-manual-footer")
                .expect("pinned footer");
            let scroll = window
                .debug_bounds("extension-manual-scroll")
                .expect("scroll body");
            assert!(
                scroll.bottom() <= footer.top(),
                "scroll body cannot cover actions"
            );
            assert!(
                footer.bottom() <= px(height),
                "footer inside minimum client"
            );
            let close = window
                .debug_bounds("extension-feedback-close")
                .expect("official close");
            assert!(close.right() <= px(width) && close.bottom() <= px(height));
        }
        let close = window
            .debug_bounds("extension-feedback-close")
            .expect("close target");
        window.simulate_click(close.center(), gpui_kit::Modifiers::default());
        window.run_until_parked();
        assert!(
            !*receiver.borrow(),
            "closing the selected running feedback must not cancel it"
        );
        assert!(!*other_receiver.borrow());
        let retained = owner.clone();
        window.update(|window, cx| show_run(retained, id, None, window, cx));
        window.update(|window, cx| window.draw(cx).clear(cx));
        window.update(|window, cx| window.draw(cx).clear(cx));
        let button = window.update(|window, _| {
            gpui_kit::base::test_support::snapshots(window)
                .into_iter()
                .find(|node| {
                    node.path().last()
                        == Some(&gpui_kit::ElementId::from("extension-cancel-this-run"))
                })
                .expect("current-frame cancel button")
                .bounds()
        });
        assert!(
            button.left() >= px(0.0)
                && button.right() <= px(280.0)
                && button.top() >= px(0.0)
                && button.bottom() <= px(280.0),
            "visible real hit target: {button:?}"
        );
        window.simulate_click(button.center(), gpui_kit::Modifiers::default());
        window.run_until_parked();
        assert!(*receiver.borrow());
        assert!(!*other_receiver.borrow());
        window.update(|window, cx| window.draw(cx).clear(cx));
        let cancelling = window.update(|window, _| {
            gpui_kit::base::test_support::snapshots(window)
                .into_iter()
                .find(|node| {
                    node.path().last()
                        == Some(&gpui_kit::ElementId::from("extension-cancel-this-run"))
                })
                .expect("cancellation stays visible until runtime terminal")
        });
        let mut observed = receiver.clone();
        let _ = observed.borrow_and_update();
        window.simulate_click(cancelling.bounds().center(), gpui_kit::Modifiers::default());
        window.run_until_parked();
        assert!(
            !observed.has_changed().expect("run still owns sender"),
            "no repeated cancellation submission"
        );
        window.update(|window, cx| window.close_dialog(cx));
        window.run_until_parked();
        assert!(
            !*other_receiver.borrow(),
            "closing feedback is presentation-only"
        );
    }
    struct Fixture;
    impl Render for Fixture {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().size_full()
        }
    }
    #[gpui_kit::test]
    fn manual_feedback_terminal_repaints_pinned_results_under_long_plugin_text(
        cx: &mut TestAppContext,
    ) {
        use crate::extensions::manual_runs::ManualState;
        cx.update(|cx| {
            gpui_kit::init(cx);
            cx.set_reduce_motion(true);
        });
        let owner = cx.new(|_| {
            ExtensionApplication::new(
                Default::default(),
                Default::default(),
                None,
                WeakEntity::new_invalid(),
            )
        });
        let id = uuid::Uuid::new_v4();
        owner.update(cx, |app, _| {
            app.begin_manual_run(
                id,
                "demo.package".into(),
                "演示插件".into(),
                "演示动作".into(),
            )
        });
        let (_, window) = cx.add_window_view(|window, cx| {
            let view = cx.new(|_| Fixture);
            gpui_kit::base::Root::new(view, window, cx)
        });
        let retained = owner.clone();
        window.update(|window, cx| show_run(retained, id, None, window, cx));
        for mode in [
            gpui_kit::component::ThemeMode::Light,
            gpui_kit::component::ThemeMode::Dark,
        ] {
            window.update(|_, cx| gpui_kit::component::Theme::change(mode, None, cx));
            for (width, height) in [(300.0, 440.0), (280.0, 280.0)] {
                window.simulate_resize(size(px(width), px(height)));
                for state in [
                    ManualState::Running,
                    ManualState::Cancelling,
                    ManualState::Succeeded,
                    ManualState::NotHandled,
                    ManualState::Cancelled,
                    ManualState::Failed(nxtools_extensions::RuntimeError::Timeout),
                ] {
                    owner.update(window, |app, cx| {
                        let run = app.manual_runs.front_mut().expect("demo run");
                        run.state = state.clone();
                        run.items = (1..=64).collect();
                        run.message = Some("演示插件长说明，非宿主保存证明。\n".repeat(60));
                        cx.notify();
                    });
                    window.run_until_parked();
                    window.update(|window, cx| window.draw(cx).clear(cx));
                    window.update(|window, cx| window.draw(cx).clear(cx));
                    let footer = window
                        .debug_bounds("extension-manual-footer")
                        .expect("pinned footer");
                    let scroll = window
                        .debug_bounds("extension-manual-scroll")
                        .expect("body scroll");
                    assert!(scroll.size.height > px(0.0) && scroll.bottom() <= footer.top());
                    let nodes =
                        window.update(|window, _| gpui_kit::base::test_support::snapshots(window));
                    for name in [
                        "extension-result-previous",
                        "extension-result-next",
                        "extension-view-result-0",
                    ] {
                        let node = nodes
                            .iter()
                            .find(|node| {
                                node.path().last() == Some(&gpui_kit::ElementId::from(name))
                            })
                            .expect("pinned result control");
                        let bounds = node.bounds();
                        assert!(
                            bounds.size.height
                                == px(if name == "extension-view-result-0" {
                                    32.0
                                } else {
                                    24.0
                                })
                                && bounds.left() >= px(0.0)
                                && bounds.right() <= px(width)
                                && bounds.top() >= footer.top()
                                && bounds.bottom() <= px(height),
                            "{name}: {bounds:?}"
                        );
                        if name == "extension-view-result-0" {
                            window.simulate_click(bounds.center(), gpui_kit::Modifiers::default());
                            window.run_until_parked();
                            assert!(
                                window.update(|window, cx| window.has_active_dialog(cx)),
                                "closed origin click must not navigate or close feedback"
                            );
                        }
                    }
                    assert_eq!(
                        nodes.iter().any(|node| node.path().last()
                            == Some(&gpui_kit::ElementId::from("extension-cancel-this-run"))),
                        state.pending(),
                        "observer must repaint body AND footer"
                    );
                }
            }
        }
        // Page to a different host ID without executing anything, then verify the
        // original indexed result-control identity is retained.
        let next = window.update(|window, _| {
            gpui_kit::base::test_support::snapshots(window)
                .into_iter()
                .find(|node| {
                    node.path().last() == Some(&gpui_kit::ElementId::from("extension-result-next"))
                })
                .expect("next result")
                .bounds()
        });
        window.simulate_click(next.center(), gpui_kit::Modifiers::default());
        window.run_until_parked();
        window.update(|window, cx| window.draw(cx).clear(cx));
        assert!(window.update(|window, _| {
            gpui_kit::base::test_support::snapshots(window)
                .iter()
                .any(|node| {
                    node.path().last()
                        == Some(&gpui_kit::ElementId::from("extension-view-result-1"))
                })
        }));
        owner.update(window, |app, cx| {
            app.manual_runs.clear();
            cx.notify();
        });
        window.run_until_parked();
        window.update(|window, cx| window.draw(cx).clear(cx));
        assert!(
            window.debug_bounds("extension-feedback-close").is_some(),
            "expired ledger still has Close"
        );
    }
    fn item_local_fixture(id: i64) -> nxtools_clipboard::ClipboardItem {
        nxtools_clipboard::ClipboardItem {
            id,
            kind: nxtools_clipboard::ClipboardKind::Text,
            text: format!("synthetic item {id}"),
            image_path: String::new(),
            image_width: 0,
            image_height: 0,
            file_paths: vec![],
            source: nxtools_clipboard::CaptureSource {
                window_handle: 0,
                process_id: 0,
                process_name: "Fixture".into(),
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
    fn manual_feedback_current_origin_result_is_enabled_and_keeps_original_window(
        cx: &mut TestAppContext,
    ) {
        cx.executor().allow_parking();
        cx.update(|cx| {
            gpui_kit::init(cx);
            cx.set_reduce_motion(true);
        });
        let notification_center = cx.new(|_| crate::notifications::NotificationCenter::default());
        cx.update(|cx| crate::notifications::register(&notification_center, cx));
        let directory = tempfile::tempdir().expect("isolated demo history");
        let worker = crate::clipboard_worker::ClipboardCaptureWorkerHandle::spawn(
            directory.path().join("history.db"),
            directory.path().join("assets"),
            nxtools_clipboard::ClipboardRetentionPolicy {
                max_items: 100,
                retention_days: 30,
            },
            100,
        )
        .expect("isolated history worker");
        let held = worker.clone();
        let session = uuid::Uuid::new_v4();
        let panel_slot = std::rc::Rc::new(std::cell::RefCell::new(None));
        let retained = panel_slot.clone();
        let (_, window) = cx.add_window_view(move |window, cx| {
            let panel = cx.new(|cx| {
                crate::ui::ClipboardPanel::new(
                    nxtools_clipboard::ClipboardHistoryPage {
                        items: vec![
                            item_local_fixture(41),
                            {
                                let mut item = item_local_fixture(42);
                                item.text = "长来源段落\n\n".repeat(30);
                                item
                            },
                            item_local_fixture(43),
                        ],
                        total: 0,
                        has_more: false,
                    },
                    vec![nxtools_clipboard::ClipboardCategory {
                        id: nxtools_clipboard::BUILTIN_ALL_CATEGORY_ID.into(),
                        name: "全部".into(),
                        kind: nxtools_clipboard::ClipboardCategoryKind::All,
                        sort_order: 0,
                    }],
                    Default::default(),
                    vec![],
                    Default::default(),
                    nxtools_protocol::ForegroundContext {
                        window_handle: 0,
                        focus_window_handle: 0,
                        process_id: 0,
                        process_name: String::new(),
                        window_title: String::new(),
                    },
                    held,
                    WeakEntity::new_invalid(),
                    session,
                    None,
                    window,
                    cx,
                )
            });
            *retained.borrow_mut() = Some(panel.clone());
            gpui_kit::base::Root::new(panel, window, cx)
        });
        let panel = panel_slot.borrow().clone().expect("demo panel");
        let source_window = window.update(|window, _| window.window_handle());
        let owner = window.update(|_, cx| {
            cx.new(|_| {
                ExtensionApplication::new(
                    Default::default(),
                    Default::default(),
                    None,
                    WeakEntity::new_invalid(),
                )
            })
        });
        let id = uuid::Uuid::new_v4();
        owner.update(window, |app, _| {
            app.begin_manual_run(id, "demo".into(), "演示插件".into(), "演示动作".into());
            let run = app.manual_runs.back_mut().expect("demo run");
            run.state = crate::extensions::manual_runs::ManualState::Succeeded;
            run.items = vec![17];
            run.message = Some("演示长说明\n".repeat(100));
            run.origin = Some(ClipboardOrigin {
                panel: panel.downgrade(),
                session_id: session,
                window: source_window,
                item_id: 42,
            });
        });
        let inline_owner = owner.clone();
        let inline_origin = ClipboardOrigin {
            panel: panel.downgrade(),
            session_id: session,
            window: source_window,
            item_id: 42,
        };
        panel.update_in(window, |panel, window, cx| {
            panel.show_extension_feedback(inline_owner, id, inline_origin, window, cx)
        });
        window.simulate_resize(size(px(300.0), px(440.0)));
        window.update(|window, cx| window.draw(cx).clear(cx));
        let source_row = window
            .debug_bounds("history-row-slot-1")
            .expect("second source row");
        let first_row = window
            .debug_bounds("history-row-slot-0")
            .expect("other row");
        let feedback = window
            .debug_bounds("clipboard-extension-feedback")
            .expect("feedback");
        assert!(
            feedback.top() >= source_row.top() && feedback.bottom() <= source_row.bottom(),
            "processing feedback must live inside its source item, not panel top: {feedback:?} vs {source_row:?}"
        );
        assert!(
            feedback.top() >= first_row.bottom(),
            "other item must not carry source feedback"
        );
        window.update(|window, cx| window.draw(cx).clear(cx));
        let compact = window
            .debug_bounds("clipboard-extension-feedback")
            .expect("inline feedback");
        assert!(
            compact.size.height <= px(112.0),
            "feedback must leave history usable: {compact:?}"
        );
        assert!(
            !window.update(|window, cx| window.has_active_dialog(cx)),
            "completion does not open modal"
        );
        // A pending run may already have host-confirmed writes. Keep every action
        // inside the original item even with a long action name and all64 results.
        window.simulate_resize(size(px(300.0), px(800.0)));
        for state in [
            crate::extensions::manual_runs::ManualState::Running,
            crate::extensions::manual_runs::ManualState::Cancelling,
            crate::extensions::manual_runs::ManualState::Succeeded,
        ] {
            for height in [120, 160, 210, 320] {
                panel.update_in(window, |panel, _, cx| {
                    panel.set_text_card_max_height(height, cx)
                });
                owner.update(window, |app, cx| {
                    let run = app
                        .manual_runs
                        .iter_mut()
                        .find(|run| run.id == id)
                        .expect("source fixture run");
                    run.action_name = "很长的扩展处理动作名称，不得挤出条目中的操作按钮".repeat(8);
                    run.items = (1..=64).collect();
                    run.state = state.clone();
                    cx.notify();
                });
                window.run_until_parked();
                window.update(|window, cx| window.draw(cx).clear(cx));
                let item = window
                    .debug_bounds("clipboard-item-42")
                    .expect("source item");
                assert_eq!(
                    item.size.height,
                    px(height as f32),
                    "feedback stays inside text card maximum"
                );
                let feedback = window
                    .debug_bounds("clipboard-item-feedback-42")
                    .expect("item feedback");
                let status = window
                    .debug_bounds("clipboard-item-status-42")
                    .expect("readable status");
                assert!(
                    status.size.width >= px(200.0),
                    "status must not be squeezed by the action row: {status:?}"
                );
                for selector in [
                    "clipboard-item-result-42",
                    "clipboard-item-next-42",
                    "clipboard-item-details-42",
                    "clipboard-item-dismiss-42",
                ]
                .into_iter()
                .chain(state.pending().then_some("clipboard-item-cancel-42"))
                {
                    let control = window.debug_bounds(selector).expect("actual item control");
                    assert!(
                        control.size.width > px(0.0)
                            && control.left() >= feedback.left()
                            && control.right() <= feedback.right()
                            && control.top() >= feedback.top()
                            && control.bottom() <= feedback.bottom()
                            && control.bottom() <= item.bottom(),
                        "{selector} must remain inside the source item: {control:?} vs {feedback:?}"
                    );
                }
            }
        }
        panel.update_in(window, |panel, _, cx| {
            panel.set_text_card_max_height(160, cx)
        });
        window.simulate_resize(size(px(300.0), px(440.0)));
        // The cap-specific long-body check is complete. Restore the original short
        // three-row geometry before the existing reorder/filter/pointer lifecycle.
        panel.update_in(window, |panel, _, cx| {
            let mut restored = panel.filter_item_feedback_fixture(42, cx);
            restored[0].text = item_local_fixture(42).text;
            panel.restore_item_feedback_fixture(restored, cx);
            panel.reorder_item_feedback_fixture(cx);
        });
        window.update(|window, cx| window.draw(cx).clear(cx));
        owner.update(window, |app, cx| {
            let run = app
                .manual_runs
                .iter_mut()
                .find(|run| run.id == id)
                .expect("source fixture run");
            run.action_name = "演示动作".into();
            run.items = vec![17];
            cx.notify();
        });
        let cancel_id = uuid::Uuid::new_v4();
        let other_id = uuid::Uuid::new_v4();
        let (cancel_tx, cancel_rx) = tokio::sync::watch::channel(false);
        let (other_tx, other_rx) = tokio::sync::watch::channel(false);
        owner.update(window, |app, cx| {
            app.begin_manual_run(
                cancel_id,
                "first".into(),
                "工具".into(),
                "处理原记录".into(),
            );
            app.begin_manual_run(
                other_id,
                "second".into(),
                "工具".into(),
                "处理另一条".into(),
            );
            app.pending_manual
                .insert(cancel_id, ("first".into(), cancel_tx));
            app.pending_manual
                .insert(other_id, ("second".into(), other_tx));
            cx.notify();
        });
        let retained_owner = owner.clone();
        panel.update_in(window, |panel, window, cx| {
            panel.show_extension_feedback(
                retained_owner.clone(),
                cancel_id,
                ClipboardOrigin {
                    panel: cx.entity().downgrade(),
                    session_id: session,
                    window: source_window,
                    item_id: 42,
                },
                window,
                cx,
            );
            panel.show_extension_feedback(
                retained_owner,
                other_id,
                ClipboardOrigin {
                    panel: cx.entity().downgrade(),
                    session_id: session,
                    window: source_window,
                    item_id: 41,
                },
                window,
                cx,
            );
        });
        window.update(|_, cx| {
            cx.set_global(crate::ui::motion::UiMotionPreference::Known {
                reduced_motion: false,
            });
            cx.set_reduce_motion(false);
        });
        window.update(|window, cx| window.draw(cx).clear(cx));
        assert!(
            window
                .debug_bounds("clipboard-item-processing-animated-42")
                .is_some()
        );
        assert!(
            window.update(|window, cx| window.simulate_next_frame(cx)) > 0,
            "visible pending item requests official spinner animation frames"
        );
        assert!(
            window.debug_bounds("clipboard-item-feedback-41").is_some(),
            "other source has independent status"
        );
        window.update(|_, cx| {
            cx.set_global(crate::ui::motion::UiMotionPreference::Known {
                reduced_motion: true,
            });
            cx.set_reduce_motion(true);
            cx.refresh_windows();
        });
        window.run_until_parked();
        window.update(|window, cx| window.draw(cx).clear(cx));
        assert!(
            window
                .debug_bounds("clipboard-item-processing-static-42")
                .is_some()
        );
        assert!(
            window
                .debug_bounds("clipboard-item-processing-animated-42")
                .is_none()
        );
        window.update(|window, cx| window.simulate_next_frame(cx));
        assert_eq!(
            window.update(|window, cx| window.simulate_next_frame(cx)),
            0,
            "reduced-motion rows must not continuously request frames"
        );
        let cancel_button = window
            .debug_bounds("clipboard-item-cancel-42")
            .expect("row cancel");
        let expanded = window
            .debug_bounds("clipboard-item-42")
            .expect("expanded source");
        assert!(
            cancel_button.top() >= expanded.top() && cancel_button.bottom() <= expanded.bottom()
        );
        assert_eq!(cancel_button.size.height, px(32.0));
        window.simulate_click(cancel_button.center(), Default::default());
        window.run_until_parked();
        assert!(*cancel_rx.borrow());
        assert!(!*other_rx.borrow(), "one row cannot cancel another");
        assert!(
            !panel.read_with(window, |panel, _| panel.item_feedback_test_state(42).1),
            "control click cannot start paste"
        );
        window.update(|_, cx| {
            cx.set_global(crate::ui::motion::UiMotionPreference::Known {
                reduced_motion: false,
            });
            cx.set_reduce_motion(false);
        });
        owner.update(window, |app, cx| {
            let run = app
                .manual_runs
                .iter_mut()
                .find(|run| run.id == cancel_id)
                .expect("source run");
            run.state = crate::extensions::manual_runs::ManualState::NotHandled;
            cx.notify();
        });
        window.run_until_parked();
        window.update(|window, cx| window.draw(cx).clear(cx));
        assert!(
            window
                .debug_bounds("clipboard-item-processing-animated-42")
                .is_none(),
            "terminal stops busy animation"
        );
        assert!(
            window
                .debug_bounds("clipboard-item-processing-static-42")
                .is_some()
        );
        owner.update(window, |app, cx| {
            app.manual_runs
                .iter_mut()
                .find(|run| run.id == other_id)
                .expect("other run")
                .state = crate::extensions::manual_runs::ManualState::NotHandled;
            cx.notify();
        });
        window.run_until_parked();
        window.update(|window, cx| window.draw(cx).clear(cx));
        window.update(|window, cx| window.simulate_next_frame(cx));
        assert_eq!(
            window.update(|window, cx| window.simulate_next_frame(cx)),
            0,
            "terminal rows must not keep repeating animation"
        );
        let feedback_entity =
            panel.read_with(window, |panel, _| panel.item_feedback_test_state(42).0);
        panel.update_in(window, |panel, _, cx| {
            panel.reorder_item_feedback_fixture(cx)
        });
        window.update(|window, cx| window.draw(cx).clear(cx));
        let moved = window
            .debug_bounds("clipboard-item-42")
            .expect("same item after reorder");
        let moved_feedback = window
            .debug_bounds("clipboard-item-feedback-42")
            .expect("same feedback after reorder");
        assert!(moved_feedback.top() >= moved.top() && moved_feedback.bottom() <= moved.bottom());
        assert_eq!(
            panel.read_with(window, |panel, _| panel.item_feedback_test_state(42).0),
            feedback_entity
        );
        let hidden = panel.update_in(window, |panel, _, cx| {
            panel.filter_item_feedback_fixture(42, cx)
        });
        window.update(|window, cx| window.draw(cx).clear(cx));
        assert!(
            window.debug_bounds("clipboard-item-feedback-42").is_none(),
            "filtered source has no feedback elsewhere"
        );
        panel.update_in(window, |panel, _, cx| {
            panel.restore_item_feedback_fixture(hidden, cx)
        });
        window.update(|window, cx| window.draw(cx).clear(cx));
        let restored = window
            .debug_bounds("clipboard-item-42")
            .expect("restored source");
        let restored_feedback = window
            .debug_bounds("clipboard-item-feedback-42")
            .expect("restored feedback");
        assert!(
            restored_feedback.top() >= restored.top()
                && restored_feedback.bottom() <= restored.bottom()
        );
        window.update(|_, cx| {
            cx.set_global(crate::ui::motion::UiMotionPreference::Known {
                reduced_motion: true,
            });
            cx.set_reduce_motion(true);
        });
        let item_close = window
            .debug_bounds("clipboard-item-dismiss-42")
            .expect("source close");
        window.simulate_click(item_close.center(), Default::default());
        window.run_until_parked();
        window.update(|window, cx| window.draw(cx).clear(cx));
        assert!(window.debug_bounds("clipboard-item-feedback-42").is_none());
        assert!(
            window.debug_bounds("clipboard-item-feedback-41").is_some(),
            "dismiss leaves other row"
        );
        assert!(!panel.read_with(window, |panel, _| panel.item_feedback_test_state(42).1));
        let retained_owner = owner.clone();
        panel.update_in(window, |panel, window, cx| {
            panel.show_extension_feedback(
                retained_owner,
                id,
                ClipboardOrigin {
                    panel: cx.entity().downgrade(),
                    session_id: session,
                    window: source_window,
                    item_id: 42,
                },
                window,
                cx,
            )
        });
        window.update(|window, cx| window.draw(cx).clear(cx));
        let dismiss = window
            .debug_bounds("clipboard-item-dismiss-42")
            .expect("item-local dismiss");
        window.simulate_click(dismiss.center(), Default::default());
        window.run_until_parked();
        window.update(|window, cx| window.draw(cx).clear(cx));
        assert!(window.debug_bounds("clipboard-item-feedback-42").is_none());

        window.update(|window, cx| show_run(owner.clone(), id, None, window, cx));
        for (width, height) in [(300.0, 440.0), (280.0, 280.0)] {
            window.simulate_resize(size(px(width), px(height)));
            window.update(|window, cx| window.draw(cx).clear(cx));
            window.update(|window, cx| window.draw(cx).clear(cx));
            let result = window.update(|window, _| {
                gpui_kit::base::test_support::snapshots(window)
                    .into_iter()
                    .find(|node| {
                        node.path().last()
                            == Some(&gpui_kit::ElementId::from("extension-view-result-0"))
                    })
                    .expect("actual result action")
            });
            assert_ne!(
                result.disabled(),
                Some(true),
                "current original panel supports explicit viewing"
            );
            let bounds = result.bounds();
            assert!(
                bounds.left() >= px(0.0)
                    && bounds.right() <= px(width)
                    && bounds.bottom() <= px(height)
            );
            assert_eq!(
                window.update(|window, _| window.window_handle()),
                source_window
            );
        }
        // Close only: this fixture never invokes native copy/paste or any plugin.
        let close = window
            .debug_bounds("extension-feedback-close")
            .expect("Close");
        window.simulate_click(close.center(), gpui_kit::Modifiers::default());
        window.run_until_parked();
        assert!(panel.read_with(window, |panel, _| panel.extension_origin_current(session)));
        let held_center = notification_center.clone();
        owner.update(window, |app, cx| {
            app.notifications = Some(held_center);
            app.notify_manual_run(id, cx);
        });
        window.run_until_parked();
        window.update(|window, cx| window.draw(cx).clear(cx));
        let toast_selector: &'static str =
            Box::leak(format!("notification-extension-manual-{id}").into_boxed_str());
        assert!(
            window.debug_bounds(toast_selector).is_some(),
            "extension terminal uses shared renderer in original Clipboard window"
        );
        assert!(
            !window.update(|window, cx| window.has_active_dialog(cx)),
            "terminal never opens details automatically"
        );
        assert!(
            !panel.read_with(window, |panel, _| panel.item_feedback_test_state(42).1),
            "notification rendering cannot paste"
        );
        // Admission uses the same center for progress, never adds a second row notice.
        let active = uuid::Uuid::new_v4();
        let (cancel, cancelled) = tokio::sync::watch::channel(false);
        owner.update(window, |app, _| {
            app.begin_manual_run(active, "demo".into(), "演示插件".into(), "等待动作".into());
            app.pending_manual.insert(active, ("demo".into(), cancel));
            app.attach_manual_origin(
                active,
                Some(ClipboardOrigin {
                    panel: panel.downgrade(),
                    session_id: session,
                    window: source_window,
                    item_id: 43,
                }),
            );
        });
        window.update(|_, cx| publish_admitted_run(owner.clone(), active, cx));
        window.run_until_parked();
        window.update(|window, cx| window.draw(cx).clear(cx));
        assert!(
            window.debug_bounds("clipboard-item-feedback-43").is_none(),
            "no parallel transient row feedback"
        );
        assert!(
            notification_center.read_with(window, |center, _| center.items().iter().any(
                |notice| notice.id.as_ref() == format!("extension-manual-{active}")
                    && notice.level == crate::notifications::NotificationLevel::Progress
            ))
        );
        owner.update(window, |app, cx| app.cancel_manual_run(active, cx));
        window.run_until_parked();
        assert!(
            *cancelled.borrow(),
            "original cancellation channel retained"
        );
        assert!(
            notification_center.read_with(window, |center, _| center.items().iter().any(
                |notice| notice.id.as_ref() == format!("extension-manual-{active}")
                    && notice.message.contains("取消")
            ))
        );
        worker.shutdown().expect("isolated worker shutdown");
    }
    #[gpui_kit::test]
    fn no_manual_actions_without_current_owner_and_no_automatic_dispatch(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        assert!(cx.update(|cx| available("downloads", cx).is_empty()));
        assert!(cx.update(|cx| available("clipboard", cx).is_empty()));
    }
    #[gpui_kit::test]
    fn expansion_progress_actual_script_manual_projection_and_official_four_dip_track(
        cx: &mut TestAppContext,
    ) {
        cx.update(|cx| {
            gpui_kit::init(cx);
            cx.set_reduce_motion(true);
        });
        let root = tempfile::tempdir().expect("owned progress fixture");
        let source = root.path().join("source");
        std::fs::create_dir(&source).expect("source");
        let manifest = serde_json::json!({"schema_version":1,"id":"fixture.progress","name":"Progress","version":"1","runtime":{"program":"python","protocol_version":2},"files":["entry.py"],"handlers":[{"id":"run","title":"Report","feature":"tools","entry":"action","action":{"type":"script","entry_file":"entry.py"}}]});
        std::fs::write(
            source.join("extension.json"),
            serde_json::to_vec(&manifest).expect("manifest"),
        )
        .expect("write manifest");
        std::fs::write(source.join("entry.py"),r#"import json,sys,time,os
i=json.loads(sys.stdin.readline()); s=i['session_id']
print(json.dumps(dict(protocol_version=2,op='ready',session_id=s)),flush=True)
f=json.loads(sys.stdin.readline()); r=f['run_id']; barrier=f['context']['owned_barrier']
def emit(op,**fields): print(json.dumps(dict(protocol_version=2,op=op,session_id=s,run_id=r,**fields)),flush=True)
emit('progress',sequence=1,completed=1,total=2,message='Known fixture')
while not os.path.exists(barrier): time.sleep(0.01)
emit('progress',sequence=2,message='Unknown fixture')
while not os.path.exists(barrier+'-finish'): time.sleep(0.01)
emit('complete',status='success',value={})
emit('request_finished')
"#).expect("owned script");
        let preview = nxtools_extensions::ImportPreview::from_directory(&source)
            .expect("preview")
            .stage_into(&root.path().join("packages"))
            .expect("stage");
        let hash = preview.content_hash.clone();
        let mut config = nxtools_core::AppConfig::default();
        config
            .extensions
            .install(preview, "owned".into(), false)
            .expect("install");
        let python = std::env::split_paths(&std::env::var_os("PATH").expect("PATH"))
            .map(|directory| directory.join("python.exe"))
            .find(|path| path.is_file())
            .expect("interpreter");
        config
            .extensions
            .approve(
                "fixture.progress",
                &hash,
                nxtools_extensions::RuntimeIdentity::inspect(&python).expect("identity"),
            )
            .expect("approve");
        config
            .extensions
            .set_enabled("fixture.progress", true, &[])
            .expect("enable");
        let engine = nxtools_download::DownloadEngine::open(root.path().join("downloads.db"))
            .expect("temporary engine");
        let (service, _) = crate::extensions::service::ExtensionService::new(
            &engine,
            config.extensions.clone(),
            None,
            false,
        )
        .expect("service");
        let broker=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).ancestors().nth(3).expect("workspace").join("csharp/NxTools.PlatformHost.Tests/bin/Debug/net8.0-windows/NxTools.PlatformHost.Tests.exe");
        {
            let executor = service.executor();
            let _entered = executor.enter();
            service
                .runtime
                .bind_owned_fixture_broker(&broker)
                .expect("owned broker");
        }
        cx.executor().allow_parking();
        let owner = cx.new(|_| {
            let mut app = ExtensionApplication::new(
                config,
                root.path().join("unused.json"),
                None,
                WeakEntity::new_invalid(),
            );
            app.service = Some(service.clone());
            app
        });
        let barrier = root.path().join("barrier");
        let id = owner.update(cx, |app, cx| {
            app.observe_runtime_progress(cx);
            app.start_manual(
                "fixture.progress".into(),
                "run".into(),
                "tools",
                serde_json::json!({"owned_barrier":barrier}),
                cx,
            )
            .expect("start")
        });
        let (_, window) = cx.add_window_view(|window, cx| {
            let content = cx.new(|_| Fixture);
            gpui_kit::base::Root::new(content, window, cx)
        });
        window.simulate_resize(size(px(300.0), px(440.0)));
        window.update(|window, cx| show_run(owner.clone(), id, None, window, cx));
        for sequence in [1, 2] {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(8);
            loop {
                window.run_until_parked();
                let run = owner.read_with(window, |app, _| {
                    app.manual_run(id).expect("manual projection")
                });
                if let Some(progress) = run
                    .progress
                    .filter(|progress| progress.sequence == sequence)
                {
                    assert_eq!(
                        progress.percentage(),
                        if sequence == 1 { Some(50.0) } else { None }
                    );
                    break;
                }
                assert!(
                    std::time::Instant::now() < deadline,
                    "actual script progress must reach current manual owner"
                );
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            window.update(|window, cx| window.draw(cx).clear(cx));
            let track = window
                .debug_bounds("extension-progress-track")
                .expect("painted official Progress");
            assert_eq!(track.size.height, px(4.0));
            assert!(
                track.size.width >= px(180.0)
                    && track.left() >= px(0.0)
                    && track.right() <= px(300.0)
            );
            let footer = window
                .debug_bounds("extension-manual-footer")
                .expect("retained footer");
            assert!(track.bottom() <= footer.top() && footer.bottom() <= px(440.0));
            if sequence == 1 {
                std::fs::write(&barrier, []).expect("next synthetic stage");
            }
        }
        // Current identity gate rejects a generation mismatch without changing the runtime.
        owner.update(window, |app, _| {
            let run = app
                .manual_runs
                .iter_mut()
                .find(|run| run.id == id)
                .expect("run");
            run.generation = run.generation.wrapping_add(1);
        });
        assert!(owner.read_with(window, |app, _| {
            app.manual_run(id).expect("run").progress.is_none()
        }));
        std::fs::write(barrier.with_file_name("barrier-finish"), [])
            .expect("finish synthetic stage");
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(8);
        while owner.read_with(window, |app, _| app.has_pending_manual()) {
            window.run_until_parked();
            assert!(
                std::time::Instant::now() < deadline,
                "native cleanup and terminal"
            );
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        window.update(|window, cx| window.draw(cx).clear(cx));
        assert!(
            window.debug_bounds("extension-progress-track").is_none(),
            "terminal clears progress"
        );
        service.runtime.shutdown();
    }
}
