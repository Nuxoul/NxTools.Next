use super::*;
use crate::ui::metrics::ICON_GLYPH_SMALL;

#[derive(Clone)]
pub(crate) enum NewDownloadInitialSource {
    Download(DownloadRequest),
    Blob(BrowserBlobDialogContext),
}

pub(crate) struct NewDownloadViewInit {
    pub(crate) application: Entity<DownloadApplication>,
    pub(crate) default_directory: PathBuf,
    pub(crate) default_connection_limit: u16,
    pub(crate) default_adaptive_segment_policy: nxtools_download::DownloadAdaptiveSegmentPolicy,
    pub(crate) categories: Vec<DownloadCategorySettings>,
    pub(crate) initial_source: Option<NewDownloadInitialSource>,
}

#[derive(Clone)]
pub(crate) struct BrowserBlobDialogContext {
    pub(crate) correlation_id: uuid::Uuid,
    pub(crate) handoff_id: uuid::Uuid,
    pub(crate) blob: BrowserBlob,
    pub(crate) save_directory: PathBuf,
}

pub(crate) fn category_matches_query(category: &DownloadCategorySettings, query: &str) -> bool {
    if query.is_empty() {
        return true;
    }
    category.name.to_lowercase().contains(query)
        || category.save_directory.to_lowercase().contains(query)
        || category
            .extensions
            .iter()
            .any(|extension| extension.to_lowercase().contains(query))
}

pub(crate) fn category_choice_ids(
    categories: &[DownloadCategorySettings],
    query: &str,
) -> Vec<Option<String>> {
    let mut choices = Vec::with_capacity(categories.len() + usize::from(query.is_empty()));
    if query.is_empty() {
        choices.push(None);
    }
    choices.extend(
        categories
            .iter()
            .filter(|category| category_matches_query(category, query))
            .map(|category| Some(category.id.clone())),
    );
    choices
}

pub(crate) fn category_choice_background(theme: Theme, selected: bool, highlighted: bool) -> Rgba {
    if highlighted {
        theme.element_selected
    } else if selected {
        theme.accent_soft
    } else {
        theme.canvas
    }
}

pub(crate) struct NewDownloadView {
    pub(crate) theme: Theme,
    pub(crate) application: Entity<DownloadApplication>,
    pub(crate) url: Entity<InputState>,
    pub(crate) directory: Entity<InputState>,
    pub(crate) save_directory_explicit: bool,
    pub(crate) file_name: Entity<InputState>,
    pub(crate) categories: Vec<DownloadCategorySettings>,
    pub(crate) selected_category_id: Option<String>,
    pub(crate) category_popover_open: bool,
    pub(crate) category_search: Entity<InputState>,
    pub(crate) category_highlighted_index: usize,
    pub(crate) category_scroll: ScrollHandle,
    pub(crate) scheduled_date: NaiveDate,
    pub(crate) scheduled_month: NaiveDate,
    pub(crate) scheduled_hour: Entity<SelectState<Vec<String>>>,
    pub(crate) scheduled_minute: Entity<SelectState<Vec<String>>>,
    pub(crate) schedule_popover_open: bool,
    pub(crate) committed_schedule: Option<DelayedStartSelection>,
    pub(crate) default_connection_limit: u16,
    pub(crate) default_adaptive_segment_policy: nxtools_download::DownloadAdaptiveSegmentPolicy,
    pub(crate) external_request: Option<DownloadRequest>,
    pub(crate) external_blob: Option<BrowserBlobDialogContext>,
    pub(crate) confirmed_request: Option<DownloadRequest>,
    pub(crate) probe: Option<DownloadProbe>,
    pub(crate) selected_hls_variant: Option<usize>,
    pub(crate) selected_dash_video_track: Option<usize>,
    pub(crate) selected_dash_audio_track: Option<usize>,
    pub(crate) probing: bool,
    pub(crate) submitting: bool,
    pub(crate) probe_generation: u64,
    pub(crate) error: Option<String>,
    pub(crate) _subscription: Subscription,
    pub(crate) _directory_subscription: Subscription,
    pub(crate) _category_search_subscription: Subscription,
    pub(crate) _category_key_subscription: Subscription,
}

