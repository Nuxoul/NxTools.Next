use super::*;

pub(crate) struct QuickLaunchPanel {
    pub(crate) theme: Theme,
    pub(crate) application: WeakEntity<QuickLaunchApplication>,
    pub(crate) target: nxtools_protocol::ForegroundContext,
    pub(crate) navigation_context: Option<nxtools_protocol::DirectoryNavigationContext>,
    pub(crate) results: Vec<SearchResult>,
    pub(crate) selected: usize,
    pub(crate) action_mode: Option<QuickLaunchActionMode>,
    pub(crate) recent_directory_mode: bool,
    pub(crate) recent_directories: Vec<RecentDirectory>,
    pub(crate) search_input: Entity<InputState>,
    pub(crate) status: Option<String>,
    pub(crate) icons: HashMap<String, PathBuf>,
    pub(crate) is_open: bool,
    pub(crate) results_visible: bool,
    pub(crate) search_pending: bool,
    pub(crate) query_dispatch_generation: u64,
    pub(crate) results_appearance_generation: u64,
    pub(crate) result_scroll: UniformListScrollHandle,
    pub(crate) action_scroll: ScrollHandle,
    pub(crate) visible_result_range: Range<usize>,
    pub(crate) last_presented_icon_range: Option<Range<usize>>,
    pub(crate) pointer_selection_gate: PointerSelectionGate,
    pub(crate) ime_enter_gate: ImeEnterGate,
    pub(crate) hide_suppressed_until: Instant,
    pub(crate) _subscriptions: Vec<Subscription>,
}

pub(crate) fn quick_launch_row_shell(theme: Theme, selected: bool) -> Div {
    div()
        .w_full()
        .h(px(COMPACT_RESULT_ROW_HEIGHT))
        .px(px(SPACING_8))
        .rounded(px(RADIUS_CONTROL))
        .flex()
        .items_center()
        .gap(px(SPACING_8))
        .bg(if selected {
            theme.element_selected
        } else {
            theme.canvas
        })
        .hover(move |row| {
            row.bg(if selected {
                theme.element_selected
            } else {
                theme.surface_hover
            })
        })
}

