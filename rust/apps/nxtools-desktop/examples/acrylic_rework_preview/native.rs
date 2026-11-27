//! DWM attributes for the preview windows. Product code keeps these calls in PlatformHost.
#![allow(unsafe_code)]
use std::ffi::c_void;

use gpui_kit::{Window, WindowBackgroundAppearance};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

const DWMWA_USE_IMMERSIVE_DARK_MODE: u32 = 20;
const DWMWA_CLOAK: u32 = 13;
const DWMWA_WINDOW_CORNER_PREFERENCE: u32 = 33;
const DWMWA_BORDER_COLOR: u32 = 34;
const DWMWA_SYSTEMBACKDROP_TYPE: u32 = 38;
const DWMWCP_DONOTROUND: u32 = 1;
const DWMWCP_ROUND: u32 = 2;
const DWMWA_COLOR_NONE: u32 = 0xFFFF_FFFE;
pub const SYSTEM_BACKDROP_BUILD: u32 = 22_621;
/// First build with `DWMWA_WINDOW_CORNER_PREFERENCE` (Windows 11 21H2).
const ROUNDED_CORNER_BUILD: u32 = 22_000;
/// Bottom frame margin covering any client height (Win32 coordinates are 16-bit bounded).
const FULL_CLIENT: i32 = 32_767;

#[repr(C)]
struct OsVersionInfo {
    size: u32,
    major: u32,
    minor: u32,
    build: u32,
    platform: u32,
    service_pack: [u16; 128],
}

#[link(name = "dwmapi")]
unsafe extern "system" {
    fn DwmSetWindowAttribute(
        hwnd: *mut c_void,
        attribute: u32,
        value: *const c_void,
        size: u32,
    ) -> i32;
    fn DwmExtendFrameIntoClientArea(hwnd: *mut c_void, margins: *const Margins) -> i32;
}

#[repr(C)]
struct Margins {
    left: i32,
    right: i32,
    top: i32,
    bottom: i32,
}

#[link(name = "ntdll")]
unsafe extern "system" {
    fn RtlGetVersion(info: *mut OsVersionInfo) -> i32;
}

#[link(name = "user32")]
unsafe extern "system" {
    fn GetDpiForWindow(hwnd: *mut c_void) -> u32;
}

/// Who shapes the window corners.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Corners {
    /// Windows 11: DWM rounding (attribute 33) with the system border turned off (34).
    System,
    /// Windows 10 (or forced): square, like every Windows 10 window.
    Square,
}

impl Corners {
    pub fn detect(force_square: bool) -> Self {
        if force_square || windows_build() < ROUNDED_CORNER_BUILD {
            Corners::Square
        } else {
            Corners::System
        }
    }

    pub fn tag(self) -> &'static str {
        match self {
            Corners::System => "dwm",
            Corners::Square => "square",
        }
    }
}

/// The WCA accent used where DWM system backdrops are unavailable (Windows 10).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccentBlur {
    /// ACCENT_ENABLE_BLURBEHIND (3): plain blur; the window follows the cursor when dragged.
    Blur,
    /// ACCENT_ENABLE_ACRYLICBLURBEHIND (4): acrylic; on Windows 10 1903+ a dragged window
    /// lags behind the cursor.
    Acrylic,
}

impl AccentBlur {
    pub fn toggled(self) -> Self {
        match self {
            AccentBlur::Blur => AccentBlur::Acrylic,
            AccentBlur::Acrylic => AccentBlur::Blur,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            AccentBlur::Blur => "Accent3 模糊",
            AccentBlur::Acrylic => "Accent4 亚克力",
        }
    }

    fn state(self) -> u32 {
        match self {
            AccentBlur::Blur => 3,
            AccentBlur::Acrylic => 4,
        }
    }
}

/// What a window is for decides which system backdrop it receives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Persistent,
    Transient,
}

/// One preview-wide material mode; F3 cycles through them in this order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Solid,
    Mica,
    MicaAlt,
    Acrylic,
    Accent,
}

impl Mode {
    pub const ALL: [Mode; 5] = [
        Mode::Solid,
        Mode::Mica,
        Mode::MicaAlt,
        Mode::Acrylic,
        Mode::Accent,
    ];

