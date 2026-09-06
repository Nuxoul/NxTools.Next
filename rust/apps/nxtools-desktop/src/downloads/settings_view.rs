use super::*;

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
    pub(crate) browser_extension_export_status: Option<String>,
    pub(crate) directory: Entity<InputState>,
    pub(crate) ffmpeg_executable: Entity<InputState>,
    pub(crate) default_request_headers: Entity<InputState>,
    pub(crate) default_cookie: Entity<InputState>,
    pub(crate) ffmpeg_discovery_state: FfmpegDiscoveryState,
    pub(crate) ffmpeg_discovery_generation: u64,
    pub(crate) ffmpeg_install_state: FfmpegInstallViewState,
    pub(crate) connection_limit: Entity<InputState>,
    pub(crate) adaptive_segment_max_segments: Entity<InputState>,
    pub(crate) adaptive_segment_min_mib: Entity<InputState>,
    pub(crate) concurrent_tasks: Entity<InputState>,
    pub(crate) speed_limit_kib: Entity<InputState>,
    pub(crate) taskbar_overlay_offset_x: Entity<InputState>,
    pub(crate) taskbar_overlay_offset_y: Entity<InputState>,
    pub(crate) taskbar_overlay_width: Entity<InputState>,
    pub(crate) taskbar_overlay_height: Entity<InputState>,
    pub(crate) scroll: ScrollHandle,
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
        let ffmpeg_executable = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(snapshot.ffmpeg_executable_path.clone())
                .placeholder("留空则自动检测 FFMPEG_PATH / PATH")
        });
        let default_request_headers = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(snapshot.default_request_headers.clone())
                .placeholder(DEFAULT_REQUEST_HEADERS_PLACEHOLDER)
                .multi_line(true)
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
        let subscription = cx.subscribe_in(
            &application,
            window,
            |this, _application, event: &DownloadApplicationEvent, window, cx| {
                match event {
                    DownloadApplicationEvent::DirectoryPicked { target, path } => {
                        if *target != DownloadDirectoryTarget::Settings {
                            return;
                        }
                        let directory = this.directory.clone();
                        let path = path.to_string_lossy().into_owned();
                        let source_window = window.window_handle();
                        cx.defer(move |cx| {
                            let _ = source_window.update(cx, |_, window, cx| {
                                directory.update(cx, |input, cx| {
                                    input.set_value(path, window, cx);
                                });
                            });
                        });
                        this.error = None;
                    }
                    DownloadApplicationEvent::FfmpegExecutablePicked { path } => {
                        let ffmpeg_executable = this.ffmpeg_executable.clone();
                        let path = path.to_string_lossy().into_owned();
                        let source_window = window.window_handle();
                        cx.defer(move |cx| {
                            let _ = source_window.update(cx, |_, window, cx| {
                                ffmpeg_executable.update(cx, |input, cx| {
                                    input.set_value(path, window, cx);
                                });
                            });
                        });
                        this.error = None;
                    }
                    DownloadApplicationEvent::BrowserExtensionExportCompleted { status } => {
                        this.browser_extension_export_status = Some(status.clone());
                        this.error = None;
                        cx.notify();
                    }
                    DownloadApplicationEvent::FfmpegInstallChanged { state } => {
                        this.ffmpeg_install_state = state.clone();
                        this.error = match state {
                            FfmpegInstallViewState::Failed(message) => Some(message.clone()),
                            _ => None,
                        };
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
            browser_extension_export_status: None,
            directory,
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
            taskbar_overlay_offset_x,
            taskbar_overlay_offset_y,
            taskbar_overlay_width,
            taskbar_overlay_height,
            scroll: ScrollHandle::new(),
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
        Ok(draft)
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
                    view.error = Some(error);
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
                    view.error = Some(error);
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
                        view.error = Some(error);
                        cx.notify();
                    });
                }
            }
        });
        cx.notify();
    }
}

