use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Local};
use gpui::{
    Animation, AnimationExt, AnyElement, AnyWindowHandle, App, Context, Corner, Entity,
    FocusHandle, IntoElement, KeyDownEvent, MouseButton, ObjectFit, Render, ScrollHandle,
    ScrollStrategy, ScrollWheelEvent, Subscription, UniformListScrollHandle, Window, div,
    ease_in_out, img, point, prelude::*, px, uniform_list,
};
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::menu::{ContextMenuExt, DropdownMenu, PopupMenu, PopupMenuItem};
use gpui_component::scroll::{ScrollableElement, Scrollbar};
use gpui_component::tooltip::Tooltip;
use gpui_component::{Disableable, Icon, PixelsExt, Sizable};
use nxtools_core::{
    ClipboardClickAction, ClipboardClickModifier, ClipboardHotKey, ClipboardHotKeyModifier,
    ClipboardSettings, CustomCommandSettings, SystemSettings,
};
use nxtools_download::{
    DownloadBatchAction, DownloadEngineTask, DownloadFault, DownloadFaultCode, DownloadProgress,
    DownloadQueuePlacement, DownloadSegment, DownloadSegmentState, DownloadStatus,
    DownloadTaskDetail, DownloadTaskId, DownloadTaskTimelineEntry, DownloadTaskTimelineEvent,
    RequestOrigin,
};
use nxtools_ipc::PlatformBridge;
use nxtools_protocol::{
    BeginWindowMoveCommand, PlatformCommand, SetWindowMinimizedCommand, SetWindowVisibilityCommand,
    ToggleWindowMaximizedCommand,
};
use nxtools_protocol::{IndexPhase, IndexStatus, ShellOpenAction};
use tracing::info;

use crate::clipboard::ClipboardApplication;
use crate::downloads::DownloadApplication;
use crate::icons::{CaptionIcon, MaterialIcon};
use crate::network_settings::NetworkSettingsApplication;
use crate::notifications::{NOTIFICATION_EXIT_DURATION, NotificationCenter, NotificationLevel};
use crate::quick_launch::{
    QuickLaunchApplication, QuickLaunchApplicationEvent, QuickLaunchHotkeyLifecycle,
    QuickLaunchPerformanceSnapshot,
};
use crate::shell::{MainModule, ShellState};
use crate::theme::Theme;
use crate::ui::components::action::{
    SegmentedOption, action_button, icon_action, labelled_icon_action, segmented_control,
};
use crate::ui::components::custom_command_editor::{
    CustomCommandEditorActions, CustomCommandEditorInputs, CustomCommandEditorState,
    custom_command_editor,
};
use crate::ui::components::hotkey::{HotkeyFieldState, hotkey_field, hotkey_setting_row};
use crate::ui::components::menu::{MenuTone, context_menu_item, context_menu_shell};
use crate::ui::components::path_list::path_exclusion_setting;
use crate::ui::components::quick_launch_diagnostics::{
    QuickLaunchDiagnosticsActions, QuickLaunchDiagnosticsData, quick_launch_diagnostics,
};
use crate::ui::components::settings::{SettingsSplitSurfaceIds, settings_split_surface};
use crate::ui::components::state::{
    empty_state, inline_error_state, interactive_state, progress_state,
};
use crate::ui::components::titlebar::ModuleHeader;
use crate::ui::components::volume_scope::{
    automatic_volume_scope_row, empty_volume_scope_row, volume_scope_row, volume_scope_setting,
};
use crate::ui::metrics::{
    BADGE_HEIGHT, CONTROL_HEIGHT_ACTION, GROUP_HEADER_HEIGHT, PILL_PADDING_X, RADIUS_CONTROL,
    SIDEBAR_ROW_HEIGHT, SIDEBAR_WIDTH, SPACING_2, SPACING_4, SPACING_6, SPACING_8, SPACING_12,
    SPACING_16, SPACING_24, TITLEBAR_HEIGHT,
};
use crate::ui::typography::{
    LINE_HEIGHT_BODY, LINE_HEIGHT_LABEL, LINE_HEIGHT_META, LINE_HEIGHT_TITLE, TEXT_BODY,
    TEXT_LABEL, TEXT_META, TEXT_TITLE, UI_FONT_FAMILY, UI_MONO_FONT_FAMILY,
};
mod clipboard_page;
mod download_page;
mod notifications;
mod quick_launch_page;
mod shell;
mod tools_page;