impl QuickLaunchPanel {
    pub(crate) fn new(
        results: Vec<SearchResult>,
        target: nxtools_protocol::ForegroundContext,
        navigation_context: Option<nxtools_protocol::DirectoryNavigationContext>,
        recent_directories: Vec<RecentDirectory>,
        application: WeakEntity<QuickLaunchApplication>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let search_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("搜索应用、文件或文件夹"));
        let change_subscription = cx.subscribe_in(
            &search_input,
            window,
            |this, state, event: &InputEvent, window, cx| match event {
                InputEvent::Change => {
                    let query = state.read(cx).value().to_string();
                    this.dismiss_preview(cx);
                    this.ime_enter_gate.observe_change();
                    this.query_dispatch_generation =
                        this.query_dispatch_generation.wrapping_add(1).max(1);
                    if let Some(mode) = this.action_mode.as_mut() {
                        mode.set_filter(&query);
                        this.search_pending = false;
                    } else if this.recent_directory_mode {
                        this.selected = 0;
                        this.results = recent_directory_results(&this.recent_directories, &query);
                        this.search_pending = false;
                        this.reset_presented_results();
                        this.result_scroll
                            .0
                            .borrow()
                            .base_handle
                            .set_offset(point(px(0.0), px(0.0)));
                    } else {
                        let _ = this.application.update(cx, |application, _| {
                            application.index_search.invalidate();
                        });
                        if query.trim().is_empty() {
                            this.run_search(query.clone(), cx);
                        } else {
                            this.schedule_search(query.clone(), cx);
                        }
                    }
                    this.sync_window_content_size(&query, window);
                    this.status = None;
                    cx.notify();
                }
                InputEvent::PressEnter { secondary } => {
                    if !this.ime_enter_gate.consume_enter() {
                        return;
                    }
                    if this.action_mode.is_some() {
                        this.activate_selected_action(window, cx);
                    } else if this.recent_directory_mode {
                        this.activate_selected_recent_directory(cx);
                    } else {
                        if let Some(result) = this.results.get(this.selected) {
                            this.activate_selected(
                                quick_launch_primary_action(
                                    result,
                                    *secondary,
                                    window.modifiers().shift,
                                ),
                                QuickLaunchActionTrigger::Enter,
                                cx,
                            );
                        }
                    }
                }
                InputEvent::Focus | InputEvent::Blur => {}
            },
        );
        let composition_subscription =
            cx.observe_in(&search_input, window, |this, state, window, cx| {
                let composing = state.update(cx, |state, cx| {
                    state.marked_text_range(window, cx).is_some()
                });
                this.ime_enter_gate.observe_composition(composing);
            });
        let key_listener = cx.listener(|this, event: &gpui::KeystrokeEvent, window, cx| {
            if !this.is_open || !this.search_input.focus_handle(cx).is_focused(window) {
                return;
            }
            if this.handle_navigation_key(
                event.keystroke.key.as_str(),
                event.keystroke.modifiers,
                window,
                cx,
            ) {
                cx.stop_propagation();
            }
        });
        let key_subscription = cx.intercept_keystrokes(key_listener);
        Self {
            theme: Theme::zed_dark(),
            application,
            target,
            navigation_context,
            results,
            selected: 0,
            action_mode: None,
            recent_directory_mode: false,
            recent_directories,
            search_input,
            status: None,
            icons: HashMap::new(),
            is_open: true,
            results_visible: false,
            search_pending: false,
            query_dispatch_generation: 0,
            results_appearance_generation: 0,
            result_scroll: UniformListScrollHandle::new(),
            action_scroll: ScrollHandle::new(),
            visible_result_range: 0..0,
            last_presented_icon_range: None,
            pointer_selection_gate: PointerSelectionGate::default(),
            ime_enter_gate: ImeEnterGate::default(),
            hide_suppressed_until: Instant::now() + OPEN_GRACE,
            _subscriptions: vec![
                change_subscription,
                composition_subscription,
                key_subscription,
            ],
        }
    }

    pub(crate) fn prepare_for_show(
        &mut self,
        target: nxtools_protocol::ForegroundContext,
        navigation_context: Option<nxtools_protocol::DirectoryNavigationContext>,
        recent_directories: Vec<RecentDirectory>,
        results: Vec<SearchResult>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.dismiss_preview(cx);
        self.target = target;
        self.navigation_context = navigation_context;
        self.results = results;
        self.icons.clear();
        self.selected = 0;
        self.action_mode = None;
        self.recent_directory_mode = false;
        self.recent_directories = recent_directories;
        self.status = None;
        self.is_open = true;
        self.results_visible = false;
        self.search_pending = false;
        self.reset_presented_results();
        self.result_scroll
            .0
            .borrow()
            .base_handle
            .set_offset(point(px(0.0), px(0.0)));
        self.query_dispatch_generation = self.query_dispatch_generation.wrapping_add(1).max(1);
        self.ime_enter_gate.reset();
        self.hide_suppressed_until = Instant::now() + OPEN_GRACE;
        window.resize(size(px(PANEL_WIDTH), px(COLLAPSED_PANEL_HEIGHT)));
        self.search_input.update(cx, |state, cx| {
            state.set_placeholder("搜索应用、文件或文件夹", window, cx);
            state.set_value(String::new(), window, cx);
        });
        cx.notify();
    }

    pub(crate) fn sync_window_content_size(&mut self, query: &str, window: &mut Window) {
        let visible = quick_launch_results_visible(
            query,
            self.action_mode.is_some(),
            self.recent_directory_mode,
        );
        if visible == self.results_visible {
            return;
        }
        self.results_visible = visible;
        if visible {
            self.results_appearance_generation =
                self.results_appearance_generation.wrapping_add(1).max(1);
        }
        window.resize(size(
            px(PANEL_WIDTH),
            px(quick_launch_panel_height(
                query,
                self.action_mode.is_some(),
                self.recent_directory_mode,
            )),
        ));
    }

    pub(crate) fn schedule_search(&mut self, query: String, cx: &mut Context<Self>) {
        let generation = self.query_dispatch_generation;
        self.search_pending = true;
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(SEARCH_INPUT_DEBOUNCE).await;
            let _ = this.update(cx, |this, cx| {
                if !this.is_open
                    || this.action_mode.is_some()
                    || this.query_dispatch_generation != generation
                    || this.search_input.read(cx).value().as_ref() != query
                {
                    return;
                }
                this.run_search(query, cx);
            });
        })
        .detach();
    }

    pub(crate) fn run_search(&mut self, query: String, cx: &mut Context<Self>) {
        let update = self
            .application
            .update(cx, |application, cx| {
                application.search_and_request_icons(&query, cx)
            })
            .unwrap_or(QuickLaunchSearchUpdate {
                results: Vec::new(),
                pending: false,
            });
        self.search_pending = update.pending;
        if apply_search_update(&mut self.results, update.results, update.pending) {
            self.selected = 0;
            self.result_scroll
                .0
                .borrow()
                .base_handle
                .set_offset(point(px(0.0), px(0.0)));
            self.reset_presented_results();
            self.retain_icons_for_results();
            self.pointer_selection_gate.reset();
        }
        cx.notify();
    }

    pub(crate) fn focus_search(&self, window: &mut Window, cx: &mut Context<Self>) {
        self.search_input.focus_handle(cx).focus(window);
    }

    pub(crate) fn enter_recent_directory_mode(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.action_mode.is_some()
            || self.recent_directory_mode
            || self.navigation_context.is_none()
        {
            return;
        }
        let results = recent_directory_results(&self.recent_directories, "");
        if results.is_empty() {
            self.recent_directories.clear();
            cx.notify();
            return;
        }
        self.dismiss_preview(cx);
        self.recent_directory_mode = true;
        self.results = results;
        self.selected = 0;
        self.search_pending = false;
        self.reset_presented_results();
        self.result_scroll
            .0
            .borrow()
            .base_handle
            .set_offset(point(px(0.0), px(0.0)));
        self.search_input.update(cx, |state, cx| {
            state.set_placeholder("筛选最近目录", window, cx);
            state.set_value(String::new(), window, cx);
        });
        self.sync_window_content_size("", window);
        self.focus_search(window, cx);
        cx.notify();
    }

    pub(crate) fn exit_recent_directory_mode(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.recent_directory_mode || self.action_mode.is_some() {
            return;
        }
        self.dismiss_preview(cx);
        self.recent_directory_mode = false;
        self.results.clear();
        self.selected = 0;
        self.reset_presented_results();
        self.search_input.update(cx, |state, cx| {
            state.set_placeholder("搜索应用、文件或文件夹", window, cx);
            state.set_value(String::new(), window, cx);
        });
        self.sync_window_content_size("", window);
        self.focus_search(window, cx);
        cx.notify();
    }

    pub(crate) fn handle_navigation_key(
        &mut self,
        key: &str,
        modifiers: gpui::Modifiers,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.action_mode.is_none()
            && !self.recent_directory_mode
            && let Some(index) = quick_launch_shortcut_target(
                key,
                modifiers,
                self.visible_result_range.clone(),
                self.results.len(),
            )
        {
            self.dismiss_preview(cx);
            self.selected = index;
            let action = quick_launch_primary_action(&self.results[index], false, false);
            cx.notify();
            self.activate_selected(action, QuickLaunchActionTrigger::Shortcut, cx);
            return true;
        }
        if modifiers.modified() {
            return false;
        }
        if key.eq_ignore_ascii_case("escape") {
            if self.recent_directory_mode && self.action_mode.is_none() {
                self.exit_recent_directory_mode(window, cx);
                return true;
            }
            match quick_launch_escape_action(
                self.action_mode.is_some(),
                self.search_input.read(cx).value().is_empty(),
            ) {
                QuickLaunchEscapeAction::ExitActions => self.exit_action_mode(window, cx),
                QuickLaunchEscapeAction::ClearQuery => {
                    self.search_input
                        .update(cx, |state, cx| state.set_value(String::new(), window, cx));
                }
                QuickLaunchEscapeAction::Close => self.close(cx),
            }
            return true;
        }
        if key.eq_ignore_ascii_case("up") {
            self.move_selection(-1, window, cx);
            return true;
        }
        if key.eq_ignore_ascii_case("down") {
            self.move_selection(1, window, cx);
            return true;
        }
        if self.action_mode.is_some()
            && (key.eq_ignore_ascii_case("left") || key.eq_ignore_ascii_case("backspace"))
            && self.search_input.read(cx).value().is_empty()
        {
            self.exit_action_mode(window, cx);
            return true;
        }
        if self.action_mode.is_some() && key.eq_ignore_ascii_case("right") {
            return self.enter_selected_submenu(window, cx);
        }
        if self.action_mode.is_none() && key.eq_ignore_ascii_case("right") {
            let caret_is_at_end = {
                let input = self.search_input.read(cx);
                input.cursor() == input.value().len()
            };
            if caret_is_at_end {
                self.enter_action_mode(self.selected, window, cx);
                return true;
            }
        }
        false
    }

    pub(crate) fn move_selection(
        &mut self,
        delta: isize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.dismiss_preview(cx);
        if let Some(mode) = self.action_mode.as_mut() {
            mode.move_selection(delta);
            self.pointer_selection_gate.reset();
            self.action_scroll
                .scroll_to_item(mode.selected().saturating_add(1));
            cx.notify();
            return;
        }
        if self.results.is_empty() {
            return;
        }
        let count = self.results.len() as isize;
        self.selected = (self.selected as isize + delta).rem_euclid(count) as usize;
        self.pointer_selection_gate.reset();
        self.result_scroll
            .scroll_to_item(self.selected, ScrollStrategy::Center);
        self.schedule_keyboard_selection_preview(window, cx);
        cx.notify();
    }

    pub(crate) fn schedule_keyboard_selection_preview(
        &self,
        window: &Window,
        cx: &mut Context<Self>,
    ) {
        let Some(result) = self.results.get(self.selected).cloned() else {
            return;
        };
        let panel_bounds = window.bounds();
        let panel_scale_factor = window.scale_factor();
        schedule_application_update(
            self.application.clone(),
            move |application, cx| {
                application.schedule_quick_launch_preview(
                    true,
                    result,
                    panel_bounds,
                    panel_scale_factor,
                    cx,
                );
            },
            cx,
        );
    }

    pub(crate) fn pointer_moved_result(
        &mut self,
        index: usize,
        position: (f32, f32),
        cx: &mut Context<Self>,
    ) {
        if self.pointer_selection_gate.observe_move(index, position) {
            self.select_result(index, cx);
            if !self.recent_directory_mode
                && let Some(result) = self.results.get(index).cloned()
            {
                schedule_application_update(
                    self.application.clone(),
                    move |application, cx| {
                        application.request_icons(std::slice::from_ref(&result), cx);
                    },
                    cx,
                );
            }
        }
    }

    pub(crate) fn dismiss_preview(&self, cx: &mut Context<Self>) {
        schedule_application_update(
            self.application.clone(),
            |application, cx| application.dismiss_quick_launch_preview(cx),
            cx,
        );
    }

    pub(crate) fn reset_presented_results(&mut self) {
        self.visible_result_range = 0..0;
        self.last_presented_icon_range = None;
    }

    pub(crate) fn update_presented_results(
        &mut self,
        visible_range: Range<usize>,
        cx: &mut Context<Self>,
    ) {
        self.visible_result_range = visible_range.clone();
        if self.action_mode.is_some()
            || self.recent_directory_mode
            || visible_range.is_empty()
            || self.results.is_empty()
        {
            return;
        }
        let range = visible_icon_prefetch_range(
            visible_range.start,
            visible_range.end.saturating_sub(1),
            self.results.len(),
        );
        if self.last_presented_icon_range.as_ref() == Some(&range) {
            return;
        }
        self.last_presented_icon_range = Some(range.clone());
        let results = self.results[range].to_vec();
        if results.is_empty() {
            return;
        }
        schedule_application_update(
            self.application.clone(),
            move |application, cx| application.request_icons(&results, cx),
            cx,
        );
    }

    pub(crate) fn pointer_moved_action(
        &mut self,
        index: usize,
        position: (f32, f32),
        cx: &mut Context<Self>,
    ) {
        if self.pointer_selection_gate.observe_move(index, position) {
            self.select_action(index, cx);
        }
    }

    pub(crate) fn select_result(&mut self, index: usize, cx: &mut Context<Self>) {
        if index >= self.results.len() || self.selected == index {
            return;
        }
        self.selected = index;
        self.pointer_selection_gate.reset();
        cx.notify();
    }

    pub(crate) fn select_action(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(mode) = self.action_mode.as_mut() else {
            return;
        };
        if index >= mode.visible_actions().len() || mode.selected() == index {
            return;
        }
        mode.set_selected(index);
        cx.notify();
    }

    pub(crate) fn enter_action_mode(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.action_mode.is_some() {
            return;
        }
        self.dismiss_preview(cx);
        let Some(target) = self.results.get(index).cloned() else {
            return;
        };
        self.selected = index;
        let query = self.search_input.read(cx).value().to_string();
        let entry_id = target.entry.id.clone();
        let target_path = target.entry.target.clone();
        let query_shell_actions = quick_launch_result_capabilities(&target).shell_actions;
        let mode = QuickLaunchActionMode::enter_in_context(
            target,
            query,
            index,
            self.navigation_context.as_ref(),
        );
        let mode_id = mode.id();
        let level_id = mode.current_level_id();
        self.action_mode = Some(mode);
        self.status = None;
        self.search_input.update(cx, |state, cx| {
            state.set_placeholder("搜索操作", window, cx);
            state.set_value(String::new(), window, cx);
        });
        self.focus_search(window, cx);
        if query_shell_actions {
            schedule_application_update(
                self.application.clone(),
                move |application, cx| {
                    application.query_shell_actions(mode_id, level_id, entry_id, target_path, cx);
                },
                cx,
            );
        }
        cx.notify();
    }

    pub(crate) fn exit_action_mode(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(mode) = self.action_mode.as_mut()
            && let Some(filter) = mode.pop_level()
        {
            self.action_scroll.set_offset(point(px(0.0), px(0.0)));
            self.pointer_selection_gate.reset();
            self.search_input.update(cx, |state, cx| {
                state.set_placeholder("搜索操作", window, cx);
                state.set_value(filter, window, cx);
            });
            self.focus_search(window, cx);
            cx.notify();
            return;
        }
        let Some(context) = self.action_mode.as_ref().map(|mode| mode.parent.clone()) else {
            return;
        };
        self.selected = selection_for_results_context(&self.results, &context);
        self.pointer_selection_gate.reset();
        self.action_scroll.set_offset(point(px(0.0), px(0.0)));
        let placeholder = if self.recent_directory_mode {
            "筛选最近目录"
        } else {
            "搜索应用、文件或文件夹"
        };
        self.search_input.update(cx, |state, cx| {
            state.set_placeholder(placeholder, window, cx);
            state.set_value(context.query, window, cx);
        });
        let restored = self
            .action_mode
            .take()
            .expect("action mode remains active while restoring its parent query")
            .exit();
        debug_assert_eq!(restored.selected_entry_id, context.selected_entry_id);
        self.focus_search(window, cx);
        cx.notify();
    }

    pub(crate) fn reset_after_history_clear(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> (String, u64) {
        let was_recent_directory_mode = self.recent_directory_mode;
        let query = if was_recent_directory_mode {
            String::new()
        } else {
            self.search_input.read(cx).value().to_string()
        };
        self.query_dispatch_generation = self.query_dispatch_generation.wrapping_add(1).max(1);
        self.recent_directory_mode = false;
        self.recent_directories.clear();
        self.results.clear();
        self.selected = 0;
        self.search_pending = false;
        self.status = None;
        self.reset_presented_results();
        self.result_scroll
            .0
            .borrow()
            .base_handle
            .set_offset(point(px(0.0), px(0.0)));
        if was_recent_directory_mode {
            self.search_input.update(cx, |state, cx| {
                state.set_placeholder("搜索应用、文件或文件夹", window, cx);
                state.set_value(String::new(), window, cx);
            });
        }
        self.sync_window_content_size(&query, window);
        cx.notify();
        (query, self.query_dispatch_generation)
    }

    pub(crate) fn replace_results(&mut self, results: Vec<SearchResult>, cx: &mut Context<Self>) {
        self.dismiss_preview(cx);
        self.selected = selection_after_results_refresh(&self.results, self.selected, &results);
        self.results = results;
        self.retain_icons_for_results();
        self.reset_presented_results();
        self.result_scroll
            .scroll_to_item(self.selected, ScrollStrategy::Center);
        self.search_pending = false;
        self.status = None;
        cx.notify();
    }
    pub(crate) fn activate_selected(
        &mut self,
        action: QuickLaunchAction,
        trigger: QuickLaunchActionTrigger,
        cx: &mut Context<Self>,
    ) {
        let Some(result) = self.results.get(self.selected).cloned() else {
            return;
        };
        let query = self.search_input.read(cx).value().to_string();
        schedule_application_update(
            self.application.clone(),
            move |application, cx| application.execute(result, action, query, trigger, cx),
            cx,
        );
    }

    pub(crate) fn activate_selected_recent_directory(&mut self, cx: &mut Context<Self>) {
        let Some(result) = self.results.get(self.selected).cloned() else {
            return;
        };
        let Some(action) =
            recent_directory_navigation_action(&result, self.navigation_context.as_ref())
        else {
            return;
        };
        schedule_application_update(
            self.application.clone(),
            move |application, cx| {
                application.execute(
                    result,
                    action,
                    String::new(),
                    QuickLaunchActionTrigger::RecentDirectory,
                    cx,
                )
            },
            cx,
        );
    }

    pub(crate) fn enter_selected_submenu(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some((mode_id, entry_id, submenu)) = self.action_mode.as_mut().and_then(|mode| {
            let mode_id = mode.id();
            let entry_id = mode.target().entry.id.clone();
            mode.push_selected_submenu()
                .map(|submenu| (mode_id, entry_id, submenu))
        }) else {
            return false;
        };
        self.action_scroll.set_offset(point(px(0.0), px(0.0)));
        self.pointer_selection_gate.reset();
        self.search_input.update(cx, |state, cx| {
            state.set_placeholder("搜索子操作", window, cx);
            state.set_value(String::new(), window, cx);
        });
        self.focus_search(window, cx);
        schedule_application_update(
            self.application.clone(),
            move |application, cx| {
                application.query_shell_submenu(mode_id, submenu, entry_id, cx);
            },
            cx,
        );
        cx.notify();
        true
    }

    pub(crate) fn activate_selected_action(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.enter_selected_submenu(window, cx) {
            return;
        }
        let Some(mode) = self.action_mode.as_ref() else {
            return;
        };
        let Some(action) = mode.selected_action() else {
            return;
        };
        let result = mode.target().clone();
        let query = mode.parent.query.clone();
        schedule_application_update(
            self.application.clone(),
            move |application, cx| {
                application.execute(
                    result,
                    action,
                    query,
                    QuickLaunchActionTrigger::ActionMode,
                    cx,
                )
            },
            cx,
        );
    }

    pub(crate) fn apply_shell_actions(
        &mut self,
        mode_id: uuid::Uuid,
        level_id: uuid::Uuid,
        entry_id: &str,
        menu: Option<ShellActionMenu>,
        cx: &mut Context<Self>,
    ) {
        let Some(mode) = self.action_mode.as_mut() else {
            return;
        };
        if !mode.matches_shell_query(mode_id, level_id, entry_id) {
            return;
        }
        mode.apply_shell_menu(level_id, menu);
        self.action_scroll.set_offset(point(px(0.0), px(0.0)));
        self.pointer_selection_gate.reset();
        cx.notify();
    }

    pub(crate) fn close(&mut self, cx: &mut Context<Self>) {
        if !self.is_open {
            return;
        }
        let captured_window_handle = self.target.window_handle;
        self.is_open = false;
        self.query_dispatch_generation = self.query_dispatch_generation.wrapping_add(1).max(1);
        cx.notify();
        schedule_application_update(
            self.application.clone(),
            move |application, cx| {
                application.hide_popup_and_restore(captured_window_handle, cx);
            },
            cx,
        );
    }

    pub(crate) fn should_dismiss_for_foreground(
        &self,
        popup_window_handle: i64,
        preview_window_handle: i64,
        foreground_window_handle: i64,
    ) -> bool {
        should_hide_for_foreground(
            self.is_open,
            Instant::now() >= self.hide_suppressed_until,
            popup_window_handle,
            preview_window_handle,
            foreground_window_handle,
        )
    }

    pub(crate) fn should_dismiss_for_pointer(
        &self,
        popup_window_handle: i64,
        pointer_root_window_handle: i64,
        pointer_process_id: u32,
    ) -> bool {
        should_hide_for_pointer(
            self.is_open,
            popup_window_handle,
            pointer_root_window_handle,
            pointer_process_id,
            std::process::id(),
        )
    }

    pub(crate) fn set_error(&mut self, message: String, cx: &mut Context<Self>) {
        self.set_status(message, cx);
    }

    pub(crate) fn set_status(&mut self, message: String, cx: &mut Context<Self>) {
        self.search_pending = false;
        self.status = Some(message);
        cx.notify();
    }

    pub(crate) fn retain_icons_for_results(&mut self) {
        let visible_ids = self
            .results
            .iter()
            .map(|result| result.entry.id.as_str())
            .collect::<HashSet<_>>();
        self.icons
            .retain(|entry_id, _| visible_ids.contains(entry_id.as_str()));
    }

    pub(crate) fn set_icons(
        &mut self,
        icons: impl IntoIterator<Item = (String, PathBuf)>,
        cx: &mut Context<Self>,
    ) {
        let mut changed = false;
        for (entry_id, path) in icons {
            if self
                .icons
                .get(&entry_id)
                .is_none_or(|existing| existing != &path)
            {
                self.icons.insert(entry_id, path);
                changed = true;
            }
        }
        if changed {
            cx.notify();
        }
    }

    pub(crate) fn render_result(
        &self,
        index: usize,
        visible_position: usize,
        result: &SearchResult,
        query: String,
        panel: WeakEntity<QuickLaunchPanel>,
    ) -> AnyElement {
        let theme = self.theme;
        let selected = index == self.selected;
        let source = result.entry.source;
        let result_for_click = result.clone();
        let application = self.application.clone();
        let click_action = if self.recent_directory_mode {
            recent_directory_navigation_action(result, self.navigation_context.as_ref())
        } else {
            Some(quick_launch_primary_action(result, false, false))
        };
        let preview_application = self.application.clone();
        let preview_result = result.clone();
        let pointer_panel = panel.clone();
        let press_panel = panel.clone();
        let right_click_panel = panel;
        let icon_path = self.icons.get(&result.entry.id).cloned();
        let has_icon_path = icon_path.is_some();
        let subtitle = if let Some(subtitle) = &result.subtitle {
            subtitle.clone()
        } else if result.copy_text().is_some() {
            "按 Enter 复制结果".to_owned()
        } else {
            result.entry.target.clone()
        };
        let shortcut_hint = quick_launch_shortcut_hint(visible_position);
        let name_highlights = quick_launch_match_ranges(&result.entry.name, &query)
            .into_iter()
            .map(|range| {
                (
                    range,
                    HighlightStyle {
                        color: Some(theme.accent.into()),
                        font_weight: Some(gpui::FontWeight::SEMIBOLD),
                        ..Default::default()
                    },
                )
            })
            .collect::<Vec<_>>();
        quick_launch_row_shell(theme, selected)
            .id(("quick-launch-result", index))
            .debug_selector(|| "quick-launch-result-row".into())
            .cursor_pointer()
            .on_hover(move |hovered, window, cx| {
                let entered = *hovered;
                let panel_bounds = window.bounds();
                let panel_scale_factor = window.scale_factor();
                let result = preview_result.clone();
                schedule_application_update(
                    preview_application.clone(),
                    move |application, cx| {
                        application.schedule_quick_launch_preview(
                            entered,
                            result,
                            panel_bounds,
                            panel_scale_factor,
                            cx,
                        )
                    },
                    cx,
                );
            })
            .on_mouse_move(move |event, _window, cx| {
                let position = (event.position.x.as_f32(), event.position.y.as_f32());
                let _ = pointer_panel.update(cx, |panel, cx| {
                    panel.pointer_moved_result(index, position, cx);
                });
            })
            .on_mouse_down(gpui::MouseButton::Left, move |_event, _window, cx| {
                let _ = press_panel.update(cx, |panel, cx| {
                    panel.select_result(index, cx);
                });
            })
            .on_mouse_down(gpui::MouseButton::Right, move |_event, window, cx| {
                let _ = right_click_panel.update(cx, |panel, cx| {
                    panel.enter_action_mode(index, window, cx);
                });
                cx.stop_propagation();
            })
            .on_click(move |_event, _window, cx| {
                let Some(action) = click_action.clone() else {
                    return;
                };
                let result = result_for_click.clone();
                let query = query.clone();
                schedule_application_update(
                    application.clone(),
                    move |application, cx| {
                        application.execute(
                            result,
                            action,
                            query,
                            QuickLaunchActionTrigger::Pointer,
                            cx,
                        )
                    },
                    cx,
                );
            })
            .child(
                div()
                    .debug_selector(move || format!("quick-launch-result-icon-{index}"))
                    .size(px(COMPACT_RESULT_ICON_RAIL_WIDTH))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(if selected {
                        theme.icon
                    } else {
                        theme.icon_muted
                    })
                    .when_some(icon_path, |container, path| {
                        container.child(
                            img(path)
                                .size(px(21.0))
                                .object_fit(ObjectFit::Contain)
                                .with_loading(move || {
                                    quick_launch_result_fallback_icon(index, source)
                                })
                                .with_fallback(move || {
                                    quick_launch_result_fallback_icon(index, source)
                                }),
                        )
                    })
                    .when(!has_icon_path, |container| {
                        container.child(quick_launch_result_fallback_icon(index, source))
                    }),
            )
            .child(
                div()
                    .debug_selector(move || format!("quick-launch-result-copy-{index}"))
                    .min_w_0()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .gap(px(SPACING_2))
                    .child(
                        div()
                            .debug_selector(move || format!("quick-launch-result-title-{index}"))
                            .h(px(LINE_HEIGHT_BODY))
                            .text_size(px(TEXT_BODY))
                            .line_height(px(LINE_HEIGHT_BODY))
                            .text_color(theme.text)
                            .truncate()
                            .child(
                                StyledText::new(result.entry.name.clone())
                                    .with_highlights(name_highlights),
                            ),
                    )
                    .child(
                        div()
                            .debug_selector(move || format!("quick-launch-result-subtitle-{index}"))
                            .h(px(LINE_HEIGHT_LABEL))
                            .text_size(px(TEXT_LABEL))
                            .line_height(px(LINE_HEIGHT_LABEL))
                            .text_color(if selected {
                                theme.text_secondary
                            } else {
                                theme.text_muted
                            })
                            .truncate()
                            .child(subtitle),
                    ),
            )
            .child(
                div()
                    .debug_selector(move || format!("quick-launch-result-shortcut-{index}"))
                    .w(px(COMPACT_RESULT_TRAILING_RAIL_WIDTH))
                    .h(px(LINE_HEIGHT_LABEL))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_end()
                    .text_size(px(TEXT_LABEL))
                    .line_height(px(LINE_HEIGHT_LABEL))
                    .text_color(theme.text_muted)
                    .when_some(shortcut_hint, |rail, hint| rail.child(hint)),
            )
            .into_any_element()
    }

    pub(crate) fn render_action(
        &self,
        index: usize,
        item: QuickLaunchActionItem,
        panel: WeakEntity<QuickLaunchPanel>,
    ) -> AnyElement {
        let theme = self.theme;
        let selected = self
            .action_mode
            .as_ref()
            .is_some_and(|mode| mode.selected() == index);
        let enabled = item.is_activatable();
        let has_submenu = item.submenu.is_some();
        let pointer_panel = panel.clone();
        let click_panel = panel;
        let section_label = item.section_label.clone();
        let divider_before = item.divider_before;
        let row = quick_launch_row_shell(theme, selected)
            .id(("quick-launch-action", index))
            .debug_selector(move || format!("quick-launch-action-row-{index}"))
            .when(enabled, |row| row.cursor_pointer())
            .when(enabled, |row| {
                row.on_mouse_move(move |event, _window, cx| {
                    let position = (event.position.x.as_f32(), event.position.y.as_f32());
                    let _ = pointer_panel.update(cx, |panel, cx| {
                        panel.pointer_moved_action(index, position, cx);
                    });
                })
                .on_click(move |_event, window, cx| {
                    let _ = click_panel.update(cx, |panel, cx| {
                        panel.select_action(index, cx);
                        panel.activate_selected_action(window, cx);
                    });
                })
            })
            .child(
                div()
                    .debug_selector(move || format!("quick-launch-action-icon-{index}"))
                    .size(px(COMPACT_RESULT_ICON_RAIL_WIDTH))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(if selected {
                        theme.icon
                    } else {
                        theme.icon_muted
                    })
                    .when_some(item.icon_path.clone(), |container, icon_path| {
                        container
                            .child(img(icon_path).size(px(18.0)).object_fit(ObjectFit::Contain))
                    })
                    .when_some(item.icon, |container, icon| {
                        container.child(gpui_component::Icon::new(icon).size(px(18.0)))
                    }),
            )
            .child(
                div()
                    .debug_selector(move || format!("quick-launch-action-copy-{index}"))
                    .min_w_0()
                    .flex_1()
                    .flex()
                    .items_center()
                    .child(
                        div()
                            .debug_selector(move || format!("quick-launch-action-title-{index}"))
                            .h(px(LINE_HEIGHT_BODY))
                            .text_size(px(TEXT_BODY))
                            .line_height(px(LINE_HEIGHT_BODY))
                            .text_color(if item.enabled {
                                theme.text
                            } else {
                                theme.text_muted
                            })
                            .truncate()
                            .child(item.label.clone()),
                    ),
            )
            .child(
                div()
                    .debug_selector(move || format!("quick-launch-action-trailing-{index}"))
                    .w(px(COMPACT_RESULT_TRAILING_RAIL_WIDTH))
                    .h(px(LINE_HEIGHT_LABEL))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_end()
                    .when(has_submenu, |rail| {
                        rail.child(
                            gpui_component::Icon::new(MaterialIcon::ChevronRight)
                                .size(px(16.0))
                                .text_color(theme.icon_muted),
                        )
                    }),
            );
        div()
            .when(divider_before, |container| {
                container
                    .mt(px(4.0))
                    .pt(px(4.0))
                    .border_t_1()
                    .border_color(theme.border_subtle)
            })
            .when_some(section_label, |container, label| {
                container.child(
                    div()
                        .h(px(22.0))
                        .px(px(8.0))
                        .flex()
                        .items_center()
                        .text_size(px(TEXT_META))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(theme.text_muted)
                        .child(label),
                )
            })
            .child(row)
            .into_any_element()
    }
}