impl Render for DownloadSettingsView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let scroll = self.scroll.clone();
        let current_directory = self.directory.read(cx).value().to_string();
        let picker_current_directory = current_directory.clone();
        let application = self.application.clone();
        let directory_control = div()
            .w(px(286.0))
            .flex()
            .items_center()
            .gap(px(SPACING_6))
            .child(download_settings_input(
                theme,
                "download-settings-default-directory-field",
                &self.directory,
                px(250.0),
            ))
            .child(
                div()
                    .debug_selector(|| "download-settings-pick-directory-control".into())
                    .child(icon_action_tooltip_surface(
                        icon_action_button(
                            "download-settings-pick-directory",
                            MaterialIcon::Folder,
                        )
                        .ghost()
                        .border_1()
                        .border_color(theme.border_subtle)
                        .bg(theme.surface_raised)
                        .on_click(move |_, _, cx| {
                            let application = application.clone();
                            let current_directory = picker_current_directory.clone();
                            cx.defer(move |cx| {
                                application.update(cx, |application, cx| {
                                    application.request_settings_directory(current_directory, cx);
                                });
                            });
                        }),
                        "选择默认保存目录",
                        "download-settings-pick-directory-tooltip",
                    )),
            )
            .into_any_element();
        let default_headers_control = multiline_field_shell(
            theme,
            "download-settings-default-headers-field",
            Input::new(&self.default_request_headers)
                .appearance(false)
                .bordered(false)
                .focus_bordered(false),
            px(DOWNLOAD_REQUEST_HEADER_EDITOR_HEIGHT),
        );
        let default_cookie_control = labeled_field(
            theme,
            "默认 Cookie",
            Some("仅在任务没有 Cookie 时带入；内容不会写入诊断日志"),
            &self.default_cookie,
            "download-settings-default-cookie-field",
        );
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
                list.child(compact_list_row(
                    theme,
                    format!("download-settings-category-{index}").into(),
                    presentation.name,
                    presentation.directory,
                    selected,
                    presentation.enabled,
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
            .w(px(228.0))
            .h_full()
            .flex_none()
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
                    .p(px(SPACING_6))
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
                    .gap(px(SPACING_8))
                    .child(
                        div()
                            .debug_selector(|| "download-settings-category-heading".into())
                            .w_full()
                            .h(px(CONTROL_HEIGHT_INPUT))
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
                                    .gap(px(SPACING_8))
                                    .child(
                                        div()
                                            .debug_selector(|| {
                                                "download-settings-category-heading-title".into()
                                            })
                                            .flex_none()
                                            .text_size(px(TEXT_BODY))
                                            .line_height(px(LINE_HEIGHT_BODY))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(theme.text)
                                            .child("分类规则"),
                                    )
                                    .child(
                                        div()
                                            .min_w_0()
                                            .truncate()
                                            .text_size(px(TEXT_LABEL))
                                            .line_height(px(LINE_HEIGHT_LABEL))
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
                                    .gap(px(SPACING_8))
                                    .child(
                                        div()
                                            .text_size(px(TEXT_LABEL))
                                            .line_height(px(LINE_HEIGHT_LABEL))
                                            .text_color(theme.text_secondary)
                                            .child("启用"),
                                    )
                                    .child(enabled_control),
                            ),
                    )
                    .child(
                        div()
                            .debug_selector(|| "download-settings-category-primary-fields".into())
                            .w_full()
                            .flex()
                            .gap(px(SPACING_8))
                            .child(
                                div()
                                    .debug_selector(|| {
                                        "download-settings-category-name-cell".into()
                                    })
                                    .min_w_0()
                                    .flex_1()
                                    .child(labeled_field(
                                        theme,
                                        "分类名称",
                                        None,
                                        &name,
                                        "download-settings-category-name-field",
                                    )),
                            )
                            .child(
                                div()
                                    .debug_selector(|| {
                                        "download-settings-category-directory-cell".into()
                                    })
                                    .min_w_0()
                                    .flex_1()
                                    .child(labeled_field(
                                        theme,
                                        "保存目录",
                                        Some("留空使用默认目录；相对路径位于默认目录下"),
                                        &directory,
                                        "download-settings-category-directory-field",
                                    )),
                            ),
                    )
                    .child(
                        div()
                            .debug_selector(|| "download-settings-category-rule-fields".into())
                            .w_full()
                            .flex()
                            .gap(px(SPACING_8))
                            .child(
                                div()
                                    .debug_selector(|| {
                                        "download-settings-category-extensions-cell".into()
                                    })
                                    .min_w_0()
                                    .flex_1()
                                    .child(labeled_field(
                                        theme,
                                        "扩展名规则",
                                        Some("逗号分隔，例如 zip, 7z, rar"),
                                        &extensions,
                                        "download-settings-category-extensions-field",
                                    )),
                            )
                            .child(
                                div()
                                    .debug_selector(|| "download-settings-category-url-cell".into())
                                    .min_w_0()
                                    .flex_1()
                                    .child(labeled_field(
                                        theme,
                                        "URL 通配规则",
                                        Some("逗号分隔；与扩展名规则同时存在时均须命中"),
                                        &url_patterns,
                                        "download-settings-category-url-field",
                                    )),
                            ),
                    )
                    .child(div().flex_1())
                    .child(
                        div()
                            .debug_selector(|| "download-settings-category-action-toolbar".into())
                            .w_full()
                            .pt(px(SPACING_8))
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
                                    .gap(px(SPACING_4))
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
                                                .ghost()
                                                .border_1()
                                                .border_color(theme.border_subtle)
                                                .bg(theme.surface_raised)
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
                                                .ghost()
                                                .border_1()
                                                .border_color(theme.border_subtle)
                                                .bg(theme.surface_raised)
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
                                        .border_1()
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
                    .text_size(px(TEXT_LABEL))
                    .line_height(px(LINE_HEIGHT_LABEL))
                    .text_color(theme.text_muted)
                    .child("暂无分类；可新增自定义规则")
                    .into_any_element()
            });
        let category_management = div()
            .w_full()
            .h(px(310.0))
            .flex()
            .items_start()
            .gap(px(SPACING_12))
            .child(category_list)
            .child(
                div()
                    .min_w_0()
                    .flex_1()
                    .h_full()
                    .pl(px(SPACING_12))
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
            .gap(px(SPACING_6))
            .child(
                div().flex_1().min_w_0().child(field_shell(
                    theme,
                    "download-settings-ffmpeg-field",
                    Input::new(&self.ffmpeg_executable)
                        .appearance(false)
                        .bordered(false)
                        .focus_bordered(false),
                )),
            )
            .child(
                div()
                    .debug_selector(|| "download-settings-pick-ffmpeg".into())
                    .child(icon_action_tooltip_surface(
                        icon_action_button(
                            "download-settings-pick-ffmpeg-button",
                            MaterialIcon::Folder,
                        )
                        .ghost()
                        .border_1()
                        .border_color(theme.border_subtle)
                        .bg(theme.surface_raised)
                        .on_click(move |_, _, cx| {
                            let application = application.clone();
                            let current_path = ffmpeg_picker_path.clone();
                            cx.defer(move |cx| {
                                application.update(cx, |application, cx| {
                                    application.request_ffmpeg_executable(current_path, cx);
                                });
                            });
                        }),
                        "选择 FFmpeg 可执行文件",
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
        let install_application = self.application.clone();
        let cancel_application = self.application.clone();
        let ffmpeg_install_control = div()
            .debug_selector(|| "download-settings-ffmpeg-install-actions".into())
            .w_full()
            .flex()
            .items_center()
            .gap(px(SPACING_6))
            .child(
                div()
                    .debug_selector(|| "download-settings-ffmpeg-install".into())
                    .child(
                        action_button(
                            "download-settings-ffmpeg-install-button",
                            if install_active {
                                "正在下载…"
                            } else {
                                "自动下载"
                            },
                        )
                        .custom(download_primary_button_variant(theme, cx))
                        .disabled(install_active)
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
            .gap(px(SPACING_6))
            .child(
                div()
                    .debug_selector(|| "download-settings-ffmpeg-gyan".into())
                    .child(
                        action_button("download-settings-ffmpeg-gyan-button", "Gyan.dev")
                            .outline()
                            .border_1()
                            .border_color(theme.border_subtle)
                            .bg(theme.surface_raised)
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
                            .border_1()
                            .border_color(theme.border_subtle)
                            .bg(theme.surface_raised)
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
            px(88.0),
        )
        .into_any_element();
        let adaptive_max_control = download_settings_input(
            theme,
            "download-settings-adaptive-max-field",
            &self.adaptive_segment_max_segments,
            px(88.0),
        )
        .into_any_element();
        let adaptive_min_control = download_settings_input(
            theme,
            "download-settings-adaptive-min-field",
            &self.adaptive_segment_min_mib,
            px(88.0),
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
            px(88.0),
        )
        .into_any_element();
        let speed_control = download_settings_input(
            theme,
            "download-settings-speed-limit-field",
            &self.speed_limit_kib,
            px(88.0),
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
            px(88.0),
        )
        .into_any_element();
        let taskbar_offset_y = download_settings_input(
            theme,
            "download-settings-taskbar-y-field",
            &self.taskbar_overlay_offset_y,
            px(88.0),
        )
        .into_any_element();
        let taskbar_width = download_settings_input(
            theme,
            "download-settings-taskbar-width-field",
            &self.taskbar_overlay_width,
            px(88.0),
        )
        .into_any_element();
        let taskbar_height = download_settings_input(
            theme,
            "download-settings-taskbar-height-field",
            &self.taskbar_overlay_height,
            px(88.0),
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
                action_button(
                    "download-settings-taskbar-edit-button",
                    if self.taskbar_overlay_editing {
                        "结束预览"
                    } else {
                        "编辑布局"
                    },
                )
                .icon(Icon::new(if self.taskbar_overlay_editing {
                    MaterialIcon::Check
                } else {
                    MaterialIcon::Edit
                }))
                .custom(download_primary_button_variant(theme, cx))
                .on_click(cx.listener(|this, _, _, cx| {
                    this.toggle_taskbar_overlay_edit(cx);
                })),
            )
            .into_any_element();
        let page_content = match self.page {
            DownloadSettingsPage::General => div()
                .w_full()
                .min_w_0()
                .flex()
                .flex_col()
                .gap(px(SPACING_8))
                .child(download_settings_section(
                    theme,
                    "保存",
                    vec![download_settings_row(
                        theme,
                        "默认保存目录",
                        "新建任务自动使用，可在单个任务中覆盖",
                        directory_control,
                    )],
                ))
                .child(download_settings_section(
                    theme,
                    "分类与重名",
                    vec![
                        download_settings_row(
                            theme,
                            "自动分类",
                            "仅为仍使用默认保存目录的新任务匹配；手动目录优先",
                            automatic_categories_toggle,
                        ),
                        download_settings_stacked_row(
                            theme,
                            "分类规则",
                            "选择左侧分类，在右侧编辑名称、独立保存目录和匹配规则",
                            category_management,
                        ),
                        download_settings_row(
                            theme,
                            "目标已存在",
                            "自动编号不会覆盖磁盘文件或活动任务产物；拒绝则不创建任务",
                            duplicate_strategy,
                        ),
                        download_settings_row(
                            theme,
                            "列表双击",
                            "双击下载列表行时的默认动作",
                            double_click_action,
                        ),
                    ],
                ))
                .child(download_settings_section(
                    theme,
                    "浏览器扩展",
                    vec![download_settings_stacked_row(
                        theme,
                        "下载拦截扩展",
                        "Chrome / Edge 用开发者模式加载包内 browser_extension 目录，或导出签名安装包",
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(SPACING_4))
                            .child(
                                div()
                                    .flex()
                                    .gap(px(SPACING_4))
                                    .child(
                                        action_button(
                                            "download-settings-extension-export-chrome-button",
                                            "导出 Chrome / Edge (.crx)",
                                        )
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            let outcome = this.application.update(
                                                cx,
                                                |application, _| {
                                                    application
                                                        .request_browser_extension_export("chrome")
                                                },
                                            );
                                            this.browser_extension_export_status =
                                                Some(match outcome {
                                                    Ok(()) => "正在导出 Chrome / Edge 扩展…".into(),
                                                    Err(error) => format!("导出失败:{error}"),
                                                });
                                            cx.notify();
                                        })),
                                    )
                                    .child(
                                        action_button(
                                            "download-settings-extension-export-firefox-button",
                                            "导出 Firefox (.xpi)",
                                        )
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            let outcome = this.application.update(
                                                cx,
                                                |application, _| {
                                                    application
                                                        .request_browser_extension_export("firefox")
                                                },
                                            );
                                            this.browser_extension_export_status =
                                                Some(match outcome {
                                                    Ok(()) => "正在导出 Firefox 扩展…".into(),
                                                    Err(error) => format!("导出失败:{error}"),
                                                });
                                            cx.notify();
                                        })),
                                    ),
                            )
                            .children(
                                self.browser_extension_export_status
                                    .clone()
                                    .map(|status| div().child(status)),
                            )
                            .into_any_element(),
                    )],
                ))
                .into_any_element(),
            DownloadSettingsPage::Request => div()
                .w_full()
                .min_w_0()
                .flex()
                .flex_col()
                .gap(px(SPACING_8))
                .child(download_settings_section(
                    theme,
                    "请求上下文",
                    vec![div()
                        .debug_selector(|| "download-settings-request-context-form".into())
                        .w_full()
                        .min_w_0()
                        .p(px(SPACING_8))
                        .flex()
                        .flex_col()
                        .gap(px(SPACING_12))
                        .child(labeled_control(
                            theme,
                            "默认 Header",
                            Some(
                                "每行 Name: Value，也可写 -H 'Name: Value'；任务或浏览器捕获值优先",
                            ),
                            default_headers_control,
                            "download-settings-default-headers-field",
                        ))
                        .child(default_cookie_control)
                        .into_any_element()],
                ))
                .child(download_settings_section(
                    theme,
                    "证书校验",
                    vec![download_settings_row(
                        theme,
                        "允许无效证书",
                        "默认严格校验证书；仅为确有需要且可信的服务器显式开启",
                        certificate_toggle,
                    )],
                ))
                .into_any_element(),
            DownloadSettingsPage::Transfer => div()
                .w_full()
                .min_w_0()
                .flex()
                .flex_col()
                .gap(px(SPACING_8))
                .child(download_settings_matrix_section(
                    theme,
                    "传输",
                    vec![
                        (
                            download_settings_row(
                                theme,
                                "连接上限",
                                "新任务从开始即使用的最大并行连接数，范围 1–64",
                                connection_control,
                            ),
                            download_settings_row(
                                theme,
                                "同时下载",
                                "默认队列允许同时运行的任务数，范围 1–64",
                                concurrent_control,
                            ),
                        ),
                        (
                            download_settings_row(
                                theme,
                                "动态分段",
                                "速度稳定时拆分长尾；连接数仍保持在上限内",
                                adaptive_toggle,
                            ),
                            download_settings_row(
                                theme,
                                "分段上限",
                                "动态拆分后的最大分段数，范围为连接数到 256",
                                adaptive_max_control,
                            ),
                        ),
                        (
                            download_settings_row(
                                theme,
                                "最小长尾",
                                "剩余大小达到此值才拆分，单位 MiB，范围 1–100",
                                adaptive_min_control,
                            ),
                            download_settings_row(
                                theme,
                                "全局限速",
                                "单位 KiB/s；填 0 表示不限速",
                                speed_control,
                            ),
                        ),
                    ],
                ))
                .child(
                    div()
                        .w_full()
                        .debug_selector(|| "download-settings-media-tools-section".into())
                        .child(download_settings_section(
                            theme,
                            "媒体工具",
                            vec![
                                download_settings_stacked_row(
                                    theme,
                                    "FFmpeg 可执行文件",
                                    ffmpeg_description,
                                    ffmpeg_control,
                                ),
                                download_settings_stacked_row(
                                    theme,
                                    "自动安装",
                                    install_description,
                                    ffmpeg_install_control,
                                ),
                                download_settings_stacked_row(
                                    theme,
                                    "公开下载来源",
                                    "手动安装备用入口；FFmpeg 不会随 NxTools 安装包携带",
                                    ffmpeg_source_control,
                                ),
                            ],
                        )),
                )
                .into_any_element(),
            DownloadSettingsPage::Window => div()
                .w_full()
                .min_w_0()
                .flex()
                .flex_col()
                .gap(px(SPACING_8))
                .child(download_settings_matrix_section(
                    theme,
                    "任务窗口",
                    vec![
                        (
                            download_settings_row(
                                theme,
                                "自动显示进度",
                                "任务开始后显示独立进度窗口",
                                progress_toggle,
                            ),
                            download_settings_row(
                                theme,
                                "显示完成窗口",
                                "进度窗已关闭时被动显示完成窗口",
                                completion_toggle,
                            ),
                        ),
                        (
                            download_settings_row(
                                theme,
                                "完成提示音",
                                "每批完成任务只播放一次系统提示音",
                                completion_sound_toggle,
                            ),
                            download_settings_row(
                                theme,
                                "完成后打开目录",
                                "每批最多打开三个目录，避免窗口风暴",
                                completion_directory_toggle,
                            ),
                        ),
                        (
                            download_settings_row(
                                theme,
                                "删除任务时删除本地文件",
                                "关闭后，删除文件只从列表移除任务记录",
                                task_removal_toggle,
                            ),
                            div().into_any_element(),
                        ),
                    ],
                ))
                .child(download_settings_matrix_section(
                    theme,
                    "任务栏进度",
                    vec![
                        (
                            download_settings_row(
                                theme,
                                "任务栏内进度",
                                "活动下载时在任务栏上沿显示总体进度",
                                taskbar_overlay_toggle,
                            ),
                            download_settings_row(
                                theme,
                                "布局预览",
                                "显示模拟数据并允许拖动、缩放",
                                taskbar_edit_control,
                            ),
                        ),
                        (
                            download_settings_row(
                                theme,
                                "锚点",
                                "从任务栏左侧或右侧计算偏移",
                                taskbar_anchor,
                            ),
                            download_settings_row(
                                theme,
                                "进度模式",
                                "总进度、按任务或按分段显示",
                                taskbar_progress_mode,
                            ),
                        ),
                        (
                            download_settings_row(
                                theme,
                                "X 偏移",
                                "正值向任务栏内部移动",
                                taskbar_offset_x,
                            ),
                            download_settings_row(
                                theme,
                                "Y 偏移",
                                "0 表示紧贴任务栏上沿",
                                taskbar_offset_y,
                            ),
                        ),
                        (
                            download_settings_row(
                                theme,
                                "宽度",
                                "逻辑像素，范围 72–640",
                                taskbar_width,
                            ),
                            download_settings_row(
                                theme,
                                "高度",
                                "逻辑像素，范围 6–72",
                                taskbar_height,
                            ),
                        ),
                        (
                            download_settings_row(
                                theme,
                                "进度条",
                                "显示所选模式的进度槽",
                                taskbar_progress_info,
                            ),
                            download_settings_row(
                                theme,
                                "速度",
                                "显示当前合计下载速度",
                                taskbar_speed_info,
                            ),
                        ),
                        (
                            download_settings_row(
                                theme,
                                "剩余时间",
                                "按当前合计速度估算",
                                taskbar_remaining_info,
                            ),
                            download_settings_row(
                                theme,
                                "已下载 / 总大小",
                                "显示活动任务的合计字节数",
                                taskbar_bytes_info,
                            ),
                        ),
                        (
                            download_settings_row(
                                theme,
                                "活动任务数",
                                "显示当前正在处理的任务数量",
                                taskbar_task_count_info,
                            ),
                            download_settings_row(
                                theme,
                                "活动连接数",
                                "显示所有活动任务的连接合计",
                                taskbar_connection_info,
                            ),
                        ),
                    ],
                ))
                .into_any_element(),
            DownloadSettingsPage::Diagnostics => div()
                .w_full()
                .min_w_0()
                .flex()
                .flex_col()
                .gap(px(SPACING_8))
                .child(download_settings_section(
                    theme,
                    "诊断",
                    vec![
                        download_settings_path_row(
                            theme,
                            "配置",
                            self.config_path.clone(),
                            DownloadSettingsPathKind::Config,
                            self.application.clone(),
                        ),
                        download_settings_path_row(
                            theme,
                            "日志",
                            self.log_directory.clone(),
                            DownloadSettingsPathKind::Logs,
                            self.application.clone(),
                        ),
                        download_settings_path_row(
                            theme,
                            "数据库",
                            self.database_path.clone(),
                            DownloadSettingsPathKind::Database,
                            self.application.clone(),
                        ),
                    ],
                ))
                .into_any_element(),
        };
        let navigation = div()
            .w(px(164.0))
            .h_full()
            .py(px(SPACING_8))
            .border_r_1()
            .border_color(theme.border_subtle)
            .bg(theme.surface)
            .flex()
            .flex_col()
            .gap(px(SPACING_4))
            .child(download_settings_navigation_item(
                theme,
                "download-settings-nav-general",
                "download-settings-nav-general",
                "常规",
                "保存位置与分类",
                self.page == DownloadSettingsPage::General,
                cx.listener(|this, _, _, cx| {
                    this.page = DownloadSettingsPage::General;
                    this.scroll.set_offset(gpui::point(px(0.0), px(0.0)));
                    cx.notify();
                }),
            ))
            .child(download_settings_navigation_item(
                theme,
                "download-settings-nav-request",
                "download-settings-nav-request",
                "请求",
                "Header、Cookie 与证书",
                self.page == DownloadSettingsPage::Request,
                cx.listener(|this, _, _, cx| {
                    this.page = DownloadSettingsPage::Request;
                    this.scroll.set_offset(gpui::point(px(0.0), px(0.0)));
                    cx.notify();
                }),
            ))
            .child(download_settings_navigation_item(
                theme,
                "download-settings-nav-transfer",
                "download-settings-nav-transfer",
                "传输",
                "并发、分段与媒体工具",
                self.page == DownloadSettingsPage::Transfer,
                cx.listener(|this, _, _, cx| {
                    this.page = DownloadSettingsPage::Transfer;
                    this.scroll.set_offset(gpui::point(px(0.0), px(0.0)));
                    cx.notify();
                }),
            ))
            .child(download_settings_navigation_item(
                theme,
                "download-settings-nav-window",
                "download-settings-nav-window",
                "窗口",
                "进度窗与任务栏显示",
                self.page == DownloadSettingsPage::Window,
                cx.listener(|this, _, _, cx| {
                    this.page = DownloadSettingsPage::Window;
                    this.scroll.set_offset(gpui::point(px(0.0), px(0.0)));
                    cx.notify();
                }),
            ))
            .child(download_settings_navigation_item(
                theme,
                "download-settings-nav-diagnostics",
                "download-settings-nav-diagnostics",
                "诊断",
                "配置、日志与数据位置",
                self.page == DownloadSettingsPage::Diagnostics,
                cx.listener(|this, _, _, cx| {
                    this.page = DownloadSettingsPage::Diagnostics;
                    this.scroll.set_offset(gpui::point(px(0.0), px(0.0)));
                    cx.notify();
                }),
            ));
        let content = div()
            .size_full()
            .min_h_0()
            .min_w_0()
            .flex()
            .child(navigation)
            .child(
                div()
                    .id("download-settings-viewport")
                    .relative()
                    .flex_1()
                    .min_h_0()
                    .min_w_0()
                    .child(
                        div()
                            .id("download-settings-scroll")
                            .debug_selector(|| "download-settings-scroll".into())
                            .size_full()
                            .overflow_y_scroll()
                            .track_scroll(&scroll)
                            .px(px(SPACING_12))
                            .py(px(SPACING_8))
                            .flex()
                            .flex_col()
                            .child(page_content)
                            .when_some(self.error.clone(), |body, error| {
                                body.child(download_inline_error(theme, error))
                            }),
                    )
                    .vertical_scrollbar(&self.scroll),
            )
            .into_any_element();
        let footer_actions = div()
            .flex()
            .items_center()
            .gap(px(SPACING_8))
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
                        action_button(
                            "download-settings-apply-button",
                            if self.saving {
                                "正在保存…"
                            } else {
                                "应用"
                            },
                        )
                        .icon(Icon::new(MaterialIcon::Check))
                        .custom(download_primary_button_variant(theme, cx))
                        .disabled(self.saving)
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
    width: gpui::Pixels,
) -> impl IntoElement {
    div().w(width).child(field_shell(
        theme,
        selector,
        Input::new(input)
            .appearance(false)
            .bordered(false)
            .focus_bordered(false),
    ))
}

