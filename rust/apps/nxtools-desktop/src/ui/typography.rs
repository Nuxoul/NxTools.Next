pub(crate) const UI_FONT_FAMILY: &str = "Microsoft YaHei UI";
pub(crate) const UI_MONO_FONT_FAMILY: &str = "Consolas";

#[allow(dead_code)] // Consumed as dialog and module headers migrate.
pub(crate) const TEXT_TITLE: f32 = 14.0;
pub(crate) const TEXT_BODY: f32 = 12.0;
pub(crate) const TEXT_LABEL: f32 = 11.0;
#[allow(dead_code)] // Reserved for actual metadata, not ordinary Chinese descriptions.
pub(crate) const TEXT_META: f32 = 10.0;

#[allow(dead_code)] // Consumed as dialog and module headers migrate.
pub(crate) const LINE_HEIGHT_TITLE: f32 = 20.0;
pub(crate) const LINE_HEIGHT_BODY: f32 = 18.0;
pub(crate) const LINE_HEIGHT_LABEL: f32 = 16.0;
#[allow(dead_code)] // Reserved for actual metadata.
pub(crate) const LINE_HEIGHT_META: f32 = 14.0;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typography_roles_use_one_integer_scale() {
        assert_eq!(
            [TEXT_TITLE, TEXT_BODY, TEXT_LABEL, TEXT_META],
            [14.0, 12.0, 11.0, 10.0]
        );
        assert_eq!(
            [
                LINE_HEIGHT_TITLE,
                LINE_HEIGHT_BODY,
                LINE_HEIGHT_LABEL,
                LINE_HEIGHT_META
            ],
            [20.0, 18.0, 16.0, 14.0]
        );
    }
}