impl Render for QuickLaunchPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let panel = cx.entity().downgrade();
        let recent_button_panel = panel.clone();
        let preview_scroll_application = self.application.clone();
        let query = self.search_input.read(cx).value().to_string();
        let recent_button_visible = !self.recent_directories.is_empty()
            && self.navigation_context.is_some()
            && self.action_mode.is_none()
            && (self.recent_directory_mode || query.trim().is_empty());
        let recent_directory_mode = self.recent_directory_mode;
        let action_target = self.action_mode.as_ref().map(|mode| {
            (
                mode.title().to_string(),
                mode.target().entry.name.clone(),
                mode.shell_pending(),
            )
        });
        let action_entries = if let Some(mode) = self.action_mode.as_ref() {
            mode.visible_actions()
                .into_iter()
                .enumerate()
                .map(|(index, item)| self.render_action(index, item, panel.clone()))
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        let result_surface = if let Some((level_title, target, pending)) = action_target {
            div()
                .id("quick-launch-actions-scroll")
                .flex_1()
                .min_h_0()
                .track_scroll(&self.action_scroll)
                .overflow_y_scroll()
                .p(px(SPACING_6))
                .flex()
                .flex_col()
                .child(
                    div()
                        .debug_selector(|| "quick-launch-action-context".into())
                        .h(px(QUICK_LAUNCH_ACTION_CONTEXT_HEIGHT))
                        .mb(px(SPACING_2))
                        .px(px(SPACING_8))
                        .flex()
                        .items_center()
                        .gap(px(SPACING_8))
                        .text_size(px(TEXT_LABEL))
                        .line_height(px(LINE_HEIGHT_LABEL))
                        .text_color(theme.text_muted)
                        .child(
                            div()
                                .flex_none()
                                .text_color(theme.text_secondary)
                                .child(level_title),
                        )
                        .child(div().min_w_0().flex_1().truncate().child(target)),
                )
                .when(pending, |container| {
                    container.child(progress_state(
                        theme,
                        "quick-launch-actions-progress",
                        "正在加载 Windows 操作",
                        "正在读取所选目标的系统操作。",
                    ))
                })
                .when(action_entries.is_empty() && !pending, |container| {
                    container.child(empty_state(
                        theme,
                        "quick-launch-actions-empty",
                        MaterialIcon::Search,
                        "没有匹配的操作",
                        "尝试缩短关键词或返回上一级操作。",
                    ))
                })
                .children(action_entries)
                .vertical_scrollbar(&self.action_scroll)
                .into_any_element()
        } else if self.results.is_empty() {
            div()
                .flex_1()
                .min_h_0()
                .when(self.search_pending, |container| {
                    container.child(progress_state(
                        theme,
                        "quick-launch-results-progress",
                        "正在搜索",
                        "正在合并应用、本地文件与系统工具结果。",
                    ))
                })
                .when(!self.search_pending, |container| {
                    container.child(empty_state(
                        theme,
                        "quick-launch-results-empty",
                        MaterialIcon::Search,
                        QUICK_LAUNCH_EMPTY_RESULTS_LABEL,
                        "尝试缩短关键词，或输入应用、文件和命令名称。",
                    ))
                })
                .into_any_element()
        } else {
            let result_count = self.results.len();
            let result_query = query.clone();
            let result_panel = panel.clone();
            let list = uniform_list(
                "quick-launch-results",
                result_count,
                cx.processor(move |panel, visible_range: Range<usize>, _window, cx| {
                    let visible_start = visible_range.start;
                    panel.update_presented_results(visible_range.clone(), cx);
                    visible_range
                        .filter_map(|index| {
                            panel.results.get(index).map(|result| {
                                panel.render_result(
                                    index,
                                    index.saturating_sub(visible_start),
                                    result,
                                    result_query.clone(),
                                    result_panel.clone(),
                                )
                            })
                        })
                        .collect::<Vec<_>>()
                }),
            )
            .size_full()
            .p(px(SPACING_6))
            .track_scroll(self.result_scroll.clone());
            div()
                .id("quick-launch-results-scroll")
                .relative()
                .flex_1()
                .min_h_0()
                .on_scroll_wheel(move |_event, _window, cx| {
                    schedule_application_update(
                        preview_scroll_application.clone(),
                        |application, cx| {
                            application.suspend_quick_launch_preview_for_scroll(cx);
                        },
                        cx,
                    );
                })
                .child(list)
                .vertical_scrollbar(&self.result_scroll)
                .into_any_element()
        };
        let result_surface = div()
            .debug_selector(|| "quick-launch-results-surface".into())
            .w_full()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .child(result_surface)
            .with_animation(
                ("quick-launch-results", self.results_appearance_generation),
                Animation::new(RESULTS_APPEAR_DURATION).with_easing(ease_out_quint()),
                |surface, delta| surface.relative().top(px(1.0 * (1.0 - delta))),
            );
        div()
            .id("quick-launch-window")
            .debug_selector(|| "quick-launch-window".into())
            .relative()
            .size_full()
            .overflow_hidden()
            .font_family(UI_FONT_FAMILY)
            .bg(theme.canvas)
            .text_color(theme.text)
            .border_1()
            .border_color(theme.window_border)
            .flex()
            .flex_col()
            .child(
                div()
                    .debug_selector(|| "quick-launch-search-header".into())
                    .h(px(SEARCH_HEADER_HEIGHT))
                    .px(px(SPACING_8))
                    .flex_none()
                    .border_b_1()
                    .border_color(theme.border_subtle)
                    .bg(theme.canvas)
                    .flex()
                    .items_center()
                    .gap(px(SPACING_8))
                    .text_size(px(TEXT_BODY))
                    .line_height(px(LINE_HEIGHT_BODY))
                    .child(
                        gpui_component::Icon::new(MaterialIcon::Search)
                            .size(px(16.0))
                            .text_color(theme.icon_muted),
                    )
                    .child(
                        div().min_w_0().flex_1().child(
                            Input::new(&self.search_input)
                                .appearance(false)
                                .bordered(false)
                                .focus_bordered(false)
                                .text_size(px(TEXT_BODY))
                                .line_height(px(LINE_HEIGHT_BODY)),
                        ),
                    )
                    .when(recent_button_visible, |header| {
                        header.child(
                            div()
                                .id("quick-launch-recent-directories")
                                .size(px(27.0))
                                .rounded(px(RADIUS_CONTROL))
                                .flex()
                                .items_center()
                                .justify_center()
                                .cursor_pointer()
                                .text_color(if recent_directory_mode {
                                    theme.icon
                                } else {
                                    theme.icon_muted
                                })
                                .bg(if recent_directory_mode {
                                    theme.element_selected
                                } else {
                                    theme.canvas
                                })
                                .hover(|button| button.bg(theme.surface_hover))
                                .on_click(move |_event, window, cx| {
                                    let _ = recent_button_panel.update(cx, |panel, cx| {
                                        if panel.recent_directory_mode {
                                            panel.exit_recent_directory_mode(window, cx);
                                        } else {
                                            panel.enter_recent_directory_mode(window, cx);
                                        }
                                    });
                                })
                                .child(
                                    gpui_component::Icon::new(MaterialIcon::History).size(px(15.0)),
                                ),
                        )
                    }),
            )
            .when(self.results_visible, |panel| panel.child(result_surface))
            .when_some(self.status.clone(), |panel, status| {
                panel.child(
                    div()
                        .absolute()
                        .left(px(12.0))
                        .right(px(12.0))
                        .bottom(px(8.0))
                        .h(px(28.0))
                        .px(px(12.0))
                        .rounded(px(RADIUS_CONTROL))
                        .border_1()
                        .border_color(theme.border)
                        .bg(theme.surface_raised)
                        .shadow_sm()
                        .flex()
                        .items_center()
                        .text_size(px(TEXT_META))
                        .text_color(theme.text_secondary)
                        .child(status),
                )
            })
    }
}

