pub(crate) const UI_FONT_FAMILY: &str = nxtools_core::DEFAULT_UI_FONT_FAMILY;
pub(crate) const UI_MONO_FONT_FAMILY: &str = "Consolas";
#[cfg(test)]
pub(crate) const TEST_UI_FONT_FAMILY: &str = "Segoe UI";

#[cfg(test)]
pub(crate) struct InstalledUiFontsFixture(pub Vec<String>);
#[cfg(test)]
impl gpui_kit::Global for InstalledUiFontsFixture {}

pub(crate) const TEXT_PAGE_TITLE: f32 = 16.0;
pub(crate) const TEXT_TITLE: f32 = 14.0;
pub(crate) const TEXT_BODY: f32 = 12.0;
pub(crate) const TEXT_LABEL: f32 = 11.0;
pub(crate) const TEXT_META: f32 = 10.0;
// Single-character status glyphs (completion/shutdown windows), not ordinary text.
pub(crate) const TEXT_STATUS_GLYPH: f32 = 28.0;

pub(crate) const LINE_HEIGHT_PAGE_TITLE: f32 = 22.0;
pub(crate) const LINE_HEIGHT_TITLE: f32 = 20.0;
pub(crate) const LINE_HEIGHT_BODY: f32 = 18.0;
pub(crate) const LINE_HEIGHT_LABEL: f32 = 16.0;
#[cfg(test)]
pub(crate) const LINE_HEIGHT_META: f32 = 14.0;

// Workbench roles are opt-in. Compact/task/summon windows retain their measured
// typography above rather than inheriting a global size increase.
pub(crate) const TEXT_WORKBENCH_BODY: f32 = 13.0;
pub(crate) const LINE_HEIGHT_WORKBENCH_BODY: f32 = 20.0;
pub(crate) const TEXT_WORKBENCH_SUPPORT: f32 = 12.0;
pub(crate) const LINE_HEIGHT_WORKBENCH_SUPPORT: f32 = 18.0;

/// Keep every valid platform family; the selector virtualizes the catalog.
pub(crate) fn installed_ui_fonts(cx: &gpui_kit::App) -> Vec<String> {
    #[cfg(test)]
    if let Some(catalog) = cx.try_global::<InstalledUiFontsFixture>() {
        return catalog.0.clone();
    }
    cx.text_system()
        .all_font_names()
        .into_iter()
        .filter(|name| nxtools_core::validate_ui_font_family(name).is_ok())
        .collect()
}

pub(crate) fn ui_font_available(family: &str, installed: &[String]) -> bool {
    family == UI_FONT_FAMILY || installed.iter().any(|name| name == family)
}

pub(crate) fn resolved_ui_font(family: &str, installed: &[String]) -> gpui_kit::SharedString {
    if nxtools_core::validate_ui_font_family(family).is_ok() && ui_font_available(family, installed)
    {
        family.to_owned().into()
    } else {
        UI_FONT_FAMILY.into()
    }
}

pub(crate) fn active_ui_font_family(cx: &gpui_kit::App) -> gpui_kit::SharedString {
    gpui_kit::component::Theme::global(cx).font_family.clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typography_roles_use_one_integer_scale() {
        assert_eq!(
            [
                TEXT_PAGE_TITLE,
                TEXT_TITLE,
                TEXT_BODY,
                TEXT_LABEL,
                TEXT_META
            ],
            [16.0, 14.0, 12.0, 11.0, 10.0]
        );
        assert_eq!(
            [
                LINE_HEIGHT_PAGE_TITLE,
                LINE_HEIGHT_TITLE,
                LINE_HEIGHT_BODY,
                LINE_HEIGHT_LABEL,
                LINE_HEIGHT_META
            ],
            [22.0, 20.0, 18.0, 16.0, 14.0]
        );
        assert_eq!(TEXT_STATUS_GLYPH, 28.0);
        assert_eq!(
            [TEXT_WORKBENCH_BODY, LINE_HEIGHT_WORKBENCH_BODY],
            [13.0, 20.0]
        );
        assert_eq!(
            [TEXT_WORKBENCH_SUPPORT, LINE_HEIGHT_WORKBENCH_SUPPORT],
            [12.0, 18.0]
        );
    }
}

#[cfg(test)]
mod font_selection_tests {
    use super::*;

    #[test]
    fn installed_english_chinese_and_missing_fonts_resolve_as_single_families() {
        let installed: Vec<String> = vec!["Segoe UI".into(), "思源黑体 CN".into()];
        for family in &installed {
            assert!(ui_font_available(family, &installed));
            assert_eq!(
                resolved_ui_font(family, &installed).as_ref(),
                family.as_str()
            );
        }
        for family in [
            "Uninstalled",
            "Segoe UI, Microsoft YaHei UI",
            "",
            "bad\nfont",
        ] {
            assert_eq!(resolved_ui_font(family, &installed), UI_FONT_FAMILY);
        }
        assert!(ui_font_available(UI_FONT_FAMILY, &[]));
        assert_eq!(UI_MONO_FONT_FAMILY, "Consolas");
    }
}
