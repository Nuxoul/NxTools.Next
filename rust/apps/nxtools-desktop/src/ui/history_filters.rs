use super::*;
use chrono::{Days, Local, NaiveDate, TimeZone};
use gpui_kit::component::empty::EmptyContent;
use gpui_kit::component::menu::{DropdownMenu, PopupMenu, PopupMenuItem};
use gpui_kit::{AnyElement, SharedString};
use nxtools_clipboard::ClipboardHistoryFilter;

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum CaptureDate {
    #[default]
    Any,
    Today,
    Yesterday,
    LastSevenDays,
}

impl CaptureDate {
    const CHOICES: [Self; 4] = [Self::Any, Self::Today, Self::Yesterday, Self::LastSevenDays];

    fn label(self) -> &'static str {
        match self {
            Self::Any => "不限时间",
            Self::Today => "今天",
            Self::Yesterday => "昨天",
            Self::LastSevenDays => "最近 7 天",
        }
    }

    fn bounds(self, today: NaiveDate, zone: &impl TimeZone) -> Option<(i64, i64)> {
        let (start, end) = match self {
            Self::Any => return None,
            Self::Today => (today, today.succ_opt()?),
            Self::Yesterday => (today.pred_opt()?, today),
            Self::LastSevenDays => (today.checked_sub_days(Days::new(6))?, today.succ_opt()?),
        };
        let start_of_day = |date: NaiveDate| {
            (0..24).find_map(|hour| {
                zone.from_local_datetime(&date.and_hms_opt(hour, 0, 0)?)
                    .earliest()
                    .map(|time| time.timestamp_millis())
            })
        };
        Some((start_of_day(start)?, start_of_day(end)?))
    }
}

#[derive(Default)]
pub(super) struct HistoryFilters {
    pub(super) query: ClipboardHistoryFilter,
    pub(super) pending: Option<(uuid::Uuid, Instant)>,
    date: CaptureDate,
}

impl HistoryFilters {
    fn active(&self) -> bool {
        self.query != ClipboardHistoryFilter::default()
    }
}

#[derive(Clone)]
enum HistoryChoice {
    Source(Option<String>),
    Date(CaptureDate),
    ClearFilters,
    ClearQuery,
    AllCategories,
}

impl ClipboardPanel {
    fn perform_history_choice(
        &mut self,
        choice: HistoryChoice,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let previous = self.history_filters.query.clone();
        match choice {
            HistoryChoice::ClearQuery => {
                self.search_input
                    .update(cx, |input, cx| input.set_value("", window, cx));
                self.dismiss_preview_for_input("history_query_cleared", cx);
                self.refresh_entries(cx);
                cx.notify();
                self.focus_search_input(window, cx);
                return;
            }
            HistoryChoice::AllCategories => {
                self.dismiss_preview_for_input("history_category_changed", cx);
                self.select_category(BUILTIN_ALL_CATEGORY_ID.into(), cx);
                self.focus_search_input(window, cx);
                return;
            }
            HistoryChoice::Source(source) => self.history_filters.query.source_process = source,
            HistoryChoice::Date(date) => {
                let bounds = date.bounds(Local::now().date_naive(), &Local);
                if date != CaptureDate::Any && bounds.is_none() {
                    return;
                }
                self.history_filters.date = date;
                self.history_filters.query.captured_between_unix_ms = bounds;
            }
            HistoryChoice::ClearFilters => self.history_filters = HistoryFilters::default(),
        }
        if previous != self.history_filters.query {
            let id = uuid::Uuid::new_v4();
            let started = Instant::now();
            info!(event = "clipboard.history_filter.accepted", correlation_id = %id,
                source_filtered = self.history_filters.query.source_process.is_some(),
                date_filtered = self.history_filters.query.captured_between_unix_ms.is_some());
            self.history_filters.pending = Some((id, started));
            self.dismiss_preview_for_input("history_filter_changed", cx);
            self.refresh_entries(cx);
            cx.notify();
        }
        self.focus_search_input(window, cx);
    }

