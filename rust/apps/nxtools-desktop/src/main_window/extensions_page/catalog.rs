//! A catalog cell exposes the extension's purpose and one route into its details.
use super::*;
use gpui_kit::component::{
    ActiveTheme as _, StyledExt as _, label::Label, list::ListItem, tag::Tag,
};
use nxtools_extensions::InstalledPackage;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum AttentionReason {
    Settings,
    Trust,
    Paused,
}

impl AttentionReason {
    pub(super) const ALL: [Self; 3] = [Self::Settings, Self::Trust, Self::Paused];

    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Settings => "待完成设置",
            Self::Trust => "待确认信任",
            Self::Paused => "自动触发已暂停",
        }
    }
}

impl ExtensionManagerView {
    pub(super) fn attention_reason(
        &self,
        package: &InstalledPackage,
        cx: &App,
    ) -> Option<AttentionReason> {
        if package.manifest.runtime.is_some() && !extensions_settings::current_trust(package) {
            Some(AttentionReason::Trust)
        } else if package.enabled && usage::has_unfilled_settings(package) {
            Some(AttentionReason::Settings)
        } else if package.enabled
            && package.manifest.handlers.iter().any(|handler| {
                usage::is_event(handler)
                    && self
                        .application
                        .read(cx)
                        .event_paused(&package.manifest.id, &handler.id)
            })
        {
            Some(AttentionReason::Paused)
        } else {
            None
        }
    }

    pub(super) fn catalog_cell(
        &self,
        package: &InstalledPackage,
        compact: bool,
        cx: &Context<Self>,
    ) -> AnyElement {
        let id = package.manifest.id.clone();
        let reason = self.attention_reason(package, cx);
        let status = reason
            .map(AttentionReason::label)
            .unwrap_or(if package.enabled {
                "可用"
            } else {
                "未启用"
            });
        let tag = if reason == Some(AttentionReason::Paused) {
            Tag::danger()
        } else if reason.is_some() {
            Tag::warning()
        } else if package.enabled {
            Tag::success()
        } else {
            Tag::secondary()
        };
        let first = package.manifest.handlers.first();
        let icon = match first.map(|handler| handler.feature.as_str()) {
            Some("clipboard") => MaterialIcon::ContentPaste,
            Some("downloads") => MaterialIcon::Download,
            Some("quick_launch") => MaterialIcon::Search,
            _ => MaterialIcon::DataObject,
        };
        let location = first.map_or_else(
            || "扩展详情".to_owned(),
            |handler| match (handler.feature.as_str(), handler.entry.as_str()) {
                ("clipboard", "action") => "剪贴板 · 条目菜单".into(),
                ("downloads", "action") => "下载 · 任务菜单".into(),
                ("quick_launch", _) if let Some(keyword) = package.keywords(handler).first() => {
                    format!("速启 · {keyword}")
                }
                ("tools", "action") => "其他工具 · 扩展详情".into(),
                (feature, _) => format!(
                    "{} · {}",
                    workspace::feature_label(feature),
                    usage::entry_label(handler)
                ),
            },
        );
        let content = ListItem::new(format!("extension-package-{id}"))
            .w_full()
            .min_w_0()
            .p_0()
            .child(
                div()
                    .w_full()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .min_w_0()
                            .child(
                                Icon::new(icon)
                                    .size(px(20.0))
                                    .text_color(cx.theme().primary),
                            )
                            .child(
                                div()
                                    .min_w_0()
                                    .line_clamp(1)
                                    .font_semibold()
                                    .child(Label::new(package.manifest.name.clone())),
                            ),
                    )
                    .child(
                        div()
                            .debug_selector(|| {
                                format!(
                                    "extension-overview-purpose-{id}:{}",
                                    usage::purpose(package)
                                )
                            })
                            .text_sm()
                            .line_clamp(2)
                            .text_color(cx.theme().muted_foreground)
                            .child(usage::purpose(package).to_owned()),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_2()
                            .pt_1()
                            .border_t_1()
                            .border_color(cx.theme().border)
                            .child(
                                div()
                                    .min_w_0()
                                    .text_xs()
                                    .line_clamp(1)
                                    .text_color(cx.theme().muted_foreground)
                                    .child(location),
                            )
                            .child(tag.small().child(status)),
                    ),
            );
        let selector = format!("extension-manage-{id}");
        let view = cx.entity().downgrade();
        Button::new(selector.clone())
            .debug_selector(move || selector.clone())
            .outline()
            .w_full()
            .min_w_0()
            .h_auto()
            .min_h(px(if compact { 100.0 } else { 114.0 }))
            .px_3()
            .py_2()
            .accessibility_label(format!("{}，{}，打开详情", package.manifest.name, status))
            .child(content)
            .on_click(move |_, window, cx| {
                let view = view.clone();
                let id = id.clone();
                window.defer(cx, move |window, cx| {
                    let _ = view.update(cx, |view, cx| {
                        view.open_package(id);
                        view.reconcile_detail_sheet(window, cx);
                        cx.notify();
                    });
                });
            })
            .into_any_element()
    }
}
