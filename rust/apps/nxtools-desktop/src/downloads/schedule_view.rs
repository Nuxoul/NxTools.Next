use super::*;

pub(crate) struct DownloadScheduleView {
    pub(crate) theme: Theme,
    pub(crate) application: Entity<DownloadApplication>,
    pub(crate) start_time: Entity<InputState>,
    pub(crate) stop_time: Entity<InputState>,
    pub(crate) days: HashSet<DownloadWeekday>,
    pub(crate) enabled_start: bool,
    pub(crate) enabled_stop: bool,
    pub(crate) queue_running: bool,
    pub(crate) saving: bool,
    pub(crate) error: Option<String>,
}

impl DownloadScheduleView {
    pub(crate) fn new(
        application: Entity<DownloadApplication>,
        queue: Option<DownloadQueue>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let daily = queue
            .as_ref()
            .and_then(|queue| queue.schedule.as_ref())
            .map(|schedule| schedule.daily.clone());
        let start_minute = daily
            .as_ref()
            .map_or(2 * 60 + 30, |daily| daily.start_minute);
        let stop_minute = daily
            .as_ref()
            .map_or(7 * 60 + 30, |daily| daily.stop_minute);
        let start_time = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(format_clock_minute(start_minute))
                .placeholder("HH:MM")
        });
        let stop_time = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(format_clock_minute(stop_minute))
                .placeholder("HH:MM")
        });
        let days = daily.as_ref().map_or_else(
            || DownloadWeekday::ALL.into_iter().collect(),
            |daily| daily.days.iter().copied().collect(),
        );
        Self {
            theme: Theme::zed_dark(),
            application,
            start_time,
            stop_time,
            days,
            enabled_start: daily.as_ref().is_some_and(|daily| daily.enabled_start),
            enabled_stop: daily.as_ref().is_some_and(|daily| daily.enabled_stop),
            queue_running: queue.is_none_or(|queue| queue.is_running),
            saving: false,
            error: None,
        }
    }

    pub(crate) fn save(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if self.saving {
            return;
        }
        let schedule = match build_download_schedule(
            self.enabled_start,
            self.enabled_stop,
            &self.days,
            self.start_time.read(cx).value().trim(),
            self.stop_time.read(cx).value().trim(),
        ) {
            Ok(schedule) => schedule,
            Err(error) => {
                self.error = Some(error);
                cx.notify();
                return;
            }
        };
        self.saving = true;
        self.error = None;
        let application = self.application.clone();
        let view = cx.entity();
        cx.defer(move |cx| {
            let result = application.update(cx, |application, cx| {
                let result = application.set_default_queue_schedule(schedule, cx);
                if result.is_ok() {
                    application.close_schedule_window(cx);
                }
                result
            });
            if let Err(error) = result {
                view.update(cx, |view, cx| {
                    view.saving = false;
                    view.error = Some(error);
                    cx.notify();
                });
            }
        });
    }
}

