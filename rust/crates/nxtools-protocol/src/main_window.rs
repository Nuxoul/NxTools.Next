//! One-time hidden primary-window preparation, sizes in logical DIP.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrepareMainWindowCommand {
    pub window_handle: i64,
    pub source_process_id: u32,
    pub width: i32,
    pub height: i32,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreparedMainWindow {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub dpi: u32,
}
impl PreparedMainWindow {
    pub fn matches(&self, width: i32, height: i32) -> bool {
        (920..=4096).contains(&width)
            && (480..=4096).contains(&height)
            && (48..=960).contains(&self.dpi)
            && self.width == (f64::from(width) * f64::from(self.dpi) / 96.0).round() as i32
            && self.height == (f64::from(height) * f64::from(self.dpi) / 96.0).round() as i32
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn r5_main_window_wire_and_native_dpi_readback_are_exact() {
        let command = crate::PlatformCommand::PrepareMainWindow(PrepareMainWindowCommand {
            window_handle: 42,
            source_process_id: 7,
            width: 1060,
            height: 650,
        });
        let wire = serde_json::to_value(&command).expect("serialize main prepare");
        assert_eq!(wire["command"], "prepare_main_window");
        assert_eq!(
            serde_json::from_value::<crate::PlatformCommand>(wire).expect("roundtrip"),
            command
        );
        for dpi in [96, 120, 144, 192] {
            let mut actual = PreparedMainWindow {
                x: -100,
                y: 20,
                width: (1060.0 * f64::from(dpi) / 96.0).round() as i32,
                height: (650.0 * f64::from(dpi) / 96.0).round() as i32,
                dpi,
            };
            assert!(actual.matches(1060, 650));
            actual.width += 1;
            assert!(!actual.matches(1060, 650));
            actual.dpi = 0;
            assert!(!actual.matches(1060, 650));
        }
        let scene = PreparedMainWindow {
            x: 208,
            y: 208,
            width: 1904,
            height: 1021,
            dpi: 96,
        };
        assert!(
            !scene.matches(1060, 650),
            "captured enlarged scene cannot pass readiness"
        );
    }
}