impl NewDownloadView {
    pub(crate) fn new(
        init: NewDownloadViewInit,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let NewDownloadViewInit {
            application,
            default_directory,
            default_connection_limit,
            default_adaptive_segment_policy,
            categories,
            initial_source,
        } = init;
        let external_request = match initial_source.as_ref() {
            Some(NewDownloadInitialSource::Download(request)) => Some(request.clone()),
            _ => None,
        };
        let external_blob = match initial_source.as_ref() {
            Some(NewDownloadInitialSource::Blob(context)) => Some(context.clone()),
            _ => None,
        };
        let initial_url = external_request.as_ref().map_or_else(
            || {
                external_blob.as_ref().map_or_else(String::new, |context| {
                    format!("Blob · 已安全接收 · {} 字节", context.blob.size)
                })
            },
            |request| request.source_url.clone(),
        );
        let initial_directory = external_request.as_ref().map_or_else(
            || {
                external_blob.as_ref().map_or_else(
                    || default_directory.to_string_lossy().into_owned(),
                    |context| context.save_directory.to_string_lossy().into_owned(),
                )
            },
            |request| request.save_directory.to_string_lossy().into_owned(),
        );
        let initial_file_name = external_request
            .as_ref()
            .and_then(|request| request.file_name.clone())
            .or_else(|| {
                external_blob
                    .as_ref()
                    .map(|context| context.blob.file_name.clone())
            })
            .unwrap_or_default();
        let selected_category_id = external_request
            .as_ref()
            .and_then(|request| request.category_id.clone());
        let url = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(initial_url)
                .placeholder("https://example.com/file.zip")
        });
        let directory = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(initial_directory)
                .placeholder("保存目录")
        });
        let file_name = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(initial_file_name)
                .placeholder("可选；留空时使用服务器文件名")
        });
        let category_search =
            cx.new(|cx| InputState::new(window, cx).placeholder("搜索分类名称、目录或扩展名"));
        let directory_subscription = cx.subscribe(&directory, |this, _, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                this.save_directory_explicit = true;
                cx.notify();
            }
        });
        let category_search_subscription =
            cx.subscribe(&category_search, |this, _, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change) {
                    this.category_highlighted_index = 0;
                    this.category_scroll.scroll_to_item(0);
                    cx.notify();
                }
            });
        let category_key_listener =
            cx.listener(|this, event: &gpui::KeystrokeEvent, window, cx| {
                if !this.category_popover_open || event.keystroke.modifiers.modified() {
                    return;
                }
                let choices = this.category_choice_ids(cx);
                let key = event.keystroke.key.as_str();
                let handled = if key.eq_ignore_ascii_case("down") {
                    if !choices.is_empty() {
                        this.category_highlighted_index = (this.category_highlighted_index + 1)
                            .min(choices.len().saturating_sub(1));
                        this.category_scroll
                            .scroll_to_item(this.category_highlighted_index);
                    }
                    true
                } else if key.eq_ignore_ascii_case("up") {
                    this.category_highlighted_index =
                        this.category_highlighted_index.saturating_sub(1);
                    this.category_scroll
                        .scroll_to_item(this.category_highlighted_index);
                    true
                } else if key.eq_ignore_ascii_case("enter") {
                    if let Some(choice) = choices.get(this.category_highlighted_index).cloned() {
                        this.select_category_choice(choice);
                        this.file_name.focus_handle(cx).focus(window);
                    }
                    true
                } else if key.eq_ignore_ascii_case("escape") {
                    this.category_popover_open = false;
                    this.file_name.focus_handle(cx).focus(window);
                    true
                } else {
                    false
                };
                if handled {
                    cx.stop_propagation();
                    cx.notify();
                }
            });
        let category_key_subscription = cx.intercept_keystrokes(category_key_listener);
        let default_scheduled_at = Local::now() + ChronoDuration::minutes(10);
        let scheduled_date = default_scheduled_at.date_naive();
        let scheduled_month = calendar_month_start(scheduled_date);
        let scheduled_hour = cx.new(|cx| {
            SelectState::new(
                (0..24).map(|hour| format!("{hour:02}")).collect(),
                Some(IndexPath::default().row(default_scheduled_at.hour() as usize)),
                window,
                cx,
            )
        });
        let scheduled_minute = cx.new(|cx| {
            SelectState::new(
                (0..60).map(|minute| format!("{minute:02}")).collect(),
                Some(IndexPath::default().row(default_scheduled_at.minute() as usize)),
                window,
                cx,
            )
        });
        let subscription = cx.subscribe_in(
            &application,
            window,
            |this, _application, event: &DownloadApplicationEvent, window, cx| {
                let DownloadApplicationEvent::DirectoryPicked { target, path } = event else {
                    return;
                };
                if *target != DownloadDirectoryTarget::NewTask {
                    return;
                }
                this.directory.update(cx, |input, cx| {
                    input.set_value(path.to_string_lossy().into_owned(), window, cx);
                });
                this.error = None;
                cx.notify();
            },
        );
        Self {
            theme: Theme::zed_dark(),
            application,
            url,
            directory,
            save_directory_explicit: external_request
                .as_ref()
                .is_some_and(|request| request.save_directory_explicit),
            file_name,
            categories,
            selected_category_id,
            category_popover_open: false,
            category_search,
            category_highlighted_index: 0,
            category_scroll: ScrollHandle::new(),
            scheduled_date,
            scheduled_month,
            scheduled_hour,
            scheduled_minute,
            schedule_popover_open: false,
            committed_schedule: None,
            default_connection_limit,
            default_adaptive_segment_policy,
            external_request,
            external_blob: external_blob.clone(),
            confirmed_request: None,
            probe: external_blob.as_ref().map(|context| DownloadProbe {
                final_url: "browser-blob://staged".into(),
                file_name: Some(context.blob.file_name.clone()),
                total_bytes: Some(context.blob.size),
                supports_ranges: false,
                content_type: context.blob.media_type.clone(),
                validator: nxtools_download::ResourceValidator::default(),
                media: None,
            }),
            selected_hls_variant: None,
            selected_dash_video_track: None,
            selected_dash_audio_track: None,
            probing: false,
            submitting: false,
            probe_generation: 0,
            error: None,
            _subscription: subscription,
            _directory_subscription: directory_subscription,
            _category_search_subscription: category_search_subscription,
            _category_key_subscription: category_key_subscription,
        }
    }

    pub(crate) fn category_choice_ids(&self, cx: &App) -> Vec<Option<String>> {
        let query = self.category_search.read(cx).value().trim().to_lowercase();
        category_choice_ids(&self.categories, &query)
    }

    pub(crate) fn select_category_choice(&mut self, choice: Option<String>) {
        self.selected_category_id = choice;
        self.category_popover_open = false;
        self.error = None;
    }

    pub(crate) fn request(&self, cx: &App) -> Result<DownloadRequest, String> {
        let url = self.url.read(cx).value().trim().to_owned();
        let directory = self.directory.read(cx).value().trim().to_owned();
        let file_name = self.file_name.read(cx).value().trim().to_owned();
        if url.is_empty() || directory.is_empty() {
            return Err("下载地址和保存目录不能为空".into());
        }
        let scheduled_at_unix_ms =
            resolve_scheduled_start(self.committed_schedule, Local::now().timestamp_millis())?;
        let mut request = self.external_request.clone().unwrap_or(DownloadRequest {
            source_url: String::new(),
            save_directory: PathBuf::new(),
            save_directory_explicit: false,
            file_name: None,
            headers: Vec::new(),
            cookie: None,
            referrer: None,
            certificate_policy: Default::default(),
            connection_limit: Some(self.default_connection_limit),
            speed_limit_bytes_per_second: None,
            expected_total_bytes: None,
            scheduled_at_unix_ms: None,
            adaptive_segment_policy: Some(self.default_adaptive_segment_policy),
            media: None,
            category_id: None,
            category_name: None,
            duplicate_strategy: nxtools_download::DownloadDuplicateStrategy::UseDefault,
        });
        request.source_url = url;
        request.save_directory = PathBuf::from(directory);
        request.save_directory_explicit = self.save_directory_explicit;
        request.file_name = (!file_name.is_empty()).then_some(file_name);
        request.category_id = self.selected_category_id.clone();
        request.category_name = None;
        request.connection_limit = Some(self.default_connection_limit);
        request.adaptive_segment_policy = Some(self.default_adaptive_segment_policy);
        request.scheduled_at_unix_ms = scheduled_at_unix_ms;
        request.validate().map_err(|error| error.to_string())?;
        Ok(request)
    }

    pub(crate) fn reset_schedule_draft(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let default = Local::now() + ChronoDuration::minutes(10);
        let selection = self.committed_schedule.unwrap_or(DelayedStartSelection {
            date: default.date_naive(),
            hour: default.hour(),
            minute: default.minute(),
        });
        self.scheduled_date = selection.date;
        self.scheduled_month = calendar_month_start(selection.date);
        self.scheduled_hour.update(cx, |hour, cx| {
            hour.set_selected_index(
                Some(IndexPath::default().row(selection.hour as usize)),
                window,
                cx,
            );
        });
        self.scheduled_minute.update(cx, |minute, cx| {
            minute.set_selected_index(
                Some(IndexPath::default().row(selection.minute as usize)),
                window,
                cx,
            );
        });
    }

    pub(crate) fn begin_probe(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.external_blob.is_some() {
            return;
        }
        let request = match self.request(cx) {
            Ok(request) => request,
            Err(error) => {
                self.error = Some(error);
                self.confirmed_request = None;
                self.probe = None;
                cx.notify();
                return;
            }
        };
        self.probe_generation = self.probe_generation.wrapping_add(1).max(1);
        let generation = self.probe_generation;
        self.probing = true;
        self.error = None;
        self.confirmed_request = None;
        self.probe = None;
        window.resize(new_download_window_size(false));
        let correlation_id = uuid::Uuid::new_v4();
        let engine = self.application.read(cx).engine.clone();
        let background = cx.background_executor().clone();
        let source_window = window.window_handle();
        cx.spawn(async move |this, cx| {
            let request_for_probe = request.clone();
            let outcome = background
                .spawn(async move {
                    engine
                        .handle_command(DownloadCommandEnvelope {
                            protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
                            request_id: correlation_id,
                            origin: RequestOrigin::Ui,
                            command: DownloadCommand::Probe {
                                request: request_for_probe,
                            },
                        })
                        .outcome
                })
                .await;
            let applied = this.update(cx, |this, cx| {
                if this.probe_generation != generation {
                    return None;
                }
                this.probing = false;
                let mut resolved_file_name = None;
                let confirmed = match outcome {
                    DownloadCommandOutcome::Probe(probe) => {
                        let mut confirmed = request;
                        let probe_file_name = resolved_probe_file_name(&probe);
                        let replace_manifest_name = should_replace_media_manifest_name(
                            confirmed.file_name.as_deref(),
                            &probe,
                        );
                        if confirmed.file_name.is_none() || replace_manifest_name {
                            confirmed.file_name = probe_file_name.clone();
                            resolved_file_name = probe_file_name;
                        }
                        this.selected_hls_variant =
                            probe.media.as_ref().and_then(|media| match media {
                                nxtools_download::DownloadMediaProbe::Hls(hls)
                                    if hls.is_master && !hls.variants.is_empty() =>
                                {
                                    Some(0)
                                }
                                _ => None,
                            });
                        if let Some(nxtools_download::DownloadMediaProbe::Dash(dash)) =
                            probe.media.as_ref()
                        {
                            this.selected_dash_video_track =
                                (!dash.video_tracks.is_empty()).then_some(0);
                            this.selected_dash_audio_track =
                                (!dash.audio_tracks.is_empty()).then_some(0);
                        } else {
                            this.selected_dash_video_track = None;
                            this.selected_dash_audio_track = None;
                        }
                        this.confirmed_request = Some(confirmed);
                        this.probe = Some(probe);
                        this.error = None;
                        true
                    }
                    DownloadCommandOutcome::Rejected { fault } => {
                        this.error = Some(fault.message);
                        false
                    }
                    _ => {
                        warn!(
                            event = "download.probe.presentation_failed",
                            %correlation_id,
                            reason = "unexpected_outcome",
                            "download probe returned an unexpected outcome"
                        );
                        this.error = Some("下载引擎返回了无效的探测结果".into());
                        false
                    }
                };
                cx.notify();
                let window_size = this
                    .probe
                    .as_ref()
                    .map(new_download_window_size_for_probe)
                    .unwrap_or_else(|| new_download_window_size(false));
                Some((
                    this.file_name.clone(),
                    resolved_file_name,
                    confirmed,
                    window_size,
                ))
            });
            if let Ok(Some((file_name, resolved_file_name, _confirmed, window_size))) = applied {
                let _ = source_window.update(cx, move |_, window, cx| {
                    if let Some(resolved_file_name) = resolved_file_name {
                        file_name.update(cx, |input, cx| {
                            input.set_value(resolved_file_name, window, cx);
                        });
                    }
                    window.resize(window_size);
                });
            }
        })
        .detach();
        cx.notify();
    }

    pub(crate) fn submit(
        &mut self,
        mode: DownloadSubmitMode,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.submitting {
            return;
        }
        if let Some(blob) = self.external_blob.clone() {
            let directory = self.directory.read(cx).value().trim().to_owned();
            let file_name = self.file_name.read(cx).value().trim().to_owned();
            if directory.is_empty() || file_name.is_empty() {
                self.error = Some("保存目录和文件名不能为空".into());
                cx.notify();
                return;
            }
            self.submitting = true;
            self.error = None;
            cx.notify();
            let application = self.application.clone();
            let category_id = self.selected_category_id.clone();
            let save_directory_explicit = self.save_directory_explicit;
            let source_window = window.window_handle();
            let view = cx.entity().downgrade();
            cx.defer(move |cx| {
                match application.update(cx, |application, cx| {
                    application.import_browser_blob(
                        blob,
                        PathBuf::from(directory),
                        file_name,
                        category_id,
                        save_directory_explicit,
                        cx,
                    )
                }) {
                    Ok(task_id) => {
                        let _ = source_window.update(cx, |_, window, _| window.remove_window());
                        let application = application.clone();
                        cx.defer(move |cx| {
                            application.update(cx, |application, cx| {
                                application.request_task_window(task_id, cx);
                            });
                        });
                    }
                    Err(message) => {
                        let _ = view.update(cx, |view, cx| {
                            view.submitting = false;
                            view.error = Some(message);
                            cx.notify();
                        });
                    }
                }
            });
            return;
        }
        let current = match self.request(cx) {
            Ok(request) => request,
            Err(error) => {
                self.error = Some(error);
                cx.notify();
                return;
            }
        };
        let Some(confirmed) = self.confirmed_request.clone() else {
            self.error = Some("请先探测链接并确认文件信息".into());
            cx.notify();
            return;
        };
        let media = match selected_download_request_media(
            self.probe.as_ref(),
            self.selected_hls_variant,
            self.selected_dash_video_track,
            self.selected_dash_audio_track,
        ) {
            Ok(media) => media,
            Err(error) => {
                self.error = Some(error);
                cx.notify();
                return;
            }
        };
        let Some(mut request) = merge_confirmed_download_request(confirmed, current) else {
            self.confirmed_request = None;
            self.probe = None;
            self.error = Some("下载地址已变更，请重新探测".into());
            cx.notify();
            return;
        };
        request.media = media;
        if let Err(error) = request.validate() {
            self.error = Some(error.to_string());
            cx.notify();
            return;
        }
        if request.save_directory.exists() && !request.save_directory.is_dir() {
            self.error = Some("保存位置不是文件夹，请重新选择".into());
            cx.notify();
            return;
        }
        self.submitting = true;
        self.error = None;
        cx.notify();

        let application = self.application.clone();
        let source_window = window.window_handle();
        let view = cx.entity().downgrade();
        info!(?mode, "scheduling download task submission");
        cx.defer(move |cx| {
            let delayed = request.scheduled_at_unix_ms.is_some();
            let result =
                application.update(cx, |application, cx| application.submit(request, mode, cx));
            match result {
                Ok(task_id) => {
                    let show_progress_window = application.read(cx).show_progress_window;
                    let plan =
                        submit_presentation_plan(mode, task_id, delayed, show_progress_window);
                    if plan.close_source_window {
                        let _ = source_window.update(cx, |_, window, _| window.remove_window());
                    }
                    if let Some(task_id) = plan.deferred_task_window {
                        let application = application.clone();
                        cx.defer(move |cx| {
                            info!(
                                task_id = %task_id.as_uuid(),
                                "opening deferred download progress window"
                            );
                            application.update(cx, |application, cx| {
                                application.request_task_window(task_id, cx);
                            });
                        });
                    }
                }
                Err(message) => {
                    error!(error = %message, ?mode, "download task submission rejected");
                    let _ = view.update(cx, |view, cx| {
                        view.submitting = false;
                        view.error = Some(message);
                        cx.notify();
                    });
                }
            }
        });
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NewDownloadFooterAction {
    Schedule,
    QueueOnly,
    StartNow,
    Cancel,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NewDownloadFooterEmphasis {
    Primary,
    Outline,
    Ghost,
}

impl NewDownloadFooterAction {
    pub(crate) fn button_id(self) -> &'static str {
        match self {
            Self::Schedule => "download-new-schedule-trigger",
            Self::QueueOnly => "download-add-to-queue-button",
            Self::StartNow => "download-start-now-button",
            Self::Cancel => "download-new-cancel",
        }
    }

    pub(crate) fn icon(self) -> Option<MaterialIcon> {
        match self {
            Self::Schedule => Some(MaterialIcon::CalendarMonth),
            Self::StartNow => Some(MaterialIcon::Download),
            Self::QueueOnly | Self::Cancel => None,
        }
    }

    pub(crate) fn role_selector(self) -> &'static str {
        match self {
            Self::Schedule => "download-new-footer-secondary-schedule-action",
            Self::QueueOnly => "download-new-footer-secondary-queue-action",
            Self::StartNow => "download-new-footer-primary-action",
            Self::Cancel => "download-new-footer-secondary-cancel-action",
        }
    }

    pub(crate) fn emphasis(self) -> NewDownloadFooterEmphasis {
        match self {
            Self::StartNow => NewDownloadFooterEmphasis::Primary,
            Self::Schedule | Self::QueueOnly => NewDownloadFooterEmphasis::Outline,
            Self::Cancel => NewDownloadFooterEmphasis::Ghost,
        }
    }
}

