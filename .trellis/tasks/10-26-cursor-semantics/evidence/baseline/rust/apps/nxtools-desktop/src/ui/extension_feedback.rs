//! Explicit navigation only; no native action and no automatic focus/selection on completion.
use super::*;
use gpui_kit::base::ElementExt as _;
use gpui_kit::component::{Disableable, Sizable, button::Button, spinner::Spinner};

pub(super) const ITEM_FEEDBACK_HEIGHT: f32 = 76.0;

pub(super) struct InlineExtensionFeedback {
    owner: Entity<crate::extensions::ExtensionApplication>,
    id: uuid::Uuid,
    origin: crate::extensions::manual_actions::ClipboardOrigin,
    selected_result: usize,
    source_item_id: i64,
    painted_visible: bool,
    _subscription: Subscription,
}
impl Render for InlineExtensionFeedback {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        crate::ui::motion::observe_motion(window, cx);
        let run = self.owner.read(cx).manual_run(self.id);
        let Some(run) = run else {
            return div().into_any_element();
        };
        let current = self.origin.panel.upgrade().is_some_and(|panel| {
            panel
                .read(cx)
                .extension_origin_current(self.origin.session_id)
        });
        let visible = self.painted_visible;
        if !current {
            return div().into_any_element();
        }
        let id = self.id;
        let source_item_id = self.source_item_id;
        let owner = self.owner.clone();
        let details_owner = self.owner.clone();
        let details_origin = self.origin.clone();
        let dismiss = self.origin.clone();
        let mut actions = div().flex().flex_wrap().items_center().gap_2();
        if run.state.pending() {
            let cancelling = matches!(
                run.state,
                crate::extensions::manual_runs::ManualState::Cancelling
            );
            actions = actions.child(
                Button::new(("clipboard-extension-cancel", source_item_id as u64))
                    .debug_selector(move || format!("clipboard-item-cancel-{source_item_id}"))
                    .label(if cancelling { "正在取消" } else { "取消" })
                    .outline()
                    .disabled(cancelling)
                    .on_click(move |_, _, cx| {
                        cx.stop_propagation();
                        let owner = owner.clone();
                        cx.defer(move |cx| {
                            owner.update(cx, |app, cx| app.cancel_manual_run(id, cx))
                        });
                    }),
            );
        }
        if !run.items.is_empty() {
            let index = self.selected_result.min(run.items.len() - 1);
            let item = run.items[index];
            let origin = self.origin.clone();
            actions = actions.child(
                Button::new(("clipboard-extension-view-result", source_item_id as u64))
                    .debug_selector(move || format!("clipboard-item-result-{source_item_id}"))
                    .label("查看结果")
                    .outline()
                    .on_click(move |_, _, cx| {
                        cx.stop_propagation();
                        let origin = origin.clone();
                        cx.defer(move |cx| {
                            let _ = origin.window.update(cx, |_, window, cx| {
                                let _ = origin.panel.update(cx, |panel, cx| {
                                    panel.locate_extension_result(
                                        origin.session_id,
                                        id,
                                        item,
                                        window,
                                        cx,
                                    )
                                });
                            });
                        });
                    }),
            );
            if run.items.len() > 1 {
                let result_count = run.items.len();
                actions = actions.child(
                    Button::new(("clipboard-extension-next-result", source_item_id as u64))
                        .debug_selector(move || format!("clipboard-item-next-{source_item_id}"))
                        .label(format!("{} / {}", index + 1, run.items.len()))
                        .outline()
                        .small()
                        .on_click(cx.listener(move |view, _, _, cx| {
                            cx.stop_propagation();
                            view.selected_result = (view.selected_result + 1) % result_count;
                            cx.notify();
                        })),
                );
            }
        }
        actions = actions.child(
            Button::new(("clipboard-extension-details", source_item_id as u64))
                .debug_selector(move || format!("clipboard-item-details-{source_item_id}"))
                .label("详情")
                .outline()
                .on_click(move |_, window, cx| {
                    cx.stop_propagation();
                    crate::extensions::manual_actions::show_run(
                        details_owner.clone(),
                        id,
                        Some(details_origin.clone()),
                        window,
                        cx,
                    )
                }),
        );
        let action_name = run.action_name.clone();
        let summary = if !run.items.is_empty() {
            let saved = format!("已保存 {} 个结果", run.items.len());
            match &run.state {
                crate::extensions::manual_runs::ManualState::Running => {
                    format!("正在处理 · {saved}")
                }
                crate::extensions::manual_runs::ManualState::Cancelling => {
                    format!("正在取消 · {saved}")
                }
                _ => format!("{saved}，原文保留"),
            }
        } else {
            match &run.state {
                crate::extensions::manual_runs::ManualState::Running => "正在处理…".into(),
                crate::extensions::manual_runs::ManualState::Cancelling => {
                    "正在取消，已产生的效果不能撤回".into()
                }
                crate::extensions::manual_runs::ManualState::NotHandled => {
                    "无需修改或不适用".into()
                }
                crate::extensions::manual_runs::ManualState::Succeeded => "插件报告完成".into(),
                crate::extensions::manual_runs::ManualState::Cancelled => "已取消".into(),
                crate::extensions::manual_runs::ManualState::Failed(_) => {
                    "未能完成，查看详情".into()
                }
            }
        };
        let pending = run.state.pending();
        let source_id = self.source_item_id;
        let indicator = if pending && visible && crate::ui::motion::motion_allowed(cx) {
            div()
                .debug_selector(move || format!("clipboard-item-processing-animated-{source_id}"))
                .child(Spinner::new().small())
                .into_any_element()
        } else {
            let icon = match &run.state {
                crate::extensions::manual_runs::ManualState::Running
                | crate::extensions::manual_runs::ManualState::Cancelling => MaterialIcon::History,
                crate::extensions::manual_runs::ManualState::Succeeded if !run.items.is_empty() => {
                    MaterialIcon::Check
                }
                crate::extensions::manual_runs::ManualState::Cancelled => MaterialIcon::Cancel,
                _ => MaterialIcon::Info,
            };
            div()
                .debug_selector(move || format!("clipboard-item-processing-static-{source_id}"))
                .child(Icon::new(icon).size(px(14.0)))
                .into_any_element()
        };
        div()
            .id(("clipboard-extension-feedback", source_id as u64))
            .debug_selector(move || format!("clipboard-item-feedback-{source_id}"))
            .w_full()
            .min_w_0()
            .h(px(ITEM_FEEDBACK_HEIGHT - 4.0))
            .flex_shrink_0()
            .border_t_1()
            .border_color(Theme::active(cx).border)
            .pt_1()
            .flex()
            .flex_col()
            .gap_1()
            // A feedback/control gesture must never bubble into OriginalPaste on the enclosing item.
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_click(|_, _, cx| cx.stop_propagation())
            .on_prepaint({
                let feedback = cx.weak_entity();
                let was_visible = self.painted_visible;
                move |bounds, window, cx| {
                    let clipped = bounds
                        .intersect(&window.content_mask().bounds)
                        .intersect(&Bounds::new(Default::default(), window.viewport_size()));
                    let visible = clipped.size.width > px(0.0) && clipped.size.height > px(0.0);
                    if visible != was_visible {
                        let feedback = feedback.clone();
                        window.defer(cx, move |_, cx| {
                            let _ = feedback.update(cx, |view, cx| {
                                if view.painted_visible != visible {
                                    view.painted_visible = visible;
                                    cx.notify();
                                }
                            });
                        });
                    }
                }
            })
            .child(
                div()
                    .id(("clipboard-extension-status", source_id as u64))
                    .tooltip(move |window, cx| Tooltip::new(action_name.clone()).build(window, cx))
                    .debug_selector(|| "clipboard-extension-feedback".into())
                    .flex()
                    .items_center()
                    .gap_2()
                    .min_w_0()
                    .h(px(24.0))
                    .flex_shrink_0()
                    .child(indicator)
                    .child(
                        div()
                            .debug_selector(move || format!("clipboard-item-status-{source_id}"))
                            .flex_1()
                            .min_w_0()
                            .text_xs()
                            .truncate()
                            .child(summary),
                    )
                    .child(
                        Button::new(("clipboard-extension-dismiss", source_id as u64))
                            .debug_selector(move || format!("clipboard-item-dismiss-{source_id}"))
                            .icon(MaterialIcon::Close)
                            .ghost()
                            .xsmall()
                            .accessibility_label("收起此条目的处理状态")
                            .on_click(move |_, _, cx| {
                                cx.stop_propagation();
                                let origin = dismiss.clone();
                                cx.defer(move |cx| {
                                    let _ = origin.panel.update(cx, |panel, cx| {
                                        panel.dismiss_extension_feedback(
                                            origin.session_id,
                                            source_id,
                                            id,
                                            cx,
                                        )
                                    });
                                });
                            }),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .min_w_0()
                    .h(px(32.0))
                    .flex_shrink_0()
                    .child(actions),
            )
            .into_any_element()
    }
}

