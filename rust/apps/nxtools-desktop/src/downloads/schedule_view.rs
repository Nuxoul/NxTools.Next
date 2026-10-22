use super::*;
use gpui_kit::component::button::{Toggle, ToggleVariants as _};
use gpui_kit::component::dialog::{DialogContent, DialogFooter};
use gpui_kit::component::form::{Field, Form};
use gpui_kit::component::status_bar::StatusBar;
use gpui_kit::component::switch::Switch;
use gpui_kit::component::time_field::{HourCycle, TimeField, TimeFieldState, TimePrecision};

pub(crate) struct DownloadScheduleView {
    pub(crate) theme: Theme,
    pub(crate) application: Entity<DownloadApplication>,
    pub(crate) start_time: Entity<TimeFieldState>,
    pub(crate) stop_time: Entity<TimeFieldState>,
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
            TimeFieldState::new(window, cx)
                .precision(TimePrecision::Minute)
                .hour_cycle(HourCycle::H23)
        });
        let stop_time = cx.new(|cx| {
            TimeFieldState::new(window, cx)
                .precision(TimePrecision::Minute)
                .hour_cycle(HourCycle::H23)
        });
        for (state, minute) in [(&start_time, start_minute), (&stop_time, stop_minute)] {
            let time = NaiveTime::from_hms_opt(u32::from(minute / 60), u32::from(minute % 60), 0)
                .unwrap_or(NaiveTime::MIN);
            state.update(cx, |state, cx| state.set_time(time, window, cx));
        }
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
    pub(super) fn finish_save_failure(&mut self, message: String, cx: &mut Context<Self>) {
        self.saving = false;
        self.error = None;
        let application = self.application.clone();
        cx.defer(move |cx| {
            let center = application.read(cx).notifications.clone();
            center.update(cx, |center, cx| {
                center.push(
                    crate::notifications::AppNotification::new(
                        "download-task-action",
                        crate::notifications::NotificationLevel::Error,
                        "无法保存每日计划",
                        message,
                    ),
                    cx,
                )
            });
        });
        cx.notify();
    }

    pub(crate) fn save(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if self.saving {
            return;
        }
        let schedule = match build_download_schedule(
            self.enabled_start,
            self.enabled_stop,
            &self.days,
            &self.start_time.read(cx).time().format("%H:%M").to_string(),
            &self.stop_time.read(cx).time().format("%H:%M").to_string(),
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
                    view.finish_save_failure(error, cx);
                });
            }
        });
    }
}

