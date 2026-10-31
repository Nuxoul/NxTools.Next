use super::*;
use crate::ui::components::tooltip::bounded_text_tooltip;
use crate::ui::metrics::ordinary_rem;
use crate::ui::typography::{
    LINE_HEIGHT_WORKBENCH_BODY as LINE_HEIGHT_BODY,
    LINE_HEIGHT_WORKBENCH_SUPPORT as LINE_HEIGHT_LABEL, TEXT_WORKBENCH_BODY as TEXT_BODY,
    TEXT_WORKBENCH_SUPPORT as TEXT_LABEL,
};
use gpui_kit::base::TestSupportExt;
use gpui_kit::component::form::{Field, Form};
use gpui_kit::component::group_box::GroupBoxVariant;
use gpui_kit::component::progress::Progress;
use gpui_kit::component::setting::{SelectIndex, SettingGroup, SettingItem, SettingPage, Settings};
use gpui_kit::component::{label::Label, list::ListItem};

pub(crate) struct DownloadSettingsInput<'a> {
    pub(crate) default_directory: &'a str,
    pub(crate) connection_limit: &'a str,
    pub(crate) adaptive_segment_scaling_enabled: bool,
    pub(crate) adaptive_segment_max_segments: &'a str,
    pub(crate) adaptive_segment_min_mib: &'a str,
    pub(crate) max_concurrent_tasks: &'a str,
    pub(crate) speed_limit_kib: &'a str,
    pub(crate) show_progress_window: bool,
    pub(crate) show_completion_window: bool,
    pub(crate) completion_sound_enabled: bool,
    pub(crate) open_directory_on_completion: bool,
    pub(crate) task_removal_deletes_file: bool,
    pub(crate) taskbar_overlay_enabled: bool,
    pub(crate) taskbar_overlay_anchor: DownloadTaskbarOverlayAnchor,
    pub(crate) taskbar_overlay_offset_x: &'a str,
    pub(crate) taskbar_overlay_offset_y: &'a str,
    pub(crate) taskbar_overlay_width: &'a str,
    pub(crate) taskbar_overlay_height: &'a str,
    pub(crate) taskbar_overlay_progress_mode: DownloadTaskbarProgressMode,
    pub(crate) taskbar_overlay_show_progress: bool,
    pub(crate) taskbar_overlay_show_speed: bool,
    pub(crate) taskbar_overlay_show_remaining_time: bool,
    pub(crate) taskbar_overlay_show_active_tasks: bool,
    pub(crate) taskbar_overlay_show_active_connections: bool,
    pub(crate) taskbar_overlay_show_transferred_bytes: bool,
}

pub(crate) fn parse_download_settings_draft(
    input: DownloadSettingsInput<'_>,
) -> Result<DownloadSettingsDraft, String> {
    let DownloadSettingsInput {
        default_directory,
        connection_limit,
        adaptive_segment_scaling_enabled,
        adaptive_segment_max_segments,
        adaptive_segment_min_mib,
        max_concurrent_tasks,
        speed_limit_kib,
        show_progress_window,
        show_completion_window,
        completion_sound_enabled,
        open_directory_on_completion,
        task_removal_deletes_file,
        taskbar_overlay_enabled,
        taskbar_overlay_anchor,
        taskbar_overlay_offset_x,
        taskbar_overlay_offset_y,
        taskbar_overlay_width,
        taskbar_overlay_height,
        taskbar_overlay_progress_mode,
        taskbar_overlay_show_progress,
        taskbar_overlay_show_speed,
        taskbar_overlay_show_remaining_time,
        taskbar_overlay_show_active_tasks,
        taskbar_overlay_show_active_connections,
        taskbar_overlay_show_transferred_bytes,
    } = input;
    let default_directory = default_directory.trim();
    if default_directory.is_empty() {
        return Err("请选择默认保存目录".into());
    }
    let default_connection_limit = connection_limit
        .trim()
        .parse::<u16>()
        .map_err(|_| "连接上限必须是 1 到 64 的整数")?;
    if !(1..=64).contains(&default_connection_limit) {
        return Err("连接上限必须在 1 到 64 之间".into());
    }
    let adaptive_segment_max_segments = adaptive_segment_max_segments
        .trim()
        .parse::<u16>()
        .map_err(|_| "动态分段上限必须是整数")?;
    if !(default_connection_limit..=256).contains(&adaptive_segment_max_segments) {
        return Err("动态分段上限必须在连接数和 256 之间".into());
    }
    let adaptive_segment_min_mib = adaptive_segment_min_mib
        .trim()
        .parse::<u16>()
        .map_err(|_| "最小可拆分大小必须是 1 到 100 MiB 的整数")?;
    if !(1..=100).contains(&adaptive_segment_min_mib) {
        return Err("最小可拆分大小必须在 1 到 100 MiB 之间".into());
    }
    let max_concurrent_tasks = max_concurrent_tasks
        .trim()
        .parse::<u16>()
        .map_err(|_| "同时下载数必须是 1 到 64 的整数")?;
    if !(1..=64).contains(&max_concurrent_tasks) {
        return Err("同时下载数必须在 1 到 64 之间".into());
    }
    let speed_limit_kib = speed_limit_kib
        .trim()
        .parse::<u64>()
        .map_err(|_| "全局限速必须是非负整数")?;
    let global_speed_limit_bytes_per_second = if speed_limit_kib == 0 {
        None
    } else {
        Some(
            speed_limit_kib
                .checked_mul(1024)
                .ok_or("全局限速数值过大")?,
        )
    };
    let taskbar_overlay_offset_x = taskbar_overlay_offset_x
        .trim()
        .parse::<i32>()
        .map_err(|_| "任务栏 X 偏移必须是整数")?;
    let taskbar_overlay_offset_y = taskbar_overlay_offset_y
        .trim()
        .parse::<i32>()
        .map_err(|_| "任务栏 Y 偏移必须是整数")?;
    if !(-4096..=4096).contains(&taskbar_overlay_offset_x)
        || !(-4096..=4096).contains(&taskbar_overlay_offset_y)
    {
        return Err("任务栏偏移必须在 -4096 到 4096 之间".into());
    }
    let taskbar_overlay_width = taskbar_overlay_width
        .trim()
        .parse::<u32>()
        .map_err(|_| "任务栏覆盖层宽度必须是整数")?;
    let mut taskbar_overlay_height = taskbar_overlay_height
        .trim()
        .parse::<u32>()
        .map_err(|_| "任务栏覆盖层高度必须是整数")?;
    if !(72..=640).contains(&taskbar_overlay_width) {
        return Err("任务栏覆盖层宽度必须在 72 到 640 之间".into());
    }
    if !(6..=72).contains(&taskbar_overlay_height) {
        return Err("任务栏覆盖层高度必须在 6 到 72 之间".into());
    }
    if !taskbar_overlay_show_progress
        && !taskbar_overlay_show_speed
        && !taskbar_overlay_show_remaining_time
        && !taskbar_overlay_show_active_tasks
        && !taskbar_overlay_show_active_connections
        && !taskbar_overlay_show_transferred_bytes
    {
        return Err("任务栏覆盖层至少要显示一项信息".into());
    }
    if taskbar_overlay_show_speed
        || taskbar_overlay_show_remaining_time
        || taskbar_overlay_show_active_tasks
        || taskbar_overlay_show_active_connections
        || taskbar_overlay_show_transferred_bytes
    {
        taskbar_overlay_height = taskbar_overlay_height.max(20);
    }
    Ok(DownloadSettingsDraft {
        default_directory: PathBuf::from(default_directory),
        chrome_web_store_extension_id: String::new(),
        automatic_categories_enabled: false,
        categories: DownloadSettings::default().categories,
        duplicate_strategy: DownloadDuplicateStrategySettings::AddNumbered,
        task_list_sort: None,
        task_list_sort_descending: None,
        double_click_action: DownloadRowDoubleClickActionSettings::OpenFile,
        ffmpeg_executable_path: String::new(),
        default_request_headers: String::new(),
        default_cookie: String::new(),
        accept_invalid_certificates: false,
        default_connection_limit,
        adaptive_segment_scaling_enabled,
        adaptive_segment_max_segments,
        adaptive_segment_min_mib,
        max_concurrent_tasks,
        global_speed_limit_bytes_per_second,
        background_speed_limit: Default::default(),
        show_progress_window,
        show_completion_window,
        completion_sound_enabled,
        open_directory_on_completion,
        task_removal_deletes_file,
        taskbar_progress_overlay: DownloadTaskbarProgressOverlaySettings {
            enabled: taskbar_overlay_enabled,
            anchor: taskbar_overlay_anchor,
            offset_x: taskbar_overlay_offset_x,
            offset_y: taskbar_overlay_offset_y,
            width: taskbar_overlay_width,
            height: taskbar_overlay_height,
            progress_mode: taskbar_overlay_progress_mode,
            show_progress: taskbar_overlay_show_progress,
            show_speed: taskbar_overlay_show_speed,
            show_remaining_time: taskbar_overlay_show_remaining_time,
            show_active_tasks: taskbar_overlay_show_active_tasks,
            show_active_connections: taskbar_overlay_show_active_connections,
            show_transferred_bytes: taskbar_overlay_show_transferred_bytes,
        },
    })
}

pub(crate) struct DownloadCategoryEditor {
    pub(crate) id: String,
    pub(crate) enabled: bool,
    pub(crate) name: Entity<InputState>,
    pub(crate) save_directory: Entity<InputState>,
    pub(crate) extensions: Entity<InputState>,
    pub(crate) url_patterns: Entity<InputState>,
}

pub(crate) struct DownloadCategoryPresentation {
    pub(crate) name: String,
    pub(crate) directory: String,
    pub(crate) enabled: bool,
}