pub(crate) fn new_download_footer_action(
    action: NewDownloadFooterAction,
    theme: Theme,
    cx: &App,
    label: impl Into<gpui::SharedString>,
    disabled: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
) -> Button {
    let button = action_button(action.button_id(), label);
    let button = if let Some(icon) = action.icon() {
        button.icon(Icon::new(icon))
    } else {
        button
    };
    let button = button.disabled(disabled).on_click(on_click);
    let button = match action.emphasis() {
        NewDownloadFooterEmphasis::Primary => button.custom(primary_button_variant(theme, cx)),
        NewDownloadFooterEmphasis::Outline => button.outline(),
        NewDownloadFooterEmphasis::Ghost => button.ghost(),
    };
    button.debug_selector(move || action.role_selector().into())
}

impl Render for NewDownloadView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let current_directory = self.directory.read(cx).value().to_string();
        let application = self.application.clone();
        let probe = self.probe.clone();
        let probing = self.probing;
        let submitting = self.submitting;
        let confirmed = probe.is_some();
        let schedule_label = if let Some(selection) = self.committed_schedule {
            format!(
                "{} {:02}:{:02}",
                selection.date.format("%m-%d"),
                selection.hour,
                selection.minute
            )
        } else {
            "稍后下载".into()
        };
        let schedule_view = cx.entity().downgrade();
        let schedule_date = self.scheduled_date;
        let schedule_month = self.scheduled_month;
        let schedule_today = Local::now().date_naive();
        let schedule_hour = self.scheduled_hour.clone();
        let schedule_minute = self.scheduled_minute.clone();
        let schedule_calendar_view = schedule_view.clone();
        let schedule_clear_view = schedule_view.clone();
        let schedule_confirm_view = schedule_view.clone();
        let schedule_confirm_hour = schedule_hour.clone();
        let schedule_confirm_minute = schedule_minute.clone();
        let schedule_primary_variant = download_primary_button_variant(theme, cx);
        let schedule_control = div()
            .id("download-new-schedule-control")
            .debug_selector(|| "download-new-schedule-control".into())
            .flex()
            .items_center()
            .child(
                Popover::new("download-new-schedule-popover")
                    .anchor(Corner::BottomLeft)
                    .open(self.schedule_popover_open)
                    .appearance(false)
                    .overlay_closable(true)
                    .on_open_change(move |open, window, cx| {
                        let open = *open;
                        let schedule_view = schedule_view.clone();
                        window.defer(cx, move |window, cx| {
                            let _ = schedule_view.update(cx, |view, cx| {
                                view.schedule_popover_open = open;
                                if open {
                                    view.reset_schedule_draft(window, cx);
                                }
                                cx.notify();
                            });
                        });
                    })
                    .trigger(new_download_footer_action(
                        NewDownloadFooterAction::Schedule,
                        theme,
                        cx,
                        schedule_label,
                        submitting,
                        |_, _, _| {},
                    ))
                    .content(move |_, _, _| {
                        let schedule_clear_view = schedule_clear_view.clone();
                        let schedule_confirm_view = schedule_confirm_view.clone();
                        let schedule_calendar_view = schedule_calendar_view.clone();
                        let schedule_confirm_hour = schedule_confirm_hour.clone();
                        let schedule_confirm_minute = schedule_confirm_minute.clone();
                        popover_shell(
                            theme,
                            "download-new-schedule-popover-shell",
                            304.0,
                            240.0,
                            div()
                                .id("download-new-schedule-popover-content")
                                .debug_selector(|| {
                                    "download-new-schedule-popover-content".into()
                                })
                                .size_full()
                                .flex()
                                .flex_col()
                                .gap(px(SPACING_6))
                                .child(delayed_start_calendar(
                                    theme,
                                    schedule_month,
                                    schedule_date,
                                    schedule_today,
                                    schedule_calendar_view,
                                ))
                                .child(
                                    div()
                                        .pt(px(SPACING_6))
                                        .border_t_1()
                                        .border_color(theme.border_subtle)
                                        .flex()
                                        .items_center()
                                        .gap(px(SPACING_6))
                                        .child(
                                            div()
                                                .mr(px(SPACING_4))
                                                .text_size(px(TEXT_LABEL))
                                                .line_height(px(LINE_HEIGHT_LABEL))
                                                .text_color(theme.text_secondary)
                                                .child("时间"),
                                        )
                                        .child(div().w(px(66.0)).child(
                                            Select::new(&schedule_hour)
                                                .menu_width(px(66.0))
                                                .small(),
                                        ))
                                        .child(
                                            div()
                                                .text_size(px(TEXT_LABEL))
                                                .text_color(theme.text_muted)
                                                .child(":"),
                                        )
                                        .child(div().w(px(66.0)).child(
                                            Select::new(&schedule_minute)
                                                .menu_width(px(66.0))
                                                .small(),
                                        ))
                                        .child(div().flex_1())
                                        .child(
                                            div()
                                                .debug_selector(|| {
                                                    "download-new-schedule-clear-action".into()
                                                })
                                                .child(
                                                    action_button(
                                                        "download-new-schedule-clear",
                                                        "清除",
                                                    )
                                                    .ghost()
                                                    .on_click(move |_, window, cx| {
                                                        let schedule_clear_view =
                                                            schedule_clear_view.clone();
                                                        window.defer(cx, move |_, cx| {
                                                            info!(
                                                                "download delayed-start selection cleared"
                                                            );
                                                            let _ = schedule_clear_view.update(
                                                                cx,
                                                                |view, cx| {
                                                                    view.committed_schedule = None;
                                                                    view.schedule_popover_open =
                                                                        false;
                                                                    view.error = None;
                                                                    cx.notify();
                                                                },
                                                            );
                                                        });
                                                    }),
                                                ),
                                        )
                                        .child(
                                            div()
                                                .debug_selector(|| {
                                                    "download-new-schedule-confirm".into()
                                                })
                                                .child(
                                                    action_button(
                                                        "download-new-schedule-confirm-button",
                                                        "确定",
                                                    )
                                                    .custom(schedule_primary_variant)
                                                    .on_click(move |_, window, cx| {
                                                        let selection = read_delayed_start_draft(
                                                            schedule_date,
                                                            &schedule_confirm_hour,
                                                            &schedule_confirm_minute,
                                                            cx,
                                                        );
                                                        let schedule_confirm_view =
                                                            schedule_confirm_view.clone();
                                                        window.defer(cx, move |_, cx| {
                                                            let _ = schedule_confirm_view.update(
                                                                cx,
                                                                |view, cx| {
                                                                    let selection = match selection {
                                                                        Ok(selection) => selection,
                                                                        Err(error) => {
                                                                            view.error =
                                                                                Some(error);
                                                                            cx.notify();
                                                                            return;
                                                                        }
                                                                    };
                                                                    info!(
                                                                        "download delayed-start selection committed"
                                                                    );
                                                                    view.committed_schedule =
                                                                        Some(selection);
                                                                    view.schedule_popover_open =
                                                                        false;
                                                                    view.error = None;
                                                                    cx.notify();
                                                                },
                                                            );
                                                        });
                                                    }),
                                                ),
                                        ),
                                )
                                .into_any_element(),
                        )
                    }),
            );
        let footer_actions = if confirmed {
            div()
                .debug_selector(|| "download-new-refresh-action".into())
                .child(
                    icon_action_button("download-refresh-probe", MaterialIcon::ManageSearch)
                        .outline()
                        .disabled(submitting)
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.begin_probe(window, cx);
                        })),
                )
                .into_any_element()
        } else {
            div()
                .debug_selector(|| "download-new-probe-action".into())
                .child(
                    action_button("download-probe", if probing { "正在探测…" } else { "继续" })
                        .icon(Icon::new(if probing {
                            MaterialIcon::ProgressActivity
                        } else {
                            MaterialIcon::ManageSearch
                        }))
                        .custom(primary_button_variant(theme, cx))
                        .disabled(probing || submitting)
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.begin_probe(window, cx);
                        })),
                )
                .into_any_element()
        };
        let selected_category_label = self
            .selected_category_id
            .as_deref()
            .and_then(|selected| {
                self.categories
                    .iter()
                    .find(|category| category.id.eq_ignore_ascii_case(selected))
            })
            .map_or_else(|| "自动分类".to_owned(), |category| category.name.clone());
        let category_view = cx.entity().downgrade();
        let category_open_view = category_view.clone();
        let category_menu_view = category_view.clone();
        let categories = self.categories.clone();
        let category_search = self.category_search.clone();
        let category_search_focus = self.category_search.focus_handle(cx);
        let category_return_focus = self.file_name.focus_handle(cx);
        let category_close_focus = category_return_focus.clone();
        let category_query = self.category_search.read(cx).value().trim().to_lowercase();
        let category_highlighted_index = self.category_highlighted_index;
        let category_scroll = self.category_scroll.clone();
        let selected_category_id_for_menu = self.selected_category_id.clone();
        let category_control = div()
            .debug_selector(|| "download-new-category-control".into())
            .w_full()
            .child(
                Popover::new("download-new-category-popover")
                    .anchor(Corner::BottomRight)
                    .open(self.category_popover_open)
                    .track_focus(&category_search_focus)
                    .appearance(false)
                    .overlay_closable(true)
                    .on_open_change(move |open, window, cx| {
                        let open = *open;
                        let category_open_view = category_open_view.clone();
                        let category_close_focus = category_close_focus.clone();
                        window.defer(cx, move |window, cx| {
                            let _ = category_open_view.update(cx, |view, cx| {
                                view.category_popover_open = open;
                                if open {
                                    let choices = view.category_choice_ids(cx);
                                    view.category_highlighted_index = view
                                        .selected_category_id
                                        .as_deref()
                                        .and_then(|selected| {
                                            choices.iter().position(|choice| {
                                                choice.as_deref().is_some_and(|category| {
                                                    category.eq_ignore_ascii_case(selected)
                                                })
                                            })
                                        })
                                        .unwrap_or(0);
                                    view.category_scroll
                                        .scroll_to_item(view.category_highlighted_index);
                                }
                                cx.notify();
                            });
                            if !open {
                                category_close_focus.focus(window);
                            }
                        });
                    })
                    .trigger(
                        Button::new("download-new-category-trigger-button")
                            .debug_selector(|| "download-new-category-trigger".into())
                            .label(selected_category_label)
                            .small()
                            .outline()
                            .w_full()
                            .h(px(CONTROL_HEIGHT_INPUT))
                            .rounded(px(RADIUS_CONTROL)),
                    )
                    .content(move |_, _, _| {
                        let mut rows = Vec::with_capacity(categories.len() + 1);
                        if category_query.is_empty() {
                            let automatic_view = category_menu_view.clone();
                            let automatic_return_focus = category_return_focus.clone();
                            rows.push(
                                div()
                                    .id("download-new-category-automatic")
                                    .w_full()
                                    .h(px(38.0))
                                    .px(px(SPACING_8))
                                    .rounded(px(RADIUS_CONTROL))
                                    .bg(category_choice_background(
                                        theme,
                                        selected_category_id_for_menu.is_none(),
                                        category_highlighted_index == 0,
                                    ))
                                    .cursor_pointer()
                                    .flex()
                                    .flex_col()
                                    .justify_center()
                                    .child(
                                        div()
                                            .text_size(px(TEXT_BODY))
                                            .line_height(px(LINE_HEIGHT_BODY))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(theme.text)
                                            .child("自动匹配"),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(TEXT_LABEL))
                                            .line_height(px(LINE_HEIGHT_LABEL))
                                            .text_color(theme.text_muted)
                                            .child("按文件名、扩展名和 URL 规则选择"),
                                    )
                                    .on_click(move |_, window, cx| {
                                        let automatic_view = automatic_view.clone();
                                        let automatic_return_focus =
                                            automatic_return_focus.clone();
                                        window.defer(cx, move |window, cx| {
                                            let _ = automatic_view.update(cx, |view, cx| {
                                                view.select_category_choice(None);
                                                cx.notify();
                                            });
                                            automatic_return_focus.focus(window);
                                        });
                                    })
                                    .into_any_element(),
                            );
                        }
                        let category_offset = rows.len();
                        rows.extend(
                            categories
                                .iter()
                                .enumerate()
                                .filter(|(_, category)| {
                                    category_matches_query(category, &category_query)
                                })
                                .enumerate()
                                .map(|(visible_index, (index, category))| {
                                let category_id = category.id.clone();
                                let selected =
                                    selected_category_id_for_menu.as_deref().is_some_and(
                                        |selected| selected.eq_ignore_ascii_case(&category_id),
                                    );
                                let highlighted = category_highlighted_index
                                    == category_offset + visible_index;
                                let category_name = category.name.clone();
                                let directory_label = if category.save_directory.is_empty() {
                                    "使用默认保存目录".to_owned()
                                } else {
                                    category.save_directory.clone()
                                };
                                let directory_tooltip = directory_label.clone();
                                let view = category_menu_view.clone();
                                let category_return_focus = category_return_focus.clone();
                                div()
                                        .id(("download-new-category-option", index))
                                        .debug_selector(move || {
                                            format!("download-new-category-option-{index}")
                                        })
                                        .w_full()
                                        .h(px(38.0))
                                        .px(px(SPACING_8))
                                        .rounded(px(RADIUS_CONTROL))
                                        .bg(category_choice_background(
                                            theme,
                                            selected,
                                            highlighted,
                                        ))
                                        .tooltip(move |window, cx| {
                                            let directory_tooltip = directory_tooltip.clone();
                                            Tooltip::element(move |_, _| {
                                                div()
                                                    .debug_selector(move || {
                                                        format!(
                                                            "download-new-category-option-tooltip-{index}"
                                                        )
                                                    })
                                                    .max_w(px(720.0))
                                                    .whitespace_normal()
                                                    .font_family(UI_MONO_FONT_FAMILY)
                                                    .text_size(px(TEXT_LABEL))
                                                    .line_height(px(LINE_HEIGHT_LABEL))
                                                    .child(directory_tooltip.clone())
                                            })
                                            .build(window, cx)
                                        })
                                        .cursor_pointer()
                                        .flex()
                                        .flex_col()
                                        .justify_center()
                                        .child(
                                            div()
                                                .debug_selector(move || {
                                                    format!(
                                                        "download-new-category-option-title-{index}"
                                                    )
                                                })
                                                .w_full()
                                                .min_w_0()
                                                .truncate()
                                                .text_size(px(TEXT_BODY))
                                                .line_height(px(LINE_HEIGHT_BODY))
                                                .font_weight(FontWeight::MEDIUM)
                                                .text_color(theme.text)
                                                .child(category_name),
                                        )
                                        .child(
                                            div()
                                                .debug_selector(move || {
                                                    format!(
                                                        "download-new-category-option-directory-{index}"
                                                    )
                                                })
                                                .w_full()
                                                .min_w_0()
                                                .truncate()
                                                .text_size(px(TEXT_LABEL))
                                                .line_height(px(LINE_HEIGHT_LABEL))
                                                .text_color(theme.text_muted)
                                                .child(directory_label),
                                        )
                                        .on_click(move |_, window, cx| {
                                            let category_id = category_id.clone();
                                            let view = view.clone();
                                            let category_return_focus =
                                                category_return_focus.clone();
                                            window.defer(cx, move |window, cx| {
                                                let _ = view.update(cx, |view, cx| {
                                                    view.select_category_choice(Some(category_id));
                                                    cx.notify();
                                                });
                                                category_return_focus.focus(window);
                                            });
                                        })
                                        .into_any_element()
                            }),
                        );
                        if rows.is_empty() {
                            rows.push(
                                div()
                                    .w_full()
                                    .h(px(42.0))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .text_size(px(TEXT_LABEL))
                                    .text_color(theme.text_muted)
                                    .child("没有匹配的分类")
                                    .into_any_element(),
                            );
                        }
                        popover_shell(
                            theme,
                            "download-new-category-popover-content",
                            320.0,
                            240.0,
                            div()
                                .size_full()
                                .p(px(SPACING_6))
                                .flex()
                                .flex_col()
                                .gap(px(SPACING_4))
                                .child(field_shell(
                                    theme,
                                    "download-new-category-search",
                                    Input::new(&category_search)
                                        .appearance(false)
                                        .bordered(false)
                                        .small(),
                                ))
                                .child(list_box(
                                    "download-new-category-list",
                                    188.0,
                                    category_scroll.clone(),
                                    rows,
                                ))
                                .into_any_element(),
                        )
                    }),
            )
            .into_any_element();
        let footer_leading_actions = if confirmed {
            div()
                .debug_selector(|| "download-new-footer-leading-actions".into())
                .flex()
                .items_center()
                .gap(px(6.0))
                .child(schedule_control)
                .child(footer_actions)
                .into_any_element()
        } else {
            footer_actions
        };
        let footer_submission_actions = confirmed.then(|| {
            div()
                .debug_selector(|| "download-new-footer-submission-actions".into())
                .flex()
                .items_center()
                .gap(px(6.0))
                .child(
                    div()
                        .debug_selector(|| "download-add-to-queue".into())
                        .child(new_download_footer_action(
                            NewDownloadFooterAction::QueueOnly,
                            theme,
                            cx,
                            if submitting {
                                "正在添加…"
                            } else {
                                "添加"
                            },
                            submitting,
                            cx.listener(|this, _, window, cx| {
                                this.submit(DownloadSubmitMode::QueueOnly, window, cx);
                            }),
                        )),
                )
                .child(div().debug_selector(|| "download-start-now".into()).child(
                    new_download_footer_action(
                        NewDownloadFooterAction::StartNow,
                        theme,
                        cx,
                        if submitting {
                            "正在添加…"
                        } else {
                            "下载"
                        },
                        submitting,
                        cx.listener(|this, _, window, cx| {
                            this.submit(DownloadSubmitMode::StartNow, window, cx);
                        }),
                    ),
                ))
                .into_any_element()
        });
        let body = if let Some(probe) = probe {
            let is_blob = self.external_blob.is_some();
            let media_options_panel = match probe.media.as_ref() {
                Some(nxtools_download::DownloadMediaProbe::Hls(hls)) if hls.is_master => {
                    Some(hls_quality_options(
                        theme,
                        &hls.variants,
                        self.selected_hls_variant.unwrap_or(0),
                        cx.entity().downgrade(),
                    ))
                }
                Some(nxtools_download::DownloadMediaProbe::Dash(dash)) => Some(dash_track_options(
                    theme,
                    dash,
                    self.selected_dash_video_track,
                    self.selected_dash_audio_track,
                    cx.entity().downgrade(),
                )),
                _ => None,
            };
            let size = match probe.media.as_ref() {
                Some(nxtools_download::DownloadMediaProbe::Hls(hls)) if hls.is_master => {
                    format!("{} 个清晰度", hls.variants.len())
                }
                Some(nxtools_download::DownloadMediaProbe::Hls(_)) => "HLS 媒体列表".into(),
                Some(nxtools_download::DownloadMediaProbe::Dash(dash)) => format!(
                    "{} 视频 · {} 音频",
                    dash.video_tracks.len(),
                    dash.audio_tracks.len()
                ),
                None => probe
                    .total_bytes
                    .map(format_download_size)
                    .unwrap_or_else(|| "大小未知".into()),
            };
            div()
                .debug_selector(|| "download-new-form-grid".into())
                .flex_1()
                .px(px(16.0))
                .py(px(10.0))
                .flex()
                .flex_col()
                .gap(px(8.0))
                .child(labeled_field(
                    theme,
                    "下载地址",
                    None,
                    &self.url,
                    "download-new-url-field",
                ))
                .child(
                    div()
                        .flex()
                        .items_start()
                        .gap(px(12.0))
                        .child(
                            div()
                                .min_w_0()
                                .flex_1()
                                .flex()
                                .flex_col()
                                .gap(px(SPACING_8))
                                .child(labeled_control(
                                    theme,
                                    "保存目录",
                                    None,
                                    path_picker_shell(
                                        "download-new-directory-control",
                                        field_shell(
                                            theme,
                                            "download-new-directory-input",
                                            Input::new(&self.directory)
                                                .appearance(false)
                                                .bordered(false)
                                                .focus_bordered(false),
                                        ),
                                        div()
                                            .id("download-new-directory-picker-wrapper")
                                            .debug_selector(|| {
                                                "download-new-directory-picker".into()
                                            })
                                            .size(px(CONTROL_HEIGHT_ACTION))
                                            .tooltip(|window, cx| {
                                                Tooltip::new("选择下载目录").build(window, cx)
                                            })
                                            .child(
                                                Button::new("download-pick-directory")
                                                    .icon(Icon::new(MaterialIcon::Folder))
                                                    .small()
                                                    .outline()
                                                    .size(px(CONTROL_HEIGHT_ACTION))
                                                    .px(px(0.0))
                                                    .rounded(px(RADIUS_CONTROL))
                                                    .on_click(move |_, _, cx| {
                                                        let application = application.clone();
                                                        let current_directory =
                                                            current_directory.clone();
                                                        cx.defer(move |cx| {
                                                            application.update(
                                                                cx,
                                                                |application, cx| {
                                                                    application
                                                                        .request_new_download_directory(
                                                                            current_directory,
                                                                            cx,
                                                                        );
                                                                },
                                                            );
                                                        });
                                                    }),
                                            )
                                            .into_any_element(),
                                    ),
                                    "download-new-directory-field",
                                ))
                                .child(
                                    div()
                                        .flex()
                                        .items_start()
                                        .gap(px(8.0))
                                        .child(
                                            div()
                                                .debug_selector(|| {
                                                    "download-new-file-name-field".into()
                                                })
                                                .min_w_0()
                                                .flex_1()
                                                .child(labeled_field(
                                                    theme,
                                                    "文件名",
                                                    None,
                                                    &self.file_name,
                                                    "download-new-file-name-control",
                                                )),
                                        )
                                        .child(
                                            div()
                                                .debug_selector(|| {
                                                    "download-new-category-field".into()
                                                })
                                                .min_w_0()
                                                .flex_1()
                                                .child(labeled_control(
                                                    theme,
                                                    "分类",
                                                    None,
                                                    category_control,
                                                    "download-new-category-control",
                                                )),
                                        ),
                                ),
                        )
                        .child(new_download_probe_inspector(theme, &probe, is_blob, size)),
                )
                .when_some(media_options_panel, |body, panel| body.child(panel))
                .when_some(self.error.clone(), |body, error| {
                    body.child(download_inline_error(theme, error))
                })
                .into_any_element()
        } else {
            div()
                .flex_1()
                .px(px(16.0))
                .py(px(10.0))
                .flex()
                .flex_col()
                .gap(px(8.0))
                .child(labeled_field(
                    theme,
                    "下载地址",
                    None,
                    &self.url,
                    "download-new-url-field",
                ))
                .child(
                    div()
                        .debug_selector(|| "download-new-probe-guidance".into())
                        .h(px(LINE_HEIGHT_BODY))
                        .min_w_0()
                        .flex()
                        .items_center()
                        .gap(px(SPACING_6))
                        .child(
                            Icon::new(if probing {
                                MaterialIcon::ProgressActivity
                            } else {
                                MaterialIcon::Info
                            })
                            .size(px(ICON_GLYPH_SMALL))
                            .text_color(if probing {
                                theme.accent
                            } else {
                                theme.icon_muted
                            }),
                        )
                        .child(
                            div()
                                .min_w_0()
                                .flex_1()
                                .truncate()
                                .text_size(px(TEXT_LABEL))
                                .line_height(px(LINE_HEIGHT_LABEL))
                                .text_color(if probing {
                                    theme.text_secondary
                                } else {
                                    theme.text_muted
                                })
                                .child(if probing {
                                    "正在读取远端文件名、大小与续传能力…"
                                } else {
                                    "输入 HTTP/HTTPS 链接，继续后自动获取文件信息"
                                }),
                        ),
                )
                .when_some(self.error.clone(), |body, error| {
                    body.child(download_inline_error(theme, error))
                })
                .into_any_element()
        };
        div()
            .id("download-new-root")
            .debug_selector(|| "download-new-root".into())
            .size_full()
            .bg(theme.canvas)
            .text_color(theme.text)
            .flex()
            .flex_col()
            .child(
                div()
                    .debug_selector(|| "download-new-titlebar".into())
                    .h(px(34.0))
                    .pl(px(14.0))
                    .border_b_1()
                    .border_color(theme.border_subtle)
                    .bg(theme.surface)
                    .window_control_area(WindowControlArea::Drag)
                    .flex()
                    .items_center()
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .items_center()
                            .gap(px(7.0))
                            .text_size(px(TEXT_BODY))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(
                                Icon::new(MaterialIcon::Download)
                                    .size(px(15.0))
                                    .text_color(theme.accent),
                            )
                            .child("新建下载"),
                    )
                    .child(download_window_close_button(theme)),
            )
            .child(body)
            .child(
                div()
                    .id("download-new-footer")
                    .debug_selector(|| "download-new-footer".into())
                    .h(px(44.0))
                    .px(px(16.0))
                    .border_t_1()
                    .border_color(theme.border_subtle)
                    .bg(theme.surface)
                    .flex()
                    .items_center()
                    .child(footer_leading_actions)
                    .child(div().flex_1())
                    .when_some(footer_submission_actions, |footer, actions| {
                        footer.child(actions).child(div().w(px(6.0)))
                    })
                    .child(
                        div()
                            .debug_selector(|| "download-new-cancel-action".into())
                            .child(new_download_footer_action(
                                NewDownloadFooterAction::Cancel,
                                theme,
                                cx,
                                "取消",
                                submitting,
                                |_, window, _| window.remove_window(),
                            )),
                    ),
            )
    }
}