impl Render for DownloadScheduleView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        crate::theme::observe_theme(window, cx);
        self.theme = Theme::active(cx);
        let theme = self.theme;
        let mut weekdays = div().flex().items_center().gap(px(SPACING_4));
        for (index, day) in DownloadWeekday::ALL.into_iter().enumerate() {
            let selected = self.days.contains(&day);
            weekdays = weekdays.child(
                div()
                    .debug_selector(move || format!("download-schedule-day-{index}"))
                    .child(
                        Toggle::new(("download-schedule-day", index))
                            .small()
                            .outline()
                            .checked(selected)
                            .label(day.short_label())
                            .on_click(cx.listener(move |this, requested: &bool, _, cx| {
                                if *requested {
                                    this.days.insert(day);
                                } else {
                                    this.days.remove(&day);
                                }
                                this.error = None;
                                cx.notify();
                            })),
                    ),
            );
        }
        let start_control = schedule_trigger_row(
            theme,
            "自动开始",
            "到点后启动等待队列",
            self.enabled_start,
            &self.start_time,
            "download-schedule-start-toggle",
            cx.listener(|this, requested: &bool, _, cx| {
                this.enabled_start = *requested;
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
            cx.listener(|this, requested: &bool, _, cx| {
                this.enabled_stop = *requested;
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
        let body = DialogContent::new()
            .size_full()
            .min_h_0()
            .overflow_hidden()
            .px(px(SPACING_12))
            .py(px(SPACING_8))
            .when(self.error.is_some(), |body| body.py(px(SPACING_4)))
            .gap(px(SPACING_8))
            .child(
                StatusBar::new()
                    .h(px(32.0))
                    .flex_none()
                    .left(
                        div()
                            .flex_1()
                            .text_size(px(TEXT_BODY))
                            .line_height(px(LINE_HEIGHT_BODY))
                            .text_color(theme.text_secondary)
                            .child("默认下载队列"),
                    )
                    .right(
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
                Form::horizontal().small().label_width(px(82.0)).child(
                    Field::new()
                        .label("生效星期")
                        .items_center()
                        .gap_0()
                        .min_h(px(32.0))
                        .child(weekdays),
                ),
            )
            .child(start_control)
            .child(stop_control)
            .when_some(self.error.clone(), |body, error| {
                let complete_error = error.clone();
                body.child(
                    div()
                        .id("download-schedule-validation")
                        .debug_selector(|| "download-schedule-validation".into())
                        .w_full()
                        .h(px(2.0 * LINE_HEIGHT_BODY))
                        .flex_none()
                        .overflow_hidden()
                        .text_size(px(TEXT_BODY))
                        .line_height(px(LINE_HEIGHT_BODY))
                        .text_color(theme.danger)
                        .whitespace_normal()
                        .line_clamp(2)
                        .tooltip(move |window, cx| {
                            Tooltip::new(complete_error.clone()).build(window, cx)
                        })
                        .child(error),
                )
            })
            .into_any_element();
        let footer = dialog_footer(
            DialogFooter::new()
                .size_full()
                .items_center()
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
                    crate::ui::components::action::primary_action_button(
                        "download-schedule-save",
                        if self.saving {
                            "正在应用…"
                        } else {
                            "应用"
                        },
                        theme,
                        self.saving,
                        cx,
                    )
                    .debug_selector(|| "download-schedule-primary-apply-action".into())
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
    input: &Entity<TimeFieldState>,
    id: &'static str,
    on_toggle: impl Fn(&bool, &mut Window, &mut App) + 'static,
) -> AnyElement {
    Form::horizontal()
        .small()
        .label_width(px(212.0))
        .child(
            Field::new()
                .items_center()
                .gap_0()
                .min_h(px(32.0))
                .label_fn(move |_, _| {
                    div()
                        .min_w_0()
                        .flex()
                        .items_center()
                        .gap(px(SPACING_4))
                        .child(
                            div()
                                .debug_selector(move || format!("{id}-title"))
                                .truncate()
                                .child(title),
                        )
                        .child(crate::ui::components::tooltip::description_help(
                            format!("{id}-description"),
                            description,
                        ))
                })
                .child(
                    div()
                        .flex()
                        .items_center()
                        .child(
                            div()
                                .debug_selector(move || id.into())
                                .w(px(50.0))
                                .h(px(32.0))
                                .flex_none()
                                .mr(px(SPACING_8))
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(
                                    Switch::new(id)
                                        .checked(enabled)
                                        .accessibility_label(title)
                                        .on_change(on_toggle),
                                ),
                        )
                        .child(if enabled {
                            div()
                                .debug_selector(move || format!("{id}-input"))
                                .flex_none()
                                .w(px(114.0))
                                .child(TimeField::new(input).w_full())
                                .into_any_element()
                        } else {
                            div()
                                .debug_selector(move || format!("{id}-disabled"))
                                .h(px(32.0))
                                .flex_none()
                                .flex()
                                .items_center()
                                .justify_center()
                                .w(px(114.0))
                                .text_size(px(TEXT_BODY))
                                .line_height(px(LINE_HEIGHT_BODY))
                                .text_color(theme.text_muted)
                                .text_center()
                                .child("未启用")
                                .into_any_element()
                        }),
                ),
        )
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