    pub(super) fn render_history_filter_button(&self, cx: &Context<Self>) -> AnyElement {
        let panel = cx.entity().downgrade();
        let session = self.session_id;
        Button::new("clipboard-history-filters")
            .label("筛选")
            .xsmall()
            .outline()
            .selected(self.history_filters.active())
            .dropdown_menu(move |menu, window, cx| {
                let Ok((sources, selected, date, active)) = panel.read_with(cx, |panel, _| {
                    (
                        panel.category_source_choices(),
                        panel.history_filters.query.source_process.clone(),
                        panel.history_filters.date,
                        panel.history_filters.active(),
                    )
                }) else {
                    return menu;
                };
                let source_panel = panel.clone();
                let date_panel = panel.clone();
                let mut menu: PopupMenu = menu
                    .submenu("来源应用", window, cx, move |mut submenu, _, _| {
                        submenu = submenu.scrollable(true).max_h(px(176.0)).max_w(px(184.0));
                        for source in std::iter::once(None).chain(sources.iter().cloned().map(Some))
                        {
                            let label = source.clone().unwrap_or_else(|| "不限应用".into());
                            let checked = source == selected;
                            submenu = submenu.item(history_menu_item(
                                label,
                                checked,
                                &source_panel,
                                session,
                                HistoryChoice::Source(source),
                            ));
                        }
                        submenu
                    })
                    .submenu("复制时间", window, cx, move |mut submenu, _, _| {
                        for option in CaptureDate::CHOICES {
                            submenu = submenu.item(history_menu_item(
                                option.label(),
                                option == date,
                                &date_panel,
                                session,
                                HistoryChoice::Date(option),
                            ));
                        }
                        submenu
                    });
                if active {
                    menu = menu.separator().item(history_menu_item(
                        "清除筛选",
                        false,
                        &panel,
                        session,
                        HistoryChoice::ClearFilters,
                    ));
                }
                menu
            })
            .into_any_element()
    }

    pub(super) fn render_active_history_filters(&self, cx: &Context<Self>) -> Div {
        let mut strip = div();
        if self.history_filters.active() {
            let filters = self
                .history_filters
                .query
                .source_process
                .as_deref()
                .into_iter()
                .chain(
                    (self.history_filters.date != CaptureDate::Any)
                        .then(|| self.history_filters.date.label()),
                )
                .collect::<Vec<_>>()
                .join(" · ");
            strip = strip
                .debug_selector(|| "clipboard-history-active-filters".into())
                .flex()
                .items_center()
                .gap_2()
                .mb_1()
                .child(
                    div()
                        .flex_1()
                        .truncate()
                        .text_size(px(TEXT_META))
                        .text_color(self.theme.text_secondary)
                        .child(filters),
                )
                .child(self.history_choice_button(
                    "clipboard-history-clear-filters",
                    "清除筛选",
                    HistoryChoice::ClearFilters,
                    cx,
                ));
        }
        strip
    }

    fn history_choice_button(
        &self,
        id: &'static str,
        label: &'static str,
        choice: HistoryChoice,
        cx: &Context<Self>,
    ) -> Button {
        let panel = cx.entity().downgrade();
        let session = self.session_id;
        Button::new(id)
            .debug_selector(move || id.into())
            .label(label)
            .xsmall()
            .outline()
            .on_click(move |_, window, cx| {
                schedule_history_choice(&panel, session, choice.clone(), window, cx);
            })
    }