pub(crate) fn quick_launch_result_fallback_icon(
    index: usize,
    source: ApplicationSource,
) -> AnyElement {
    div()
        .debug_selector(move || format!("quick-launch-result-icon-fallback-{index}"))
        .size(px(18.0))
        .flex_none()
        .child(gpui_component::Icon::new(source_icon(source)).size(px(18.0)))
        .into_any_element()
}

pub(crate) fn source_icon(source: ApplicationSource) -> MaterialIcon {
    match source {
        ApplicationSource::Calculator => MaterialIcon::Calculate,
        ApplicationSource::EnvironmentVariable => MaterialIcon::DataObject,
        ApplicationSource::CustomCommand => MaterialIcon::Terminal,
        ApplicationSource::StartMenu => MaterialIcon::Dashboard,
        ApplicationSource::Desktop => MaterialIcon::OpenInNew,
        ApplicationSource::System => MaterialIcon::ProgressActivity,
        ApplicationSource::IndexedFile => MaterialIcon::TextFields,
        ApplicationSource::IndexedDirectory => MaterialIcon::Folder,
    }
}

pub(crate) fn quick_launch_bounds(
    request: &QuickLaunchPanelRequest,
    expanded: bool,
    cx: &App,
) -> (Bounds<gpui::Pixels>, Option<gpui::DisplayId>) {
    let desired = size(
        px(PANEL_WIDTH),
        px(if expanded {
            PANEL_HEIGHT
        } else {
            COLLAPSED_PANEL_HEIGHT
        }),
    );
    let display = cx
        .displays()
        .into_iter()
        .find(|display| u32::from(display.id()) == request.cursor_display_index)
        .or_else(|| cx.displays().into_iter().next());
    let Some(display) = display else {
        return (Bounds::new(point(px(100.0), px(100.0)), desired), None);
    };
    let screen = display.bounds();
    let panel_size = size(
        desired.width.min(screen.size.width),
        desired.height.min(screen.size.height),
    );
    let x = screen.left() + (screen.size.width - panel_size.width) / 2.0;
    let y = screen.top() + (screen.size.height - panel_size.height) / 4.0;
    (Bounds::new(point(x, y), panel_size), Some(display.id()))
}