pub(crate) fn download_settings_anchor(
    theme: Theme,
    selected: DownloadTaskbarOverlayAnchor,
    on_left: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
    on_right: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
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
    on_numbered: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
    on_reject: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
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
    on_open_file: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
    on_open_directory: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
    on_task_window: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
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
    on_aggregate: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
    on_task: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
    on_segment: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
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

pub(crate) fn download_settings_section(
    theme: Theme,
    title: &'static str,
    rows: Vec<AnyElement>,
) -> AnyElement {
    // Zed settings grammar: sections are flat groups separated by hairline
    // dividers, not boxed cards; the section title reads as a label above
    // the rows.
    div()
        .debug_selector(move || format!("download-settings-section-{title}"))
        .w_full()
        .flex()
        .flex_col()
        .gap(px(SPACING_8))
        .child(
            div()
                .text_size(px(TEXT_LABEL))
                .line_height(px(LINE_HEIGHT_LABEL))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme.text_secondary)
                .child(title),
        )
        .children(rows)
        .child(div().h(px(1.0)).w_full().bg(theme.border_subtle))
        .into_any_element()
}

pub(crate) fn download_settings_matrix_section(
    theme: Theme,
    title: &'static str,
    rows: Vec<(AnyElement, AnyElement)>,
) -> AnyElement {
    div()
        .debug_selector(move || format!("download-settings-section-{title}"))
        .w_full()
        .rounded(px(RADIUS_CONTROL))
        .border_1()
        .border_color(theme.border_subtle)
        .bg(theme.surface)
        .flex()
        .flex_col()
        .child(
            div()
                .h(px(28.0))
                .px(px(SPACING_8))
                .border_b_1()
                .border_color(theme.border_subtle)
                .flex()
                .items_center()
                .debug_selector(move || format!("download-settings-section-{title}-title"))
                .text_size(px(TEXT_LABEL))
                .line_height(px(LINE_HEIGHT_LABEL))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme.text_secondary)
                .child(title),
        )
        .child(
            div()
                .w_full()
                .flex()
                .flex_col()
                .children(rows.into_iter().enumerate().map(move |(row_index, row)| {
                    let (left, right) = row;
                    let left_index = row_index * 2;
                    let right_index = left_index + 1;
                    div()
                        .debug_selector(move || {
                            format!("download-settings-matrix-{title}-row-{row_index}")
                        })
                        .w_full()
                        .min_w_0()
                        .flex()
                        .child(
                            div()
                                .debug_selector(move || {
                                    format!(
                                        "download-settings-matrix-{title}-cell-{row_index}-left"
                                    )
                                })
                                .flex_1()
                                .min_w_0()
                                .child(
                                    div()
                                        .debug_selector(move || {
                                            format!("download-settings-grid-{title}-{left_index}")
                                        })
                                        .w_full()
                                        .min_w_0()
                                        .child(left),
                                ),
                        )
                        .child(
                            div()
                                .debug_selector(move || {
                                    format!("download-settings-matrix-{title}-seam-{row_index}")
                                })
                                .w(px(1.0))
                                .flex_none()
                                .bg(theme.border_subtle),
                        )
                        .child(
                            div()
                                .debug_selector(move || {
                                    format!(
                                        "download-settings-matrix-{title}-cell-{row_index}-right"
                                    )
                                })
                                .flex_1()
                                .min_w_0()
                                .child(
                                    div()
                                        .debug_selector(move || {
                                            format!("download-settings-grid-{title}-{right_index}")
                                        })
                                        .w_full()
                                        .min_w_0()
                                        .child(right),
                                ),
                        )
                })),
        )
        .into_any_element()
}

