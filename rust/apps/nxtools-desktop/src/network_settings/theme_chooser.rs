//! Clean-room visual selection; public Kit Radio owns every gesture and marker.
use super::*;
use gpui_kit::component::radio::{Radio, RadioGroup};
use nxtools_core::ThemePreference;

const LIGHT_CHOICES: [ThemePreference; 3] = [
    ThemePreference::Light,
    ThemePreference::GruvboxLight,
    ThemePreference::SolarizedLight,
];
const DARK_CHOICES: [ThemePreference; 5] = [
    ThemePreference::Dark,
    ThemePreference::CatppuccinMacchiato,
    ThemePreference::CatppuccinMocha,
    ThemePreference::GruvboxDark,
    ThemePreference::SolarizedDark,
];

fn config_name(preference: ThemePreference, system_dark: bool) -> &'static str {
    match preference {
        ThemePreference::Light => "Catppuccin Latte",
        ThemePreference::Dark => "Catppuccin Frappe",
        ThemePreference::System if system_dark => "Catppuccin Frappe",
        ThemePreference::System => "Catppuccin Latte",
        explicit => explicit.label(),
    }
}

fn preview(preference: ThemePreference, cx: &gpui_kit::App) -> gpui_kit::AnyElement {
    let system_dark = cx
        .try_global::<crate::theme::UiThemeState>()
        .map(|state| state.system_dark)
        .unwrap_or(true);
    static PALETTES: std::sync::OnceLock<
        std::collections::HashMap<String, gpui_kit::component::ThemeColor>,
    > = std::sync::OnceLock::new();
    let palettes = PALETTES.get_or_init(|| {
        gpui_kit::component::ThemeRegistry::global(cx)
            .themes()
            .iter()
            .map(|(name, config)| {
                let mut sample = gpui_kit::component::Theme::default();
                sample.apply_config(config);
                (name.to_string(), sample.colors)
            })
            .collect()
    });
    let colors = *palettes
        .get(config_name(preference, system_dark))
        .expect("bundled theme preview");
    // A noninteractive miniature of NxTools search/results, not another control.
    div()
        .debug_selector(move || format!("theme-preview-{}", preference.label()))
        .w_full()
        .h(px(58.0))
        .rounded_sm()
        .overflow_hidden()
        .border_1()
        .border_color(colors.border)
        .bg(colors.background)
        .flex()
        .child(div().w(px(18.0)).h_full().bg(colors.sidebar))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .p_1()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .h(px(16.0))
                        .px_1()
                        .border_1()
                        .border_color(colors.input)
                        .rounded_sm()
                        .text_xs()
                        .text_color(colors.foreground)
                        .child("搜索文件…"),
                )
                .child(
                    div()
                        .h(px(12.0))
                        .px_1()
                        .bg(colors.list_active)
                        .flex()
                        .items_center()
                        .gap_1()
                        .child(div().size(px(6.0)).bg(colors.primary).rounded_sm())
                        .child(div().h(px(3.0)).w(px(64.0)).bg(colors.foreground)),
                )
                .child(div().h(px(3.0)).w(px(88.0)).bg(colors.muted_foreground)),
        )
        .into_any_element()
}

fn radio(preference: ThemePreference, cx: &gpui_kit::App) -> Radio {
    let name = match preference {
        ThemePreference::Light => "默认浅色 · Latte",
        ThemePreference::Dark => "默认深色 · Frappe",
        _ => preference.label(),
    };
    Radio::new(gpui_kit::SharedString::from(preference.label()))
        .accessibility_label(preference.label())
        .w_full()
        .min_w_0()
        .p_2()
        .border_1()
        .border_color(gpui_kit::component::Theme::global(cx).colors.border)
        .rounded_md()
        .child(
            div()
                .w_full()
                .min_w_0()
                .flex()
                .flex_col()
                .gap_1()
                .child(div().text_sm().child(name))
                .child(preview(preference, cx)),
        )
}

impl NetworkSettingsView {
    pub(super) fn render_theme_chooser(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let selected = match self.theme_preference {
            ThemePreference::CatppuccinLatte => ThemePreference::Light,
            ThemePreference::CatppuccinFrappe => ThemePreference::Dark,
            other => other,
        };
        let owner = cx.entity().downgrade();
        let system_owner = owner.clone();
        let mut columns = div().flex().items_start().gap_3().w_full().min_w_0();
        for (group, title, choices) in [
            ("global-theme-light", "浅色", LIGHT_CHOICES.as_slice()),
            ("global-theme-dark", "深色", DARK_CHOICES.as_slice()),
        ] {
            let owner = owner.clone();
            let choices = choices.to_vec();
            let selected_index = choices
                .iter()
                .position(|preference| *preference == selected);
            let radios = choices
                .iter()
                .map(|preference| radio(*preference, cx))
                .collect::<Vec<_>>();
            columns = columns.child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(div().text_sm().child(title))
                    .child(
                        RadioGroup::vertical(group)
                            .w_full()
                            .selected_index(selected_index)
                            .disabled(self.saving)
                            .children(radios)
                            .on_change(move |index, _, cx| {
                                if let Some(preference) = choices.get(*index).copied() {
                                    let owner = owner.clone();
                                    cx.defer(move |cx| {
                                        let _ = owner.update(cx, |view, cx| {
                                            if !view.saving {
                                                view.theme_preference = preference;
                                                cx.notify();
                                            }
                                        });
                                    });
                                }
                            }),
                    ),
            );
        }
        div()
            .debug_selector(|| "global-theme-chooser".into())
            .w_full()
            .min_w_0()
            .flex()
            .flex_col()
            .gap_2()
            .child(div().text_sm().child("主题"))
            .child(
                div()
                    .text_sm()
                    .text_color(self.theme.text_secondary)
                    .child("选择预览，应用后统一生效。默认浅色为 Latte，默认深色为 Frappe。"),
            )
            .child(
                Radio::new("global-theme-system")
                    .label("跟随系统")
                    .checked(selected == ThemePreference::System)
                    .disabled(self.saving)
                    .on_change(move |_, _, cx| {
                        let owner = system_owner.clone();
                        cx.defer(move |cx| {
                            let _ = owner.update(cx, |view, cx| {
                                if !view.saving {
                                    view.theme_preference = ThemePreference::System;
                                    cx.notify();
                                }
                            });
                        });
                    }),
            )
            .child(columns)
            .into_any_element()
    }
}
