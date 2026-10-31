pub(crate) const TITLEBAR_HEIGHT: f32 = 36.0;
pub(crate) const MODULE_HEADER_HEIGHT: f32 = 48.0;
pub(crate) const SIDEBAR_WIDTH: f32 = 176.0;
#[cfg(test)] // Historical custom navigation geometry, not a Kit control override.
pub(crate) const SIDEBAR_ROW_HEIGHT: f32 = 40.0;
#[allow(dead_code)] // Reserved compact tier; current shared actions use 28px.
pub(crate) const CONTROL_HEIGHT_COMPACT: f32 = 24.0;
#[cfg(test)] // Historical custom control tier used only by the legacy token test.
pub(crate) const CONTROL_HEIGHT_DEFAULT: f32 = 24.0;
pub(crate) const CONTROL_HEIGHT_ACTION: f32 = 28.0;
pub(crate) const CONTROL_HEIGHT_INPUT: f32 = 32.0;
/// Two normal text lines (18+4+16) fit inside a compact grid-aligned row.
pub(crate) const SETTINGS_ROW_MIN_HEIGHT: f32 = 40.0;
/// Non-action badges and labels have separate measured content roles.
pub(crate) const BADGE_HEIGHT: f32 = 24.0;
/// Small icon glyph size (close crosses, inline carets).
pub(crate) const ICON_GLYPH_SMALL: f32 = 13.0;
/// Progress track height and hairline separators between metric rows.
pub(crate) const PROGRESS_TRACK_HEIGHT: f32 = 5.0;
/// Notification card two-line meta block height.
pub(crate) const NOTIFICATION_META_BLOCK_HEIGHT: f32 = 21.0;
pub(crate) const GROUP_HEADER_HEIGHT: f32 = 28.0;
pub(crate) const SETTINGS_FOOTER_HEIGHT: f32 = 48.0;
/// Feature-window title row uses normal Kit caption hit geometry.
pub(crate) const FEATURE_WINDOW_TITLE_LANE_HEIGHT: f32 = 32.0;

pub(crate) const COMPACT_RESULT_ROW_HEIGHT: f32 = 40.0;
pub(crate) const COMPACT_RESULT_ICON_RAIL_WIDTH: f32 = 26.0;
pub(crate) const COMPACT_RESULT_TRAILING_RAIL_WIDTH: f32 = 48.0;

/// Elevation contract (mirrors Zed's ElevationIndex). Every floating layer
/// renders exactly one of these combinations on `surface_raised`:
///
/// - ElevatedSurface (hover cards, hint bars, tooltips): shadow_sm
/// - ModalSurface (popovers, menus, inline dialogs): shadow_md
/// - Transient modals (clipboard edit sheet, pinned overlays): shadow_lg
///
/// The ui-consistency architecture gate enforces the pairing.
pub(crate) const RADIUS_NONE: f32 = 0.0;
pub(crate) const RADIUS_CONTROL: f32 = 3.0;
pub(crate) const RADIUS_FLOATING: f32 = 6.0;

pub(crate) const SPACING_4: f32 = 4.0;
pub(crate) const SPACING_8: f32 = 8.0;
pub(crate) const SPACING_12: f32 = 12.0;
pub(crate) const SPACING_16: f32 = 16.0;
pub(crate) const SPACING_24: f32 = 24.0;

/// Express an ordinary layout/type role in rem while preserving its density at
/// the product Root's verified 16px design base. Native, bitmap and fixed-client
/// budgets must keep their explicit pixel roles instead.
pub(crate) fn ordinary_rem(design_pixels: f32) -> gpui_kit::Rems {
    gpui_kit::rems(design_pixels / 16.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metrics_match_the_foundation_density_contract() {
        assert_eq!(
            [TITLEBAR_HEIGHT, MODULE_HEADER_HEIGHT, SIDEBAR_ROW_HEIGHT],
            [36.0, 48.0, 40.0]
        );
        assert_eq!(SIDEBAR_WIDTH, 176.0);
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
            [24.0, 24.0, 28.0, 32.0]
        );
        assert_eq!(
            [RADIUS_NONE, RADIUS_CONTROL, RADIUS_FLOATING],
            [0.0, 3.0, 6.0]
        );
        assert_eq!(
            [SPACING_4, SPACING_8, SPACING_12, SPACING_16, SPACING_24],
            [4.0, 8.0, 12.0, 16.0, 24.0]
        );
    }
}
