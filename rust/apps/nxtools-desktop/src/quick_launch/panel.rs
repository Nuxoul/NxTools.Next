use super::*;
use crate::ui::metrics::SPACING_4;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::input::Input;
use gpui_kit::component::label::Label;
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::component::{Sizable as _, Size as ControlSize};

/// Keep keyboard focus/ring outside the normal-result and compact-action rails.
pub(super) fn quick_launch_focusable_row(
    id: impl Into<gpui_kit::ElementId>,
    theme: Theme,
    disabled: bool,
    content: AnyElement,
    window: &mut Window,
    cx: &mut App,
    on_click: impl Fn(&gpui_kit::ClickEvent, &mut Window, &mut App) + 'static,
) -> gpui_kit::base::Button {
    let id = id.into();
    let focus = window
        .use_keyed_state(id.clone(), cx, |_, cx| cx.focus_handle().tab_stop(true))
        .read(cx)
        .clone();
    crate::ui::components::action::focused_action(id, &focus, theme, disabled, window, cx, on_click)
        .border_0()
        .w_full()
        .flex_none()
        .child(content)
        .when(
            focus.is_focused(window) && !disabled && window.last_input_was_keyboard(),
            |row| {
                row.child(
                    div()
                        .absolute()
                        .inset_0()
                        .border_2()
                        .rounded(px(RADIUS_CONTROL))
                        .border_color(theme.focus_ring),
                )
            },
        )
}