pub(crate) const NEW_DOWNLOAD_PROBE_INSPECTOR_WIDTH: f32 = 120.0;
pub(crate) const NEW_DOWNLOAD_PROBE_INSPECTOR_HEIGHT: f32 = 112.0;

pub(crate) fn new_download_probe_inspector(
    theme: Theme,
    probe: &DownloadProbe,
    is_blob: bool,
    size_label: String,
) -> AnyElement {
    let status = if is_blob {
        "Blob 已接收"
    } else if probe.media.is_some() {
        match probe.media.as_ref() {
            Some(nxtools_download::DownloadMediaProbe::Dash(_)) => "DASH 已解析",
            _ => "HLS 已解析",
        }
    } else {
        "链接可用"
    };
    let capability = if is_blob {
        probe
            .content_type
            .clone()
            .unwrap_or_else(|| "类型未知".into())
    } else if let Some(nxtools_download::DownloadMediaProbe::Hls(hls)) = probe.media.as_ref() {
        if hls.is_master {
            "请选择清晰度".into()
        } else {
            "媒体播放列表".into()
        }
    } else if matches!(
        probe.media.as_ref(),
        Some(nxtools_download::DownloadMediaProbe::Dash(_))
    ) {
        "请选择视频与音频轨".into()
    } else if probe.supports_ranges {
        "支持断点续传".into()
    } else {
        "单连接传输".into()
    };

    div()
        .debug_selector(|| "download-new-probe-inspector".into())
        .w(px(NEW_DOWNLOAD_PROBE_INSPECTOR_WIDTH))
        .h(px(NEW_DOWNLOAD_PROBE_INSPECTOR_HEIGHT))
        .flex_none()
        .min_w_0()
        .overflow_hidden()
        .pl(px(SPACING_12))
        .border_l_1()
        .border_color(theme.border_subtle)
        .flex()
        .flex_col()
        .justify_center()
        .gap(px(SPACING_4))
        .child(
            div()
                .debug_selector(|| "download-new-probe-status".into())
                .w_full()
                .h(px(LINE_HEIGHT_LABEL))
                .min_w_0()
                .flex()
                .items_center()
                .gap(px(SPACING_6))
                .child(
                    Icon::new(MaterialIcon::Check)
                        .size(px(14.0))
                        .text_color(theme.success),
                )
                .child(
                    div()
                        .min_w_0()
                        .flex_1()
                        .truncate()
                        .text_size(px(TEXT_LABEL))
                        .line_height(px(LINE_HEIGHT_LABEL))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(theme.success)
                        .child(status),
                ),
        )
        .child(
            div()
                .debug_selector(|| "download-new-probe-size".into())
                .w_full()
                .h(px(LINE_HEIGHT_BODY))
                .truncate()
                .text_size(px(TEXT_BODY))
                .line_height(px(LINE_HEIGHT_BODY))
                .font_weight(FontWeight::MEDIUM)
                .child(size_label),
        )
        .child(
            div()
                .debug_selector(|| "download-new-probe-capability".into())
                .w_full()
                .h(px(LINE_HEIGHT_META))
                .truncate()
                .text_size(px(TEXT_META))
                .line_height(px(LINE_HEIGHT_META))
                .text_color(if is_blob || probe.supports_ranges {
                    theme.success
                } else {
                    theme.text_muted
                })
                .child(capability),
        )
        .into_any_element()
}