impl Render for DownloadScheduleView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let mut weekdays = div().flex().items_center().gap(px(SPACING_4));
        for (index, day) in DownloadWeekday::ALL.into_iter().enumerate() {
            let selected = self.days.contains(&day);
            weekdays = weekdays.child(
                div()
                    .id(("download-schedule-day", index))
                    .w(px(38.0))
                    .h(px(26.0))
                    .rounded(px(RADIUS_CONTROL))
                    .border_1()
                    .border_color(if selected {
                        theme.accent
                    } else {
                        theme.border_subtle
                    })
                    .bg(if selected {
                        theme.accent_soft
                    } else {
                        theme.canvas
                    })
                    .cursor_pointer()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(TEXT_LABEL))
                    .line_height(px(LINE_HEIGHT_LABEL))
                    .text_color(if selected {
                        theme.accent_text
                    } else {
                        theme.text_muted
                    })
                    .child(day.short_label())
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if !this.days.insert(day) {
                            this.days.remove(&day);
                        }
                        this.error = None;
                        cx.notify();
                    })),
            );
        }
        let start_control = schedule_trigger_row(
            theme,
            "自动开始",
            "到点后启动等待队列",
            self.enabled_start,
            &self.start_time,
            "download-schedule-start-toggle",
            cx.listener(|this, _, _, cx| {
                this.enabled_start = !this.enabled_start;
                this.error = None;
                cx.notify();
            }),
        );
        let stop_control = schedule_trigger_row(
            theme,
            "自动停止",
            "到点后暂停活动任务",
            self.enabled_stop,
            &self.stop_time,
            "download-schedule-stop-toggle",
            cx.listener(|this, _, _, cx| {
                this.enabled_stop = !this.enabled_stop;
                this.error = None;
                cx.notify();
            }),
        );
        let close_application = self.application.clone();
        let cancel_application = self.application.clone();
        let titlebar = client_titlebar(
            theme,
            "download-schedule-titlebar",
            "download-schedule-close",
            "每日计划",
            move |_, _, cx| {
                let application = close_application.clone();
                cx.defer(move |cx| {
                    application.update(cx, |application, cx| {
                        application.close_schedule_window(cx);
                    });
                });
            },
        );
        let body = div()
            .size_full()
            .px(px(SPACING_12))
            .py(px(SPACING_8))
            .flex()
            .flex_col()
            .gap(px(SPACING_8))
            .child(
                div()
                    .h(px(32.0))
                    .px(px(SPACING_8))
                    .rounded(px(RADIUS_CONTROL))
                    .border_1()
                    .border_color(theme.border_subtle)
                    .bg(theme.surface)
                    .flex()
                    .items_center()
                    .child(
                        div()
                            .flex_1()
                            .text_size(px(TEXT_BODY))
                            .line_height(px(LINE_HEIGHT_BODY))
                            .text_color(theme.text_secondary)
                            .child("默认下载队列"),
                    )
                    .child(
                        div()
                            .text_size(px(TEXT_LABEL))
                            .line_height(px(LINE_HEIGHT_LABEL))
                            .text_color(if self.queue_running {
                                theme.success
                            } else {
                                theme.text_muted
                            })
                            .child(if self.queue_running {
                                "运行中"
                            } else {
                                "计划暂停"
                            }),
                    ),
            )
            .child(
                div()
                    .h(px(44.0))
                    .px(px(SPACING_8))
                    .rounded(px(RADIUS_CONTROL))
                    .border_1()
                    .border_color(theme.border_subtle)
                    .bg(theme.surface)
                    .flex()
                    .items_center()
                    .child(
                        div()
                            .w(px(82.0))
                            .text_size(px(TEXT_BODY))
                            .line_height(px(LINE_HEIGHT_BODY))
                            .text_color(theme.text_secondary)
                            .child("生效星期"),
                    )
                    .child(weekdays),
            )
            .child(start_control)
            .child(stop_control)
            .when_some(self.error.clone(), |body, error| {
                body.child(download_inline_error(theme, error))
            })
            .into_any_element();
        let footer = dialog_footer(
            div()
                .size_full()
                .flex()
                .items_center()
                .gap(px(SPACING_8))
                .child(
                    div()
                        .flex_1()
                        .text_size(px(TEXT_LABEL))
                        .line_height(px(LINE_HEIGHT_LABEL))
                        .text_color(theme.text_muted)
                        .child("本地时间 · 支持跨午夜"),
                )
                .child(
                    action_button("download-schedule-cancel", "取消")
                        .debug_selector(|| "download-schedule-secondary-cancel-action".into())
                        .outline()
                        .disabled(self.saving)
                        .on_click(move |_, _, cx| {
                            let application = cancel_application.clone();
                            cx.defer(move |cx| {
                                application.update(cx, |application, cx| {
                                    application.close_schedule_window(cx);
                                });
                            });
                        }),
                )
                .child(
                    action_button(
                        "download-schedule-save",
                        if self.saving {
                            "正在应用…"
                        } else {
                            "应用"
                        },
                    )
                    .debug_selector(|| "download-schedule-primary-apply-action".into())
                    .custom(primary_button_variant(theme, cx))
                    .disabled(self.saving)
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.save(window, cx);
                    })),
                )
                .into_any_element(),
        );
        dialog_shell(
            theme,
            DialogShellIds {
                root: "download-schedule-dialog-shell",
                titlebar: "download-schedule-dialog-titlebar",
                body: "download-schedule-dialog-body",
                footer: "download-schedule-dialog-footer",
            },
            titlebar,
            body,
            footer,
        )
    }
}