use self::clipboard_page::*;
use self::download_page::*;
use self::quick_launch_page::*;

const NOTIFICATION_ENTER_DURATION: Duration = Duration::from_millis(150);
const NOTIFICATION_CARD_WIDTH: f32 = 336.0;
const NOTIFICATION_CARD_MAX_HEIGHT: f32 = 112.0;
const QUICK_FIND_SCROLL_FRAME_DURATION: Duration = Duration::from_millis(12);
const QUICK_FIND_SCROLL_LINE_HEIGHT: f32 = 38.0;
const QUICK_FIND_SCROLL_EASING: f32 = 0.42;
const QUICK_FIND_SCROLL_TARGET_SNAP_DISTANCE: f32 = 128.0;
const QUICK_LAUNCH_SETTINGS_NAV_WIDTH: f32 = 160.0;
const QUICK_LAUNCH_SETTINGS_NAV_ROW_HEIGHT: f32 = 40.0;
const QUICK_LAUNCH_SETTINGS_PAGE_HEADER_HEIGHT: f32 = 48.0;
const CLIPBOARD_SETTINGS_SECONDARY_WIDTH: f32 = 410.0;
const DOWNLOAD_FILTER_STRIP_HEIGHT: f32 = 38.0;
const DOWNLOAD_FILTER_SEGMENTED_WIDTH: f32 = 408.0;
const DOWNLOAD_TABLE_HEADER_HEIGHT: f32 = 28.0;
const DOWNLOAD_TASK_ROW_HEIGHT: f32 = 46.0;
const DOWNLOAD_BATCH_TOOLBAR_HEIGHT: f32 = 36.0;
const DOWNLOAD_DETAIL_HEIGHT: f32 = 176.0;
const DOWNLOAD_DETAIL_TAB_HEIGHT: f32 = 30.0;
const DOWNLOAD_DETAIL_LABEL_WIDTH: f32 = 52.0;
const DOWNLOAD_SEARCH_DIAGNOSTIC_DELAY: Duration = Duration::from_millis(250);
const DOWNLOAD_TRANSFERRED_COLUMN_WIDTH: f32 = 90.0;
const DOWNLOAD_SPEED_COLUMN_WIDTH: f32 = 54.0;
const DOWNLOAD_REMAINING_COLUMN_WIDTH: f32 = 36.0;
const DOWNLOAD_ERROR_TOOLTIP_WIDTH: f32 = 320.0;
const DOWNLOAD_ERROR_TOOLTIP_MAX_HEIGHT: f32 = 144.0;
const DOWNLOAD_ERROR_TOOLTIP_SURFACE_MAX_WIDTH: f32 = 340.0;
const DOWNLOAD_ERROR_TOOLTIP_SURFACE_MAX_HEIGHT: f32 = 170.0;
const DOWNLOAD_PROGRESS_COLUMN_WIDTH: f32 = DOWNLOAD_TRANSFERRED_COLUMN_WIDTH
    + DOWNLOAD_SPEED_COLUMN_WIDTH
    + DOWNLOAD_REMAINING_COLUMN_WIDTH;