pub(crate) fn hls_quality_options(
    theme: Theme,
    variants: &[nxtools_download::HlsVariantProbe],
    selected: usize,
    view: gpui::WeakEntity<NewDownloadView>,
) -> AnyElement {
    let rows = variants.iter().enumerate().map(|(index, variant)| {
        let selected_row = index == selected;
        let view = view.clone();
        radio_list_row(
            theme,
            format!("download-hls-quality-{index}").into(),
            hls_variant_primary_label(variant),
            hls_variant_detail_label(variant),
            selected_row,
            move |_, _, cx| {
                let _ = view.update(cx, |view, cx| {
                    view.selected_hls_variant = Some(index);
                    view.error = None;
                    cx.notify();
                });
            },
        )
    });
    div()
        .debug_selector(|| "download-hls-quality-options".into())
        .w_full()
        .flex()
        .flex_col()
        .gap(px(SPACING_4))
        .child(
            div()
                .flex()
                .items_center()
                .child(
                    div()
                        .flex_1()
                        .text_size(px(TEXT_LABEL))
                        .line_height(px(LINE_HEIGHT_LABEL))
                        .text_color(theme.text_muted)
                        .child("HLS 清晰度"),
                )
                .child(
                    div()
                        .text_size(px(TEXT_LABEL))
                        .line_height(px(LINE_HEIGHT_LABEL))
                        .text_color(theme.text_muted)
                        .child("支持未加密点播分片传输"),
                ),
        )
        .child(
            div()
                .max_h(px(SETTINGS_ROW_MIN_HEIGHT * 3.0))
                .overflow_y_scrollbar()
                .flex()
                .flex_col()
                .children(rows),
        )
        .into_any_element()
}

