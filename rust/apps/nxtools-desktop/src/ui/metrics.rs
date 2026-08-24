pub(crate) const TITLEBAR_HEIGHT: f32 = 36.0;
pub(crate) const MODULE_HEADER_HEIGHT: f32 = 48.0;
pub(crate) const SIDEBAR_WIDTH: f32 = 208.0;
pub(crate) const SIDEBAR_ROW_HEIGHT: f32 = 42.0;
#[allow(dead_code)] // Consumed by compact icon controls as later surfaces migrate.
pub(crate) const CONTROL_HEIGHT_COMPACT: f32 = 18.0;
pub(crate) const CONTROL_HEIGHT_DEFAULT: f32 = 22.0;
pub(crate) const CONTROL_HEIGHT_ACTION: f32 = 28.0;
pub(crate) const CONTROL_HEIGHT_INPUT: f32 = 32.0;
pub(crate) const SETTINGS_ROW_MIN_HEIGHT: f32 = 42.0;
pub(crate) const SETTINGS_FOOTER_HEIGHT: f32 = 48.0;

pub(crate) const COMPACT_RESULT_ROW_HEIGHT: f32 = 40.0;
pub(crate) const COMPACT_RESULT_ICON_RAIL_WIDTH: f32 = 26.0;
pub(crate) const COMPACT_RESULT_TRAILING_RAIL_WIDTH: f32 = 48.0;

pub(crate) const RADIUS_CONTROL: f32 = 4.0;
pub(crate) const RADIUS_FLOATING: f32 = 6.0;

pub(crate) const SPACING_2: f32 = 2.0;
#[allow(dead_code)]
pub(crate) const SPACING_4: f32 = 4.0;
pub(crate) const SPACING_6: f32 = 6.0;
pub(crate) const SPACING_8: f32 = 8.0;
pub(crate) const SPACING_12: f32 = 12.0;
#[allow(dead_code)]
pub(crate) const SPACING_16: f32 = 16.0;
#[allow(dead_code)]
pub(crate) const SPACING_24: f32 = 24.0;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metrics_match_the_zed_density_contract() {
        assert_eq!(
            [TITLEBAR_HEIGHT, MODULE_HEADER_HEIGHT, SIDEBAR_ROW_HEIGHT],
            [36.0, 48.0, 42.0]
        );
        assert_eq!(SIDEBAR_WIDTH, 208.0);
        assert_eq!(
            [
                COMPACT_RESULT_ROW_HEIGHT,
                COMPACT_RESULT_ICON_RAIL_WIDTH,
                COMPACT_RESULT_TRAILING_RAIL_WIDTH
            ],
            [40.0, 26.0, 48.0]
        );
        assert_eq!(
            [
                CONTROL_HEIGHT_COMPACT,
                CONTROL_HEIGHT_DEFAULT,
                CONTROL_HEIGHT_ACTION,
                CONTROL_HEIGHT_INPUT
            ],
            [18.0, 22.0, 28.0, 32.0]
        );
        assert_eq!([RADIUS_CONTROL, RADIUS_FLOATING], [4.0, 6.0]);
        assert_eq!(
            [
                SPACING_2, SPACING_4, SPACING_6, SPACING_8, SPACING_12, SPACING_16, SPACING_24
            ],
            [2.0, 4.0, 6.0, 8.0, 12.0, 16.0, 24.0]
        );
    }
}
