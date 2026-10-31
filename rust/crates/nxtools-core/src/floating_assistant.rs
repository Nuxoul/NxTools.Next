use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ClickAction {
    None,
    #[default]
    OpenMain,
    AddDownload,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TextDropAction {
    None,
    #[default]
    Automatic,
    AddDownload,
    QuickLaunchSearch,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ImageDropAction {
    None,
    #[default]
    Automatic,
    AddDownload,
    WriteClipboard,
}

/// Durable revealed anchor in physical screen pixels, not a temporary edge-hidden position.
/// Monitor identity and DPI are hints for the desktop's work-area recovery, not native handles.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct FloatingAssistantPosition {
    pub x: i32,
    pub y: i32,
    pub monitor_id: Option<String>,
    pub dpi: Option<u32>,
}

impl Default for FloatingAssistantPosition {
    fn default() -> Self {
        Self {
            x: 40,
            y: 180,
            monitor_id: None,
            dpi: None,
        }
    }
}

/// Preferences only: actual HWND state and native apply status belong to Desktop/PlatformHost.
/// Topmost is a native feature invariant, not a user preference.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct FloatingAssistantSettings {
    pub enabled: bool,
    pub image_path: Option<String>,
    /// Logical pixels; 20..=300.
    pub size: u32,
    /// Maximum GIF display frames per second; 1..=60.
    pub frame_rate: u32,
    pub synchronize_download_speed: bool,
    /// Percent; 10..=100. Hover opacity must not be below base opacity.
    pub base_opacity: u32,
    pub hover_opacity: u32,
    /// Logical pixels; 4..=120.
    pub edge_snap_distance: u32,
    pub auto_hide_at_edge: bool,
    pub hide_in_fullscreen: bool,
    pub position_locked: bool,
    pub click_through: bool,
    pub position: FloatingAssistantPosition,
    pub left_click_action: ClickAction,
    pub right_click_action: ClickAction,
    pub text_drop_action: TextDropAction,
    pub image_drop_action: ImageDropAction,
}

impl Default for FloatingAssistantSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            image_path: None,
            size: 72,
            frame_rate: 24,
            synchronize_download_speed: false,
            base_opacity: 48,
            hover_opacity: 96,
            edge_snap_distance: 24,
            auto_hide_at_edge: true,
            hide_in_fullscreen: true,
            position_locked: false,
            click_through: false,
            position: FloatingAssistantPosition::default(),
            left_click_action: ClickAction::OpenMain,
            right_click_action: ClickAction::AddDownload,
            text_drop_action: TextDropAction::Automatic,
            image_drop_action: ImageDropAction::Automatic,
        }
    }
}

impl FloatingAssistantSettings {
    pub fn normalize(&mut self) {
        self.size = self.size.clamp(20, 300);
        self.frame_rate = self.frame_rate.clamp(1, 60);
        self.base_opacity = self.base_opacity.clamp(10, 100);
        self.hover_opacity = self.hover_opacity.clamp(self.base_opacity, 100);
        self.edge_snap_distance = self.edge_snap_distance.clamp(4, 120);
        self.image_path = self.image_path.take().and_then(|path| {
            let path = path.trim();
            (!path.is_empty() && path.len() <= 4096 && !path.contains('\0'))
                .then(|| path.to_owned())
        });
        self.position.monitor_id = self.position.monitor_id.take().and_then(|id| {
            let id = id.trim();
            (!id.is_empty() && id.len() <= 256 && !id.contains('\0')).then(|| id.to_owned())
        });
        // Windows DPI is normally 96; unknown/out-of-range hints are discarded rather than
        // clamping to an invented scale. The desktop will query the current monitor on restore.
        self.position.dpi = self.position.dpi.filter(|dpi| (48..=960).contains(dpi));
    }
}