pub(crate) fn hls_variant_detail_label(variant: &nxtools_download::HlsVariantProbe) -> String {
    let bandwidth = variant.average_bandwidth.unwrap_or(variant.bandwidth);
    let codec = if variant.codecs.is_empty() {
        "编码未知"
    } else {
        variant.codecs.as_str()
    };
    let mut detail = format!("{codec} · {:.2} Mbps", bandwidth as f64 / 1_000_000.0);
    if let Some(frame_rate_milli) = variant.frame_rate_milli {
        detail.push_str(&format!(" · {:.2} fps", frame_rate_milli as f64 / 1000.0));
    }
    detail
}

pub(crate) fn hls_variant_primary_label(variant: &nxtools_download::HlsVariantProbe) -> String {
    if let Some((width, height)) = variant.resolution {
        return format!("{width}×{height}");
    }

    let codecs = variant.codecs.to_ascii_lowercase();
    if ["avc1", "avc3", "hev1", "hvc1", "vp09", "av01"]
        .iter()
        .any(|codec| codecs.contains(codec))
    {
        "视频流".into()
    } else if ["mp4a", "ac-3", "ec-3", "opus"]
        .iter()
        .any(|codec| codecs.contains(codec))
    {
        "音频流".into()
    } else {
        "未标注分辨率".into()
    }
}