pub(crate) fn download_settings_row(
    theme: Theme,
    title: &'static str,
    description: impl Into<gpui::SharedString>,
    control: AnyElement,
) -> AnyElement {
    let description = description.into();
    div()
        .min_h(px(SETTINGS_ROW_MIN_HEIGHT))
        .px(px(SPACING_8))
        .py(px(SPACING_4))
        .flex()
        .items_center()
        .gap(px(SPACING_12))
        .child(
            div()
                .min_w_0()
                .flex_1()
                .flex()
                .flex_col()
                .gap(px(SPACING_2))
                .child(
                    div()
                        .debug_selector(move || format!("download-settings-row-{title}-title"))
                        .text_size(px(TEXT_BODY))
                        .line_height(px(LINE_HEIGHT_BODY))
                        .child(title),
                )
                .child(
                    div()
                        .debug_selector(move || {
                            format!("download-settings-row-{title}-description")
                        })
                        .text_size(px(TEXT_LABEL))
                        .line_height(px(LINE_HEIGHT_LABEL))
                        .text_color(theme.text_muted)
                        .child(description),
                ),
        )
        .child(control)
        .into_any_element()
}

pub(crate) fn download_settings_stacked_row(
    theme: Theme,
    title: &'static str,
    description: impl Into<gpui::SharedString>,
    control: AnyElement,
) -> AnyElement {
    div()
        .debug_selector(move || format!("download-settings-stacked-row-{title}"))
        .w_full()
        .min_h(px(72.0))
        .px(px(SPACING_8))
        .py(px(SPACING_6))
        .border_b_1()
        .border_color(theme.border_subtle)
        .flex()
        .flex_col()
        .gap(px(SPACING_6))
        .child(
            div()
                .flex()
                .items_end()
                .gap(px(SPACING_8))
                .child(
                    div()
                        .debug_selector(move || {
                            format!("download-settings-stacked-row-{title}-title")
                        })
                        .text_size(px(TEXT_BODY))
                        .line_height(px(LINE_HEIGHT_BODY))
                        .font_weight(FontWeight::MEDIUM)
                        .child(title),
                )
                .child(
                    div()
                        .debug_selector(move || {
                            format!("download-settings-stacked-row-{title}-description")
                        })
                        .min_w_0()
                        .flex_1()
                        .truncate()
                        .text_size(px(TEXT_LABEL))
                        .line_height(px(LINE_HEIGHT_LABEL))
                        .text_color(theme.text_muted)
                        .child(description.into()),
                ),
        )
        .child(div().w_full().flex().child(control))
        .into_any_element()
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
        .size(px(CONTROL_HEIGHT_ACTION))
        .child(icon_action_tooltip_surface(
            icon_action_button(presentation.copy_button_id, MaterialIcon::ContentCopy)
                .ghost()
                .border_1()
                .border_color(theme.border_subtle)
                .bg(theme.surface_raised)
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
        .size(px(CONTROL_HEIGHT_ACTION))
        .child(icon_action_tooltip_surface(
            icon_action_button(presentation.open_button_id, MaterialIcon::OpenInNew)
                .ghost()
                .border_1()
                .border_color(theme.border_subtle)
                .bg(theme.surface_raised)
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
        .min_h(px(SETTINGS_ROW_MIN_HEIGHT))
        .px(px(SPACING_8))
        .border_b_1()
        .border_color(theme.border_subtle)
        .flex()
        .items_center()
        .gap(px(SPACING_8))
        .child(
            div()
                .w(px(36.0))
                .flex_none()
                .text_size(px(TEXT_LABEL))
                .line_height(px(LINE_HEIGHT_LABEL))
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
                .text_size(px(TEXT_LABEL))
                .line_height(px(LINE_HEIGHT_LABEL))
                .text_color(theme.text_secondary)
                .tooltip(move |window, cx| {
                    let complete_path = path_tooltip.clone();
                    Tooltip::element(move |_, _| {
                        div()
                            .debug_selector(move || value_tooltip_selector.into())
                            .max_w(px(720.0))
                            .whitespace_normal()
                            .font_family(UI_MONO_FONT_FAMILY)
                            .text_size(px(TEXT_LABEL))
                            .line_height(px(LINE_HEIGHT_LABEL))
                            .child(complete_path.clone())
                    })
                    .build(window, cx)
                })
                .child(path),
        )
        .child(
            div()
                .debug_selector(move || format!("download-settings-path-{key}-actions"))
                .flex_none()
                .flex()
                .items_center()
                .gap(px(SPACING_4))
                .child(copy_action)
                .child(open_action),
        )
        .into_any_element()
}

pub(crate) fn download_settings_navigation_item(
    theme: Theme,
    id: &'static str,
    selector: &'static str,
    title: &'static str,
    description: &'static str,
    selected: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
) -> AnyElement {
    div()
        .id(id)
        .debug_selector(move || selector.into())
        .mx(px(SPACING_8))
        .min_h(px(48.0))
        .px(px(SPACING_8))
        .py(px(SPACING_6))
        .rounded(px(RADIUS_CONTROL))
        .border_1()
        .border_color(if selected {
            theme.border_active
        } else {
            theme.surface
        })
        .bg(if selected {
            theme.element_selected
        } else {
            theme.surface
        })
        .cursor_pointer()
        .flex()
        .flex_col()
        .justify_center()
        .gap(px(SPACING_2))
        .child(
            div()
                .debug_selector(move || format!("{selector}-title"))
                .text_size(px(TEXT_BODY))
                .line_height(px(LINE_HEIGHT_BODY))
                .font_weight(if selected {
                    FontWeight::SEMIBOLD
                } else {
                    FontWeight::NORMAL
                })
                .text_color(if selected {
                    theme.text
                } else {
                    theme.text_secondary
                })
                .child(title),
        )
        .child(
            div()
                .debug_selector(move || format!("{selector}-description"))
                .text_size(px(TEXT_LABEL))
                .line_height(px(LINE_HEIGHT_LABEL))
                .text_color(theme.text_muted)
                .child(description),
        )
        .on_click(on_click)
        .into_any_element()
}