impl ClipboardPanel {
    fn notify_extension_operation(
        &self,
        level: crate::notifications::NotificationLevel,
        message: &'static str,
        cx: &mut Context<Self>,
    ) {
        if let Some(center) = crate::notifications::current(cx) {
            let notice = crate::notifications::AppNotification::new(
                "clipboard-extension-operation",
                level,
                "剪贴板扩展",
                message,
            );
            cx.defer(move |cx| center.update(cx, |center, cx| center.push(notice, cx)));
        }
    }

    #[cfg(any(test, debug_assertions))]
    pub(crate) fn show_extension_feedback(
        // Retained only for old geometry fixtures and the isolated visual preview.
        &mut self,
        owner: Entity<crate::extensions::ExtensionApplication>,
        id: uuid::Uuid,
        origin: crate::extensions::manual_actions::ClipboardOrigin,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.extension_origin_current(origin.session_id) {
            return;
        }
        if origin.item_id <= 0 {
            return;
        }
        let source_item_id = origin.item_id;
        let retired = self
            .extension_feedback
            .iter()
            .filter_map(|(key, feedback)| {
                owner
                    .read(cx)
                    .manual_run(feedback.read(cx).id)
                    .is_none()
                    .then_some(*key)
            })
            .collect::<Vec<_>>();
        for retired in retired {
            self.extension_feedback.remove(&retired);
            self.remeasure_extension_item(retired);
        }
        if self.extension_feedback.len() >= 36
            && !self.extension_feedback.contains_key(&source_item_id)
        {
            let retired = self.extension_feedback.iter().find_map(|(key, feedback)| {
                owner
                    .read(cx)
                    .manual_run(feedback.read(cx).id)
                    .is_some_and(|run| !run.state.pending())
                    .then_some(*key)
            });
            if let Some(retired) = retired {
                self.extension_feedback.remove(&retired);
                self.remeasure_extension_item(retired);
            } else {
                return;
            }
        }
        let feedback = cx.new(|cx| {
            let subscription = cx.observe(&owner, |_, _, cx| cx.notify());
            InlineExtensionFeedback {
                owner,
                id,
                origin,
                source_item_id,
                selected_result: 0,
                painted_visible: true,
                _subscription: subscription,
            }
        });
        self.extension_feedback.insert(source_item_id, feedback);
        self.remeasure_extension_item(source_item_id);
        cx.notify();
    }
    fn remeasure_extension_item(&self, item_id: i64) {
        if let Some(index) = self
            .entries
            .borrow()
            .iter()
            .position(|item| item.id == item_id)
        {
            self.list_scroll.remeasure_items(index..index + 1);
        }
    }
    pub(crate) fn dismiss_extension_feedback(
        &mut self,
        session: uuid::Uuid,
        item_id: i64,
        run_id: uuid::Uuid,
        cx: &mut Context<Self>,
    ) {
        if self.extension_origin_current(session)
            && self
                .extension_feedback
                .get(&item_id)
                .is_some_and(|feedback| feedback.read(cx).id == run_id)
        {
            self.extension_feedback.remove(&item_id);
            self.remeasure_extension_item(item_id);
            cx.notify();
        }
    }
    #[cfg(test)]
    pub(crate) fn item_feedback_test_state(
        &self,
        item_id: i64,
    ) -> (Option<gpui_kit::EntityId>, bool) {
        (
            self.extension_feedback.get(&item_id).map(Entity::entity_id),
            self.pending_action.is_some(),
        )
    }
    #[cfg(test)]
    pub(crate) fn reorder_item_feedback_fixture(&mut self, cx: &mut Context<Self>) {
        self.selected_entry_index = Some(0);
        self.entries.borrow_mut().swap(1, 2);
        self.list_scroll.remeasure_items(0..3);
        cx.notify();
    }
    #[cfg(test)]
    pub(crate) fn filter_item_feedback_fixture(
        &mut self,
        item_id: i64,
        cx: &mut Context<Self>,
    ) -> Vec<ClipboardItem> {
        let removed = self
            .entries
            .borrow()
            .iter()
            .filter(|item| item.id == item_id)
            .cloned()
            .collect::<Vec<_>>();
        self.entries.borrow_mut().retain(|item| item.id != item_id);
        self.list_scroll
            .remeasure_items(0..self.list_scroll.item_count());
        self.list_scroll.reset(self.entries.borrow().len());
        cx.notify();
        removed
    }
    #[cfg(test)]
    pub(crate) fn restore_item_feedback_fixture(
        &mut self,
        items: Vec<ClipboardItem>,
        cx: &mut Context<Self>,
    ) {
        self.entries.borrow_mut().extend(items);
        self.list_scroll.reset(self.entries.borrow().len());
        cx.notify();
    }
    pub(crate) fn extension_start_failed(
        &mut self,
        session: uuid::Uuid,
        message: &'static str,
        cx: &mut Context<Self>,
    ) {
        if self.extension_origin_current(session) {
            self.status = PanelStatus::Ready(String::new());
            self.notify_extension_operation(
                crate::notifications::NotificationLevel::Error,
                message,
                cx,
            );
            cx.notify();
        }
    }
    pub(crate) fn extension_origin_current(&self, session: uuid::Uuid) -> bool {
        self.is_open && self.session_id == session
    }
    pub(crate) fn locate_extension_result(
        &mut self,
        session: uuid::Uuid,
        correlation_id: uuid::Uuid,
        item_id: i64,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.extension_origin_current(session)
            || self.editing_item_id.is_some()
            || self.category_editor.is_some()
        {
            return;
        }
        self.query_dispatch_generation = self.query_dispatch_generation.wrapping_add(1).max(1);
        let generation = self.query_dispatch_generation;
        self.active_category_id = BUILTIN_ALL_CATEGORY_ID.into();
        self.search_scope = ClipboardSearchScope::Content;
        self.search_input
            .update(cx, |input, cx| input.set_value("", window, cx));
        self.search_pending = true;
        self.page_request_pending = false;
        self.status = PanelStatus::Ready(String::new());
        self.notify_extension_operation(
            crate::notifications::NotificationLevel::Progress,
            "正在定位扩展结果…",
            cx,
        );
        let worker = self.history_worker.clone();
        let panel = cx.entity().downgrade();
        cx.spawn(async move |_,cx| {
            let result=worker.summary_page_containing_item_async(item_id,HISTORY_PAGE_SIZE).await;
            let _=panel.update(cx,|panel,cx| {
                if !panel.extension_origin_current(session) || panel.query_dispatch_generation!=generation
                    || !panel.search_input.read(cx).value().is_empty() || panel.active_category_id!=BUILTIN_ALL_CATEGORY_ID
                    || panel.search_scope!=ClipboardSearchScope::Content { return; }
                panel.search_pending=false; panel.page_request_pending=false;
                match result {
                    Ok((offset,page))=> {
                        panel.open_targets.clear();
                        let page=panel.unpack_summary_page(page);
                        let selected=page.items.iter().position(|item|item.id==item_id);
                        panel.next_page_cursor=page.items.last().map(ClipboardPageCursor::from);
                        panel.total_entries=page.total; panel.has_more_entries=page.has_more;
                        panel.window_start_offset=offset;
                        *panel.entries.borrow_mut()=page.items;
                        panel.prune_list_metadata();
                        panel.selected_entry_index=selected;
                        panel.list_scroll.reset(panel.entries.borrow().len());
                        if let Some(index)=selected {panel.list_scroll.scroll_to_reveal_item(index);}
                        panel.list_is_scrolled=offset>0; panel.query_error=None;
                        panel.status=PanelStatus::Ready(String::new());
                        panel.notify_extension_operation(crate::notifications::NotificationLevel::Success, "已定位保存的结果；未复制或粘贴，原条目保留。", cx);
                        info!(event="clipboard.extension_result.located",%correlation_id,session_id=%session,"extension result located");
                    }
                    Err(_)=> {
                        panel.status=PanelStatus::Ready(String::new());
                        panel.notify_extension_operation(crate::notifications::NotificationLevel::Error, "结果已删除、不可用或历史查询失败；没有重新运行插件，请检查历史后再决定。", cx);
                        warn!(event="clipboard.extension_result.failed",%correlation_id,session_id=%session,reason="item_unavailable","extension result unavailable");
                    }
                }
                cx.notify();
            });
        }).detach();
        cx.notify();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::TestAppContext;
    #[gpui_kit::test]
    fn explicit_result_location_uses_real_worker_page_and_ignores_closed_origin(
        cx: &mut TestAppContext,
    ) {
        cx.executor().allow_parking();
        cx.update(gpui_kit::init);
        let center = cx.new(|_| crate::notifications::NotificationCenter::default());
        cx.update(|cx| crate::notifications::register(&center, cx));
        let directory = tempfile::tempdir().expect("fixture");
        let database = directory.path().join("history.db");
        let mut history = nxtools_clipboard::ClipboardHistory::open(&database).expect("history");
        let mut target = 0;
        for index in 0..180 {
            let result = history
                .capture(nxtools_clipboard::TextCapture {
                    text: format!("synthetic-{index}"),
                    native_formats: Vec::new(),
                    source: nxtools_clipboard::CaptureSource {
                        window_handle: 0,
                        process_id: 0,
                        process_name: String::new(),
                        process_icon_path: String::new(),
                        window_title: String::new(),
                    },
                    captured_at_unix_ms: index + 1,
                })
                .expect("capture");
            if index == 10
                && let nxtools_clipboard::CaptureOutcome::Inserted(item) = result
            {
                target = item.id;
            }
        }
        let page = history
            .query_category_page(
                "",
                BUILTIN_ALL_CATEGORY_ID,
                ClipboardSearchScope::Content,
                0,
                80,
            )
            .expect("first page");
        let categories = history.categories().expect("categories");
        drop(history);
        let worker = crate::clipboard_worker::ClipboardCaptureWorkerHandle::spawn(
            database,
            directory.path().join("assets"),
            nxtools_clipboard::ClipboardRetentionPolicy {
                max_items: 1000,
                retention_days: 30,
            },
            180,
        )
        .expect("worker");
        let shutdown = worker.clone();
        let session = uuid::Uuid::new_v4();
        let panel_slot = Rc::new(RefCell::new(None));
        let retained_panel = panel_slot.clone();
        let (_root, window) = cx.add_window_view(move |window, cx| {
            let panel = cx.new(|cx| {
                ClipboardPanel::new(
                    page,
                    categories,
                    HashMap::new(),
                    Vec::new(),
                    ClipboardModifierActions::default(),
                    nxtools_protocol::ForegroundContext {
                        window_handle: 0,
                        focus_window_handle: 0,
                        process_id: 0,
                        process_name: String::new(),
                        window_title: String::new(),
                    },
                    worker,
                    WeakEntity::new_invalid(),
                    session,
                    None,
                    window,
                    cx,
                )
            });
            *retained_panel.borrow_mut() = Some(panel.clone());
            gpui_kit::base::Root::new(panel, window, cx)
        });
        let panel = panel_slot
            .borrow()
            .clone()
            .expect("production panel entity");
        panel.update_in(window, |panel, window, cx| {
            panel.locate_extension_result(session, uuid::Uuid::new_v4(), target, window, cx)
        });
        for _ in 0..100 {
            window.run_until_parked();
            if panel.update_in(window, |panel, _, _| !panel.search_pending) {
                break;
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        panel.update_in(window, |panel, _, _| {
            assert!(!panel.search_pending);
            assert!(panel.window_start_offset >= 80);
            assert_eq!(
                panel.entries.borrow()[panel.selected_entry_index.expect("selected")].id,
                target
            );
            assert!(
                panel.pending_action.is_none(),
                "locating never starts native copy/paste"
            );
        });
        assert!(
            center.read_with(window, |center, _| center
                .items()
                .iter()
                .any(|notice| notice.level
                    == crate::notifications::NotificationLevel::Success
                    && notice.message.contains("已定位")))
        );
        panel.read_with(window, |panel, _| {
            assert!(
                !panel.status.should_render(),
                "result location must not leave a second header notice"
            )
        });
        panel.update_in(window, |panel, window, cx| {
            panel.locate_extension_result(session, uuid::Uuid::new_v4(), target, window, cx);
            panel.query_dispatch_generation = panel.query_dispatch_generation.wrapping_add(1);
            panel.status = PanelStatus::Notice("newer query".into());
        });
        for _ in 0..10 {
            window.run_until_parked();
            std::thread::sleep(Duration::from_millis(2));
        }
        panel.update_in(window, |panel, _, _| {
            assert_eq!(panel.status.label(), "newer query")
        });
        panel.update_in(window, |panel, window, cx| {
            panel.locate_extension_result(session, uuid::Uuid::new_v4(), target, window, cx);
            panel.is_open = false;
            panel.status = PanelStatus::Notice("closed".into());
        });
        for _ in 0..10 {
            window.run_until_parked();
            std::thread::sleep(Duration::from_millis(2));
        }
        panel.update_in(window, |panel, _, _| {
            assert_eq!(
                panel.status.label(),
                "closed",
                "late locate cannot alter closed origin"
            )
        });
        shutdown.shutdown().expect("shutdown");
    }
}
