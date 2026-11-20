use gpui_kit::component::select::{SearchableVec, Select, SelectItem, SelectState};
use gpui_kit::component::{IndexPath, Sizable, Size};
use gpui_kit::{
    AnyElement, App, AppContext, Context, Entity, IntoElement, SharedString, Window, prelude::*, px,
};

use crate::ui::typography::{UI_FONT_FAMILY, installed_ui_fonts, ui_font_available};

#[derive(Clone)]
struct FontChoice {
    family: String,
    title: SharedString,
}

impl SelectItem for FontChoice {
    type Value = String;

    fn title(&self) -> SharedString {
        self.title.clone()
    }

    fn value(&self) -> &String {
        &self.family
    }
}

/// Window-local selection only. Dropping the draft is Cancel; it never publishes a theme.
/// The parent reads validated_family before its existing atomic Apply transaction.
pub(super) struct FontSettingsDraft {
    baseline: String,
    selector: Entity<SelectState<SearchableVec<FontChoice>>>,
}

impl FontSettingsDraft {
    pub(super) fn new<T: 'static>(family: &str, window: &mut Window, cx: &mut Context<T>) -> Self {
        let installed = installed_ui_fonts(cx);
        let mut families = installed.clone();
        families.push(UI_FONT_FAMILY.into());
        families.push(family.into());
        families.sort_unstable();
        families.dedup();
        let selected = families.iter().position(|name| name == family);
        let choices = families
            .into_iter()
            .map(|name| {
                let title = if name == UI_FONT_FAMILY {
                    format!("{name}（默认）")
                } else if !ui_font_available(&name, &installed) {
                    format!("{name}（未安装，使用默认字体）")
                } else {
                    name.clone()
                };
                FontChoice {
                    family: name,
                    title: title.into(),
                }
            })
            .collect::<Vec<_>>();
        Self {
            baseline: family.into(),
            selector: cx.new(|cx| {
                SelectState::new(
                    SearchableVec::new(choices),
                    selected.map(|row| IndexPath::default().row(row)),
                    window,
                    cx,
                )
                .searchable(true)
            }),
        }
    }

    pub(super) fn validated_family(&self, cx: &App) -> Result<String, String> {
        let selected = self
            .selector
            .read(cx)
            .selected_value()
            .cloned()
            .ok_or_else(|| "请选择界面字体".to_owned())?;
        validate_selection(&selected, &self.baseline, &installed_ui_fonts(cx))?;
        Ok(selected)
    }

    pub(super) fn render(&self, saving: bool) -> AnyElement {
        Select::new(&self.selector)
            .id("global-ui-font")
            .accessibility_label("界面字体")
            .search_placeholder("搜索可用的中英文字体")
            .with_size(Size::Medium)
            .w_full()
            .menu_max_h(px(240.0))
            .disabled(saving)
            .into_any_element()
    }
}

