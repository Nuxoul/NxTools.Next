pub(crate) const UI_FONT_FAMILY: &str = "Microsoft YaHei UI";
pub(crate) const UI_MONO_FONT_FAMILY: &str = "Consolas";

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