pub(crate) struct QuickLaunchPanel {
    pub(crate) theme: Theme,
    pub(crate) row_style: nxtools_core::QuickLaunchRowStyle,
    pub(crate) result_entrance: nxtools_core::QuickLaunchResultEntrance,
    pub(crate) result_entrances: super::result_motion::ResultEntrances,
    pub(crate) result_motion_available: bool,
    pub(crate) result_admission_pending: bool,
    pub(crate) application: WeakEntity<QuickLaunchApplication>,
    pub(crate) target: nxtools_protocol::ForegroundContext,
    pub(crate) navigation_context: Option<nxtools_protocol::DirectoryNavigationContext>,
    pub(crate) results: QuickLaunchResults,
    pub(crate) selected: usize,
    pub(crate) selection_explicit: bool,
    pub(crate) committed_query: gpui_kit::SharedString,
    pub(crate) action_mode: Option<QuickLaunchActionMode>,
    pub(crate) recent_directory_mode: bool,
    pub(crate) recent_directories: Vec<RecentDirectory>,
    pub(crate) search_input: Entity<InputState>,
    pub(crate) status: Option<gpui_kit::SharedString>,
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
                    let application = this.application.clone();
                    cx.defer(move |cx| {
                        let _ = application.update(cx, |application, _| {
                            application.cancel_extension_providers()
                        });
                    });
                    this.ime_enter_gate.observe_change();
                    this.query_dispatch_generation =
                        this.query_dispatch_generation.wrapping_add(1).max(1);
                    if let Some(mode) = this.action_mode.as_mut() {
                        mode.set_filter(&query);
                        this.search_pending = false;
                    } else if this.recent_directory_mode {
                        this.selected = 0;
                        this.results =
                            recent_directory_results(&this.recent_directories, &query).into();
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
                    this.sync_window_content_size(&query, window, cx);
                    this.status = None;
                    cx.notify();
                }
                InputEvent::PressEnter { secondary, shift } => {
                    // The native IME may consume its confirming key without
                    // emitting PressEnter. A live preedit must never execute.
                    let composing = state.update(cx, |state, cx| {
                        state.marked_text_range(window, cx).is_some()
                    });
                    if composing {
                        return;
                    }
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
                                quick_launch_primary_action(result, *secondary, *shift),
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
        let key_listener = cx.listener(|this, event: &gpui_kit::KeystrokeEvent, window, cx| {
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
        // Notifications run after the publishing owner releases its lease. Read
        // live panel state here so a queued update cannot resize a retired session.
        let content_size_subscription =
            cx.observe_in(&cx.entity(), window, |this, _, window, cx| {
                if this.is_open {
                    this.resize_to_content(window, cx);
                }
            });
        Self {
            theme: Theme::zed_dark(),
            row_style: nxtools_core::QuickLaunchRowStyle::default(),
            result_entrance: nxtools_core::QuickLaunchResultEntrance::default(),
            result_entrances: super::result_motion::ResultEntrances::default(),
            result_admission_pending: false,
            result_motion_available: true,
            application,
            target,
            navigation_context,
            results: results.into(),
            selected: 0,
            selection_explicit: false,
            committed_query: gpui_kit::SharedString::default(),
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
                content_size_subscription,
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
        self.finish_result_entrances("show", cx);
        self.target = target;
        self.navigation_context = navigation_context;
        self.results = results.into();
        self.icons.clear();
        self.selected = 0;
        self.selection_explicit = false;
        self.committed_query = gpui_kit::SharedString::default();
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

    pub(crate) fn prepare_for_hide(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.is_open = false;
        self.finish_result_entrances("hide", cx);
        self.results_visible = false;
        self.results_appearance_generation =
            self.results_appearance_generation.wrapping_add(1).max(1);
        self.query_dispatch_generation = self.query_dispatch_generation.wrapping_add(1).max(1);
        self.search_pending = false;
        self.action_mode = None;
        let application = self.application.clone();
        cx.defer(move |cx| {
            let _ = application.update(cx, |application, _| {
                application.cancel_extension_providers()
            });
        });
        self.dismiss_preview(cx);
        window.resize(size(px(PANEL_WIDTH), px(COLLAPSED_PANEL_HEIGHT)));
        cx.notify();
    }

    pub(crate) fn sync_window_content_size(
        &mut self,
        query: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let visible = quick_launch_results_visible(
            query,
            self.action_mode.is_some(),
            self.recent_directory_mode,
        );
        let was_visible = self.results_visible;
        self.results_visible = visible;
        if !visible || self.action_mode.is_some() || self.recent_directory_mode {
            self.finish_result_entrances("mode", cx);
        }
        if visible && !was_visible {
            self.results_appearance_generation =
                self.results_appearance_generation.wrapping_add(1).max(1);
        }
        self.resize_to_content(window, cx);
    }

    pub(crate) fn content_height(&self) -> f32 {
        if !self.results_visible {
            return quick_launch_panel_height(0.0, self.status.is_some());
        }
        let list_height = if let Some(mode) = &self.action_mode {
            let actions = mode.visible_actions();
            let rows = actions
                .iter()
                .map(|item| {
                    COMPACT_RESULT_ROW_HEIGHT
                        + self.row_style.gap()
                        + if item.divider_before {
                            2.0 * SPACING_4 + 1.0
                        } else {
                            0.0
                        }
                        + if item.section_label.is_some() {
                            24.0
                        } else {
                            0.0
                        }
                })
                .sum::<f32>();
            (2.0 * SPACING_8
                + QUICK_LAUNCH_ACTION_CONTEXT_HEIGHT
                + SPACING_4
                + rows
                + if mode.shell_pending() || actions.is_empty() {
                    LAUNCHER_STATE_MIN_HEIGHT
                } else {
                    0.0
                })
            .min(LAUNCHER_LIST_MAX_HEIGHT)
        } else if self.recent_directory_mode {
            if self.results.is_empty() {
                LAUNCHER_STATE_MIN_HEIGHT
            } else {
                (2.0 * SPACING_8
                    + self.results.len() as f32
                        * (LAUNCHER_RESULT_ROW_HEIGHT + self.row_style.gap()))
                .min(LAUNCHER_LIST_MAX_HEIGHT)
            }
        } else {
            return PANEL_HEIGHT;
        };
        quick_launch_panel_height(list_height, self.status.is_some())
    }

    pub(crate) fn desired_content_size(&self, window: &Window, cx: &App) -> Size<Pixels> {
        let mut desired = size(px(PANEL_WIDTH), px(self.content_height()));
        if let Some(display) = window.display(cx) {
            let screen = display.bounds();
            desired.width = desired.width.min(screen.size.width);
            desired.height = desired
                .height
                .min((screen.bottom() - window.bounds().top()).max(px(COLLAPSED_PANEL_HEIGHT)));
        }
        desired
    }

    fn resize_to_content(&self, window: &mut Window, cx: &mut Context<Self>) {
        let desired = self.desired_content_size(window, cx);
        if window.bounds().size == desired {
            return;
        }
        if native_window_handle(window).is_none() {
            window.resize(desired);
            return;
        }
        let panel = cx.entity().downgrade();
        let generation = self.query_dispatch_generation;
        schedule_application_update(
            self.application.clone(),
            move |application, cx| {
                let current = panel.upgrade().is_some_and(|panel| {
                    let panel = panel.read(cx);
                    panel.is_open && panel.query_dispatch_generation == generation
                });
                if current {
                    application.resize_popup_content(cx);
                }
            },
            cx,
        );
    }

    pub(crate) fn schedule_search(&mut self, query: String, cx: &mut Context<Self>) {
        self.finish_result_entrances("query", cx);
        self.result_entrances.begin(
            self.query_dispatch_generation,
            cx.background_executor().now(),
        );
        let generation = self.query_dispatch_generation;
        self.search_pending = true;
        self.selected = 0;
        self.selection_explicit = false;
        self.pointer_selection_gate.reset();
        self.result_scroll
            .0
            .borrow()
            .base_handle
            .set_offset(point(px(0.0), px(0.0)));
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
        self.result_entrances.begin(
            self.query_dispatch_generation,
            cx.background_executor().now(),
        );
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
        let mut committed = Vec::new();
        if apply_search_update(&mut committed, update.results, update.pending) {
            self.selected = 0;
            self.selection_explicit = false;
            self.committed_query = query.into();
            self.results = committed.into();
            self.queue_result_entrances(cx);
            self.retain_icons_for_results();
            self.reset_presented_results();
            self.pointer_selection_gate.reset();
            self.result_scroll
                .0
                .borrow()
                .base_handle
                .set_offset(point(px(0.0), px(0.0)));
        }
        cx.notify();
    }

    pub(crate) fn focus_search(&self, window: &mut Window, cx: &mut Context<Self>) {
        self.search_input.focus_handle(cx).focus(window, cx);
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
        self.results = results.into();
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
        self.sync_window_content_size("", window, cx);
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
        self.sync_window_content_size("", window, cx);
        self.focus_search(window, cx);
        cx.notify();
    }

    pub(crate) fn handle_navigation_key(
        &mut self,
        key: &str,
        modifiers: gpui_kit::Modifiers,
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
            if self.search_pending {
                return true;
            }
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
                    self.query_dispatch_generation =
                        self.query_dispatch_generation.wrapping_add(1).max(1);
                    self.ime_enter_gate.reset();
                    self.dismiss_preview(cx);
                    self.run_search(String::new(), cx);
                    self.sync_window_content_size("", window, cx);
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
        if self.results.is_empty() || self.search_pending {
            return;
        }
        let count = self.results.len() as isize;
        self.selected = (self.selected as isize + delta).rem_euclid(count) as usize;
        self.selection_explicit = true;
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
        // Icon prefetch presentation is independent of query-owned motion identity.
        self.visible_result_range = 0..0;
        self.last_presented_icon_range = None;
    }

    pub(crate) fn finish_result_entrances(&mut self, category: &'static str, cx: &App) {
        let now = cx.background_executor().now();
        if self.result_admission_pending {
            self.result_entrances.admit(
                self.query_dispatch_generation,
                self.results
                    .iter()
                    .map(|result| (result.entry.id.clone(), false)),
                self.result_entrance,
                false,
                now,
            );
        }
        self.result_entrances.finish(category, now);
        self.result_admission_pending = false;
    }

    fn queue_result_entrances(&mut self, cx: &App) {
        self.result_entrances.begin(
            self.query_dispatch_generation,
            cx.background_executor().now(),
        );
        self.result_admission_pending = true;
    }

    pub(crate) fn present_result_entrances(&mut self, window: &Window, cx: &App) {
        if !self.result_admission_pending {
            return;
        }
        let now = cx.background_executor().now();
        let allowed = self.is_open
            && self.results_visible
            && self.result_motion_available
            && !self.search_pending
            && self.action_mode.is_none()
            && !self.recent_directory_mode
            && self.result_entrance != nxtools_core::QuickLaunchResultEntrance::None
            && crate::ui::motion::motion_allowed(cx);
        let height = (window.viewport_size().height.as_f32()
            - COLLAPSED_PANEL_HEIGHT
            - if self.status.is_some() {
                LAUNCHER_STATUS_FOOTER_HEIGHT
            } else {
                0.0
            })
        .clamp(0.0, LAUNCHER_LIST_MAX_HEIGHT);
        if allowed && height == 0.0 {
            // Wait for the existing native-bounds owner to expose the viewport.
            // No frame or timer is requested while the search-only area is shown.
            return;
        }
        let offset = -self
            .result_scroll
            .0
            .borrow()
            .base_handle
            .offset()
            .y
            .as_f32();
        let pitch = LAUNCHER_RESULT_ROW_HEIGHT + self.row_style.gap();
        let visible = ((offset - SPACING_8).max(0.0) / pitch).floor() as usize
            ..((offset + height - SPACING_8).max(0.0) / pitch).ceil() as usize;
        self.result_entrances.admit(
            self.query_dispatch_generation,
            self.results
                .iter()
                .enumerate()
                .map(|(index, result)| (result.entry.id.clone(), visible.contains(&index))),
            self.result_entrance,
            allowed,
            now,
        );
        self.result_admission_pending = false;
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
        if index >= self.results.len() || self.search_pending {
            return;
        }
        self.selection_explicit = true;
        if self.selected == index {
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
        if !self.is_open || self.search_pending {
            return;
        }
        if self.action_mode.is_some() {
            return;
        }
        self.dismiss_preview(cx);
        let Some(target) = self.results.get(index).cloned() else {
            return;
        };
        self.selected = index;
        self.selection_explicit = true;
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
        self.query_dispatch_generation = self.query_dispatch_generation.wrapping_add(1).max(1);
        self.search_pending = false;
        self.ime_enter_gate.reset();
        self.sync_window_content_size("", window, cx);
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
        let query = self.search_input.read(cx).value().to_string();
        self.query_dispatch_generation = self.query_dispatch_generation.wrapping_add(1).max(1);
        self.search_pending = false;
        self.ime_enter_gate.reset();
        self.sync_window_content_size(&query, window, cx);
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
        self.selection_explicit = false;
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
        self.sync_window_content_size(&query, window, cx);
        cx.notify();
        (query, self.query_dispatch_generation)
    }

    pub(crate) fn replace_results(&mut self, results: Vec<SearchResult>, cx: &mut Context<Self>) {
        let previous = self
            .results
            .get(self.selected)
            .map(|result| result.entry.id.clone());
        let visible_range = self.visible_result_range.clone();
        let selected = if self.selection_explicit {
            selection_after_results_refresh(&self.results, self.selected, &results)
        } else {
            0
        };
        let preserved = previous.as_ref().is_some_and(|id| {
            results
                .get(selected)
                .is_some_and(|result| &result.entry.id == id)
        });
        if !preserved {
            self.dismiss_preview(cx);
        }
        self.selected = selected;
        self.selection_explicit &= preserved;
        self.committed_query = self.search_input.read(cx).value();
        self.results = results.into();
        self.queue_result_entrances(cx);
        self.retain_icons_for_results();
        self.reset_presented_results();
        if !preserved || (!visible_range.is_empty() && !visible_range.contains(&selected)) {
            self.result_scroll
                .scroll_to_item(selected, ScrollStrategy::Center);
        }
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
        if !self.is_open || self.search_pending {
            self.status = Some("正在搜索，请等待当前结果后重试".into());
            cx.notify();
            return;
        }
        let Some(result) = self.results.get(self.selected).cloned() else {
            return;
        };
        let query = self.search_input.read(cx).value().to_string();
        let generation = self.query_dispatch_generation;
        let panel = cx.entity().downgrade();
        let application = self.application.clone();
        cx.defer(move |cx| {
            let current = panel.upgrade().is_some_and(|panel| {
                let panel = panel.read(cx);
                panel.is_open
                    && !panel.search_pending
                    && panel.query_dispatch_generation == generation
                    && panel.search_input.read(cx).value().as_ref() == query
                    && panel
                        .results
                        .get(panel.selected)
                        .is_some_and(|selected| selected.entry.id == result.entry.id)
            });
            if current && let Some(application) = application.upgrade() {
                application.update(cx, |application, cx| {
                    application.execute(result, action, query, trigger, cx)
                });
            }
        });
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
        if !self.is_open || self.search_pending {
            self.status = Some("正在搜索，请等待当前结果后重试".into());
            cx.notify();
            return;
        }
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
        let generation = self.query_dispatch_generation;
        let input = self.search_input.read(cx).value().to_string();
        let mode_id = mode.id();
        let level_id = mode.current_level_id();
        let panel = cx.entity().downgrade();
        let application = self.application.clone();
        cx.defer(move |cx| {
            let current = panel.upgrade().is_some_and(|panel| {
                let panel = panel.read(cx);
                panel.is_open
                    && !panel.search_pending
                    && panel.query_dispatch_generation == generation
                    && panel.search_input.read(cx).value().as_ref() == input
                    && panel.action_mode.as_ref().is_some_and(|mode| {
                        mode.matches_shell_query(mode_id, level_id, &result.entry.id)
                            && mode.parent.query == query
                            && mode.selected_action().as_ref() == Some(&action)
                    })
            });
            if current && let Some(application) = application.upgrade() {
                application.update(cx, |application, cx| {
                    application.execute(
                        result,
                        action,
                        query,
                        QuickLaunchActionTrigger::ActionMode,
                        cx,
                    )
                });
            }
        });
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
        self.finish_result_entrances("close", cx);
        self.query_dispatch_generation = self.query_dispatch_generation.wrapping_add(1).max(1);
        let panel = cx.entity().downgrade();
        let generation = self.query_dispatch_generation;
        cx.notify();
        schedule_application_update(
            self.application.clone(),
            move |application, cx| {
                let current = panel.upgrade().is_some_and(|panel| {
                    application
                        .popup
                        .as_ref()
                        .is_some_and(|popup| popup.panel.entity_id() == panel.entity_id())
                        && !panel.read(cx).is_open
                        && panel.read(cx).query_dispatch_generation == generation
                });
                if current {
                    application.hide_popup_and_restore(captured_window_handle, cx);
                }
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

    /// Query/engine health stays attached to the last-good search results.
    pub(crate) fn set_error(&mut self, message: String, cx: &mut Context<Self>) {
        self.search_pending &= !self.results.is_empty();
        self.status = Some(message.into());
        cx.notify();
    }

    pub(crate) fn set_status(&mut self, message: String, cx: &mut Context<Self>) {
        self.notify_operation(
            crate::notifications::NotificationLevel::Success,
            message,
            cx,
        );
    }

    pub(crate) fn notify_operation(
        &mut self,
        level: crate::notifications::NotificationLevel,
        message: String,
        cx: &mut Context<Self>,
    ) {
        self.search_pending = false;
        self.status = None;
        let application = self.application.clone();
        // Even the center lookup crosses an owner lease; a result callback may still hold
        // QuickLaunchApplication while updating this panel. Resolve it on the next turn.
        cx.defer(move |cx| {
            let center = application
                .upgrade()
                .map(|app| app.read(cx).notifications.clone())
                .or_else(|| crate::notifications::current(cx));
            if let Some(center) = center {
                let notice = crate::notifications::AppNotification::new(
                    format!("quick-launch-operation-{}", uuid::Uuid::new_v4()),
                    level,
                    "快速启动",
                    message,
                );
                center.update(cx, |center, cx| center.push(notice, cx));
            }
        });
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
        query: gpui_kit::SharedString,
        panel: WeakEntity<QuickLaunchPanel>,
        render: (&mut Window, &mut App),
    ) -> AnyElement {
        let (window, cx) = render;
        let theme = self.theme;
        let stale = self.search_pending;
        let selected = index == self.selected;
        let query = if stale {
            self.committed_query.clone()
        } else {
            query
        };
        let source = result.entry.source;
        let click_action = if self.recent_directory_mode {
            recent_directory_navigation_action(result, self.navigation_context.as_ref())
        } else {
            Some(quick_launch_primary_action(result, false, false))
        };
        let preview_application = self.application.clone();
        let preview_results = self.results.clone();
        let pointer_panel = panel.clone();
        let press_panel = panel.clone();
        let right_click_panel = panel.clone();
        let navigation_panel = panel.clone();
        let click_panel = panel;
        let icon_path = self.icons.get(&result.entry.id).cloned();
        let has_icon_path = icon_path.is_some();
        let display = self.results.display(index);
        let subtitle = display.subtitle.clone();
        let title = display.title.clone();
        let title_size = if matches!(
            source,
            ApplicationSource::StartMenu | ApplicationSource::Desktop | ApplicationSource::System
        ) {
            LAUNCHER_APPLICATION_TITLE_SIZE
        } else {
            LAUNCHER_FILE_TITLE_SIZE
        };
        let title_line_height = if subtitle.is_empty() {
            LAUNCHER_RESULT_ICON_SIZE
        } else {
            LAUNCHER_TITLE_LINE_HEIGHT
        };
        let full_label = display.accessible.clone();
        #[cfg(test)]
        {
            super::retained_text_tests::observe(format!("result-title-{index}"), &title);
            super::retained_text_tests::observe(format!("result-subtitle-{index}"), &subtitle);
        }
        let shortcut_hint = quick_launch_shortcut_hint(visible_position);
        let name_highlights = if result
            .subtitle
            .as_deref()
            .is_some_and(|label| label.contains("同音近似"))
        {
            nxtools_finder::FileQuery::parse(&query).homophone_match_ranges(&result.entry.name)
        } else {
            quick_launch_match_ranges(&result.entry.name, &query)
        }
        .into_iter()
        .map(|range| {
            (
                range,
                HighlightStyle {
                    color: Some(theme.accent.into()),
                    font_weight: Some(gpui_kit::FontWeight::SEMIBOLD),
                    ..Default::default()
                },
            )
        })
        .collect::<Vec<_>>();
        let entrance = self
            .result_entrances
            .sample(&result.entry.id, cx.background_executor().now());
        let row = quick_launch_row_shell(theme, selected)
            .h(px(LAUNCHER_RESULT_ROW_HEIGHT))
            .id(("quick-launch-result", index))
            .relative()
            .top(entrance.translation.y)
            .left(entrance.translation.x)
            .opacity(entrance.opacity)
            .when(!stale, |row| {
                row.active(move |row| row.bg(theme.surface_pressed))
            })
            .debug_selector(move || format!("quick-launch-result-row-{index}"))
            .when(!stale, |row| row.cursor_pointer())
            .on_hover(move |hovered, window, cx| {
                if stale {
                    return;
                }
                let entered = *hovered;
                let panel_bounds = window.bounds();
                let panel_scale_factor = window.scale_factor();
                let Some(result) = preview_results.get(index).cloned() else {
                    return;
                };
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
            .when(!stale, |row| {
                row.on_mouse_move(move |event, _window, cx| {
                    let position = (event.position.x.as_f32(), event.position.y.as_f32());
                    let _ = pointer_panel.update(cx, |panel, cx| {
                        panel.pointer_moved_result(index, position, cx);
                    });
                })
                .on_mouse_down(gpui_kit::MouseButton::Left, move |_event, _window, cx| {
                    let _ = press_panel.update(cx, |panel, cx| panel.select_result(index, cx));
                })
                .on_mouse_down(
                    gpui_kit::MouseButton::Right,
                    move |_event, window, cx| {
                        let _ = right_click_panel.update(cx, |panel, cx| {
                            panel.enter_action_mode(index, window, cx);
                        });
                        cx.stop_propagation();
                    },
                )
            })
            .child(
                div()
                    .debug_selector(move || format!("quick-launch-result-icon-{index}"))
                    .size(px(LAUNCHER_RESULT_ICON_SIZE))
                    .flex_none()
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
                                .size(px(LAUNCHER_RESULT_ICON_SIZE))
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
                    .gap_0()
                    .child(
                        div()
                            .debug_selector(move || format!("quick-launch-result-title-{index}"))
                            .h(px(title_line_height))
                            .text_size(px(title_size))
                            .line_height(px(title_line_height))
                            .text_color(theme.text)
                            .truncate()
                            .child(StyledText::new(title).with_highlights(name_highlights)),
                    )
                    .when(!subtitle.is_empty(), |copy| {
                        copy.child(
                            div()
                                .debug_selector(move || {
                                    format!("quick-launch-result-subtitle-{index}")
                                })
                                .h(px(LAUNCHER_SUBTITLE_LINE_HEIGHT))
                                .child(
                                    Label::new(subtitle)
                                        .w_full()
                                        .text_size(px(TEXT_BODY))
                                        .line_height(px(LAUNCHER_SUBTITLE_LINE_HEIGHT))
                                        .text_color(theme.text_secondary)
                                        .truncate()
                                        .text_ellipsis_middle(),
                                ),
                        )
                    }),
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
            .into_any_element();
        let row = quick_launch_focusable_row(
            gpui_kit::SharedString::from(format!("quick-launch-result-focus-{}", result.entry.id)),
            theme,
            click_action.is_none() || stale,
            row,
            window,
            cx,
            move |event, _window, cx| {
                let Some(action) = click_action.clone() else {
                    return;
                };
                let _ = click_panel.update(cx, |panel, cx| {
                    panel.select_result(index, cx);
                    let trigger = if event.is_keyboard() {
                        QuickLaunchActionTrigger::Enter
                    } else {
                        QuickLaunchActionTrigger::Pointer
                    };
                    panel.activate_selected(action, trigger, cx);
                });
            },
        )
        .aria_label(full_label.clone())
        .tooltip(move |window, cx| Tooltip::new(full_label.clone()).build(window, cx))
        .on_key_down(move |event, window, cx| {
            if event.keystroke.modifiers.modified()
                || !matches!(
                    event.keystroke.key.as_str(),
                    "up" | "down" | "right" | "escape"
                )
            {
                return;
            }
            let _ = navigation_panel.update(cx, |panel, cx| {
                panel.select_result(index, cx);
                panel.focus_search(window, cx);
                if panel.handle_navigation_key(
                    &event.keystroke.key,
                    event.keystroke.modifiers,
                    window,
                    cx,
                ) {
                    window.prevent_default();
                    cx.stop_propagation();
                }
            });
        })
        .into_any_element();
        div()
            .w_full()
            .flex_none()
            .flex()
            .flex_col()
            .child(row)
            .child(div().h(px(self.row_style.gap())).flex_none())
            .into_any_element()
    }

    pub(crate) fn render_action(
        &self,
        index: usize,
        item: QuickLaunchActionItem,
        panel: WeakEntity<QuickLaunchPanel>,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        let theme = self.theme;
        let selected = self
            .action_mode
            .as_ref()
            .is_some_and(|mode| mode.selected() == index);
        let enabled = item.is_activatable();
        let has_submenu = item.submenu.is_some();
        let pointer_panel = panel.clone();
        let navigation_panel = panel.clone();
        let click_panel = panel;
        let section_label = item.section_label.clone();
        let label = item.label.clone();
        #[cfg(test)]
        super::retained_text_tests::observe(format!("action-label-{index}"), &label);
        let divider_before = item.divider_before;
        let row = quick_launch_row_shell(theme, selected)
            .id(("quick-launch-action", index))
            .when(enabled, |row| {
                row.active(move |row| row.bg(theme.surface_pressed))
            })
            .debug_selector(move || format!("quick-launch-action-row-{index}"))
            .when(!enabled, |row| {
                row.bg(theme.canvas).text_color(theme.text_muted)
            })
            .when(enabled, |row| row.cursor_pointer())
            .when(enabled, |row| {
                row.on_mouse_move(move |event, _window, cx| {
                    let position = (event.position.x.as_f32(), event.position.y.as_f32());
                    let _ = pointer_panel.update(cx, |panel, cx| {
                        panel.pointer_moved_action(index, position, cx);
                    });
                })
            })
            .child(
                div()
                    .debug_selector(move || format!("quick-launch-action-icon-{index}"))
                    .size(px(COMPACT_RESULT_ICON_RAIL_WIDTH))
                    .flex_none()
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
                        container.child(gpui_kit::component::Icon::new(icon).size(px(18.0)))
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
                            .min_w_0()
                            .w_full()
                            .h(px(LINE_HEIGHT_BODY))
                            .text_size(px(TEXT_BODY))
                            .line_height(px(LINE_HEIGHT_BODY))
                            .text_color(if enabled {
                                theme.text
                            } else {
                                theme.text_muted
                            })
                            .truncate()
                            .child(label),
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
                            gpui_kit::component::Icon::new(MaterialIcon::ChevronRight)
                                .size(px(16.0))
                                .text_color(theme.icon_muted),
                        )
                    }),
            );
        let focus_id = gpui_kit::SharedString::from(format!(
            "quick-launch-action-focus-{}-{}-{index}",
            self.action_mode
                .as_ref()
                .expect("rendered action mode")
                .id(),
            self.action_mode
                .as_ref()
                .expect("rendered action level")
                .current_level_id(),
        ));
        let row = quick_launch_focusable_row(
            focus_id,
            theme,
            !enabled,
            row.into_any_element(),
            window,
            cx,
            move |_event, window, cx| {
                let _ = click_panel.update(cx, |panel, cx| {
                    panel.select_action(index, cx);
                    panel.activate_selected_action(window, cx);
                });
            },
        )
        .on_key_down(move |event, window, cx| {
            if !enabled
                || event.keystroke.modifiers.modified()
                || !matches!(
                    event.keystroke.key.as_str(),
                    "up" | "down" | "right" | "left" | "backspace" | "escape"
                )
            {
                return;
            }
            let _ = navigation_panel.update(cx, |panel, cx| {
                panel.select_action(index, cx);
                panel.focus_search(window, cx);
                if panel.handle_navigation_key(
                    &event.keystroke.key,
                    event.keystroke.modifiers,
                    window,
                    cx,
                ) {
                    window.prevent_default();
                    cx.stop_propagation();
                }
            });
        });
        div()
            .when(divider_before, |container| {
                container
                    .mt(px(SPACING_4))
                    .pt(px(SPACING_4))
                    .border_t_1()
                    .border_color(theme.border_subtle)
            })
            .when_some(section_label, |container, label| {
                #[cfg(test)]
                super::retained_text_tests::observe(format!("action-section-{index}"), &label);
                container.child(
                    div()
                        .h(px(24.0))
                        .px(px(SPACING_8))
                        .flex()
                        .items_center()
                        .text_size(px(TEXT_META))
                        .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                        .text_color(theme.text_muted)
                        .child(label),
                )
            })
            .child(row)
            .child(div().h(px(self.row_style.gap())).flex_none())
            .into_any_element()
    }
}

impl Render for QuickLaunchPanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        crate::ui::motion::observe_motion(window, cx);
        crate::theme::observe_theme(window, cx);
        self.theme = Theme::active(cx);
        let now = cx.background_executor().now();
        self.present_result_entrances(window, cx);
        if !self.is_open
            || !self.results_visible
            || self.search_pending
            || !self.result_motion_available
            || self.action_mode.is_some()
            || self.recent_directory_mode
            || self.result_entrance == nxtools_core::QuickLaunchResultEntrance::None
            || !crate::ui::motion::motion_allowed(cx)
        {
            self.finish_result_entrances("suppressed", cx);
        }
        self.result_entrances.advance(now);
        if self.result_entrances.is_active() {
            window.request_animation_frame();
        }
        let theme = self.theme;
        let panel = cx.entity().downgrade();
        let recent_button_panel = panel.clone();
        let preview_scroll_application = self.application.clone();
        let query = self.search_input.read(cx).value();
        let recent_button_visible = !self.recent_directories.is_empty()
            && self.navigation_context.is_some()
            && self.action_mode.is_none()
            && (self.recent_directory_mode || query.trim().is_empty());
        let recent_directory_mode = self.recent_directory_mode;
        let action_target = self.action_mode.as_ref().map(|mode| {
            (
                mode.title(),
                mode.target_title.clone(),
                mode.shell_pending(),
            )
        });
        let action_entries = if let Some(mode) = self.action_mode.as_ref() {
            mode.visible_actions()
                .into_iter()
                .enumerate()
                .map(|(index, item)| self.render_action(index, item, panel.clone(), window, cx))
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        let result_surface = if let Some((level_title, target, pending)) = action_target {
            #[cfg(test)]
            {
                super::retained_text_tests::observe("action-level-title".into(), &level_title);
                super::retained_text_tests::observe("action-target-title".into(), &target);
            }
            div()
                .id("quick-launch-actions-scroll")
                .flex_1()
                .min_h_0()
                .track_scroll(&self.action_scroll)
                .overflow_y_scroll()
                .p(px(SPACING_8))
                .flex()
                .flex_col()
                .child(
                    div()
                        .debug_selector(|| "quick-launch-action-context".into())
                        .h(px(QUICK_LAUNCH_ACTION_CONTEXT_HEIGHT))
                        .mb(px(SPACING_4))
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
                .when(self.search_pending && self.status.is_none(), |container| {
                    container.child(progress_state(
                        theme,
                        "quick-launch-results-progress",
                        "正在搜索",
                        "正在合并应用、本地文件与系统工具结果。",
                    ))
                })
                .when(self.status.is_some(), |container| {
                    container.child(empty_state(
                        theme,
                        "quick-launch-results-unavailable",
                        MaterialIcon::Info,
                        "搜索暂不可用",
                        "检查索引服务后重试；下方显示具体原因。",
                    ))
                })
                .when(!self.search_pending && self.status.is_none(), |container| {
                    container.child(empty_state(
                        theme,
                        "quick-launch-results-empty",
                        MaterialIcon::Search,
                        QUICK_LAUNCH_EMPTY_RESULTS_LABEL,
                        if recent_modified_filter(&query).is_some() {
                            "输入 recent: 后可加文件名筛选；旧索引缺少修改时间时需重建索引。"
                        } else {
                            "尝试缩短关键词，或输入 recent: 查看最近修改文件。"
                        },
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
                cx.processor(move |panel, visible_range: Range<usize>, window, cx| {
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
                                    (window, cx),
                                )
                            })
                        })
                        .collect::<Vec<_>>()
                }),
            )
            .size_full()
            .p(px(SPACING_8))
            .track_scroll(&self.result_scroll);
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
            .overflow_hidden()
            .flex()
            .flex_col()
            .child(result_surface);
        let result_surface = result_surface.into_any_element();
        div()
            .id("quick-launch-window")
            .debug_selector(|| "quick-launch-window".into())
            .on_key_up(cx.listener(|this, _event, window, cx| {
                if this.search_input.focus_handle(cx).is_focused(window) {
                    let composing = this.search_input.update(cx, |state, cx| {
                        state.marked_text_range(window, cx).is_some()
                    });
                    if !composing {
                        this.ime_enter_gate.reset();
                    }
                }
            }))
            .relative()
            .size_full()
            .overflow_hidden()
            .font_family(UI_FONT_FAMILY)
            .bg(theme.canvas)
            .text_color(theme.text)
            .border_1()
            // Reserve the original inset without enclosing the Kit input in
            // another outline, in collapsed, result and action presentations.
            .border_color(theme.canvas)
            .flex()
            .flex_col()
            .child(
                div()
                    .debug_selector(|| "quick-launch-search-header".into())
                    .h(px(SEARCH_HEADER_HEIGHT))
                    .px(px(SPACING_4))
                    .flex_none()
                    .border_b_1()
                    .border_color(theme.canvas)
                    .bg(theme.canvas)
                    .flex()
                    .items_center()
                    .gap(px(SPACING_8))
                    .text_size(px(TEXT_BODY))
                    .line_height(px(LINE_HEIGHT_BODY))
                    .child(
                        div()
                            .debug_selector(|| "quick-launch-search-group".into())
                            .min_w_0()
                            .flex_1()
                            .on_mouse_down(gpui_kit::MouseButton::Left, {
                                let input = self.search_input.clone();
                                move |_, window, cx| input.focus_handle(cx).focus(window, cx)
                            })
                            .child(
                                Input::new(&self.search_input)
                                    .aria_label("速启搜索")
                                    .focus_bordered(false)
                                    .with_size(ControlSize::Large)
                                    .py(px(SPACING_4))
                                    .w_full()
                                    .text_size(px(SEARCH_INPUT_TEXT_SIZE))
                                    .line_height(px(SEARCH_INPUT_LINE_HEIGHT))
                                    .prefix(
                                        div().id("quick-launch-search-icon").child(
                                            gpui_kit::component::Icon::new(MaterialIcon::Search)
                                                .size(px(24.0))
                                                .text_color(theme.icon_muted),
                                        ),
                                    ),
                            ),
                    )
                    .when(recent_button_visible, |header| {
                        header.child(
                            Button::new("quick-launch-recent-directories")
                                .debug_selector(|| "quick-launch-recent-action".into())
                                .accessibility_label("最近目录")
                                .ghost()
                                .tooltip("最近目录")
                                .size(px(28.0))
                                .rounded(px(RADIUS_CONTROL))
                                .flex()
                                .items_center()
                                .justify_center()
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
                                .on_click(move |_event, window, cx| {
                                    let _ = recent_button_panel.update(cx, |panel, cx| {
                                        if panel.recent_directory_mode {
                                            panel.exit_recent_directory_mode(window, cx);
                                        } else {
                                            panel.enter_recent_directory_mode(window, cx);
                                        }
                                    });
                                })
                                .icon(
                                    gpui_kit::component::Icon::new(MaterialIcon::History)
                                        .size(px(15.0)),
                                ),
                        )
                    }),
            )
            .when(self.results_visible, |panel| panel.child(result_surface))
            .when_some(self.status.clone(), |panel, status| {
                #[cfg(test)]
                super::retained_text_tests::observe("status".into(), &status);
                panel.child(
                    div()
                        .debug_selector(|| "quick-launch-status-footer".into())
                        .mx(px(SPACING_12))
                        .mb(px(SPACING_8))
                        .h(px(LAUNCHER_STATUS_FOOTER_HEIGHT - SPACING_8))
                        .flex_none()
                        .px(px(12.0))
                        .rounded(px(RADIUS_CONTROL))
                        .border_1()
                        .border_color(theme.border)
                        .bg(theme.surface_raised)
                        .flex()
                        .min_w_0()
                        .items_center()
                        .text_size(px(TEXT_META))
                        .text_color(theme.text_secondary)
                        .truncate()
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
        .size(px(LAUNCHER_RESULT_ICON_SIZE))
        .flex_none()
        .child(
            gpui_kit::component::Icon::new(source_icon(source)).size(px(LAUNCHER_RESULT_ICON_SIZE)),
        )
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
) -> (Bounds<gpui_kit::Pixels>, Option<gpui_kit::DisplayId>) {
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
        .find(|display| u64::from(display.id()) == u64::from(request.cursor_display_index))
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

#[cfg(test)]
#[path = "auxiliary_theme_tests.rs"]
mod auxiliary_theme_tests;