pub(crate) fn schedule_trigger_row(
    theme: Theme,
    title: &'static str,
    description: &'static str,
    enabled: bool,
    input: &Entity<InputState>,
    id: &'static str,
    on_toggle: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
) -> AnyElement {
    div()
        .h(px(44.0))
        .px(px(SPACING_8))
        .rounded(px(RADIUS_CONTROL))
        .border_1()
        .border_color(theme.border_subtle)
        .bg(theme.surface)
        .flex()
        .items_center()
        .child(
            div()
                .flex_1()
                .flex()
                .flex_col()
                .gap(px(SPACING_2))
                .child(
                    div()
                        .text_size(px(TEXT_BODY))
                        .line_height(px(LINE_HEIGHT_BODY))
                        .child(title),
                )
                .child(
                    div()
                        .text_size(px(TEXT_LABEL))
                        .line_height(px(LINE_HEIGHT_LABEL))
                        .text_color(theme.text_muted)
                        .child(description),
                ),
        )
        .child(
            div()
                .id(id)
                .debug_selector(move || id.into())
                .h(px(BADGE_HEIGHT))
                .w(px(50.0))
                .mr(px(SPACING_8))
                .rounded(px(RADIUS_CONTROL))
                .border_1()
                .border_color(if enabled {
                    theme.accent
                } else {
                    theme.border_subtle
                })
                .bg(if enabled {
                    theme.accent_soft
                } else {
                    theme.canvas
                })
                .cursor_pointer()
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(TEXT_LABEL))
                .line_height(px(LINE_HEIGHT_LABEL))
                .text_color(if enabled {
                    theme.accent_text
                } else {
                    theme.text_muted
                })
                .child(if enabled { "开启" } else { "关闭" })
                .on_click(on_toggle),
        )
        .child(if enabled {
            div()
                .w(px(82.0))
                .h(px(28.0))
                .rounded(px(RADIUS_CONTROL))
                .border_1()
                .border_color(theme.border_subtle)
                .bg(theme.canvas)
                .px(px(SPACING_6))
                .flex()
                .items_center()
                .child(
                    Input::new(input)
                        .appearance(false)
                        .bordered(false)
                        .focus_bordered(false),
                )
                .into_any_element()
        } else {
            div()
                .w(px(82.0))
                .text_size(px(TEXT_LABEL))
                .line_height(px(LINE_HEIGHT_LABEL))
                .text_color(theme.text_muted)
                .text_center()
                .child("未启用")
                .into_any_element()
        })
        .into_any_element()
}

pub(crate) fn parse_clock_minute(value: &str) -> Result<u16, &'static str> {
    let Some((hours, minutes)) = value.split_once(':') else {
        return Err("请输入 HH:MM");
    };
    let hours = hours.parse::<u16>().map_err(|_| "请输入 HH:MM")?;
    let minutes = minutes.parse::<u16>().map_err(|_| "请输入 HH:MM")?;
    if hours >= 24 || minutes >= 60 {
        return Err("时间必须在 00:00 到 23:59 之间");
    }
    Ok(hours * 60 + minutes)
}