const DOWNLOAD_STATUS_COLUMN_WIDTH: f32 = 68.0;
const DOWNLOAD_CREATED_COLUMN_WIDTH: f32 = 68.0;
pub(crate) struct MainWindow {
    theme: Theme,
    shell: ShellState,
    clipboard: gpui::WeakEntity<ClipboardApplication>,
    quick_launch: gpui::WeakEntity<QuickLaunchApplication>,
    downloads: gpui::WeakEntity<DownloadApplication>,
    network_settings: Option<Entity<NetworkSettingsApplication>>,
    platform: Option<Rc<PlatformBridge>>,
    native_window_handle: i64,
    system_settings: Rc<RefCell<SystemSettings>>,
    notifications: Entity<NotificationCenter>,
    download_search_input: gpui::Entity<InputState>,
    exclusions_input: gpui::Entity<InputState>,
    custom_command_keyword: gpui::Entity<InputState>,
    custom_command_title: gpui::Entity<InputState>,
    custom_command_program: gpui::Entity<InputState>,
    custom_command_arguments: gpui::Entity<InputState>,
    custom_command_working_directory: gpui::Entity<InputState>,
    quick_launch_settings_page: QuickLaunchSettingsPage,
    quick_find_scroll: ScrollHandle,
    quick_find_scroll_target: f32,
    quick_find_scroll_animation: u64,
    selected_index_exclusion: Option<usize>,
    selected_custom_command: Option<usize>,
    custom_command_draft_generation: u64,
    pending_custom_command_program_picker: Option<PendingCustomCommandProgramPicker>,
    selected_download_task: Option<DownloadTaskId>,
    selected_download_tasks: HashSet<DownloadTaskId>,
    download_selection_anchor: Option<DownloadTaskId>,
    download_filter: DownloadTaskFilter,
    download_sort: DownloadSortState,
    download_list_scroll: UniformListScrollHandle,
    tools_scroll: ScrollHandle,
    tool_settings_windows: HashMap<&'static str, AnyWindowHandle>,
    download_search_log_generation: u64,
    download_detail_tab: DownloadDetailTab,
    download_detail_scroll: ScrollHandle,
    requested_download_detail: Option<DownloadTaskId>,
    custom_command_run_as_admin: bool,
    custom_command_run_silently: bool,
    hotkey_focus: FocusHandle,
    recording_hotkey: Option<HotkeyRecordingTarget>,
    clipboard_overview: ClipboardOverview,
    clipboard_overview_generation: u64,
    clipboard_overview_loading: bool,
    clipboard_overview_dirty: bool,
    _subscriptions: Vec<Subscription>,
}

pub(crate) struct MainWindowShellContext {
    pub(crate) settings: Option<Entity<NetworkSettingsApplication>>,
    pub(crate) platform: Option<Rc<PlatformBridge>>,
    pub(crate) system_settings: Rc<RefCell<SystemSettings>>,
}
impl MainWindow {
    pub(crate) fn render_tools(&mut self, cx: &mut Context<Self>) -> AnyElement {
        tools_page::render_tools(self, cx)
    }