    /// Label on a system with DWM system backdrops; Windows 10 calls the accent mode "磨砂".
    pub fn label_for(self, system_backdrops: bool) -> &'static str {
        if self == Mode::Accent && !system_backdrops {
            "磨砂"
        } else {
            self.label()
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Mode::Solid => "实底",
            Mode::Mica => "Mica",
            Mode::MicaAlt => "Mica Alt",
            Mode::Acrylic => "亚克力",
            Mode::Accent => "Accent 回退",
        }
    }

    /// What a transient surface actually receives in this mode.
    pub fn transient_label(self) -> &'static str {
        match self {
            Mode::Mica | Mode::MicaAlt => Mode::Acrylic.label(),
            mode => mode.label(),
        }
    }

    pub fn tag(self) -> &'static str {
        match self {
            Mode::Solid => "solid",
            Mode::Mica => "mica",
            Mode::MicaAlt => "mica_alt",
            Mode::Acrylic => "acrylic",
            Mode::Accent => "accent4",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|mode| mode.tag() == value)
    }

    /// Modes this system can actually show. Without DWM system backdrops (Windows 10) Mica,
    /// Mica Alt and acrylic all fall back to the same accent blur, so only one frosted mode
    /// is offered.
    pub fn available(system_backdrops: bool) -> &'static [Mode] {
        if system_backdrops {
            &Self::ALL
        } else {
            &[Mode::Solid, Mode::Accent]
        }
    }

    /// The mode a system can show for a requested mode.
    pub fn effective(self, system_backdrops: bool) -> Self {
        if !system_backdrops && self.frosted() {
            Mode::Accent
        } else {
            self
        }
    }

    pub fn next(self, system_backdrops: bool) -> Self {
        let modes = Self::available(system_backdrops);
        let index = modes.iter().position(|mode| *mode == self).unwrap_or(0);
        modes[(index + 1) % modes.len()]
    }

    pub fn frosted(self) -> bool {
        self != Mode::Solid
    }

    /// DWMSBT value written to attribute 38 for a window of `role`; `None` keeps GPUI's own policy.
    fn backdrop(self, role: Role) -> Option<u32> {
        match (self, role) {
            (Mode::Solid | Mode::Accent, _) => Some(1),
            (_, Role::Transient) | (Mode::Acrylic, _) => Some(3),
            (Mode::Mica, Role::Persistent) => Some(2),
            (Mode::MicaAlt, Role::Persistent) => Some(4),
        }
    }

    fn appearance(self) -> WindowBackgroundAppearance {
        match self {
            Mode::Solid => WindowBackgroundAppearance::Opaque,
            Mode::Accent => WindowBackgroundAppearance::Blurred,
            _ => WindowBackgroundAppearance::Transparent,
        }
    }
}

/// Scalar outcome of one material application, printed as a diagnostic receipt.
#[derive(Clone, Copy, Debug)]
pub struct Receipt {
    pub backdrop: Option<u32>,
    pub backdrop_hr: i32,
    pub frame_hr: i32,
    pub accent: Option<u32>,
    pub corners: Corners,
    pub corner_hr: i32,
    pub border_hr: i32,
    pub dark_hr: i32,
    pub dpi: u32,
}

impl std::fmt::Display for Receipt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let backdrop = self
            .backdrop
            .map_or("gpui".to_string(), |value| value.to_string());
        write!(
            f,
            "backdrop={backdrop} backdrop_hr={:#x} frame_hr={:#x} accent={} dark_hr={:#x} corners={} corner_hr={:#x} border_hr={:#x} dpi={}",
            self.backdrop_hr,
            self.frame_hr,
            self.accent
                .map_or("gpui".to_string(), |state| state.to_string()),
            self.dark_hr,
            self.corners.tag(),
            self.corner_hr,
            self.border_hr,
            self.dpi
        )
    }
}

pub fn describe(receipt: Option<Receipt>) -> String {
    receipt.map_or_else(|| "unavailable".to_string(), |receipt| receipt.to_string())
}

pub fn hwnd(window: &Window) -> Option<*mut c_void> {
    let handle = HasWindowHandle::window_handle(window).ok()?;
    match handle.as_raw() {
        RawWindowHandle::Win32(handle) => Some(handle.hwnd.get() as *mut c_void),
        _ => None,
    }
}

pub fn windows_build() -> u32 {
    let mut info = OsVersionInfo {
        size: std::mem::size_of::<OsVersionInfo>() as u32,
        major: 0,
        minor: 0,
        build: 0,
        platform: 0,
        service_pack: [0; 128],
    };
    // SAFETY: `info` is a correctly sized OSVERSIONINFOW owned by this frame.
    if unsafe { RtlGetVersion(&mut info) } == 0 {
        info.build
    } else {
        0
    }
}

fn set_u32(hwnd: *mut c_void, attribute: u32, value: u32) -> i32 {
    // SAFETY: every attribute written here takes a 4-byte DWORD/BOOL value.
    unsafe {
        DwmSetWindowAttribute(
            hwnd,
            attribute,
            (&value as *const u32).cast(),
            std::mem::size_of::<u32>() as u32,
        )
    }
}

pub fn set_cloaked(hwnd: *mut c_void, cloaked: bool) -> i32 {
    set_u32(hwnd, DWMWA_CLOAK, u32::from(cloaked))
}

