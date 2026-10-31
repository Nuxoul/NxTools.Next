//! Opt-in currency tool on the existing tool-tile / feature-window owners.
use super::*;
use crate::clipboard::CurrencyStatus;
use crate::tools::currency::{self, CurrencyNames};
use gpui_kit::component::alert::Alert;
use gpui_kit::component::select::SearchableVec;

pub(super) const CARD: ToolCard = ToolCard {
    id: "tool-currency",
    title: "汇率换算",
    description: "复制独立金额或短列表后通知换算结果；仅本地计算，不改写剪贴板。$ 按 USD 处理。",
    icon: MaterialIcon::Calculate,
};
pub(super) fn render_card(
    this: &MainWindow,
    window: &Window,
    cx: &mut Context<MainWindow>,
) -> AnyElement {
    let theme = this.theme;
    let enabled = this
        .clipboard
        .upgrade()
        .is_some_and(|owner| owner.read(cx).currency_status().enabled);
    let toggle = cx.listener(|this, _, _, cx| {
        let clipboard = this.clipboard.clone();
        cx.defer(move |cx| {
            if let Some(owner) = clipboard.upgrade() {
                owner.update(cx, |owner, cx| {
                    let status = owner.currency_status();
                    owner.set_currency_settings(!status.enabled, status.target, cx);
                });
            }
        });
    });
    focused_action(
        CARD.id,
        &this.currency_tool_focus,
        theme,
        false,
        window,
        cx,
        toggle,
    )
    .debug_selector(|| "tool-currency-card".into())
    .tooltip(|window, cx| Tooltip::new(CARD.description).build(window, cx))
    .w(px(180.0))
    .h(px(160.0))
    .flex_none()
    .child(
        ListItem::new("tool-currency-item")
            .accessibility_label("汇率换算工具")
            .size_full()
            .rounded_lg()
            .bg(theme.surface)
            .child(
                div()
                    .size_full()
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .gap_2()
                    .child(
                        Icon::new(CARD.icon)
                            .size(px(32.0))
                            .text_color(theme.text_secondary),
                    )
                    .child(Label::new(CARD.title))
                    .child(tool_status(theme, enabled, "tool-currency-status-dot"))
                    .child(
                        Button::new("tool-currency-configure")
                            .debug_selector(|| "tool-currency-configure".into())
                            .label("配置…")
                            .small()
                            .outline()
                            .on_click(cx.listener(|this, _, _, cx| {
                                cx.stop_propagation();
                                open_tool_settings_window(this, CARD, cx);
                                cx.notify();
                            })),
                    ),
            ),
    )
    .into_any_element()
}
#[derive(Clone)]
struct CurrencyChoice {
    code: String,
    label: String,
}
impl SelectItem for CurrencyChoice {
    type Value = String;
    fn title(&self) -> gpui_kit::SharedString {
        self.label.clone().into()
    }
    fn value(&self) -> &String {
        &self.code
    }
}
fn currency_choices(catalog: &CurrencyNames, selected: &str) -> Vec<CurrencyChoice> {
    let mut choices = catalog
        .iter()
        .map(|(code, name)| CurrencyChoice {
            code: code.clone(),
            label: currency::choice_label(code, name),
        })
        .collect::<Vec<_>>();
    if !catalog.contains_key(selected) {
        choices.push(CurrencyChoice {
            code: selected.into(),
            label: format!("{selected} · 当前币种暂不可用"),
        });
    }
    choices.sort_by_key(|choice| {
        (
            choice.code != "CNY",
            choice.code != "USD",
            choice.code.clone(),
        )
    });
    choices
}
pub(super) struct CurrencySettingsBody {
    clipboard: gpui_kit::WeakEntity<ClipboardApplication>,
    target: Entity<SelectState<SearchableVec<CurrencyChoice>>>,
    catalog_revision: (u32, i64),
    _subscriptions: Vec<Subscription>,
}
impl CurrencySettingsBody {
    pub(super) fn new(
        clipboard: gpui_kit::WeakEntity<ClipboardApplication>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let selected = clipboard
            .upgrade()
            .map(|owner| owner.read(cx).currency_status().target)
            .unwrap_or_else(|| "CNY".into());
        let (choices, catalog_revision) = clipboard
            .upgrade()
            .map(|owner| {
                let owner = owner.read(cx);
                (
                    currency_choices(owner.currency_catalog(), &selected),
                    owner.currency_catalog_revision(),
                )
            })
            .unwrap_or_else(|| {
                (
                    currency_choices(currency::bundled_catalog(), &selected),
                    (0, 0),
                )
            });
        let index = choices
            .iter()
            .position(|choice| choice.code == selected)
            .map(|index| IndexPath::default().row(index));
        let target = cx.new(|cx| {
            SelectState::new(SearchableVec::new(choices), index, window, cx).searchable(true)
        });
        let confirmation = cx.subscribe(
            &target,
            |this, _, event: &SelectEvent<SearchableVec<CurrencyChoice>>, cx| {
                let SelectEvent::Confirm(Some(target)) = event else {
                    return;
                };
                let target = target.clone();
                let clipboard = this.clipboard.clone();
                cx.defer(move |cx| {
                    if let Some(owner) = clipboard.upgrade() {
                        owner.update(cx, |owner, cx| {
                            owner.set_currency_settings(owner.currency_status().enabled, target, cx)
                        });
                    }
                });
            },
        );
        let mut subscriptions = vec![confirmation];
        if let Some(owner) = clipboard.upgrade() {
            subscriptions.push(cx.observe_in(&owner, window, |this, owner, window, cx| {
                let application = owner.read(cx);
                let target = application.currency_status().target;
                let revision = application.currency_catalog_revision();
                let choices = (revision != this.catalog_revision)
                    .then(|| currency_choices(application.currency_catalog(), &target));
                cx.defer_in(window, move |this, window, cx| {
                    this.target.update(cx, |state, cx| {
                        let changed = choices.is_some();
                        if let Some(choices) = choices {
                            state.set_items(SearchableVec::new(choices), window, cx);
                        }
                        if changed || state.selected_value() != Some(&target) {
                            state.set_selected_value(&target, window, cx);
                        }
                    });
                    this.catalog_revision = revision;
                    cx.notify();
                });
            }));
        }
        Self {
            clipboard,
            target,
            catalog_revision,
            _subscriptions: subscriptions,
        }
    }
    fn status(&self, cx: &App) -> CurrencyStatus {
        self.clipboard
            .upgrade()
            .map(|owner| owner.read(cx).currency_status())
            .unwrap_or_else(|| CurrencyStatus {
                target: "CNY".into(),
                ..Default::default()
            })
    }
}
impl Render for CurrencySettingsBody {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        crate::theme::observe_theme(window, cx);
        let theme = Theme::active(cx);
        let status = self.status(cx);
        let toggle = cx.listener(|this, requested: &bool, _, cx| {
            let enabled = *requested;
            let clipboard = this.clipboard.clone();
            cx.defer(move |cx| {
                if let Some(owner) = clipboard.upgrade() {
                    owner.update(cx, |owner, cx| {
                        owner.set_currency_settings(enabled, owner.currency_status().target, cx)
                    });
                }
            });
        });
        let update = cx.listener(|this, _, _, cx| {
            let clipboard = this.clipboard.clone();
            cx.defer(move |cx| {
                if let Some(owner) = clipboard.upgrade() {
                    owner.update(cx, |owner, cx| owner.refresh_currency_rates(true, cx));
                }
            });
        });
        let summary = match (&status.date, &status.provider) {
            (Some(date), Some(provider)) => format!(
                "参考日期 {date} · {provider}{}",
                if status.stale {
                    " · 缓存已过期"
                } else {
                    ""
                }
            ),
            _ => "暂无可用汇率，请更新数据".into(),
        };
        let content = div().w_full().flex().flex_col().gap_3()
            .child(Form::horizontal().small().label_width(tool_settings_label_width(window))
                .child(compact_tool_field("currency-target-label","currency-target-help","currency-target-control","目标币种","搜索代码、接口名称或常用中文名；数据源包含法币、贵金属及加密货币。",div().debug_selector(|| "currency-target".into()).w_full().min_w_0().max_w(ordinary_rem(164.0)).child(Select::new(&self.target).w_full().menu_width(ordinary_rem(320.0)).menu_max_h(px(160.0)).search_placeholder("搜索币种代码或名称").accessibility_label("目标币种"))))
                .child(compact_tool_field("currency-update-label","currency-update-help","currency-update-control","汇率数据","启用后每天刷新，也可手动更新。请求只包含固定基准币种，不发送复制内容。",Button::new("currency-update").debug_selector(|| "currency-update".into()).small().outline().label(if status.busy {"更新中…"} else {"更新汇率"}).disabled(status.busy).on_click(update))))
            .child(div().debug_selector(|| "currency-rate-status".into()).child(Label::new(summary).text_sm()))
            .when_some(status.updated_at, |view, updated| view.child(Label::new(format!("上次获取 {updated}")).text_sm()))
            .when_some(status.error, |view, error| view.child(Alert::warning("currency-error", error)))
            .child(Label::new("每日参考汇率，非实时成交价。来源：fawazahmed0 Currency API；jsDelivr / Cloudflare。$、刀、美刀按 USD；₩ 按 KRW。").text_sm().text_color(theme.text_secondary))
            .into_any_element();
        feature_settings_window(
            theme,
            FeatureSettingsWindowSpec {
                ids: FeatureSettingsWindowIds {
                    root: "currency-settings-window",
                    title: "currency-settings-title",
                    description: "currency-settings-description",
                    status: "currency-settings-status",
                    enabled_toggle: "currency-settings-toggle",
                    close: "currency-settings-close",
                    content: "currency-settings-content",
                },
                title: CARD.title.into(),
                description: CARD.description.into(),
                enabled_label: "启用汇率换算".into(),
                enabled: status.enabled,
            },
            toggle,
            cx.listener(|_, _, window, _| window.remove_window()),
            content,
        )
    }
}