    pub(crate) fn new(
        clipboard: gpui::WeakEntity<ClipboardApplication>,
        quick_launch: gpui::WeakEntity<QuickLaunchApplication>,
        downloads: gpui::WeakEntity<DownloadApplication>,
        shell_context: MainWindowShellContext,
        notifications: Entity<NotificationCenter>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let excluded_processes = clipboard
            .upgrade()
            .map(|application| {
                application
                    .read(cx)
                    .clipboard_settings()
                    .excluded_processes
                    .join("; ")
            })
            .unwrap_or_default();
        let exclusions_input = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(excluded_processes)
                .placeholder("KeePass.exe; Bitwarden.exe")
        });
        let download_search_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("搜索文件名或地址"));
        let initial_custom_command = quick_launch.upgrade().and_then(|application| {
            application
                .read(cx)
                .settings()
                .custom_commands
                .first()
                .cloned()
        });
        let initial_keyword = initial_custom_command
            .as_ref()
            .map_or_else(String::new, |command| command.keyword.clone());
        let initial_title = initial_custom_command
            .as_ref()
            .map_or_else(String::new, |command| command.title.clone());
        let initial_program = initial_custom_command
            .as_ref()
            .map_or_else(String::new, |command| command.program.clone());
        let initial_arguments = initial_custom_command
            .as_ref()
            .map_or_else(String::new, |command| command.argument_template.clone());
        let initial_working_directory = initial_custom_command
            .as_ref()
            .map_or_else(String::new, |command| command.working_directory.clone());
        let custom_command_keyword = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(initial_keyword.clone())
                .placeholder("关键词")
        });
        let custom_command_title = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(initial_title.clone())
                .placeholder("显示名称")
        });
        let custom_command_program = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(initial_program.clone())
                .placeholder(r"C:\Tools\tool.exe")
        });
        let custom_command_arguments = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(initial_arguments.clone())
                .placeholder("--query %s 或 --name %s1")
        });
        let custom_command_working_directory = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(initial_working_directory.clone())
                .placeholder("可选工作目录")
        });
        let exclusions_subscription = cx.subscribe_in(
            &exclusions_input,
            window,
            |this, input, event: &InputEvent, _window, cx| {
                if matches!(event, InputEvent::Blur | InputEvent::PressEnter { .. }) {
                    let value = input.read(cx).value().to_string();
                    let _ = this.clipboard.update(cx, |application, _| {
                        application.set_clipboard_excluded_processes(&value);
                    });
                    cx.notify();
                }
            },
        );
        let download_search_subscription = cx.subscribe_in(
            &download_search_input,
            window,
            |this, input, event: &InputEvent, _window, cx| {
                if !matches!(event, InputEvent::Change) {
                    return;
                }
                let query = input.read(cx).value().trim().to_owned();
                this.download_list_scroll
                    .scroll_to_item(0, ScrollStrategy::Top);
                this.download_search_log_generation =
                    this.download_search_log_generation.wrapping_add(1).max(1);
                let generation = this.download_search_log_generation;
                cx.notify();
                let background = cx.background_executor().clone();
                cx.spawn(async move |this, cx| {
                    background.timer(DOWNLOAD_SEARCH_DIAGNOSTIC_DELAY).await;
                    let _ = this.update(cx, |this, cx| {
                        if this.download_search_log_generation != generation
                            || this.download_search_input.read(cx).value().trim() != query
                        {
                            return;
                        }
                        let result_count = this.downloads.upgrade().map_or(0, |application| {
                            application
                                .read(cx)
                                .tasks()
                                .iter()
                                .filter(|task| this.download_filter.matches(task.task.status()))
                                .filter(|task| download_task_matches_search(task, &query))
                                .count()
                        });
                        info!(
                            event = "download.list.search_changed",
                            query_length = query.chars().count(),
                            result_count,
                            "download task search changed"
                        );
                    });
                })
                .detach();
            },
        );
        let clipboard_subscription = clipboard.upgrade().map(|application| {
            cx.observe(&application, |this, _, cx| {
                this.clipboard_overview_dirty = true;
                this.request_clipboard_overview(cx);
                cx.notify();
            })
        });
        let quick_launch_subscription = quick_launch
            .upgrade()
            .map(|application| cx.observe(&application, |_, _, cx| cx.notify()));
        let downloads_subscription = downloads
            .upgrade()
            .map(|application| cx.observe(&application, |_, _, cx| cx.notify()));
        let quick_launch_event_subscription = quick_launch.upgrade().map(|application| {
            cx.subscribe_in(
                &application,
                window,
                |this, _application, event: &QuickLaunchApplicationEvent, window, cx| {
                    let QuickLaunchApplicationEvent::CustomCommandProgramPicked {
                        correlation_id,
                        index,
                        path,
                    } = event;
                    let matches_current_draft = this
                        .pending_custom_command_program_picker
                        .is_some_and(|pending| {
                            pending.correlation_id == *correlation_id
                                && pending.command_index == *index
                                && pending.draft_generation == this.custom_command_draft_generation
                        })
                        && this.selected_custom_command == Some(*index);
                    if matches_current_draft {
                        let generation = this.custom_command_draft_generation;
                        this.pending_custom_command_program_picker = None;
                        let index = *index;
                        let path = path.clone();
                        let input = this.custom_command_program.clone();
                        let view = cx.entity().downgrade();
                        let source_window = window.window_handle();
                        cx.defer(move |cx| {
                            let still_selected = view
                                .update(cx, |this, _| {
                                    this.selected_custom_command == Some(index)
                                        && this.custom_command_draft_generation == generation
                                })
                                .unwrap_or(false);
                            if !still_selected {
                                return;
                            }
                            cx.defer(move |cx| {
                                let _ = source_window.update(cx, |_, window, cx| {
                                    input.update(cx, |input, cx| {
                                        input.set_value(path, window, cx);
                                    });
                                });
                            });
                        });
                    }
                },
            )
        });
        let notification_subscription = cx.observe(&notifications, |_, _, cx| cx.notify());
        let hotkey_focus = cx.focus_handle();
        let hotkey_focus_subscription =
            cx.on_focus_out(&hotkey_focus, window, |this, _event, _window, cx| {
                if this.recording_hotkey.take().is_some() {
                    this.defer_all_hotkey_recording(false, cx);
                    cx.notify();
                }
            });
        let mut subscriptions = vec![exclusions_subscription, download_search_subscription];
        subscriptions.extend(clipboard_subscription);
        subscriptions.extend(quick_launch_subscription);
        subscriptions.extend(downloads_subscription);
        subscriptions.extend(quick_launch_event_subscription);
        subscriptions.push(notification_subscription);
        subscriptions.push(hotkey_focus_subscription);
        let initial_download_sort = downloads
            .clone()
            .upgrade()
            .map(|application| {
                let (key, descending) = application.read(cx).task_list_sort();
                let key = key
                    .as_deref()
                    .and_then(DownloadSortKey::from_diagnostic_name)
                    .unwrap_or_default();
                DownloadSortState {
                    key,
                    descending: descending.unwrap_or_else(|| key.default_descending()),
                }
            })
            .unwrap_or_default();
        let mut main_window = Self {
            theme: Theme::zed_dark(),
            shell: ShellState::default(),
            clipboard,
            quick_launch,
            downloads: downloads.clone(),
            network_settings: shell_context.settings,
            platform: shell_context.platform,
            native_window_handle: crate::ui::native_window_handle(window).unwrap_or_default(),
            system_settings: shell_context.system_settings,
            notifications,
            download_search_input,
            exclusions_input,
            custom_command_keyword,
            custom_command_title,
            custom_command_program,
            custom_command_arguments,
            custom_command_working_directory,
            quick_launch_settings_page: QuickLaunchSettingsPage::default(),
            quick_find_scroll: ScrollHandle::new(),
            quick_find_scroll_target: 0.0,
            quick_find_scroll_animation: 0,
            selected_index_exclusion: None,
            selected_custom_command: initial_custom_command.as_ref().map(|_| 0),
            custom_command_draft_generation: 0,
            pending_custom_command_program_picker: None,
            selected_download_task: None,
            selected_download_tasks: HashSet::new(),
            download_selection_anchor: None,
            download_filter: DownloadTaskFilter::default(),
            download_sort: initial_download_sort,
            download_list_scroll: UniformListScrollHandle::new(),
            tools_scroll: ScrollHandle::new(),
            tool_settings_windows: HashMap::new(),
            download_search_log_generation: 0,
            download_detail_tab: DownloadDetailTab::default(),
            download_detail_scroll: ScrollHandle::new(),
            requested_download_detail: None,
            clipboard_overview: ClipboardOverview::default(),
            clipboard_overview_generation: 0,
            clipboard_overview_loading: false,
            clipboard_overview_dirty: false,
            custom_command_run_as_admin: initial_custom_command
                .as_ref()
                .is_some_and(|command| command.run_as_admin),
            custom_command_run_silently: initial_custom_command
                .as_ref()
                .is_some_and(|command| command.run_silently),
            hotkey_focus,
            recording_hotkey: None,
            _subscriptions: subscriptions,
        };
        main_window.request_clipboard_overview(cx);
        main_window
    }
}
impl Render for MainWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let content = match self.shell.active_module() {
            MainModule::Downloads => self.render_downloads(cx),
            MainModule::QuickFind => self.render_quick_find(cx),
            MainModule::Clipboard => self.render_clipboard(cx),
            MainModule::Tools => self.render_tools(cx),
        };
        div()
            .id("nxtools-main-window")
            .debug_selector(|| "nxtools-main-window".into())
            .size_full()
            .relative()
            .overflow_hidden()
            .track_focus(&self.hotkey_focus)
            .on_key_down(cx.listener(|this, event, _window, cx| {
                if this.recording_hotkey.is_some() {
                    this.handle_hotkey_recording(event, cx);
                } else {
                    this.handle_index_exclusion_keyboard(event, cx);
                }
            }))
            .font_family(UI_FONT_FAMILY)
            .bg(self.theme.canvas)
            .text_color(self.theme.text)
            .flex()
            .flex_col()
            .child(self.render_titlebar(window))
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .child(self.render_sidebar(cx))
                    .child(div().flex_1().min_w_0().h_full().child(content)),
            )
            .child(self.render_notifications(cx))
    }
}
#[cfg(test)]
mod tests;