impl DownloadCategoryEditor {
    pub(crate) fn new(
        category: DownloadCategorySettings,
        window: &mut Window,
        cx: &mut App,
    ) -> Self {
        let name = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(category.name)
                .placeholder("分类名称")
        });
        let save_directory = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(category.save_directory)
                .placeholder("相对目录，如 Documents")
        });
        let extensions = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(category.extensions.join(", "))
                .placeholder("扩展名，以逗号分隔")
        });
        let url_patterns = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(category.url_patterns.join(", "))
                .placeholder("URL 通配规则，以逗号分隔")
        });
        Self {
            id: category.id,
            enabled: category.enabled,
            name,
            save_directory,
            extensions,
            url_patterns,
        }
    }

    pub(crate) fn settings(&self, cx: &App) -> DownloadCategorySettings {
        let values = |input: &Entity<InputState>| {
            input
                .read(cx)
                .value()
                .split([',', ';', '\n'])
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned)
                .collect()
        };
        DownloadCategorySettings {
            id: self.id.clone(),
            name: self.name.read(cx).value().trim().to_owned(),
            enabled: self.enabled,
            save_directory: self.save_directory.read(cx).value().trim().to_owned(),
            extensions: values(&self.extensions),
            url_patterns: values(&self.url_patterns),
        }
    }

    pub(crate) fn presentation(
        &self,
        default_directory: &str,
        cx: &App,
    ) -> DownloadCategoryPresentation {
        let name = self.name.read(cx).value().trim().to_owned();
        let directory = self.save_directory.read(cx).value().trim().to_owned();
        let directory = if directory.is_empty() {
            PathBuf::from(default_directory)
        } else {
            let directory = PathBuf::from(directory);
            if directory.is_absolute() {
                directory
            } else {
                PathBuf::from(default_directory).join(directory)
            }
        };
        DownloadCategoryPresentation {
            name: if name.is_empty() {
                "未命名分类".to_owned()
            } else {
                name
            },
            directory: directory.display().to_string(),
            enabled: self.enabled,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum DownloadSettingsPage {
    #[default]
    General,
    Request,
    Transfer,
    Window,
    Diagnostics,
}

pub(crate) struct DownloadSettingsView {
    pub(crate) theme: Theme,
    pub(crate) application: Entity<DownloadApplication>,
    pub(crate) directory: Entity<InputState>,
    pub(super) directory_picker: DownloadDirectoryDraft,
    pub(crate) chrome_web_store_extension_id: Entity<InputState>,
    pub(crate) ffmpeg_executable: Entity<InputState>,
    pub(crate) default_request_headers: Entity<TextareaState>,
    pub(crate) default_cookie: Entity<InputState>,
    pub(crate) ffmpeg_discovery_state: FfmpegDiscoveryState,
    pub(crate) ffmpeg_discovery_generation: u64,
    pub(crate) ffmpeg_install_state: FfmpegInstallViewState,
    pub(crate) connection_limit: Entity<InputState>,
    pub(crate) adaptive_segment_max_segments: Entity<InputState>,
    pub(crate) adaptive_segment_min_mib: Entity<InputState>,
    pub(crate) concurrent_tasks: Entity<InputState>,
    pub(crate) speed_limit_kib: Entity<InputState>,
    background_speed_limit_enabled: bool,
    background_speed_limit_kib: Entity<InputState>,
    background_speed_limit_saved: u64,
    pub(crate) taskbar_overlay_offset_x: Entity<InputState>,
    pub(crate) taskbar_overlay_offset_y: Entity<InputState>,
    pub(crate) taskbar_overlay_width: Entity<InputState>,
    pub(crate) taskbar_overlay_height: Entity<InputState>,
    // Initial seed only; released Settings owns active selection/search/scroll.
    pub(crate) page: DownloadSettingsPage,
    pub(crate) show_progress_window: bool,
    pub(crate) show_completion_window: bool,
    pub(crate) completion_sound_enabled: bool,
    pub(crate) open_directory_on_completion: bool,
    pub(crate) task_removal_deletes_file: bool,
    pub(crate) task_list_sort: Option<String>,
    pub(crate) task_list_sort_descending: Option<bool>,
    pub(crate) automatic_categories_enabled: bool,
    pub(crate) categories: Vec<DownloadCategoryEditor>,
    pub(crate) selected_category: Option<usize>,
    pub(crate) duplicate_strategy: DownloadDuplicateStrategySettings,
    pub(crate) double_click_action: DownloadRowDoubleClickActionSettings,
    pub(crate) adaptive_segment_scaling_enabled: bool,
    pub(crate) accept_invalid_certificates: bool,
    pub(crate) taskbar_overlay_enabled: bool,
    pub(crate) taskbar_overlay_anchor: DownloadTaskbarOverlayAnchor,
    pub(crate) taskbar_overlay_progress_mode: DownloadTaskbarProgressMode,
    pub(crate) taskbar_overlay_show_progress: bool,
    pub(crate) taskbar_overlay_show_speed: bool,
    pub(crate) taskbar_overlay_show_remaining_time: bool,
    pub(crate) taskbar_overlay_show_active_tasks: bool,
    pub(crate) taskbar_overlay_show_active_connections: bool,
    pub(crate) taskbar_overlay_show_transferred_bytes: bool,
    pub(crate) taskbar_overlay_editing: bool,
    pub(crate) config_path: String,
    pub(crate) log_directory: String,
    pub(crate) database_path: String,
    pub(crate) saving: bool,
    pub(crate) error: Option<String>,
    pub(crate) _subscriptions: Vec<Subscription>,
    #[cfg(test)]
    pub(crate) category_presentation_revision: usize,
    #[cfg(test)]
    pub(crate) category_action_invocations: usize,
}

// gpui-component 0.5.1 passes the full placeholder run to each rendered line of an empty
// multiline input on Windows. Keep this placeholder on one line until that native text boundary
// is upgraded; a newline here can panic DirectWrite before the settings window paints.
pub(crate) const DEFAULT_REQUEST_HEADERS_PLACEHOLDER: &str =
    "每行 Name: Value，例如 User-Agent: ... 或 Referer: ...";

impl DownloadSettingsView {
    pub(crate) fn subscribe_category_presentation_input(
        input: &Entity<InputState>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Subscription {
        cx.subscribe_in(input, window, |_this, _, event: &InputEvent, _, cx| {
            if matches!(event, InputEvent::Change) {
                #[cfg(test)]
                {
                    _this.category_presentation_revision =
                        _this.category_presentation_revision.wrapping_add(1);
                }
                cx.notify();
            }
        })
    }

    pub(crate) fn new(
        application: Entity<DownloadApplication>,
        initial: DownloadSettingsViewInitial,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let DownloadSettingsViewInitial {
            draft: snapshot,
            config_path,
            log_directory,
            database_path,
            ffmpeg_install_state,
        } = initial;
        let directory = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(snapshot.default_directory.to_string_lossy().into_owned())
                .placeholder("选择默认保存目录")
        });
        let chrome_web_store_extension_id = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(snapshot.chrome_web_store_extension_id.clone())
                .placeholder("可留空；上传成功后填写商店 ID")
        });
        let ffmpeg_executable = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(snapshot.ffmpeg_executable_path.clone())
                .placeholder("留空则自动检测 FFMPEG_PATH / PATH")
        });
        let default_request_headers = cx.new(|cx| {
            TextareaState::new(window, cx)
                .default_value(snapshot.default_request_headers.clone())
                .placeholder(DEFAULT_REQUEST_HEADERS_PLACEHOLDER)
        });
        let default_cookie = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(snapshot.default_cookie.clone())
                .placeholder("Cookie: a=b; c=d")
        });
        let categories = snapshot
            .categories
            .iter()
            .cloned()
            .map(|category| DownloadCategoryEditor::new(category, window, cx))
            .collect::<Vec<_>>();
        let mut category_presentation_subscriptions = Vec::with_capacity(categories.len() * 2 + 1);
        category_presentation_subscriptions.push(Self::subscribe_category_presentation_input(
            &directory, window, cx,
        ));
        for category in &categories {
            category_presentation_subscriptions.push(Self::subscribe_category_presentation_input(
                &category.name,
                window,
                cx,
            ));
            category_presentation_subscriptions.push(Self::subscribe_category_presentation_input(
                &category.save_directory,
                window,
                cx,
            ));
        }
        let ffmpeg_input_subscription = cx.subscribe_in(
            &ffmpeg_executable,
            window,
            |this, _, event: &InputEvent, _, cx| {
                if matches!(event, InputEvent::Change) {
                    this.schedule_ffmpeg_discovery(cx);
                }
            },
        );
        let connection_limit = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(snapshot.default_connection_limit.to_string())
                .placeholder("1-64")
        });
        let adaptive_segment_max_segments = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(snapshot.adaptive_segment_max_segments.to_string())
                .placeholder("8-256")
        });
        let adaptive_segment_min_mib = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(snapshot.adaptive_segment_min_mib.to_string())
                .placeholder("1-100")
        });
        let concurrent_tasks = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(snapshot.max_concurrent_tasks.to_string())
                .placeholder("1-64")
        });
        let speed_limit_kib = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(
                    snapshot
                        .global_speed_limit_bytes_per_second
                        .map_or(0, |value| value / 1024)
                        .to_string(),
                )
                .placeholder("0")
        });
        let background_speed_limit_kib = cx.new(|cx| {
            InputState::new(window, cx).default_value(
                snapshot
                    .background_speed_limit
                    .bytes_per_second
                    .div_ceil(1024)
                    .to_string(),
            )
        });
        let taskbar_overlay_offset_x = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(snapshot.taskbar_progress_overlay.offset_x.to_string())
                .placeholder("12")
        });
        let taskbar_overlay_offset_y = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(snapshot.taskbar_progress_overlay.offset_y.to_string())
                .placeholder("0")
        });
        let taskbar_overlay_width = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(snapshot.taskbar_progress_overlay.width.to_string())
                .placeholder("132")
        });
        let taskbar_overlay_height = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(snapshot.taskbar_progress_overlay.height.to_string())
                .placeholder("8")
        });
        let directory_subscription = cx.subscribe(&directory, |this, _, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                this.directory_picker.edited();
                let application = this.application.downgrade();
                cx.defer(move |cx| {
                    let _ = application.update(cx, |application, cx| {
                        application.cleanup_directory_picker(cx)
                    });
                });
            }
        });
        let subscription = cx.subscribe_in(
            &application,
            window,
            |this, _application, event: &DownloadApplicationEvent, window, cx| {
                match event {
                    DownloadApplicationEvent::FfmpegExecutablePicked { path } => {
                        let ffmpeg_executable = this.ffmpeg_executable.clone();
                        let path = path.to_string_lossy().into_owned();
                        let source_window = window.window_handle();
                        let view = cx.entity().downgrade();
                        cx.defer(move |cx| {
                            let _ = source_window.update(cx, |_, window, cx| {
                                ffmpeg_executable.update(cx, |input, cx| {
                                    input.set_value(path, window, cx);
                                });
                                let _ = view.update(cx, |view, cx| {
                                    view.schedule_ffmpeg_discovery(cx);
                                });
                            });
                        });
                        this.error = None;
                    }
                    DownloadApplicationEvent::FfmpegInstallChanged { state } => {
                        // Durable capability/repair state stays beside the install controls.
                        this.ffmpeg_install_state = state.clone();
                    }
                    DownloadApplicationEvent::TaskbarOverlayEditChanged { settings } => {
                        this.taskbar_overlay_offset_x.update(cx, |input, cx| {
                            input.set_value(settings.offset_x.to_string(), window, cx);
                        });
                        this.taskbar_overlay_offset_y.update(cx, |input, cx| {
                            input.set_value(settings.offset_y.to_string(), window, cx);
                        });
                        this.taskbar_overlay_width.update(cx, |input, cx| {
                            input.set_value(settings.width.to_string(), window, cx);
                        });
                        this.taskbar_overlay_height.update(cx, |input, cx| {
                            input.set_value(settings.height.to_string(), window, cx);
                        });
                        this.taskbar_overlay_editing = true;
                    }
                }
                cx.notify();
            },
        );
        let offset_x_subscription = cx.subscribe_in(
            &taskbar_overlay_offset_x,
            window,
            |this, _, event: &InputEvent, _, cx| {
                if matches!(event, InputEvent::Blur | InputEvent::PressEnter { .. }) {
                    this.sync_taskbar_overlay_edit(cx);
                }
            },
        );
        let offset_y_subscription = cx.subscribe_in(
            &taskbar_overlay_offset_y,
            window,
            |this, _, event: &InputEvent, _, cx| {
                if matches!(event, InputEvent::Blur | InputEvent::PressEnter { .. }) {
                    this.sync_taskbar_overlay_edit(cx);
                }
            },
        );
        let width_subscription = cx.subscribe_in(
            &taskbar_overlay_width,
            window,
            |this, _, event: &InputEvent, _, cx| {
                if matches!(event, InputEvent::Blur | InputEvent::PressEnter { .. }) {
                    this.sync_taskbar_overlay_edit(cx);
                }
            },
        );
        let height_subscription = cx.subscribe_in(
            &taskbar_overlay_height,
            window,
            |this, _, event: &InputEvent, _, cx| {
                if matches!(event, InputEvent::Blur | InputEvent::PressEnter { .. }) {
                    this.sync_taskbar_overlay_edit(cx);
                }
            },
        );
        let view = cx.entity().downgrade();
        cx.defer(move |cx| {
            let _ = view.update(cx, |view, cx| view.schedule_ffmpeg_discovery(cx));
        });
        Self {
            theme: Theme::zed_dark(),
            application,
            directory,
            directory_picker: DownloadDirectoryDraft::default(),
            chrome_web_store_extension_id,
            ffmpeg_executable,
            default_request_headers,
            default_cookie,
            ffmpeg_discovery_state: FfmpegDiscoveryState::Checking,
            ffmpeg_discovery_generation: 0,
            ffmpeg_install_state,
            connection_limit,
            adaptive_segment_max_segments,
            adaptive_segment_min_mib,
            concurrent_tasks,
            speed_limit_kib,
            background_speed_limit_enabled: snapshot.background_speed_limit.enabled,
            background_speed_limit_kib,
            background_speed_limit_saved: snapshot.background_speed_limit.bytes_per_second,
            taskbar_overlay_offset_x,
            taskbar_overlay_offset_y,
            taskbar_overlay_width,
            taskbar_overlay_height,
            page: DownloadSettingsPage::General,
            show_progress_window: snapshot.show_progress_window,
            show_completion_window: snapshot.show_completion_window,
            completion_sound_enabled: snapshot.completion_sound_enabled,
            task_removal_deletes_file: snapshot.task_removal_deletes_file,
            task_list_sort: snapshot.task_list_sort.clone(),
            task_list_sort_descending: snapshot.task_list_sort_descending,
            open_directory_on_completion: snapshot.open_directory_on_completion,
            automatic_categories_enabled: snapshot.automatic_categories_enabled,
            selected_category: (!categories.is_empty()).then_some(0),
            categories,
            duplicate_strategy: snapshot.duplicate_strategy,
            double_click_action: snapshot.double_click_action,
            adaptive_segment_scaling_enabled: snapshot.adaptive_segment_scaling_enabled,
            accept_invalid_certificates: snapshot.accept_invalid_certificates,
            taskbar_overlay_enabled: snapshot.taskbar_progress_overlay.enabled,
            taskbar_overlay_anchor: snapshot.taskbar_progress_overlay.anchor,
            taskbar_overlay_progress_mode: snapshot.taskbar_progress_overlay.progress_mode,
            taskbar_overlay_show_progress: snapshot.taskbar_progress_overlay.show_progress,
            taskbar_overlay_show_speed: snapshot.taskbar_progress_overlay.show_speed,
            taskbar_overlay_show_remaining_time: snapshot
                .taskbar_progress_overlay
                .show_remaining_time,
            taskbar_overlay_show_active_tasks: snapshot.taskbar_progress_overlay.show_active_tasks,
            taskbar_overlay_show_active_connections: snapshot
                .taskbar_progress_overlay
                .show_active_connections,
            taskbar_overlay_show_transferred_bytes: snapshot
                .taskbar_progress_overlay
                .show_transferred_bytes,
            taskbar_overlay_editing: false,
            config_path,
            log_directory,
            database_path,
            saving: false,
            error: None,
            _subscriptions: vec![
                subscription,
                directory_subscription,
                ffmpeg_input_subscription,
                offset_x_subscription,
                offset_y_subscription,
                width_subscription,
                height_subscription,
            ]
            .into_iter()
            .chain(category_presentation_subscriptions)
            .collect(),
            #[cfg(test)]
            category_presentation_revision: 0,
            #[cfg(test)]
            category_action_invocations: 0,
        }
    }

    pub(super) fn request_directory(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.saving {
            return;
        }
        let Some((command_id, revision)) = self.directory_picker.begin() else {
            return;
        };
        let request = DownloadDirectoryPickerRequest {
            command_id,
            revision,
            window: window.window_handle(),
            owner: DownloadDirectoryOwner::Settings(cx.entity().downgrade()),
            current_directory: self.directory.read(cx).value().to_string(),
        };
        let application = self.application.clone();
        cx.defer(move |cx| {
            application.update(cx, |application, cx| {
                application.request_directory(request, cx)
            })
        });
    }

    pub(crate) fn schedule_ffmpeg_discovery(&mut self, cx: &mut Context<Self>) {
        self.ffmpeg_discovery_generation = self.ffmpeg_discovery_generation.wrapping_add(1).max(1);
        let generation = self.ffmpeg_discovery_generation;
        let configured = self.ffmpeg_executable.read(cx).value().trim().to_owned();
        self.ffmpeg_discovery_state = FfmpegDiscoveryState::Checking;
        let background = cx.background_executor().clone();
        cx.spawn(async move |this, cx| {
            let resolution = background
                .spawn(async move {
                    let configured = (!configured.is_empty()).then(|| PathBuf::from(configured));
                    resolve_ffmpeg_executable(configured.as_deref())
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                if this.ffmpeg_discovery_generation != generation {
                    return;
                }
                this.ffmpeg_discovery_state = resolution
                    .map_or(FfmpegDiscoveryState::Missing, |resolution| {
                        FfmpegDiscoveryState::Found(resolution.source)
                    });
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    pub(crate) fn current_draft(&self, cx: &App) -> Result<DownloadSettingsDraft, String> {
        let mut draft = parse_download_settings_draft(DownloadSettingsInput {
            default_directory: &self.directory.read(cx).value(),
            connection_limit: &self.connection_limit.read(cx).value(),
            adaptive_segment_scaling_enabled: self.adaptive_segment_scaling_enabled,
            adaptive_segment_max_segments: &self.adaptive_segment_max_segments.read(cx).value(),
            adaptive_segment_min_mib: &self.adaptive_segment_min_mib.read(cx).value(),
            max_concurrent_tasks: &self.concurrent_tasks.read(cx).value(),
            speed_limit_kib: &self.speed_limit_kib.read(cx).value(),
            show_progress_window: self.show_progress_window,
            show_completion_window: self.show_completion_window,
            completion_sound_enabled: self.completion_sound_enabled,
            open_directory_on_completion: self.open_directory_on_completion,
            task_removal_deletes_file: self.task_removal_deletes_file,
            taskbar_overlay_enabled: self.taskbar_overlay_enabled,
            taskbar_overlay_anchor: self.taskbar_overlay_anchor,
            taskbar_overlay_offset_x: &self.taskbar_overlay_offset_x.read(cx).value(),
            taskbar_overlay_offset_y: &self.taskbar_overlay_offset_y.read(cx).value(),
            taskbar_overlay_width: &self.taskbar_overlay_width.read(cx).value(),
            taskbar_overlay_height: &self.taskbar_overlay_height.read(cx).value(),
            taskbar_overlay_progress_mode: self.taskbar_overlay_progress_mode,
            taskbar_overlay_show_progress: self.taskbar_overlay_show_progress,
            taskbar_overlay_show_speed: self.taskbar_overlay_show_speed,
            taskbar_overlay_show_remaining_time: self.taskbar_overlay_show_remaining_time,
            taskbar_overlay_show_active_tasks: self.taskbar_overlay_show_active_tasks,
            taskbar_overlay_show_active_connections: self.taskbar_overlay_show_active_connections,
            taskbar_overlay_show_transferred_bytes: self.taskbar_overlay_show_transferred_bytes,
        })?;
        draft.chrome_web_store_extension_id = self
            .chrome_web_store_extension_id
            .read(cx)
            .value()
            .trim()
            .to_owned();
        if !nxtools_core::is_valid_chrome_web_store_extension_id(
            &draft.chrome_web_store_extension_id,
        ) {
            return Err("Chrome 商店扩展 ID 必须留空或为 32 位 a–p 小写字母".into());
        }
        draft.ffmpeg_executable_path = self.ffmpeg_executable.read(cx).value().trim().to_owned();
        draft.default_request_headers = self
            .default_request_headers
            .read(cx)
            .value()
            .trim()
            .to_owned();
        draft.default_cookie = self.default_cookie.read(cx).value().trim().to_owned();
        draft.accept_invalid_certificates = self.accept_invalid_certificates;
        draft.automatic_categories_enabled = self.automatic_categories_enabled;
        draft.categories = self
            .categories
            .iter()
            .map(|category| category.settings(cx))
            .collect();
        draft.duplicate_strategy = self.duplicate_strategy;
        draft.double_click_action = self.double_click_action;
        // Saving any setting must not wipe the task-list sort preference the
        // user set through the table header or the sort menu; the draft parse
        // hardcodes None, so restore the view-open-time values here.
        draft.task_list_sort = self.task_list_sort.clone();
        draft.task_list_sort_descending = self.task_list_sort_descending;
        let bytes_per_second = if self.background_speed_limit_enabled {
            let value = self
                .background_speed_limit_kib
                .read(cx)
                .value()
                .trim()
                .parse::<u64>()
                .map_err(|_| "后台限速必须是正整数 KiB/s")?;
            value
                .checked_mul(1024)
                .filter(|value| *value > 0)
                .ok_or("后台限速必须大于 0 且不能过大")?
        } else {
            self.background_speed_limit_saved
        };
        draft.background_speed_limit = nxtools_core::BackgroundDownloadLimitSettings {
            enabled: self.background_speed_limit_enabled,
            bytes_per_second,
        };
        Ok(draft)
    }

    fn notify_operation(
        &self,
        id: impl Into<gpui_kit::SharedString>,
        level: crate::notifications::NotificationLevel,
        message: String,
        cx: &mut Context<Self>,
    ) {
        let application = self.application.clone();
        let id = id.into();
        cx.defer(move |cx| {
            let center = application.read(cx).notifications.clone();
            center.update(cx, |center, cx| {
                center.push(
                    crate::notifications::AppNotification::new(id, level, "下载", message),
                    cx,
                )
            });
        });
    }

    pub(crate) fn toggle_taskbar_overlay_edit(&mut self, cx: &mut Context<Self>) {
        let correlation_id = uuid::Uuid::new_v4();
        if self.taskbar_overlay_editing {
            let application = self.application.clone();
            cx.defer(move |cx| {
                application.update(cx, |application, cx| {
                    application.finish_taskbar_overlay_edit(false, correlation_id, cx);
                });
            });
            self.taskbar_overlay_editing = false;
            cx.notify();
            return;
        }
        let settings = match self.current_draft(cx) {
            Ok(draft) => draft.taskbar_progress_overlay,
            Err(error) => {
                self.error = Some(error);
                cx.notify();
                return;
            }
        };
        let application = self.application.clone();
        let view = cx.entity().downgrade();
        cx.defer(move |cx| {
            let result = application.update(cx, |application, cx| {
                application.begin_taskbar_overlay_edit(settings, correlation_id, cx)
            });
            let _ = view.update(cx, |view, cx| match result {
                Ok(()) => {
                    view.taskbar_overlay_editing = true;
                    view.error = None;
                    cx.notify();
                }
                Err(error) => {
                    view.notify_operation(
                        format!("download-overlay-edit-{correlation_id}"),
                        crate::notifications::NotificationLevel::Error,
                        error,
                        cx,
                    );
                    cx.notify();
                }
            });
        });
    }

    pub(crate) fn sync_taskbar_overlay_edit(&mut self, cx: &mut Context<Self>) {
        if !self.taskbar_overlay_editing {
            return;
        }
        let settings = match self.current_draft(cx) {
            Ok(draft) => draft.taskbar_progress_overlay,
            Err(error) => {
                self.error = Some(error);
                cx.notify();
                return;
            }
        };
        let application = self.application.clone();
        let view = cx.entity().downgrade();
        let correlation_id = uuid::Uuid::new_v4();
        cx.defer(move |cx| {
            let result = application.update(cx, |application, cx| {
                application.update_taskbar_overlay_edit(settings, correlation_id, cx)
            });
            let _ = view.update(cx, |view, cx| match result {
                Ok(()) => {
                    view.error = None;
                    cx.notify();
                }
                Err(error) => {
                    view.notify_operation(
                        format!("download-overlay-edit-{correlation_id}"),
                        crate::notifications::NotificationLevel::Error,
                        error,
                        cx,
                    );
                    cx.notify();
                }
            });
        });
    }

    pub(crate) fn save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.saving {
            return;
        }
        let correlation_id = uuid::Uuid::new_v4();
        let draft = match self.current_draft(cx) {
            Ok(draft) => draft,
            Err(error) => {
                warn!(
                    event = "download.settings.validation_failed",
                    %correlation_id,
                    error_kind = "validation",
                    "download settings validation failed"
                );
                self.error = Some(error);
                cx.notify();
                return;
            }
        };
        self.saving = true;
        self.error = None;
        let application = self.application.clone();
        let source_window = window.window_handle();
        let view = cx.entity().downgrade();
        cx.defer(move |cx| {
            let result = application.update(cx, |application, cx| {
                application.apply_settings(draft, correlation_id, cx)
            });
            match result {
                Ok(()) => {
                    let _ = source_window.update(cx, |_, window, _| window.remove_window());
                }
                Err(error) => {
                    let _ = view.update(cx, |view, cx| {
                        view.saving = false;
                        view.notify_operation(
                            format!("download-settings-saved-{correlation_id}"),
                            crate::notifications::NotificationLevel::Error,
                            error,
                            cx,
                        );
                        cx.notify();
                    });
                }
            }
        });
        cx.notify();
    }
}

impl DownloadSettingsView {
    fn render_setting_control(
        &mut self,
        page: DownloadSettingsPage,
        slot: usize,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        self.theme = Theme::active(cx);
        let theme = self.theme;
        // The released Settings mounts only the requested field; draft ownership stays here.
        let current_directory = self.directory.read(cx).value().to_string();
        let directory_control = div()
            .w(ordinary_rem(286.0))
            .max_w_full()
            .min_w_0()
            .flex()
            .items_center()
            .gap_2()
            .child(download_settings_input(
                theme,
                "download-settings-default-directory-field",
                &self.directory,
                ordinary_rem(250.0),
            ))
            .child(
                div()
                    .debug_selector(|| "download-settings-pick-directory-control".into())
                    .child(icon_action_tooltip_surface(
                        icon_action_button(
                            "download-settings-pick-directory",
                            MaterialIcon::Folder,
                            "选择默认保存目录…",
                        )
                        .outline()
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.request_directory(window, cx);
                        })),
                        "选择默认保存目录…",
                        "download-settings-pick-directory-tooltip",
                    )),
            )
            .into_any_element();
        let default_headers_control = div()
            .debug_selector(|| "download-settings-default-headers-field".into())
            .w_full()
            .min_w_0()
            .child(
                Textarea::new(&self.default_request_headers)
                    .aria_label("默认 Header")
                    .h(px(DOWNLOAD_REQUEST_HEADER_EDITOR_HEIGHT)),
            )
            .into_any_element();
        let default_cookie_control = div()
            .debug_selector(|| "download-settings-default-cookie-field".into())
            .w_full()
            .min_w_0()
            .child(Input::new(&self.default_cookie).aria_label("默认 Cookie"))
            .into_any_element();
        let certificate_toggle = settings_toggle(
            theme,
            "download-settings-accept-invalid-certificates",
            self.accept_invalid_certificates,
            cx.listener(|this, _, _, cx| {
                this.accept_invalid_certificates = !this.accept_invalid_certificates;
                this.error = None;
                cx.notify();
            }),
        );
        let automatic_categories_toggle = settings_toggle(
            theme,
            "download-settings-automatic-categories",
            self.automatic_categories_enabled,
            cx.listener(|this, _, _, cx| {
                this.automatic_categories_enabled = !this.automatic_categories_enabled;
                this.error = None;
                cx.notify();
            }),
        );
        let double_click_action = download_settings_double_click_action(
            theme,
            self.double_click_action,
            cx.listener(|this, _, _, cx| {
                this.double_click_action = DownloadRowDoubleClickActionSettings::OpenFile;
                this.error = None;
                cx.notify();
            }),
            cx.listener(|this, _, _, cx| {
                this.double_click_action = DownloadRowDoubleClickActionSettings::OpenDirectory;
                this.error = None;
                cx.notify();
            }),
            cx.listener(|this, _, _, cx| {
                this.double_click_action = DownloadRowDoubleClickActionSettings::OpenTaskWindow;
                this.error = None;
                cx.notify();
            }),
            cx,
        );
        let duplicate_strategy = download_settings_duplicate_strategy(
            theme,
            self.duplicate_strategy,
            cx.listener(|this, _, _, cx| {
                this.duplicate_strategy = DownloadDuplicateStrategySettings::AddNumbered;
                this.error = None;
                cx.notify();
            }),
            cx.listener(|this, _, _, cx| {
                this.duplicate_strategy = DownloadDuplicateStrategySettings::Reject;
                this.error = None;
                cx.notify();
            }),
            cx,
        );
        let category_rows = self.categories.iter().enumerate().fold(
            div().w_full().flex().flex_col(),
            |list, (index, category)| {
                let selected = self.selected_category == Some(index);
                let presentation = category.presentation(&current_directory, cx);
                let selector = format!("download-settings-category-{index}");
                let tooltip = presentation.directory.clone();
                let tooltip_selector = format!("download-settings-category-{index}-tooltip");
                let content = ListItem::new(gpui_kit::SharedString::from(selector.clone()))
                    .debug_selector(move || selector.clone())
                    .selected(selected)
                    .aria_selected(selected)
                    .accessibility_label(presentation.name.clone())
                    .min_h(ordinary_rem(40.0))
                    .py_0()
                    .w_full()
                    .tooltip(move |window, cx| {
                        let selector = tooltip_selector.clone();
                        let directory = tooltip.clone();
                        Tooltip::element(move |_, _| {
                            let selector = selector.clone();
                            div()
                                .debug_selector(move || selector.clone())
                                .child(directory.clone())
                        })
                        .build(window, cx)
                    })
                    .child(
                        div()
                            .w_full()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .debug_selector(move || {
                                        format!("download-settings-category-{index}-title")
                                    })
                                    .w_full()
                                    .min_w_0()
                                    .child(Label::new(presentation.name).text_sm().truncate()),
                            )
                            .child(
                                div()
                                    .debug_selector(move || {
                                        format!("download-settings-category-{index}-detail")
                                    })
                                    .w_full()
                                    .min_w_0()
                                    .child(Label::new(presentation.directory).text_xs().truncate()),
                            )
                            .when(!presentation.enabled, |row| row.opacity(0.62)),
                    )
                    .into_any_element();
                list.child(crate::ui::components::action::focusable_content(
                    gpui_kit::SharedString::from(format!(
                        "download-settings-category-{index}-action"
                    )),
                    theme,
                    false,
                    content,
                    cx.listener(move |this, _, _, cx| {
                        this.selected_category = Some(index);
                        this.error = None;
                        cx.notify();
                    }),
                ))
            },
        );
        let category_list = div()
            .debug_selector(|| "download-settings-category-master".into())
            .w(ordinary_rem(228.0))
            .h_full()
            .flex_shrink_1()
            .min_w_0()
            .rounded(px(RADIUS_CONTROL))
            .border_1()
            .border_color(theme.border_subtle)
            .bg(theme.canvas)
            .overflow_hidden()
            .flex()
            .flex_col()
            .child(
                div()
                    .id("download-settings-category-master-scroll")
                    .debug_selector(|| "download-settings-category-master-scroll".into())
                    .w_full()
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scrollbar()
                    .child(category_rows),
            )
            .child(
                div()
                    .debug_selector(|| "download-settings-category-add".into())
                    .w_full()
                    .p_2()
                    .flex_none()
                    .border_t_1()
                    .border_color(theme.border_subtle)
                    .child(
                        action_button("download-settings-category-add-button", "+ 新增")
                            .ghost()
                            .w_full()
                            .on_click(cx.listener(|this, _, window, cx| {
                                let mut suffix = this.categories.len() + 1;
                                let id = loop {
                                    let candidate = format!("custom-{suffix}");
                                    if this
                                        .categories
                                        .iter()
                                        .all(|category| category.id != candidate)
                                    {
                                        break candidate;
                                    }
                                    suffix += 1;
                                };
                                let editor = DownloadCategoryEditor::new(
                                    DownloadCategorySettings {
                                        id,
                                        name: "新分类".into(),
                                        enabled: true,
                                        save_directory: "NewCategory".into(),
                                        extensions: Vec::new(),
                                        url_patterns: Vec::new(),
                                    },
                                    window,
                                    cx,
                                );
                                let name_subscription = Self::subscribe_category_presentation_input(
                                    &editor.name,
                                    window,
                                    cx,
                                );
                                let directory_subscription =
                                    Self::subscribe_category_presentation_input(
                                        &editor.save_directory,
                                        window,
                                        cx,
                                    );
                                this._subscriptions
                                    .extend([name_subscription, directory_subscription]);
                                this.categories.push(editor);
                                this.selected_category = Some(this.categories.len() - 1);
                                this.error = None;
                                cx.notify();
                            })),
                    ),
            )
            .into_any_element();
        let category_editor = self
            .selected_category
            .and_then(|index| self.categories.get(index).map(|category| (index, category)))
            .map(|(index, category)| {
                let enabled = category.enabled;
                let name = category.name.clone();
                let directory = category.save_directory.clone();
                let extensions = category.extensions.clone();
                let url_patterns = category.url_patterns.clone();
                let enabled_control = settings_toggle(
                    theme,
                    "download-settings-category-enabled-button",
                    enabled,
                    cx.listener(move |this, _, _, cx| {
                        if let Some(category) = this.categories.get_mut(index) {
                            category.enabled = !category.enabled;
                        }
                        cx.stop_propagation();
                        cx.notify();
                    }),
                );
                div()
                    .w_full()
                    .min_w_0()
                    .h_full()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .debug_selector(|| "download-settings-category-heading".into())
                            .w_full()
                            .min_h(ordinary_rem(CONTROL_HEIGHT_INPUT))
                            .flex_none()
                            .border_b_1()
                            .border_color(theme.border_subtle)
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .min_w_0()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        div()
                                            .debug_selector(|| {
                                                "download-settings-category-heading-title".into()
                                            })
                                            .flex_none()
                                            .text_size(ordinary_rem(TEXT_BODY))
                                            .line_height(ordinary_rem(LINE_HEIGHT_BODY))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(theme.text)
                                            .child("分类规则"),
                                    )
                                    .child(
                                        div()
                                            .min_w_0()
                                            .truncate()
                                            .text_size(ordinary_rem(TEXT_LABEL))
                                            .line_height(ordinary_rem(LINE_HEIGHT_LABEL))
                                            .text_color(theme.text_muted)
                                            .child(if enabled {
                                                "参与自动匹配"
                                            } else {
                                                "已停用，不参与自动匹配"
                                            }),
                                    ),
                            )
                            .child(
                                div()
                                    .flex_none()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        div()
                                            .text_size(ordinary_rem(TEXT_LABEL))
                                            .line_height(ordinary_rem(LINE_HEIGHT_LABEL))
                                            .text_color(theme.text_secondary)
                                            .child("启用"),
                                    )
                                    .child(enabled_control),
                            ),
                    )
                    .child(
                        div()
                            .debug_selector(|| "download-settings-category-form".into())
                            .w_full()
                            .child(
                                Form::new()
                                    .columns(2)
                                    .child(download_draft_field(
                                        "分类名称",
                                        "",
                                        "download-settings-category-name-field",
                                        &name,
                                    ))
                                    .child(download_draft_field(
                                        "保存目录",
                                        "留空使用默认目录；相对路径位于默认目录下",
                                        "download-settings-category-directory-field",
                                        &directory,
                                    ))
                                    .child(download_draft_field(
                                        "扩展名规则",
                                        "逗号分隔，例如 zip, 7z, rar",
                                        "download-settings-category-extensions-field",
                                        &extensions,
                                    ))
                                    .child(download_draft_field(
                                        "URL 通配规则",
                                        "逗号分隔；与扩展名规则同时存在时均须命中",
                                        "download-settings-category-url-field",
                                        &url_patterns,
                                    )),
                            ),
                    )
                    .child(div().flex_1())
                    .child(
                        div()
                            .debug_selector(|| "download-settings-category-action-toolbar".into())
                            .w_full()
                            .pt_2()
                            .flex_none()
                            .border_t_1()
                            .border_color(theme.border_subtle)
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .debug_selector(|| {
                                        "download-settings-category-order-group".into()
                                    })
                                    .flex()
                                    .items_center()
                                    .gap_1()
                                    .child(
                                        div()
                                            .debug_selector(|| {
                                                "download-settings-category-up-control".into()
                                            })
                                            .child(
                                                action_button(
                                                    "download-settings-category-up",
                                                    "上移",
                                                )
                                                .outline()
                                                .disabled(index == 0)
                                                .on_click(cx.listener(move |this, _, _, cx| {
                                                    #[cfg(test)]
                                                    {
                                                        this.category_action_invocations = this
                                                            .category_action_invocations
                                                            .wrapping_add(1);
                                                    }
                                                    if index > 0 && index < this.categories.len() {
                                                        this.categories.swap(index, index - 1);
                                                        this.selected_category = Some(index - 1);
                                                        cx.notify();
                                                    }
                                                })),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .debug_selector(|| {
                                                "download-settings-category-down-control".into()
                                            })
                                            .child(
                                                action_button(
                                                    "download-settings-category-down",
                                                    "下移",
                                                )
                                                .outline()
                                                .disabled(index + 1 >= self.categories.len())
                                                .on_click(cx.listener(move |this, _, _, cx| {
                                                    #[cfg(test)]
                                                    {
                                                        this.category_action_invocations = this
                                                            .category_action_invocations
                                                            .wrapping_add(1);
                                                    }
                                                    if index + 1 < this.categories.len() {
                                                        this.categories.swap(index, index + 1);
                                                        this.selected_category = Some(index + 1);
                                                        cx.notify();
                                                    }
                                                })),
                                            ),
                                    ),
                            )
                            .child(
                                div()
                                    .debug_selector(|| {
                                        "download-settings-category-delete-control".into()
                                    })
                                    .child(
                                        action_button(
                                            "download-settings-category-delete",
                                            "删除分类",
                                        )
                                        .custom(danger_hover_button_variant(theme, cx))
                                        .on_click(
                                            cx.listener(move |this, _, _, cx| {
                                                if index < this.categories.len() {
                                                    this.categories.remove(index);
                                                    this.selected_category =
                                                        (!this.categories.is_empty()).then_some(
                                                            index.min(this.categories.len() - 1),
                                                        );
                                                    cx.notify();
                                                }
                                            }),
                                        ),
                                    ),
                            ),
                    )
                    .into_any_element()
            })
            .unwrap_or_else(|| {
                div()
                    .text_size(ordinary_rem(TEXT_LABEL))
                    .line_height(ordinary_rem(LINE_HEIGHT_LABEL))
                    .text_color(theme.text_muted)
                    .child("暂无分类；可新增自定义规则")
                    .into_any_element()
            });
        let category_management = div()
            .w_full()
            .h(px(310.0))
            .flex()
            .items_start()
            .gap_3()
            .child(category_list)
            .child(
                div()
                    .min_w_0()
                    .flex_1()
                    .h_full()
                    .id("download-settings-category-editor-scroll")
                    .min_h_0()
                    .overflow_y_scroll()
                    .pl_3()
                    .border_l_1()
                    .border_color(theme.border_subtle)
                    .child(category_editor),
            )
            .into_any_element();
        let current_ffmpeg = self.ffmpeg_executable.read(cx).value().trim().to_owned();
        let ffmpeg_description = match self.ffmpeg_discovery_state {
            FfmpegDiscoveryState::Checking => "正在后台检查用户路径、FFMPEG_PATH 与系统 PATH",
            FfmpegDiscoveryState::Found(nxtools_download::FfmpegResolutionSource::Configured) => {
                "已发现用户指定文件；双轨下载时会运行 -version 验证"
            }
            FfmpegDiscoveryState::Found(nxtools_download::FfmpegResolutionSource::Environment) => {
                "已从 FFMPEG_PATH 发现候选文件；双轨下载时会验证版本"
            }
            FfmpegDiscoveryState::Found(nxtools_download::FfmpegResolutionSource::Path) => {
                "已从系统 PATH 发现候选文件；双轨下载时会验证版本"
            }
            FfmpegDiscoveryState::Missing => {
                "未发现 FFmpeg；可自动下载到用户目录，也可手动指定或使用环境变量"
            }
        };
        let application = self.application.clone();
        let ffmpeg_picker_path = current_ffmpeg.clone();
        let ffmpeg_control = div()
            .debug_selector(|| "download-settings-ffmpeg-control".into())
            .w_full()
            .min_w_0()
            .flex()
            .items_center()
            .gap_2()
            .child(div().flex_1().min_w_0().child(field_shell(
                theme,
                "download-settings-ffmpeg-field",
                Input::new(&self.ffmpeg_executable),
            )))
            .child(
                div()
                    .debug_selector(|| "download-settings-pick-ffmpeg".into())
                    .child(icon_action_tooltip_surface(
                        icon_action_button(
                            "download-settings-pick-ffmpeg-button",
                            MaterialIcon::Folder,
                            "选择 FFmpeg 可执行文件…",
                        )
                        .outline()
                        .on_click(move |_, _, cx| {
                            let application = application.clone();
                            let current_path = ffmpeg_picker_path.clone();
                            cx.defer(move |cx| {
                                application.update(cx, |application, cx| {
                                    application.request_ffmpeg_executable(current_path, cx);
                                });
                            });
                        }),
                        "选择 FFmpeg 可执行文件…",
                        "download-settings-pick-ffmpeg-tooltip",
                    )),
            )
            .into_any_element();
        let install_active = self.ffmpeg_install_state.is_active();
        let install_description = match &self.ffmpeg_install_state {
            FfmpegInstallViewState::Idle => {
                "从官方 Gyan.dev 下载，失败时切换 BtbN；校验后自动配置".to_owned()
            }
            FfmpegInstallViewState::Installing {
                phase,
                percent,
                downloaded_bytes,
                total_bytes,
                cancelling,
            } => {
                let phase = if *cancelling {
                    "正在取消"
                } else {
                    ffmpeg_install_phase_label(*phase)
                };
                match total_bytes {
                    Some(total) => format!(
                        "{phase} · {percent}% · {} / {}",
                        format_download_size(*downloaded_bytes),
                        format_download_size(*total)
                    ),
                    None => format!(
                        "{phase} · 已接收 {}",
                        format_download_size(*downloaded_bytes)
                    ),
                }
            }
            FfmpegInstallViewState::Completed => {
                "已安装并自动配置，可直接用于 DASH 合并".to_owned()
            }
            FfmpegInstallViewState::Cancelled => {
                "自动下载已取消；未更改当前 FFmpeg 配置".to_owned()
            }
            FfmpegInstallViewState::Failed(message) => message.clone(),
        };
        // Only the download phase has a measured denominator. Later phase
        // percentages are engine milestones, not measured completion fractions.
        let install_percent = match &self.ffmpeg_install_state {
            FfmpegInstallViewState::Installing {
                phase: FfmpegInstallPhase::Downloading,
                percent,
                total_bytes: Some(total),
                ..
            } if *total > 0 && *percent <= 100 => Some(*percent),
            _ => None,
        };
        let install_application = self.application.clone();
        let cancel_application = self.application.clone();
        let ffmpeg_install_control = div()
            .debug_selector(|| "download-settings-ffmpeg-install-actions".into())
            .w_full()
            .flex()
            .items_center()
            .gap_2()
            .child(
                div()
                    .debug_selector(|| "download-settings-ffmpeg-install".into())
                    .child(
                        crate::ui::components::action::primary_action_button(
                            "download-settings-ffmpeg-install-button",
                            "自动下载 FFmpeg",
                            if install_active {
                                "正在下载…"
                            } else {
                                "自动下载"
                            },
                            theme,
                            install_active,
                            cx,
                        )
                        .on_click(move |_, _, cx| {
                            let application = install_application.clone();
                            cx.defer(move |cx| {
                                application.update(cx, |application, cx| {
                                    application.start_ffmpeg_install(cx);
                                });
                            });
                        }),
                    ),
            )
            .when(install_active, |control| {
                control.child(
                    div()
                        .debug_selector(|| "download-settings-ffmpeg-cancel".into())
                        .child(
                            action_button("download-settings-ffmpeg-cancel-button", "取消")
                                .ghost()
                                .on_click(move |_, _, cx| {
                                    let application = cancel_application.clone();
                                    cx.defer(move |cx| {
                                        application.update(cx, |application, cx| {
                                            application.cancel_ffmpeg_install(cx);
                                        });
                                    });
                                }),
                        ),
                )
            })
            .into_any_element();
        let gyan_application = self.application.clone();
        let btbn_application = self.application.clone();
        let ffmpeg_source_control = div()
            .debug_selector(|| "download-settings-ffmpeg-source-actions".into())
            .w_full()
            .flex()
            .items_center()
            .gap_2()
            .child(
                div()
                    .debug_selector(|| "download-settings-ffmpeg-gyan".into())
                    .child(
                        action_button("download-settings-ffmpeg-gyan-button", "Gyan.dev")
                            .outline()
                            .on_click(move |_, _, cx| {
                                let application = gyan_application.clone();
                                cx.defer(move |cx| {
                                    application.update(cx, |application, cx| {
                                        application.open_ffmpeg_download_page(
                                            FfmpegDownloadSource::Gyan,
                                            cx,
                                        );
                                    });
                                });
                            }),
                    ),
            )
            .child(
                div()
                    .debug_selector(|| "download-settings-ffmpeg-btbn".into())
                    .child(
                        action_button("download-settings-ffmpeg-btbn-button", "BtbN GitHub")
                            .outline()
                            .on_click(move |_, _, cx| {
                                let application = btbn_application.clone();
                                cx.defer(move |cx| {
                                    application.update(cx, |application, cx| {
                                        application.open_ffmpeg_download_page(
                                            FfmpegDownloadSource::Btbn,
                                            cx,
                                        );
                                    });
                                });
                            }),
                    ),
            )
            .into_any_element();
        let connection_control = download_settings_input(
            theme,
            "download-settings-connection-limit-field",
            &self.connection_limit,
            ordinary_rem(88.0),
        )
        .into_any_element();
        let adaptive_max_control = download_settings_input(
            theme,
            "download-settings-adaptive-max-field",
            &self.adaptive_segment_max_segments,
            ordinary_rem(88.0),
        )
        .into_any_element();
        let adaptive_min_control = download_settings_input(
            theme,
            "download-settings-adaptive-min-field",
            &self.adaptive_segment_min_mib,
            ordinary_rem(88.0),
        )
        .into_any_element();
        let adaptive_toggle = settings_toggle(
            theme,
            "download-settings-adaptive-segments",
            self.adaptive_segment_scaling_enabled,
            cx.listener(|this, _, _, cx| {
                this.adaptive_segment_scaling_enabled = !this.adaptive_segment_scaling_enabled;
                this.error = None;
                cx.notify();
            }),
        );
        let concurrent_control = download_settings_input(
            theme,
            "download-settings-concurrent-tasks-field",
            &self.concurrent_tasks,
            ordinary_rem(88.0),
        )
        .into_any_element();
        let speed_control = download_settings_input(
            theme,
            "download-settings-speed-limit-field",
            &self.speed_limit_kib,
            ordinary_rem(88.0),
        )
        .into_any_element();
        let progress_toggle = settings_toggle(
            theme,
            "download-settings-progress-window",
            self.show_progress_window,
            cx.listener(|this, _, _, cx| {
                this.show_progress_window = !this.show_progress_window;
                this.error = None;
                cx.notify();
            }),
        );
        let completion_toggle = settings_toggle(
            theme,
            "download-settings-completion-window",
            self.show_completion_window,
            cx.listener(|this, _, _, cx| {
                this.show_completion_window = !this.show_completion_window;
                this.error = None;
                cx.notify();
            }),
        );
        let completion_sound_toggle = settings_toggle(
            theme,
            "download-settings-completion-sound",
            self.completion_sound_enabled,
            cx.listener(|this, _, _, cx| {
                this.completion_sound_enabled = !this.completion_sound_enabled;
                this.error = None;
                cx.notify();
            }),
        );
        let completion_directory_toggle = settings_toggle(
            theme,
            "download-settings-completion-directory",
            self.open_directory_on_completion,
            cx.listener(|this, _, _, cx| {
                this.open_directory_on_completion = !this.open_directory_on_completion;
                this.error = None;
                cx.notify();
            }),
        );
        let task_removal_toggle = settings_toggle(
            theme,
            "download-settings-task-removal-delete-file",
            self.task_removal_deletes_file,
            cx.listener(|this, _, _, cx| {
                this.task_removal_deletes_file = !this.task_removal_deletes_file;
                this.error = None;
                cx.notify();
            }),
        );
        let taskbar_overlay_toggle = settings_toggle(
            theme,
            "download-settings-taskbar-overlay",
            self.taskbar_overlay_enabled,
            cx.listener(|this, _, _, cx| {
                this.taskbar_overlay_enabled = !this.taskbar_overlay_enabled;
                this.error = None;
                cx.notify();
            }),
        );
        let taskbar_anchor = download_settings_anchor(
            theme,
            self.taskbar_overlay_anchor,
            cx.listener(|this, _, _, cx| {
                this.taskbar_overlay_anchor = DownloadTaskbarOverlayAnchor::Left;
                this.error = None;
                this.sync_taskbar_overlay_edit(cx);
                cx.notify();
            }),
            cx.listener(|this, _, _, cx| {
                this.taskbar_overlay_anchor = DownloadTaskbarOverlayAnchor::Right;
                this.error = None;
                this.sync_taskbar_overlay_edit(cx);
                cx.notify();
            }),
            cx,
        );
        let taskbar_offset_x = download_settings_input(
            theme,
            "download-settings-taskbar-x-field",
            &self.taskbar_overlay_offset_x,
            ordinary_rem(88.0),
        )
        .into_any_element();
        let taskbar_offset_y = download_settings_input(
            theme,
            "download-settings-taskbar-y-field",
            &self.taskbar_overlay_offset_y,
            ordinary_rem(88.0),
        )
        .into_any_element();
        let taskbar_width = download_settings_input(
            theme,
            "download-settings-taskbar-width-field",
            &self.taskbar_overlay_width,
            ordinary_rem(88.0),
        )
        .into_any_element();
        let taskbar_height = download_settings_input(
            theme,
            "download-settings-taskbar-height-field",
            &self.taskbar_overlay_height,
            ordinary_rem(88.0),
        )
        .into_any_element();
        let taskbar_progress_mode = download_settings_progress_mode(
            theme,
            self.taskbar_overlay_progress_mode,
            cx.listener(|this, _, _, cx| {
                this.taskbar_overlay_progress_mode = DownloadTaskbarProgressMode::Aggregate;
                this.error = None;
                this.sync_taskbar_overlay_edit(cx);
                cx.notify();
            }),
            cx.listener(|this, _, _, cx| {
                this.taskbar_overlay_progress_mode = DownloadTaskbarProgressMode::PerTask;
                this.error = None;
                this.sync_taskbar_overlay_edit(cx);
                cx.notify();
            }),
            cx.listener(|this, _, _, cx| {
                this.taskbar_overlay_progress_mode = DownloadTaskbarProgressMode::PerSegment;
                this.error = None;
                this.sync_taskbar_overlay_edit(cx);
                cx.notify();
            }),
            cx,
        );
        let taskbar_progress_info = settings_toggle(
            theme,
            "download-settings-taskbar-show-progress",
            self.taskbar_overlay_show_progress,
            cx.listener(|this, _, _, cx| {
                this.taskbar_overlay_show_progress = !this.taskbar_overlay_show_progress;
                this.error = None;
                this.sync_taskbar_overlay_edit(cx);
                cx.notify();
            }),
        );
        let taskbar_speed_info = settings_toggle(
            theme,
            "download-settings-taskbar-show-speed",
            self.taskbar_overlay_show_speed,
            cx.listener(|this, _, _, cx| {
                this.taskbar_overlay_show_speed = !this.taskbar_overlay_show_speed;
                this.error = None;
                this.sync_taskbar_overlay_edit(cx);
                cx.notify();
            }),
        );
        let taskbar_remaining_info = settings_toggle(
            theme,
            "download-settings-taskbar-show-remaining",
            self.taskbar_overlay_show_remaining_time,
            cx.listener(|this, _, _, cx| {
                this.taskbar_overlay_show_remaining_time =
                    !this.taskbar_overlay_show_remaining_time;
                this.error = None;
                this.sync_taskbar_overlay_edit(cx);
                cx.notify();
            }),
        );
        let taskbar_task_count_info = settings_toggle(
            theme,
            "download-settings-taskbar-show-tasks",
            self.taskbar_overlay_show_active_tasks,
            cx.listener(|this, _, _, cx| {
                this.taskbar_overlay_show_active_tasks = !this.taskbar_overlay_show_active_tasks;
                this.error = None;
                this.sync_taskbar_overlay_edit(cx);
                cx.notify();
            }),
        );
        let taskbar_connection_info = settings_toggle(
            theme,
            "download-settings-taskbar-show-connections",
            self.taskbar_overlay_show_active_connections,
            cx.listener(|this, _, _, cx| {
                this.taskbar_overlay_show_active_connections =
                    !this.taskbar_overlay_show_active_connections;
                this.error = None;
                this.sync_taskbar_overlay_edit(cx);
                cx.notify();
            }),
        );
        let taskbar_bytes_info = settings_toggle(
            theme,
            "download-settings-taskbar-show-bytes",
            self.taskbar_overlay_show_transferred_bytes,
            cx.listener(|this, _, _, cx| {
                this.taskbar_overlay_show_transferred_bytes =
                    !this.taskbar_overlay_show_transferred_bytes;
                this.error = None;
                this.sync_taskbar_overlay_edit(cx);
                cx.notify();
            }),
        );
        let taskbar_edit_control = div()
            .debug_selector(|| "download-settings-taskbar-edit".into())
            .child(
                crate::ui::components::action::primary_action_button(
                    "download-settings-taskbar-edit-button",
                    if self.taskbar_overlay_editing {
                        "结束任务栏布局预览"
                    } else {
                        "编辑任务栏布局"
                    },
                    if self.taskbar_overlay_editing {
                        "结束预览"
                    } else {
                        "编辑布局"
                    },
                    theme,
                    false,
                    cx,
                )
                .icon(Icon::new(if self.taskbar_overlay_editing {
                    MaterialIcon::Check
                } else {
                    MaterialIcon::Edit
                }))
                .on_click(cx.listener(|this, _, _, cx| {
                    this.toggle_taskbar_overlay_edit(cx);
                })),
            )
            .into_any_element();
        match (page, slot) {
            (DownloadSettingsPage::General, 0) => directory_control,
            (DownloadSettingsPage::General, 1) => automatic_categories_toggle,
            (DownloadSettingsPage::General, 2) => category_management,
            (DownloadSettingsPage::General, 3) => duplicate_strategy,
            (DownloadSettingsPage::General, 4) => double_click_action,
            (DownloadSettingsPage::General, 5) => div()
                .flex()
                .flex_col()
                .gap_2()
                .child(
                    div()
                        .flex()
                        .flex_wrap()
                        .gap_2()
                        .child(
                            action_button(
                                "download-settings-extension-export-chrome-button",
                                "导出 Chrome / Edge (.crx)…",
                            )
                            .on_click(cx.listener(|this, _, _, cx| {
                                let operation_id = uuid::Uuid::new_v4();
                                this.notify_operation(
                                    format!("download-browser-export-{operation_id}"),
                                    crate::notifications::NotificationLevel::Progress,
                                    "正在导出 Chrome / Edge 扩展…".into(),
                                    cx,
                                );
                                let application = this.application.clone();
                                let view = cx.entity().downgrade();
                                cx.defer(move |cx| {
                                    let outcome = application.update(cx, |application, _| {
                                        application.request_browser_extension_export(
                                            "chrome",
                                            operation_id,
                                        )
                                    });
                                    if let Err(error) = outcome {
                                        let _ = view.update(cx, |view, cx| {
                                            view.notify_operation(
                                                format!("download-browser-export-{operation_id}"),
                                                crate::notifications::NotificationLevel::Error,
                                                format!("导出失败:{error}"),
                                                cx,
                                            );
                                            cx.notify();
                                        });
                                    }
                                });
                                cx.notify();
                            })),
                        )
                        .child(
                            action_button(
                                "download-settings-extension-export-firefox-button",
                                "导出 Firefox (.xpi)…",
                            )
                            .on_click(cx.listener(|this, _, _, cx| {
                                let operation_id = uuid::Uuid::new_v4();
                                this.notify_operation(
                                    format!("download-browser-export-{operation_id}"),
                                    crate::notifications::NotificationLevel::Progress,
                                    "正在导出 Firefox 扩展…".into(),
                                    cx,
                                );
                                let application = this.application.clone();
                                let view = cx.entity().downgrade();
                                cx.defer(move |cx| {
                                    let outcome = application.update(cx, |application, _| {
                                        application.request_browser_extension_export(
                                            "firefox",
                                            operation_id,
                                        )
                                    });
                                    if let Err(error) = outcome {
                                        let _ = view.update(cx, |view, cx| {
                                            view.notify_operation(
                                                format!("download-browser-export-{operation_id}"),
                                                crate::notifications::NotificationLevel::Error,
                                                format!("导出失败:{error}"),
                                                cx,
                                            );
                                            cx.notify();
                                        });
                                    }
                                });
                                cx.notify();
                            })),
                        )
                        .child(
                            action_button(
                                "download-settings-extension-export-store-button",
                                "导出 Chrome 商店 (.zip)…",
                            )
                            .outline()
                            .on_click(cx.listener(|this, _, _, cx| {
                                let operation_id = uuid::Uuid::new_v4();
                                this.notify_operation(
                                    format!("download-browser-export-{operation_id}"),
                                    crate::notifications::NotificationLevel::Progress,
                                    "正在导出 Chrome 商店上传包…".into(),
                                    cx,
                                );
                                let application = this.application.clone();
                                let view = cx.entity().downgrade();
                                cx.defer(move |cx| {
                                    let outcome = application.update(cx, |application, _| {
                                        application.request_browser_extension_export(
                                            "chrome_store",
                                            operation_id,
                                        )
                                    });
                                    if let Err(error) = outcome {
                                        let _ = view.update(cx, |view, cx| {
                                            view.notify_operation(
                                                format!("download-browser-export-{operation_id}"),
                                                crate::notifications::NotificationLevel::Error,
                                                format!("导出失败:{error}"),
                                                cx,
                                            );
                                        });
                                    }
                                });
                            })),
                        ),
                )
                .into_any_element(),
            (DownloadSettingsPage::General, 6) => download_settings_input(
                theme,
                "download-settings-chrome-store-id",
                &self.chrome_web_store_extension_id,
                ordinary_rem(286.0),
            )
            .into_any_element(),
            (DownloadSettingsPage::General, 7) => {
                action_button("download-settings-firefox-store", "打开 Firefox 扩展商店")
                    .small()
                    .outline()
                    .on_click(cx.listener(|this, _, _, cx| {
                        let application = this.application.clone();
                        cx.defer(move |cx| {
                            application.update(cx, |owner, cx| owner.open_firefox_store(cx))
                        });
                    }))
                    .into_any_element()
            }
            (DownloadSettingsPage::Request, 0) => div()
                .debug_selector(|| "download-settings-request-context-form".into())
                .w_full()
                .min_w_0()
                .child(
                    Form::new()
                        .child(download_form_field(
                            "默认 Header",
                            "每行 Name: Value，也可写 -H 'Name: Value'；任务或浏览器捕获值优先",
                            "download-settings-default-headers-field",
                            default_headers_control,
                        ))
                        .child(download_form_field(
                            "默认 Cookie",
                            "仅在任务没有 Cookie 时带入；内容不会写入诊断日志",
                            "download-settings-default-cookie-field",
                            default_cookie_control,
                        )),
                )
                .into_any_element(),
            (DownloadSettingsPage::Request, 1) => certificate_toggle,
            (DownloadSettingsPage::Transfer, 0) => connection_control,
            (DownloadSettingsPage::Transfer, 1) => concurrent_control,
            (DownloadSettingsPage::Transfer, 2) => adaptive_toggle,
            (DownloadSettingsPage::Transfer, 3) => adaptive_max_control,
            (DownloadSettingsPage::Transfer, 4) => adaptive_min_control,
            (DownloadSettingsPage::Transfer, 5) => speed_control,
            (DownloadSettingsPage::Transfer, 6) => div()
                .w_full()
                .flex()
                .flex_col()
                .gap_2()
                .child(
                    div()
                        .debug_selector(|| "download-settings-ffmpeg-discovery-status".into())
                        .text_sm()
                        .child(ffmpeg_description),
                )
                .child(ffmpeg_control)
                .into_any_element(),
            (DownloadSettingsPage::Transfer, 7) => div()
                .w_full()
                .flex()
                .flex_col()
                .gap_2()
                .child(
                    div()
                        .debug_selector(|| "download-settings-ffmpeg-install-status".into())
                        .text_sm()
                        .child(install_description),
                )
                .when_some(install_percent, |control, percent| {
                    control.child(
                        div()
                            .debug_selector(|| "download-settings-ffmpeg-progress".into())
                            .w_full()
                            .flex_none()
                            .child(
                                Progress::new("download-settings-ffmpeg-install-progress")
                                    .xsmall()
                                    .value(f32::from(percent))
                                    .accessibility_label("FFmpeg 下载进度"),
                            ),
                    )
                })
                .child(ffmpeg_install_control)
                .into_any_element(),
            (DownloadSettingsPage::Transfer, 9) => settings_toggle(
                theme,
                "download-settings-background-limit",
                self.background_speed_limit_enabled,
                cx.listener(|this, _, _, cx| {
                    if !this.saving {
                        this.background_speed_limit_enabled = !this.background_speed_limit_enabled;
                        this.error = None;
                        cx.notify();
                    }
                }),
            ),
            (DownloadSettingsPage::Transfer, 10) => div()
                .debug_selector(|| "download-settings-background-limit-field".into())
                .w(ordinary_rem(120.0))
                .max_w_full()
                .child(
                    Input::new(&self.background_speed_limit_kib)
                        .aria_label("后台限速 KiB/s")
                        .disabled(self.saving || !self.background_speed_limit_enabled),
                )
                .into_any_element(),
            (DownloadSettingsPage::Transfer, 8) => ffmpeg_source_control,
            (DownloadSettingsPage::Window, 0) => progress_toggle,
            (DownloadSettingsPage::Window, 1) => completion_toggle,
            (DownloadSettingsPage::Window, 2) => completion_sound_toggle,
            (DownloadSettingsPage::Window, 3) => completion_directory_toggle,
            (DownloadSettingsPage::Window, 4) => task_removal_toggle,
            (DownloadSettingsPage::Window, 5) => taskbar_overlay_toggle,
            (DownloadSettingsPage::Window, 6) => taskbar_edit_control,
            (DownloadSettingsPage::Window, 7) => taskbar_anchor,
            (DownloadSettingsPage::Window, 8) => taskbar_progress_mode,
            (DownloadSettingsPage::Window, 9) => taskbar_offset_x,
            (DownloadSettingsPage::Window, 10) => taskbar_offset_y,
            (DownloadSettingsPage::Window, 11) => taskbar_width,
            (DownloadSettingsPage::Window, 12) => taskbar_height,
            (DownloadSettingsPage::Window, 13) => taskbar_progress_info,
            (DownloadSettingsPage::Window, 14) => taskbar_speed_info,
            (DownloadSettingsPage::Window, 15) => taskbar_remaining_info,
            (DownloadSettingsPage::Window, 16) => taskbar_bytes_info,
            (DownloadSettingsPage::Window, 17) => taskbar_task_count_info,
            (DownloadSettingsPage::Window, 18) => taskbar_connection_info,
            (DownloadSettingsPage::Diagnostics, slot) => {
                let (label, path, kind) = match slot {
                    0 => (
                        "配置",
                        self.config_path.clone(),
                        DownloadSettingsPathKind::Config,
                    ),
                    1 => (
                        "日志",
                        self.log_directory.clone(),
                        DownloadSettingsPathKind::Logs,
                    ),
                    _ => (
                        "数据库",
                        self.database_path.clone(),
                        DownloadSettingsPathKind::Database,
                    ),
                };
                download_settings_path_row(theme, label, path, kind, self.application.clone())
            }
            _ => div().into_any_element(),
        }
    }
}