fn validate_selection(selected: &str, baseline: &str, installed: &[String]) -> Result<(), String> {
    nxtools_core::validate_ui_font_family(selected)?;
    if selected == baseline || ui_font_available(selected, installed) {
        Ok(())
    } else {
        Err("所选字体当前未安装，请重新打开设置后选择".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_selection_requires_installed_family_but_missing_saved_intent_is_retained() {
        let english = crate::ui::typography::TEST_UI_FONT_FAMILY;
        let installed = vec![english.into(), "思源黑体 CN".into()];
        for family in [english, "思源黑体 CN", UI_FONT_FAMILY] {
            assert!(validate_selection(family, UI_FONT_FAMILY, &installed).is_ok());
        }
        assert!(validate_selection("Not installed", UI_FONT_FAMILY, &installed).is_err());
        assert!(
            validate_selection(
                &format!("{english}, 思源黑体 CN"),
                UI_FONT_FAMILY,
                &installed
            )
            .is_err()
        );
        assert!(validate_selection("Removed font", "Removed font", &installed).is_ok());
        assert!(validate_selection("", "", &installed).is_err());
    }
    #[gpui_kit::test]
    fn font_draft_cancel_apply_and_failed_save_use_the_original_global_transaction(
        cx: &mut gpui_kit::TestAppContext,
    ) {
        use super::super::NetworkSettingsView;
        use nxtools_core::AppConfig;
        fn select(
            view: &NetworkSettingsView,
            family: &str,
            window: &mut Window,
            cx: &mut Context<NetworkSettingsView>,
        ) {
            let mut families = installed_ui_fonts(cx);
            families.extend([UI_FONT_FAMILY.into(), view.ui_font.baseline.clone()]);
            families.sort_unstable();
            families.dedup();
            let index = families
                .iter()
                .position(|name| name == family)
                .expect("installed choice");
            view.ui_font.selector.update(cx, |state, cx| {
                state.set_selected_index(Some(IndexPath::default().row(index)), window, cx)
            });
        }
        cx.update(gpui_kit::init);
        cx.update(crate::theme::apply_component_theme);
        let chosen = crate::ui::typography::TEST_UI_FONT_FAMILY.to_owned();
        cx.update(|cx| {
            cx.set_global(crate::ui::typography::InstalledUiFontsFixture(vec![
                UI_FONT_FAMILY.into(),
                chosen.clone(),
                "思源黑体 CN".into(),
            ]));
        });
        let (app, root, _) = super::super::tests::application_fixture(cx, "ui-font-transaction");
        app.update(cx, |app, cx| {
            *app.system_settings.borrow_mut() = app.config.system.clone();
            app.open_window(cx);
        });
        cx.run_until_parked();
        let (handle, view) = cx.read(|cx| {
            let app = app.read(cx);
            (
                app.window.expect("window"),
                app.settings_view.clone().expect("view"),
            )
        });
        let mut surface = gpui_kit::VisualTestContext::from_window(handle, cx);
        view.update_in(&mut surface, |view, window, cx| {
            select(view, &chosen, window, cx)
        });
        assert_eq!(
            AppConfig::load_from(&root.join("config.json"))
                .expect("unapplied config")
                .system
                .ui_font_family,
            UI_FONT_FAMILY
        );
        surface.read(|cx| {
            assert_eq!(
                crate::ui::typography::active_ui_font_family(cx),
                UI_FONT_FAMILY
            )
        });
        app.update(&mut surface, |app, cx| app.close_window(cx));
        app.update(&mut surface, |app, cx| app.open_window(cx));
        surface.run_until_parked();
        let (handle, reopened) = app.read_with(&surface, |app, _| {
            (
                app.window.expect("reopened window"),
                app.settings_view.clone().expect("reopened view"),
            )
        });
        let mut surface = gpui_kit::VisualTestContext::from_window(handle, cx);
        reopened.read_with(&surface, |view, cx| {
            assert_eq!(
                view.ui_font.validated_family(cx).expect("cancelled draft"),
                UI_FONT_FAMILY
            )
        });
        reopened.update_in(&mut surface, |view, window, cx| {
            select(view, &chosen, window, cx);
            view.save(window, cx);
        });
        surface.run_until_parked();
        assert_eq!(
            AppConfig::load_from(&root.join("config.json"))
                .expect("applied config")
                .system
                .ui_font_family,
            chosen
        );
        surface.read(|cx| {
            assert_eq!(
                crate::ui::typography::active_ui_font_family(cx).as_ref(),
                chosen.as_str()
            )
        });
        app.update(&mut surface, |app, cx| app.open_window(cx));
        surface.run_until_parked();
        let (handle, failed) = app.read_with(&surface, |app, _| {
            (
                app.window.expect("retry window"),
                app.settings_view.clone().expect("retry view"),
            )
        });
        let mut surface = gpui_kit::VisualTestContext::from_window(handle, cx);
        std::fs::remove_file(root.join("config.json")).expect("inject commit failure");
        failed.update_in(&mut surface, |view, window, cx| {
            select(view, UI_FONT_FAMILY, window, cx);
            view.save(window, cx);
        });
        surface.run_until_parked();
        failed.read_with(&surface, |view, cx| {
            assert!(!view.saving);
            assert_eq!(
                view.ui_font.validated_family(cx).expect("retained draft"),
                UI_FONT_FAMILY
            );
        });
        surface.read(|cx| {
            assert_eq!(
                crate::ui::typography::active_ui_font_family(cx).as_ref(),
                chosen.as_str()
            )
        });
        app.update(&mut surface, |app, cx| app.close_window(cx));
        std::fs::remove_dir_all(root).expect("cleanup");
    }
}