pub(crate) fn dash_track_options(
    theme: Theme,
    dash: &nxtools_download::DashProbe,
    selected_video: Option<usize>,
    selected_audio: Option<usize>,
    view: gpui::WeakEntity<NewDownloadView>,
) -> AnyElement {
    let video_rows = dash.video_tracks.iter().enumerate().map(|(index, track)| {
        dash_track_option_row(
            theme,
            DashTrackKind::Video,
            index,
            track,
            selected_video == Some(index),
            view.clone(),
        )
    });
    let audio_rows = dash.audio_tracks.iter().enumerate().map(|(index, track)| {
        dash_track_option_row(
            theme,
            DashTrackKind::Audio,
            index,
            track,
            selected_audio == Some(index),
            view.clone(),
        )
    });
    let column = |label: &'static str, rows: Vec<AnyElement>| {
        div()
            .min_w_0()
            .flex_1()
            .flex()
            .flex_col()
            .gap(px(SPACING_4))
            .child(
                div()
                    .text_size(px(TEXT_LABEL))
                    .line_height(px(LINE_HEIGHT_LABEL))
                    .text_color(theme.text_muted)
                    .child(label),
            )
            .child(
                div()
                    .max_h(px(SETTINGS_ROW_MIN_HEIGHT * 3.0))
                    .overflow_y_scrollbar()
                    .flex()
                    .flex_col()
                    .children(rows),
            )
    };
    div()
        .debug_selector(|| "download-dash-track-options".into())
        .w_full()
        .flex()
        .flex_col()
        .gap(px(SPACING_4))
        .child(
            div()
                .flex()
                .items_center()
                .child(
                    div()
                        .flex_1()
                        .text_size(px(TEXT_LABEL))
                        .line_height(px(LINE_HEIGHT_LABEL))
                        .text_color(theme.text_muted)
                        .child("DASH 轨道"),
                )
                .child(
                    div()
                        .text_size(px(TEXT_LABEL))
                        .line_height(px(LINE_HEIGHT_LABEL))
                        .text_color(theme.text_muted)
                        .child("音频可再次点击取消 · 双轨用 FFmpeg 无损封装"),
                ),
        )
        .child(
            div()
                .w_full()
                .flex()
                .gap(px(8.0))
                .child(column("视频", video_rows.collect()))
                .child(column("音频", audio_rows.collect())),
        )
        .into_any_element()
}

#[derive(Clone, Copy)]
pub(crate) enum DashTrackKind {
    Video,
    Audio,
}

impl DashTrackKind {
    pub(crate) fn selector_segment(self) -> &'static str {
        match self {
            Self::Video => "video",
            Self::Audio => "audio",
        }
    }

    pub(crate) fn primary_label(self, track: &nxtools_download::DashTrackProbe) -> String {
        match self {
            Self::Video => track.resolution.map_or_else(
                || "视频轨".into(),
                |(width, height)| format!("{width}×{height}"),
            ),
            Self::Audio => track.language.clone().unwrap_or_else(|| "默认音轨".into()),
        }
    }

    pub(crate) fn select(self, view: &mut NewDownloadView, index: usize) {
        match self {
            Self::Video => view.selected_dash_video_track = Some(index),
            Self::Audio => {
                view.selected_dash_audio_track =
                    (view.selected_dash_audio_track != Some(index)).then_some(index);
            }
        }
    }
}

pub(crate) fn dash_track_option_row(
    theme: Theme,
    kind: DashTrackKind,
    index: usize,
    track: &nxtools_download::DashTrackProbe,
    selected: bool,
    view: gpui::WeakEntity<NewDownloadView>,
) -> AnyElement {
    let label = kind.primary_label(track);
    let detail = if track.codecs.is_empty() {
        format!("{:.2} Mbps", track.bandwidth as f64 / 1_000_000.0)
    } else {
        format!(
            "{} · {:.2} Mbps",
            track.codecs,
            track.bandwidth as f64 / 1_000_000.0
        )
    };
    radio_list_row(
        theme,
        format!("download-dash-{}-track-{index}", kind.selector_segment()).into(),
        label,
        detail,
        selected,
        move |_, _, cx| {
            let _ = view.update(cx, |view, cx| {
                kind.select(view, index);
                view.error = None;
                cx.notify();
            });
        },
    )
}

pub(crate) fn delayed_start_calendar(
    theme: Theme,
    month: NaiveDate,
    selected_date: NaiveDate,
    today: NaiveDate,
    view: gpui::WeakEntity<NewDownloadView>,
) -> AnyElement {
    let previous_view = view.clone();
    let next_view = view.clone();
    let weekdays = ["日", "一", "二", "三", "四", "五", "六"];
    let days = delayed_start_calendar_days(month);
    div()
        .w_full()
        .flex()
        .flex_col()
        .gap(px(2.0))
        .child(
            div()
                .h(px(28.0))
                .flex()
                .items_center()
                .child(
                    div()
                        .debug_selector(|| "download-new-schedule-previous-month-action".into())
                        .child(
                            icon_action(
                                "download-new-schedule-previous-month",
                                MaterialIcon::ChevronLeft,
                                "上个月",
                            )
                            .ghost()
                            .on_click(move |_, window, cx| {
                                let previous_view = previous_view.clone();
                                window.defer(cx, move |_, cx| {
                                    let _ = previous_view.update(cx, |view, cx| {
                                        view.scheduled_month =
                                            shift_calendar_month(view.scheduled_month, -1);
                                        cx.notify();
                                    });
                                });
                            }),
                        ),
                )
                .child(
                    div()
                        .debug_selector(|| "download-new-schedule-month-title".into())
                        .flex_1()
                        .text_center()
                        .text_size(px(TEXT_BODY))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(theme.text)
                        .child(delayed_start_calendar_title(month)),
                )
                .child(
                    div()
                        .debug_selector(|| "download-new-schedule-next-month-action".into())
                        .child(
                            icon_action(
                                "download-new-schedule-next-month",
                                MaterialIcon::ChevronRight,
                                "下个月",
                            )
                            .ghost()
                            .on_click(move |_, window, cx| {
                                let next_view = next_view.clone();
                                window.defer(cx, move |_, cx| {
                                    let _ = next_view.update(cx, |view, cx| {
                                        view.scheduled_month =
                                            shift_calendar_month(view.scheduled_month, 1);
                                        cx.notify();
                                    });
                                });
                            }),
                        ),
                ),
        )
        .child(
            div()
                .h(px(16.0))
                .flex()
                .children(weekdays.into_iter().map(|weekday| {
                    div()
                        .w(px(40.0))
                        .h_full()
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(px(TEXT_META))
                        .text_color(theme.text_muted)
                        .child(weekday)
                })),
        )
        .children((0..6).map(|row| {
            div()
                .h(px(22.0))
                .flex()
                .children(days[row * 7..row * 7 + 7].iter().copied().map(|date| {
                    let disabled = date < today;
                    let selected = date == selected_date;
                    let current_month = date.month() == month.month();
                    let is_today = date == today;
                    let day_view = view.clone();
                    div()
                        .id(("download-new-schedule-day", date.num_days_from_ce() as u64))
                        .when(date == selected_date + ChronoDuration::days(1), |cell| {
                            cell.debug_selector(|| "download-new-schedule-next-day".into())
                        })
                        .w(px(40.0))
                        .h(px(22.0))
                        .rounded(px(RADIUS_CONTROL))
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(px(TEXT_LABEL))
                        .text_color(if selected {
                            theme.accent_text
                        } else if current_month {
                            theme.text_secondary
                        } else {
                            theme.text_muted
                        })
                        .when(disabled, |cell| cell.opacity(0.32))
                        .when(is_today && !selected, |cell| {
                            cell.border_1().border_color(theme.border_active)
                        })
                        .when(selected, |cell| cell.bg(theme.accent))
                        .when(!disabled, |cell| {
                            cell.cursor_pointer()
                                .hover(|cell| {
                                    if selected {
                                        cell.bg(theme.accent)
                                    } else {
                                        cell.bg(theme.surface_hover)
                                    }
                                })
                                .on_click(move |_, window, cx| {
                                    let day_view = day_view.clone();
                                    window.defer(cx, move |_, cx| {
                                        let _ = day_view.update(cx, |view, cx| {
                                            view.scheduled_date = date;
                                            view.scheduled_month = calendar_month_start(date);
                                            cx.notify();
                                        });
                                    });
                                })
                        })
                        .child(date.day().to_string())
                }))
        }))
        .into_any_element()
}