impl Render for DownloadSettingsView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        crate::theme::observe_theme(window, cx);
        self.theme = Theme::active(cx);
        let theme = self.theme;
        let owner = cx.entity().downgrade();
        let item = |page: DownloadSettingsPage,
                    slot: usize,
                    title: &'static str,
                    help: &'static str,
                    wide: bool| {
            let owner = owner.clone();
            let content = move |_: &gpui_kit::component::setting::RenderOptions,
                                _: &mut Window,
                                cx: &mut App| {
                let control = owner
                    .update(cx, |view, cx| view.render_setting_control(page, slot, cx))
                    .unwrap_or_else(|_| div().into_any_element());
                div()
                    .id(gpui_kit::SharedString::from(format!(
                        "download-settings-{}-{slot}",
                        page.label()
                    )))
                    .test_support()
                    .role(gpui_kit::Role::Group)
                    .aria_label(page.label())
                    .w_full()
                    .min_w_0()
                    .when(!wide, |content| content.flex().items_center().justify_end())
                    .child(control)
            };
            if wide {
                SettingItem::render(move |options, window, cx| {
                    div()
                        .w_full()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(div().flex().items_center().gap_1().child(title).when(
                            !help.is_empty(),
                            |label| {
                                label.child(crate::ui::components::tooltip::description_help(
                                    format!("download-settings-{title}-help"),
                                    help,
                                ))
                            },
                        ))
                        .child(content(options, window, cx))
                })
                .keywords([title, help])
            } else {
                SettingItem::render(move |options, window, cx| {
                    crate::ui::components::settings::aligned_setting_row(
                        format!("download-setting-{title}"),
                        title,
                        help,
                        format!("download-settings-{title}-help"),
                        ordinary_rem(440.0),
                        content(options, window, cx).into_any_element(),
                    )
                })
                .keywords([title, help])
            }
        };
        let mut pages = Vec::new();
        for page in [
            DownloadSettingsPage::General,
            DownloadSettingsPage::Request,
            DownloadSettingsPage::Transfer,
            DownloadSettingsPage::Window,
            DownloadSettingsPage::Diagnostics,
        ] {
            let groups = download_setting_groups(page)
                .into_iter()
                .map(|(title, fields)| {
                    SettingGroup::new().title(title).items(
                        fields
                            .into_iter()
                            .map(|(slot, label, help, wide)| item(page, slot, label, help, wide)),
                    )
                });
            pages.push(
                SettingPage::new(page.label())
                    .resettable(false)
                    .groups(groups),
            );
        }
        let content = div()
            .size_full()
            .min_h_0()
            .min_w_0()
            .flex()
            .flex_col()
            .child(
                div().flex_1().min_h_0().w_full().child(
                    Settings::new("download-settings-kit")
                        .sidebar_width(ordinary_rem(164.0).to_pixels(window.rem_size()))
                        .default_selected_index(SelectIndex {
                            page_ix: self.page.index(),
                            group_ix: None,
                        })
                        .with_group_variant(GroupBoxVariant::Fill)
                        .pages(pages),
                ),
            )
            .when_some(self.error.clone(), |body, error| {
                body.child(
                    div()
                        .id("download-settings-error-scroll")
                        .debug_selector(|| "download-settings-error-scroll".into())
                        .flex_none()
                        .max_h(px(96.0))
                        .overflow_y_scroll()
                        .child(download_inline_error(theme, error)),
                )
            })
            .into_any_element();
        let footer_actions = div()
            .flex()
            .items_center()
            .gap_2()
            .child(
                div()
                    .debug_selector(|| "download-settings-cancel".into())
                    .child(
                        action_button("download-settings-cancel-button", "取消")
                            .ghost()
                            .disabled(self.saving)
                            .on_click(cx.listener(|this, _, window, cx| {
                                if this.taskbar_overlay_editing {
                                    let application = this.application.clone();
                                    let correlation_id = uuid::Uuid::new_v4();
                                    cx.defer(move |cx| {
                                        application.update(cx, |application, cx| {
                                            application.finish_taskbar_overlay_edit(
                                                false,
                                                correlation_id,
                                                cx,
                                            );
                                        });
                                    });
                                    this.taskbar_overlay_editing = false;
                                }
                                window.remove_window();
                            })),
                    ),
            )
            .child(
                div()
                    .debug_selector(|| "download-settings-apply".into())
                    .child(
                        crate::ui::components::action::primary_action_button(
                            "download-settings-apply-button",
                            "应用下载设置",
                            if self.saving {
                                "正在保存…"
                            } else {
                                "应用"
                            },
                            theme,
                            self.saving,
                            cx,
                        )
                        .icon(Icon::new(MaterialIcon::Check))
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.save(window, cx);
                        })),
                    ),
            )
            .into_any_element();
        settings_shell(
            theme,
            SettingsShellIds {
                root: "download-settings-shell",
                content: "download-settings-shell-content",
                footer: "download-settings-shell-footer",
            },
            download_settings_titlebar(
                theme,
                self.application.clone(),
                self.taskbar_overlay_editing,
            )
            .into_any_element(),
            content,
            dialog_footer(footer_actions),
        )
    }
}