pub(crate) fn build_download_schedule(
    enabled_start: bool,
    enabled_stop: bool,
    days: &HashSet<DownloadWeekday>,
    start_time: &str,
    stop_time: &str,
) -> Result<Option<DownloadSchedule>, String> {
    if !enabled_start && !enabled_stop {
        return Ok(None);
    }
    if days.is_empty() {
        return Err("至少选择一个生效星期".into());
    }
    let start_minute = if enabled_start {
        parse_clock_minute(start_time).map_err(|error| format!("开始时间：{error}"))?
    } else {
        0
    };
    let stop_minute = if enabled_stop {
        parse_clock_minute(stop_time).map_err(|error| format!("停止时间：{error}"))?
    } else {
        0
    };
    Ok(Some(DownloadSchedule {
        daily: DownloadDailySchedule {
            days: DownloadWeekday::ALL
                .into_iter()
                .filter(|day| days.contains(day))
                .collect(),
            start_minute,
            stop_minute,
            enabled_start,
            enabled_stop,
        },
    }))
}

pub(crate) fn format_clock_minute(value: u16) -> String {
    format!("{:02}:{:02}", value / 60, value % 60)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DelayedStartSelection {
    pub(crate) date: NaiveDate,
    pub(crate) hour: u32,
    pub(crate) minute: u32,
}

impl DelayedStartSelection {
    pub(crate) fn resolve_unix_ms(self, now_unix_ms: i64) -> Result<i64, String> {
        let time = NaiveTime::from_hms_opt(self.hour, self.minute, 0)
            .ok_or("计划时间必须是 00:00 到 23:59")?;
        let resolved = Local
            .from_local_datetime(&self.date.and_time(time))
            .single()
            .ok_or("计划时间在当前本地时区无效或存在歧义，请换一个时间")?;
        let unix_ms = resolved.timestamp_millis();
        if unix_ms <= now_unix_ms {
            return Err("计划时间必须晚于当前时间".into());
        }
        Ok(unix_ms)
    }
}

pub(crate) fn resolve_scheduled_start(
    selection: Option<DelayedStartSelection>,
    now_unix_ms: i64,
) -> Result<Option<i64>, String> {
    selection
        .map(|selection| selection.resolve_unix_ms(now_unix_ms))
        .transpose()
}

pub(crate) fn read_delayed_start_draft(
    date: NaiveDate,
    hour: &Entity<SelectState<Vec<String>>>,
    minute: &Entity<SelectState<Vec<String>>>,
    cx: &App,
) -> Result<DelayedStartSelection, String> {
    let hour = hour
        .read(cx)
        .selected_value()
        .and_then(|value| value.parse().ok())
        .ok_or("请选择计划下载小时")?;
    let minute = minute
        .read(cx)
        .selected_value()
        .and_then(|value| value.parse().ok())
        .ok_or("请选择计划下载分钟")?;
    Ok(DelayedStartSelection { date, hour, minute })
}

pub(crate) fn calendar_month_start(date: NaiveDate) -> NaiveDate {
    NaiveDate::from_ymd_opt(date.year(), date.month(), 1)
        .expect("a chrono date always has a valid first day")
}

pub(crate) fn shift_calendar_month(month: NaiveDate, delta: i32) -> NaiveDate {
    let total_months = month.year() * 12 + month.month0() as i32 + delta;
    let year = total_months.div_euclid(12);
    let month = total_months.rem_euclid(12) as u32 + 1;
    NaiveDate::from_ymd_opt(year, month, 1).expect("normalized year and month are valid")
}

pub(crate) fn delayed_start_calendar_days(month: NaiveDate) -> Vec<NaiveDate> {
    let grid_start = month - ChronoDuration::days(month.weekday().num_days_from_sunday() as i64);
    (0..42)
        .map(|offset| grid_start + ChronoDuration::days(offset))
        .collect()
}

pub(crate) fn delayed_start_calendar_title(month: NaiveDate) -> String {
    format!("{}年{}月", month.year(), month.month())
}