pub(crate) fn download_inline_error(theme: Theme, error: String) -> AnyElement {
    inline_error(theme, "download-inline-error", error)
}

pub(crate) fn format_download_size(bytes: u64) -> String {
    const KIB: f64 = 1024.0;
    const MIB: f64 = KIB * 1024.0;
    const GIB: f64 = MIB * 1024.0;
    let bytes = bytes as f64;
    if bytes >= GIB {
        format!("{:.2} GB", bytes / GIB)
    } else if bytes >= MIB {
        format!("{:.1} MB", bytes / MIB)
    } else if bytes >= KIB {
        format!("{:.1} KB", bytes / KIB)
    } else {
        format!("{bytes:.0} B")
    }
}

pub(crate) fn ffmpeg_install_phase_label(phase: FfmpegInstallPhase) -> &'static str {
    match phase {
        FfmpegInstallPhase::Connecting => "正在连接官方来源",
        FfmpegInstallPhase::Downloading => "正在下载",
        FfmpegInstallPhase::Verifying => "正在校验 SHA-256",
        FfmpegInstallPhase::Extracting => "正在安全解压",
        FfmpegInstallPhase::Validating => "正在验证 FFmpeg",
        FfmpegInstallPhase::Publishing => "正在配置",
    }
}

pub(crate) fn new_download_window_size(confirmed: bool) -> gpui::Size<gpui::Pixels> {
    if confirmed {
        size(px(560.0), px(278.0))
    } else {
        size(px(560.0), px(176.0))
    }
}

pub(crate) fn new_download_window_size_for_probe(
    probe: &DownloadProbe,
) -> gpui::Size<gpui::Pixels> {
    match probe.media.as_ref() {
        Some(nxtools_download::DownloadMediaProbe::Hls(hls)) if hls.is_master => {
            let rows_height = bounded_media_rows_height(
                hls.variants.len(),
                SETTINGS_ROW_MIN_HEIGHT,
                0.0,
                SETTINGS_ROW_MIN_HEIGHT * 3.0,
            );
            size(px(560.0), px(260.0 + 16.0 + rows_height))
        }
        Some(nxtools_download::DownloadMediaProbe::Dash(dash)) => {
            let row_count = dash.video_tracks.len().max(dash.audio_tracks.len());
            let rows_height = bounded_media_rows_height(
                row_count,
                SETTINGS_ROW_MIN_HEIGHT,
                0.0,
                SETTINGS_ROW_MIN_HEIGHT * 3.0,
            );
            size(px(560.0), px(260.0 + 16.0 + 15.0 + rows_height))
        }
        _ => new_download_window_size(true),
    }
}

pub(crate) fn bounded_media_rows_height(
    row_count: usize,
    row_height: f32,
    row_gap: f32,
    maximum_height: f32,
) -> f32 {
    if row_count == 0 {
        return 0.0;
    }
    (row_count as f32 * row_height + row_count.saturating_sub(1) as f32 * row_gap)
        .min(maximum_height)
}

pub(crate) fn selected_download_request_media(
    probe: Option<&DownloadProbe>,
    selected_hls_variant: Option<usize>,
    selected_dash_video_track: Option<usize>,
    selected_dash_audio_track: Option<usize>,
) -> Result<Option<nxtools_download::DownloadRequestMedia>, String> {
    match probe.and_then(|probe| probe.media.as_ref()) {
        Some(nxtools_download::DownloadMediaProbe::Hls(hls)) => {
            let playlist_url = if hls.is_master {
                hls.variants
                    .get(selected_hls_variant.ok_or("请选择 HLS 清晰度")?)
                    .map(|variant| variant.playlist_url.clone())
                    .ok_or("所选 HLS 清晰度已失效，请重新探测")?
            } else {
                probe
                    .map(|probe| probe.final_url.clone())
                    .ok_or("HLS 探测结果已失效，请重新探测")?
            };
            Ok(Some(nxtools_download::DownloadRequestMedia::Hls(
                nxtools_download::HlsDownloadRequest { playlist_url },
            )))
        }
        Some(nxtools_download::DownloadMediaProbe::Dash(dash)) => {
            let video_representation_id = selected_dash_video_track
                .map(|index| {
                    dash.video_tracks
                        .get(index)
                        .map(|track| track.representation_id.clone())
                        .ok_or("所选 DASH 视频轨已失效，请重新探测")
                })
                .transpose()?;
            let audio_representation_id = selected_dash_audio_track
                .map(|index| {
                    dash.audio_tracks
                        .get(index)
                        .map(|track| track.representation_id.clone())
                        .ok_or("所选 DASH 音频轨已失效，请重新探测")
                })
                .transpose()?;
            if video_representation_id.is_none() && audio_representation_id.is_none() {
                return Err("DASH 至少需要选择一条视频轨或音频轨".into());
            }
            Ok(Some(nxtools_download::DownloadRequestMedia::Dash(
                nxtools_download::DashDownloadRequest {
                    manifest_url: probe
                        .map(|probe| probe.final_url.clone())
                        .ok_or("DASH 探测结果已失效，请重新探测")?,
                    video_representation_id,
                    audio_representation_id,
                },
            )))
        }
        None => Ok(None),
    }
}

pub(crate) fn resolved_probe_file_name(probe: &DownloadProbe) -> Option<String> {
    let file_name = probe.file_name.as_deref()?;
    if probe.media.is_none() {
        return Some(file_name.to_owned());
    }
    let stem = Path::new(file_name).file_stem()?.to_string_lossy();
    (!stem.is_empty()).then(|| match probe.media {
        Some(nxtools_download::DownloadMediaProbe::Hls(_)) => format!("{stem}.ts"),
        Some(nxtools_download::DownloadMediaProbe::Dash(_)) => format!("{stem}.mp4"),
        None => file_name.to_owned(),
    })
}

pub(crate) fn should_replace_media_manifest_name(
    current_file_name: Option<&str>,
    probe: &DownloadProbe,
) -> bool {
    let expected_extension = match probe.media.as_ref() {
        Some(nxtools_download::DownloadMediaProbe::Hls(_)) => "m3u8",
        Some(nxtools_download::DownloadMediaProbe::Dash(_)) => "mpd",
        None => return false,
    };
    probe.media.is_some()
        && current_file_name == probe.file_name.as_deref()
        && current_file_name.is_some_and(|file_name| {
            Path::new(file_name)
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case(expected_extension))
        })
}

pub(crate) fn initial_new_download_window_size(
    initial_source: Option<&NewDownloadInitialSource>,
) -> gpui::Size<gpui::Pixels> {
    new_download_window_size(matches!(
        initial_source,
        Some(NewDownloadInitialSource::Blob(_))
    ))
}

pub(crate) fn new_download_window_options(window_bounds: WindowBounds) -> WindowOptions {
    WindowOptions {
        window_bounds: Some(window_bounds),
        titlebar: None,
        focus: true,
        show: true,
        kind: WindowKind::Normal,
        is_movable: true,
        is_resizable: false,
        is_minimizable: false,
        display_id: None,
        window_background: WindowBackgroundAppearance::Opaque,
        app_id: Some("NxTools.NewDownload".into()),
        window_min_size: Some(new_download_window_size(false)),
        window_decorations: Some(WindowDecorations::Client),
        tabbing_identifier: None,
    }
}