/// GPUI appearance first (it rewrites the WCA accent), then accent reset, frame extension and
/// DWM 38/20/33/34, so the system backdrop is what shows through transparent pixels.
pub fn apply(
    window: &mut Window,
    role: Role,
    mode: Mode,
    dark: bool,
    corners: Corners,
    accent_blur: AccentBlur,
) -> Option<Receipt> {
    let hwnd = hwnd(window)?;
    let build = windows_build();
    let mut mode = mode;
    if build < SYSTEM_BACKDROP_BUILD && matches!(mode, Mode::Mica | Mode::MicaAlt | Mode::Acrylic) {
        mode = Mode::Accent;
    }
    window.set_background_appearance(mode.appearance());
    // GPUI's Blurred appearance is acrylic (4); the accent fallback chooses its own state.
    let accent = if mode == Mode::Accent {
        set_accent(hwnd, accent_blur.state(), 0x0100_0000);
        Some(accent_blur.state())
    } else {
        None
    };
    let backdrop = if build >= SYSTEM_BACKDROP_BUILD {
        mode.backdrop(role)
    } else {
        None
    };
    let system_backdrop = matches!(backdrop, Some(2..=4));
    if system_backdrop {
        // GPUI's Transparent appearance leaves a transparent-gradient accent that would show
        // the windows behind instead of the system backdrop.
        set_accent(hwnd, 0, 0);
    }
    // The system backdrop is drawn only under the DWM frame. Extending the bottom frame edge
    // over the whole client carries it under the DirectComposition content; the top edge
    // stays unextended so DWM paints no caption tint and no caption buttons of its own.
    let margins = Margins {
        left: 0,
        right: 0,
        top: 0,
        bottom: if system_backdrop { FULL_CLIENT } else { 0 },
    };
    // SAFETY: `hwnd` belongs to a live GPUI window and `margins` outlives the call.
    let frame_hr = unsafe { DwmExtendFrameIntoClientArea(hwnd, &margins) };
    let backdrop_hr = backdrop.map_or(0, |value| set_u32(hwnd, DWMWA_SYSTEMBACKDROP_TYPE, value));
    let dark_hr = set_u32(hwnd, DWMWA_USE_IMMERSIVE_DARK_MODE, u32::from(dark));
    let (corner_hr, border_hr) = match corners {
        Corners::System => (
            set_u32(hwnd, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND),
            set_u32(hwnd, DWMWA_BORDER_COLOR, DWMWA_COLOR_NONE),
        ),
        // Windows 10 has neither attribute and its windows are square. On Windows 11 the forced
        // Windows 10 path turns system rounding off to show the same square window.
        Corners::Square => (
            set_u32(hwnd, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_DONOTROUND),
            0,
        ),
    };
    // SAFETY: `hwnd` belongs to a live GPUI window on this thread.
    let dpi = unsafe { GetDpiForWindow(hwnd) };
    Some(Receipt {
        backdrop,
        backdrop_hr,
        frame_hr,
        accent,
        corners,
        corner_hr,
        border_hr,
        dark_hr,
        dpi,
    })
}

#[repr(C)]
struct AccentPolicy {
    state: u32,
    flags: u32,
    gradient: u32,
    animation: u32,
}

#[repr(C)]
struct CompositionData {
    attribute: u32,
    data: *mut c_void,
    size: usize,
}

type SetWindowCompositionAttribute =
    unsafe extern "system" fn(hwnd: *mut c_void, data: *mut CompositionData) -> i32;

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetModuleHandleA(name: *const u8) -> *mut c_void;
    fn GetProcAddress(module: *mut c_void, name: *const u8) -> *mut c_void;
}

/// WCA_ACCENT_POLICY. State 0 clears the transparent-gradient accent GPUI's Transparent
/// appearance leaves behind; states 3/4 are the Windows 10 blur fallbacks.
fn set_accent(hwnd: *mut c_void, state: u32, gradient: u32) {
    let mut accent = AccentPolicy {
        state,
        flags: 0,
        gradient,
        animation: 0,
    };
    let mut data = CompositionData {
        attribute: 0x13,
        data: (&mut accent as *mut AccentPolicy).cast(),
        size: std::mem::size_of::<AccentPolicy>(),
    };
    // SAFETY: user32 is loaded in every GUI process; the export is resolved by name and called
    // with WCA_ACCENT_POLICY and a correctly sized, frame-owned ACCENT_POLICY.
    unsafe {
        let user32 = GetModuleHandleA(c"user32.dll".as_ptr().cast());
        let export = GetProcAddress(user32, c"SetWindowCompositionAttribute".as_ptr().cast());
        if !export.is_null() {
            let call: SetWindowCompositionAttribute = std::mem::transmute(export);
            call(hwnd, &mut data);
        }
    }
}