    pub(super) fn render_history_empty(&self, query: &str, cx: &Context<Self>) -> AnyElement {
        let category = self.active_category().map(|category| category.kind);
        let (icon, title, description, action) = if !query.is_empty() {
            (
                MaterialIcon::Search,
                "没有匹配的剪贴板记录",
                "尝试缩短关键词，或清除关键词后再查看。",
                Some(("清除关键词", HistoryChoice::ClearQuery)),
            )
        } else if self.history_filters.active() {
            (
                MaterialIcon::Search,
                "当前筛选没有匹配记录",
                "更换来源应用或复制时间，或清除全部筛选。",
                Some(("清除筛选", HistoryChoice::ClearFilters)),
            )
        } else {
            match category {
                Some(ClipboardCategoryKind::Favorite) => (
                    MaterialIcon::History,
                    "收藏夹为空",
                    "在历史记录中点星标即可收藏。",
                    Some(("查看全部", HistoryChoice::AllCategories)),
                ),
                Some(ClipboardCategoryKind::All) | None => (
                    MaterialIcon::History,
                    "暂无剪贴板记录",
                    "复制内容后会自动出现在这里。",
                    None,
                ),
                _ => (
                    MaterialIcon::History,
                    "当前分类暂无记录",
                    "查看全部历史或切换到其他分类。",
                    Some(("查看全部", HistoryChoice::AllCategories)),
                ),
            }
        };
        let mut empty = Empty::new().header(
            EmptyHeader::new()
                .media(
                    EmptyMedia::new().child(
                        div()
                            .debug_selector(|| "clipboard-kit-empty-icon".into())
                            .child(Icon::new(icon)),
                    ),
                )
                .title(
                    EmptyTitle::new().child(
                        div()
                            .debug_selector(|| "clipboard-kit-empty-title".into())
                            .child(title),
                    ),
                )
                .description(
                    EmptyDescription::new().child(
                        div()
                            .debug_selector(|| "clipboard-kit-empty-description".into())
                            .child(description),
                    ),
                ),
        );
        if let Some((label, choice)) = action {
            empty = empty.content(EmptyContent::new().child(self.history_choice_button(
                "clipboard-history-empty-action",
                label,
                choice,
                cx,
            )));
        }
        div()
            .id("clipboard-history-empty")
            .debug_selector(|| "clipboard-history-empty".into())
            .size_full()
            .flex()
            .flex_col()
            .child(empty)
            .into_any_element()
    }
}

fn history_menu_item(
    label: impl Into<SharedString>,
    checked: bool,
    panel: &WeakEntity<ClipboardPanel>,
    session: uuid::Uuid,
    choice: HistoryChoice,
) -> PopupMenuItem {
    let panel = panel.clone();
    PopupMenuItem::new(label)
        .checked(checked)
        .on_click(move |_, window, cx| {
            schedule_history_choice(&panel, session, choice.clone(), window, cx);
        })
}

fn schedule_history_choice(
    panel: &WeakEntity<ClipboardPanel>,
    session: uuid::Uuid,
    choice: HistoryChoice,
    window: &mut Window,
    cx: &mut App,
) {
    let panel = panel.clone();
    window.defer(cx, move |window, cx| {
        let _ = panel.update(cx, |panel, cx| {
            if panel.is_open && panel.session_id == session {
                panel.perform_history_choice(choice, window, cx);
            }
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn date_presets_are_half_open_calendar_ranges() {
        let date = NaiveDate::from_ymd_opt(2026, 11, 17).expect("valid fixture calendar date");
        let today = CaptureDate::Today
            .bounds(date, &chrono::Utc)
            .expect("bounded today preset");
        let yesterday = CaptureDate::Yesterday
            .bounds(date, &chrono::Utc)
            .expect("bounded yesterday preset");
        let week = CaptureDate::LastSevenDays
            .bounds(date, &chrono::Utc)
            .expect("bounded seven-day preset");
        assert_eq!(yesterday.1, today.0);
        assert_eq!(today.1 - today.0, 86_400_000);
        assert_eq!(week, (today.0 - 6 * 86_400_000, today.1));
        assert_eq!(CaptureDate::Any.bounds(date, &chrono::Utc), None);
    }
}