pub(crate) fn download_settings_input(
    theme: Theme,
    selector: &'static str,
    input: &Entity<InputState>,
    width: impl Into<gpui_kit::Length>,
) -> impl IntoElement {
    let width: gpui_kit::Length = width.into();
    div()
        .w(width)
        .max_w_full()
        .min_w_0()
        .child(field_shell(theme, selector, Input::new(input)))
}

pub(crate) fn download_settings_anchor(
    theme: Theme,
    selected: DownloadTaskbarOverlayAnchor,
    on_left: impl Fn(&gpui_kit::ClickEvent, &mut Window, &mut App) + 'static,
    on_right: impl Fn(&gpui_kit::ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> AnyElement {
    let selected_id = match selected {
        DownloadTaskbarOverlayAnchor::Left => "download-settings-taskbar-left",
        DownloadTaskbarOverlayAnchor::Right => "download-settings-taskbar-right",
    };
    segmented_control(
        theme,
        "download-settings-taskbar-anchor",
        selected_id,
        vec![
            SegmentedOption::new("download-settings-taskbar-left", "靠左", on_left),
            SegmentedOption::new("download-settings-taskbar-right", "靠右", on_right),
        ],
        cx,
    )
}

pub(crate) fn download_settings_duplicate_strategy(
    theme: Theme,
    selected: DownloadDuplicateStrategySettings,
    on_numbered: impl Fn(&gpui_kit::ClickEvent, &mut Window, &mut App) + 'static,
    on_reject: impl Fn(&gpui_kit::ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> AnyElement {
    let selected_id = match selected {
        DownloadDuplicateStrategySettings::AddNumbered => "download-settings-duplicate-numbered",
        DownloadDuplicateStrategySettings::Reject => "download-settings-duplicate-reject",
    };
    segmented_control(
        theme,
        "download-settings-duplicate-strategy",
        selected_id,
        vec![
            SegmentedOption::new(
                "download-settings-duplicate-numbered",
                "自动编号",
                on_numbered,
            ),
            SegmentedOption::new("download-settings-duplicate-reject", "拒绝创建", on_reject),
        ],
        cx,
    )
}

pub(crate) fn download_settings_double_click_action(
    theme: Theme,
    selected: DownloadRowDoubleClickActionSettings,
    on_open_file: impl Fn(&gpui_kit::ClickEvent, &mut Window, &mut App) + 'static,
    on_open_directory: impl Fn(&gpui_kit::ClickEvent, &mut Window, &mut App) + 'static,
    on_task_window: impl Fn(&gpui_kit::ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> AnyElement {
    let selected_id = match selected {
        DownloadRowDoubleClickActionSettings::OpenFile => {
            "download-settings-double-click-open-file"
        }
        DownloadRowDoubleClickActionSettings::OpenDirectory => {
            "download-settings-double-click-open-directory"
        }
        DownloadRowDoubleClickActionSettings::OpenTaskWindow => {
            "download-settings-double-click-task-window"
        }
    };
    segmented_control(
        theme,
        "download-settings-double-click-action",
        selected_id,
        vec![
            SegmentedOption::new(
                "download-settings-double-click-open-file",
                "打开文件",
                on_open_file,
            ),
            SegmentedOption::new(
                "download-settings-double-click-open-directory",
                "打开所在目录",
                on_open_directory,
            ),
            SegmentedOption::new(
                "download-settings-double-click-task-window",
                "任务信息",
                on_task_window,
            ),
        ],
        cx,
    )
}

pub(crate) fn download_settings_progress_mode(
    theme: Theme,
    selected: DownloadTaskbarProgressMode,
    on_aggregate: impl Fn(&gpui_kit::ClickEvent, &mut Window, &mut App) + 'static,
    on_task: impl Fn(&gpui_kit::ClickEvent, &mut Window, &mut App) + 'static,
    on_segment: impl Fn(&gpui_kit::ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> AnyElement {
    let selected_id = match selected {
        DownloadTaskbarProgressMode::Aggregate => "download-settings-taskbar-mode-aggregate",
        DownloadTaskbarProgressMode::PerTask => "download-settings-taskbar-mode-task",
        DownloadTaskbarProgressMode::PerSegment => "download-settings-taskbar-mode-segment",
    };
    segmented_control(
        theme,
        "download-settings-taskbar-progress-mode",
        selected_id,
        vec![
            SegmentedOption::new(
                "download-settings-taskbar-mode-aggregate",
                "总进度",
                on_aggregate,
            ),
            SegmentedOption::new("download-settings-taskbar-mode-task", "按任务", on_task),
            SegmentedOption::new(
                "download-settings-taskbar-mode-segment",
                "按分段",
                on_segment,
            ),
        ],
        cx,
    )
}

pub(crate) fn download_settings_path_row(
    theme: Theme,
    label: &'static str,
    path: String,
    path_kind: DownloadSettingsPathKind,
    application: Entity<DownloadApplication>,
) -> AnyElement {
    let presentation = path_kind.presentation();
    let key = presentation.key;
    let value_selector = presentation.value_selector;
    let value_tooltip_selector = presentation.value_tooltip_selector;
    let path_tooltip = path.clone();
    let copy_application = application.clone();
    let copy_action = div()
        .debug_selector(move || format!("download-settings-path-{key}-copy"))
        .size(ordinary_rem(CONTROL_HEIGHT_ACTION))
        .child(icon_action_tooltip_surface(
            icon_action_button(
                presentation.copy_button_id,
                MaterialIcon::ContentCopy,
                presentation.copy_tooltip,
            )
            .outline()
            .on_click(move |_, _, cx| {
                let application = copy_application.clone();
                cx.defer(move |cx| {
                    application.update(cx, |application, cx| {
                        application.perform_download_settings_path_action(
                            path_kind,
                            DownloadSettingsPathAction::Copy,
                            cx,
                        );
                    });
                });
            }),
            presentation.copy_tooltip,
            presentation.copy_tooltip_selector,
        ));
    let open_action = div()
        .debug_selector(move || format!("download-settings-path-{key}-open"))
        .size(ordinary_rem(CONTROL_HEIGHT_ACTION))
        .child(icon_action_tooltip_surface(
            icon_action_button(
                presentation.open_button_id,
                MaterialIcon::OpenInNew,
                presentation.open_tooltip,
            )
            .outline()
            .on_click(move |_, _, cx| {
                let application = application.clone();
                cx.defer(move |cx| {
                    application.update(cx, |application, cx| {
                        application.perform_download_settings_path_action(
                            path_kind,
                            DownloadSettingsPathAction::Open,
                            cx,
                        );
                    });
                });
            }),
            presentation.open_tooltip,
            presentation.open_tooltip_selector,
        ));
    div()
        .debug_selector(move || format!("download-settings-path-{label}"))
        .w_full()
        .min_w_0()
        .min_h(ordinary_rem(SETTINGS_ROW_MIN_HEIGHT))
        .px_2()
        .border_b_1()
        .border_color(theme.border_subtle)
        .flex()
        .items_center()
        .gap_2()
        .child(
            div()
                .w(ordinary_rem(36.0))
                .flex_none()
                .text_size(ordinary_rem(TEXT_LABEL))
                .line_height(ordinary_rem(LINE_HEIGHT_LABEL))
                .text_color(theme.text_muted)
                .child(label),
        )
        .child(
            div()
                .id(value_selector)
                .debug_selector(move || value_selector.into())
                .min_w_0()
                .flex_1()
                .overflow_hidden()
                .whitespace_nowrap()
                .line_clamp(1)
                .font_family(UI_MONO_FONT_FAMILY)
                .text_size(ordinary_rem(TEXT_LABEL))
                .line_height(ordinary_rem(LINE_HEIGHT_LABEL))
                .text_color(theme.text_secondary)
                .tooltip(bounded_text_tooltip(
                    value_tooltip_selector,
                    path_tooltip,
                    true,
                ))
                .child(path),
        )
        .child(
            div()
                .debug_selector(move || format!("download-settings-path-{key}-actions"))
                .flex_none()
                .flex()
                .items_center()
                .gap_1()
                .child(copy_action)
                .child(open_action),
        )
        .into_any_element()
}

impl DownloadSettingsPage {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::General => "常规",
            Self::Request => "请求",
            Self::Transfer => "传输",
            Self::Window => "窗口",
            Self::Diagnostics => "诊断",
        }
    }
    pub(crate) fn index(self) -> usize {
        match self {
            Self::General => 0,
            Self::Request => 1,
            Self::Transfer => 2,
            Self::Window => 3,
            Self::Diagnostics => 4,
        }
    }
}

type DownloadSettingSpec = (usize, &'static str, &'static str, bool);
fn download_setting_groups(
    page: DownloadSettingsPage,
) -> Vec<(&'static str, Vec<DownloadSettingSpec>)> {
    use DownloadSettingsPage::*;
    match page {
        General => vec![
            (
                "保存",
                vec![(
                    0,
                    "默认保存目录",
                    "新建任务自动使用，可在单个任务中覆盖",
                    false,
                )],
            ),
            (
                "分类与重名",
                vec![
                    (
                        1,
                        "自动分类",
                        "仅为仍使用默认保存目录的新任务匹配；手动目录优先",
                        false,
                    ),
                    (
                        2,
                        "分类规则",
                        "选择左侧分类，在右侧编辑名称、独立保存目录和匹配规则",
                        true,
                    ),
                    (
                        3,
                        "目标已存在",
                        "自动编号不会覆盖磁盘文件或活动任务产物；拒绝则不创建任务",
                        false,
                    ),
                    (4, "列表双击", "双击下载列表行时的默认动作", false),
                ],
            ),
            (
                "浏览器扩展",
                vec![
                    (
                        7,
                        "Firefox 商店",
                        "打开已发布的 Firefox 下载拦截扩展页面",
                        false,
                    ),
                    (
                        5,
                        "下载拦截扩展",
                        "本地安装包与 Chrome 商店上传包分别导出",
                        true,
                    ),
                    (
                        6,
                        "Chrome 商店扩展 ID",
                        "可留空；首次上传成功后填写 32 位商店 ID 并保存，保留本地安装版连接",
                        false,
                    ),
                ],
            ),
        ],
        Request => vec![
            (
                "请求上下文",
                vec![(0, "请求元数据", "Header、Cookie 与输入说明", true)],
            ),
            (
                "证书校验",
                vec![(
                    1,
                    "允许无效证书",
                    "默认严格校验证书；仅为确有需要且可信的服务器显式开启",
                    false,
                )],
            ),
        ],
        Transfer => vec![
            (
                "传输",
                vec![
                    (
                        0,
                        "连接上限",
                        "新任务从开始即使用的最大并行连接数，范围 1–64",
                        false,
                    ),
                    (
                        1,
                        "同时下载",
                        "默认队列允许同时运行的任务数，范围 1–64",
                        false,
                    ),
                    (
                        2,
                        "动态分段",
                        "速度稳定时拆分长尾；连接数仍保持在上限内",
                        false,
                    ),
                    (
                        3,
                        "分段上限",
                        "动态拆分后的最大分段数，范围为连接数到 256",
                        false,
                    ),
                    (
                        4,
                        "最小长尾",
                        "剩余大小达到此值才拆分，单位 MiB，范围 1–100",
                        false,
                    ),
                    (5, "全局限速", "单位 KiB/s；填 0 表示不限速", false),
                    (
                        9,
                        "使用其他应用时限速",
                        "前台切到其他应用时临时限速；返回 NxTools 或关闭后恢复手动上限",
                        false,
                    ),
                    (
                        10,
                        "后台速度",
                        "单位 KiB/s；与手动全局和单任务上限取较低值",
                        false,
                    ),
                ],
            ),
            (
                "媒体工具",
                vec![
                    (6, "FFmpeg 可执行文件", "", true),
                    (7, "自动安装", "", true),
                    (
                        8,
                        "公开下载来源",
                        "手动安装备用入口；FFmpeg 不会随 NxTools 安装包携带",
                        true,
                    ),
                ],
            ),
        ],
        Window => vec![
            (
                "任务窗口",
                vec![
                    (0, "自动显示进度", "任务开始后显示独立进度窗口", false),
                    (1, "显示完成窗口", "进度窗已关闭时被动显示完成窗口", false),
                    (2, "完成提示音", "每批完成任务只播放一次系统提示音", false),
                    (
                        3,
                        "完成后打开目录",
                        "每批最多打开三个目录，避免窗口风暴",
                        false,
                    ),
                    (
                        4,
                        "删除任务时删除本地文件",
                        "关闭后，删除文件只从列表移除任务记录",
                        false,
                    ),
                ],
            ),
            (
                "任务栏进度",
                vec![
                    (
                        5,
                        "任务栏内进度",
                        "活动下载时在任务栏上沿显示总体进度",
                        false,
                    ),
                    (6, "布局预览", "显示模拟数据并允许拖动、缩放", false),
                    (7, "锚点", "从任务栏左侧或右侧计算偏移", false),
                    (8, "进度模式", "总进度、按任务或按分段显示", false),
                    (9, "X 偏移", "正值向任务栏内部移动", false),
                    (10, "Y 偏移", "0 表示紧贴任务栏上沿", false),
                    (11, "宽度", "逻辑像素，范围 72–640", false),
                    (12, "高度", "逻辑像素，范围 6–72", false),
                    (13, "进度条", "显示所选模式的进度槽", false),
                    (14, "速度", "显示当前合计下载速度", false),
                    (15, "剩余时间", "按当前合计速度估算", false),
                    (16, "已下载 / 总大小", "显示活动任务的合计字节数", false),
                    (17, "活动任务数", "显示当前正在处理的任务数量", false),
                    (18, "活动连接数", "显示所有活动任务的连接合计", false),
                ],
            ),
        ],
        Diagnostics => vec![(
            "诊断",
            vec![
                (0, "配置", "", true),
                (1, "日志", "", true),
                (2, "数据库", "", true),
            ],
        )],
    }
}

fn download_form_field(
    label: &'static str,
    help: &'static str,
    selector: &'static str,
    control: AnyElement,
) -> Field {
    Field::new()
        .label_fn(move |_, _| {
            div()
                .flex()
                .items_center()
                .gap_1()
                .child(
                    div()
                        .debug_selector(move || format!("{selector}-label"))
                        .child(label),
                )
                .when(!help.is_empty(), |row| {
                    row.child(crate::ui::components::tooltip::description_help(
                        format!("{selector}-help"),
                        help,
                    ))
                })
        })
        .child(control)
}

fn download_draft_field(
    label: &'static str,
    help: &'static str,
    selector: &'static str,
    input: &Entity<InputState>,
) -> Field {
    download_form_field(
        label,
        help,
        selector,
        div()
            .debug_selector(move || selector.into())
            .w_full()
            .min_w_0()
            .child(Input::new(input).aria_label(label))
            .into_any_element(),
    )
}

#[cfg(test)]
mod background_limit_ui_tests {
    use super::*;

    #[gpui_kit::test]
    fn background_limit_setting_keeps_manual_limit_and_validates_only_when_enabled(
        cx: &mut gpui_kit::TestAppContext,
    ) {
        cx.update(gpui_kit::init);
        cx.update(crate::theme::apply_component_theme);
        let (application, directory) =
            super::super::tests::download_application_for_window_test(cx, "background-setting");
        let mut initial = application.read_with(cx, |app, _| app.settings_view_initial());
        initial.draft.global_speed_limit_bytes_per_second = Some(8192);
        let (view, window) = cx.add_window_view(|window, cx| {
            DownloadSettingsView::new(application, initial, window, cx)
        });
        window.simulate_resize(size(px(720.0), px(480.0)));
        window.refresh().expect("actual settings");
        super::super::polish_tests::select_official_download_page(
            window,
            DownloadSettingsPage::Transfer,
        );
        let mut toggle = window
            .debug_bounds("download-settings-background-limit")
            .expect("background Switch");
        for _ in 0..20 {
            if toggle.top() >= px(100.0) && toggle.bottom() <= px(400.0) {
                break;
            }
            window.simulate_event(gpui_kit::ScrollWheelEvent {
                position: gpui_kit::point(px(500.0), px(300.0)),
                delta: gpui_kit::ScrollDelta::Lines(gpui_kit::point(0.0, -2.0)),
                ..Default::default()
            });
            window.run_until_parked();
            window.refresh().expect("scroll to real Switch");
            toggle = window
                .debug_bounds("download-settings-background-limit")
                .expect("background Switch");
        }
        assert!(
            toggle.top() >= px(100.0) && toggle.bottom() <= px(400.0),
            "Switch is reachable: {toggle:?}"
        );
        window.simulate_click(toggle.center(), gpui_kit::Modifiers::default());
        window.run_until_parked();
        view.update_in(window, |view, window, cx| {
            assert!(view.background_speed_limit_enabled);
            view.background_speed_limit_kib
                .update(cx, |input, cx| input.set_value("0", window, cx));
            assert!(view.current_draft(cx).is_err());
            view.background_speed_limit_kib
                .update(cx, |input, cx| input.set_value("256", window, cx));
            let draft = view.current_draft(cx).expect("positive rate");
            assert_eq!(draft.background_speed_limit.bytes_per_second, 256 * 1024);
            assert_eq!(draft.global_speed_limit_bytes_per_second, Some(8192));
            view.background_speed_limit_enabled = false;
            view.background_speed_limit_kib
                .update(cx, |input, cx| input.set_value("invalid", window, cx));
            assert!(
                !view
                    .current_draft(cx)
                    .expect("disabled draft retains saved rate")
                    .background_speed_limit
                    .enabled
            );
        });
        let _ = std::fs::remove_dir_all(directory);
    }
}

#[cfg(test)]
#[path = "settings_task_rem_tests.rs"]
mod settings_task_rem_tests;
