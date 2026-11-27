//! Win32 side of the Windows 10 corner lab: accent blur, window region, DWM shadow policy,
//! margin hit-testing and the drag-time material switch. Product code keeps these in PlatformHost.
#![allow(unsafe_code)]
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::ffi::c_void;
use std::time::Instant;

use gpui_kit::{Window, WindowBackgroundAppearance};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

const GWLP_WNDPROC: i32 = -4;
const WM_SIZE: u32 = 0x0005;
const WM_CANCELMODE: u32 = 0x001F;
const WM_WINDOWPOSCHANGED: u32 = 0x0047;
const WM_NCHITTEST: u32 = 0x0084;
const WM_SYSCOMMAND: u32 = 0x0112;
const SC_MOVE_CAPTION: usize = 0xF012;
const WM_MOVING: u32 = 0x0216;
const WM_ENTERSIZEMOVE: u32 = 0x0231;
const WM_EXITSIZEMOVE: u32 = 0x0232;
const SWP_NOZORDER: u32 = 0x0004;
const SWP_NOACTIVATE: u32 = 0x0010;
const DWMWA_NCRENDERING_POLICY: u32 = 2;
const DWMWA_WINDOW_CORNER_PREFERENCE: u32 = 33;
const DWMWCP_DONOTROUND: u32 = 1;
const ROUNDED_CORNER_BUILD: u32 = 22_000;
const ACCENT_DISABLED: u32 = 0;
const ACCENT_GRADIENT: u32 = 1;
const ACCENT_BLUR: u32 = 3;
const ACCENT_HOST_BACKDROP: u32 = 5;
const DWMWA_USE_HOSTBACKDROPBRUSH: u32 = 17;
const WM_TIMER: u32 = 0x0113;
const SWP_NOSIZE: u32 = 0x0001;
/// Lab-owned timer id; GPUI uses 1 and 2 on the same window.
const THROTTLE_TIMER_ID: usize = 0x4E58;
/// Packed `DWM_TIMING_INFO` size.
const TIMING_INFO_SIZE: usize = 292;

/// Source of the frosted material.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Backdrop {
    /// Accent-policy blur behind the window (the current product path).
    Accent,
    /// DWM host backdrop painted by a Windows.UI.Composition visual (the Terminal path).
    Host,
}

impl Backdrop {
    pub fn tag(self) -> &'static str {
        match self {
            Backdrop::Accent => "accent",
            Backdrop::Host => "host",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Backdrop::Accent => "强调色模糊（现状）",
            Backdrop::Host => "宿主背景（Terminal 方式）",
        }
    }
}

/// What a frosted window shows while it is dragged or sized.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DragMode {
    /// The accent blur stays on.
    Blur,
    /// The accent switches to an opaque gradient tint; GPUI content stays translucent.
    Tint,
    /// The accent turns off and GPUI repaints the body opaque: the window is solid for the drag.
    Solid,
}

impl DragMode {
    pub const ALL: [DragMode; 3] = [DragMode::Blur, DragMode::Tint, DragMode::Solid];

    pub fn tag(self) -> &'static str {
        match self {
            DragMode::Blur => "blur",
            DragMode::Tint => "tint",
            DragMode::Solid => "solid",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            DragMode::Blur => "保持模糊",
            DragMode::Tint => "强调色实色",
            DragMode::Solid => "整窗实底",
        }
    }

    pub fn next(self) -> Self {
        let index = Self::ALL.iter().position(|value| *value == self).unwrap_or(0);
        Self::ALL[(index + 1) % Self::ALL.len()]
    }
}

/// How often a dragged window may actually move.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Throttle {
    /// Every mouse step moves the window (system default).
    Off,
    /// At most once per display refresh.
    Frame,
    /// At most once every second display refresh.
    TwoFrames,
}

impl Throttle {
    pub const ALL: [Throttle; 3] = [Throttle::Off, Throttle::Frame, Throttle::TwoFrames];

    pub fn tag(self) -> &'static str {
        match self {
            Throttle::Off => "off",
            Throttle::Frame => "frame",
            Throttle::TwoFrames => "frame2",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Throttle::Off => "不节流",
            Throttle::Frame => "每帧移动一次",
            Throttle::TwoFrames => "每两帧移动一次",
        }
    }

    pub fn next(self) -> Self {
        let index = Self::ALL.iter().position(|value| *value == self).unwrap_or(0);
        Self::ALL[(index + 1) % Self::ALL.len()]
    }

    fn frames(self) -> u32 {
        match self {
            Throttle::Off => 0,
            Throttle::Frame => 1,
            Throttle::TwoFrames => 2,
        }
    }
}

/// How the window gets its corners.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Corners {
    /// Today's Windows 10 product: square window with the DWM shadow.
    Square,
    /// `SetWindowRgn` rounded rectangle: clips the window (and the blur); no DWM shadow.
    Region,
    /// Transparent shadow margin around a self-drawn rounded body; DWM shadow off.
    Margin,
    /// The rounded region plus a separate click-through layered window that draws the shadow
    /// and an antialiased rim around it (the Visual Studio / Office technique).
    RegionShadow,
}

impl Corners {
    pub const ALL: [Corners; 4] = [
        Corners::Square,
        Corners::Region,
        Corners::Margin,
        Corners::RegionShadow,
    ];

    pub fn tag(self) -> &'static str {
        match self {
            Corners::Square => "square",
            Corners::Region => "region",
            Corners::Margin => "margin",
            Corners::RegionShadow => "region-shadow",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Corners::Square => "① 方角（现状）",
            Corners::Region => "② 窗口区域裁圆角",
            Corners::Margin => "③ 自绘圆角 + 自绘阴影",
            Corners::RegionShadow => "④ 区域圆角 + 独立阴影窗口",
        }
    }

    pub fn next(self) -> Self {
        let index = Self::ALL.iter().position(|value| *value == self).unwrap_or(0);
        Self::ALL[(index + 1) % Self::ALL.len()]
    }

    /// The window itself is clipped to the rounded shape.
    pub fn clips(self) -> bool {
        matches!(self, Corners::Region | Corners::RegionShadow)
    }
}

/// Everything the native side needs for one window.
#[derive(Clone, Copy, Debug)]
pub struct Config {
    pub frosted: bool,
    /// Where the frosted material comes from.
    pub backdrop: Backdrop,
    pub corners: Corners,
    /// What a frosted window shows while it is dragged or sized.
    pub drag: DragMode,
    /// Hide the separate shadow window while the window is dragged or sized.
    pub drag_hide_shadow: bool,
    /// Move the window at most once per display refresh (or every second one) while dragging.
    pub throttle: Throttle,
    /// Opaque tint for the drag-time gradient accent, 0xAABBGGRR.
    pub tint: u32,
    pub radius_dip: f32,
    pub margin_dip: f32,
    pub resizable: bool,
    pub dark: bool,
}

#[derive(Clone, Copy)]
struct Lab {
    old_proc: isize,
    config: Config,
    /// Margin currently added around the body, device pixels.
    margin_px: i32,
    radius_px: i32,
    drag: Option<DragStats>,
    /// Layered shadow window of the region-shadow strategy, 0 until first needed.
    shadow: isize,
    /// Body size and palette the shadow bitmap was last rendered for.
    shadow_rendered: Option<(i32, i32, bool, i32)>,
}

/// One native move/size loop: mouse-step cadence on the window's thread.
#[derive(Clone, Copy)]
struct DragStats {
    started: Instant,
    last: Instant,
    moves: u32,
    max_gap_ms: f64,
    slow_gaps: u32,
    via: &'static str,
    /// Minimum time between applied window moves, 0 when moves are not throttled.
    interval_ms: f64,
    applied_at: Option<Instant>,
    /// Last rectangle the window was actually moved to.
    applied: Rect,
    /// A step was held back by the throttle and has not landed yet.
    pending: bool,
    /// Window rectangle and cursor position when the loop started.
    origin: Rect,
    cursor: Point,
    held: u32,
    flushed: u32,
}

thread_local! {
    static LABS: RefCell<HashMap<isize, Lab>> = RefCell::new(HashMap::new());
    /// Per-window channel that tells the GPUI view a solid drag started (`true`) or ended.
    static DRAG_EVENTS: RefCell<HashMap<isize, async_channel::Sender<bool>>> =
        RefCell::new(HashMap::new());
    /// Entry path of the next move loop; `begin_system_move` marks the product main-window path.
    static NEXT_VIA: Cell<&'static str> = const { Cell::new("caption") };
}

#[repr(C)]
#[derive(Default, Clone, Copy)]
struct Point {
    x: i32,
    y: i32,
}

#[repr(C)]
#[derive(Default, Clone, Copy)]
struct Rect {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
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

#[repr(C)]
struct OsVersionInfo {
    size: u32,
    major: u32,
    minor: u32,
    build: u32,
    platform: u32,
    service_pack: [u16; 128],
}

type WndProc = unsafe extern "system" fn(*mut c_void, u32, usize, isize) -> isize;
type SetWindowCompositionAttribute =
    unsafe extern "system" fn(hwnd: *mut c_void, data: *mut CompositionData) -> i32;

#[link(name = "user32")]
unsafe extern "system" {
    fn SetWindowLongPtrW(hwnd: *mut c_void, index: i32, value: isize) -> isize;
    fn CallWindowProcW(proc: isize, hwnd: *mut c_void, msg: u32, w: usize, l: isize) -> isize;
    fn GetWindowRect(hwnd: *mut c_void, rect: *mut Rect) -> i32;
    fn SetWindowPos(
        hwnd: *mut c_void,
        after: *mut c_void,
        x: i32,
        y: i32,
        cx: i32,
        cy: i32,
        flags: u32,
    ) -> i32;
    fn SetWindowRgn(hwnd: *mut c_void, region: *mut c_void, redraw: i32) -> i32;
    fn GetDpiForWindow(hwnd: *mut c_void) -> u32;
    fn IsZoomed(hwnd: *mut c_void) -> i32;
    fn IsIconic(hwnd: *mut c_void) -> i32;
    fn IsWindowVisible(hwnd: *mut c_void) -> i32;
    fn DefWindowProcW(hwnd: *mut c_void, msg: u32, w: usize, l: isize) -> isize;
    fn PostMessageW(hwnd: *mut c_void, msg: u32, w: usize, l: isize) -> i32;
    fn SetTimer(hwnd: *mut c_void, id: usize, elapse: u32, proc: *const c_void) -> usize;
    fn KillTimer(hwnd: *mut c_void, id: usize) -> i32;
    fn GetCursorPos(point: *mut Point) -> i32;
}

#[link(name = "gdi32")]
unsafe extern "system" {
    fn CreateRoundRectRgn(l: i32, t: i32, r: i32, b: i32, w: i32, h: i32) -> *mut c_void;
}

#[link(name = "dwmapi")]
unsafe extern "system" {
    fn DwmSetWindowAttribute(hwnd: *mut c_void, attr: u32, value: *const c_void, size: u32)
    -> i32;
    fn DwmGetCompositionTimingInfo(hwnd: *mut c_void, info: *mut u8) -> i32;
}

#[link(name = "ntdll")]
unsafe extern "system" {
    fn RtlGetVersion(info: *mut OsVersionInfo) -> i32;
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetModuleHandleA(name: *const u8) -> *mut c_void;
    fn GetProcAddress(module: *mut c_void, name: *const u8) -> *mut c_void;
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
    // SAFETY: every attribute written here takes a 4-byte DWORD value.
    unsafe { DwmSetWindowAttribute(hwnd, attribute, (&value as *const u32).cast(), 4) }
}

/// WCA_ACCENT_POLICY. Returns the call's BOOL result.
fn set_accent(hwnd: *mut c_void, state: u32, flags: u32, gradient: u32) -> i32 {
    let mut accent = AccentPolicy {
        state,
        flags,
        gradient,
        animation: 0,
    };
    let mut data = CompositionData {
        attribute: 0x13,
        data: (&mut accent as *mut AccentPolicy).cast(),
        size: std::mem::size_of::<AccentPolicy>(),
    };
    // SAFETY: user32 is loaded in every GUI process; the export is resolved by name and called
    // with WCA_ACCENT_POLICY and a frame-owned ACCENT_POLICY.
    unsafe {
        let user32 = GetModuleHandleA(c"user32.dll".as_ptr().cast());
        let export = GetProcAddress(user32, c"SetWindowCompositionAttribute".as_ptr().cast());
        if export.is_null() {
            return 0;
        }
        let call: SetWindowCompositionAttribute = std::mem::transmute(export);
        call(hwnd, &mut data)
    }
}

/// Display refresh period from DWM, 60 Hz when unavailable.
fn refresh_ms() -> f64 {
    let mut info = [0u8; TIMING_INFO_SIZE];
    info[..4].copy_from_slice(&(TIMING_INFO_SIZE as u32).to_le_bytes());
    // SAFETY: `info` is a frame-owned buffer of the packed DWM_TIMING_INFO size with cbSize set;
    // since Windows 8.1 the call requires a null window.
    let hr = unsafe { DwmGetCompositionTimingInfo(std::ptr::null_mut(), info.as_mut_ptr()) };
    let field = |offset: usize| {
        let mut bytes = [0u8; 4];
        bytes.copy_from_slice(&info[offset..offset + 4]);
        u32::from_le_bytes(bytes)
    };
    // rateRefresh is a numerator/denominator pair in Hz.
    let (numerator, denominator) = (field(4), field(8));
    if hr == 0 && numerator > 0 && denominator > 0 {
        1000.0 * f64::from(denominator) / f64::from(numerator)
    } else {
        1000.0 / 60.0
    }
}

fn cursor_position() -> Point {
    let mut point = Point::default();
    // SAFETY: `point` is a frame-owned POINT.
    unsafe { GetCursorPos(&mut point) };
    point
}

/// Where the cursor says the window belongs now: the start rectangle shifted by the cursor's
/// travel since the loop began.
fn drag_target(drag: &DragStats) -> Rect {
    let cursor = cursor_position();
    let (dx, dy) = (cursor.x - drag.cursor.x, cursor.y - drag.cursor.y);
    Rect {
        left: drag.origin.left + dx,
        top: drag.origin.top + dy,
        right: drag.origin.right + dx,
        bottom: drag.origin.bottom + dy,
    }
}

/// Moves the window to `rect`'s origin without resizing or reordering it.
fn move_window(hwnd: *mut c_void, rect: Rect) {
    // SAFETY: `hwnd` is a live window of this thread.
    unsafe {
        SetWindowPos(
            hwnd,
            std::ptr::null_mut(),
            rect.left,
            rect.top,
            0,
            0,
            SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
        );
    }
}

fn blur_accent(hwnd: *mut c_void) -> i32 {
    set_accent(hwnd, ACCENT_BLUR, 0, 0x0100_0000)
}


/// Receiver of this window's solid-drag transitions.
pub fn drag_events(hwnd: *mut c_void) -> async_channel::Receiver<bool> {
    let (sender, receiver) = async_channel::unbounded();
    DRAG_EVENTS.with(|events| events.borrow_mut().insert(hwnd as isize, sender));
    receiver
}

fn send_drag_event(hwnd: *mut c_void, active: bool) {
    DRAG_EVENTS.with(|events| {
        if let Some(sender) = events.borrow().get(&(hwnd as isize)) {
            let _ = sender.try_send(active);
        }
    });
}

/// Solid drag body for the GPUI side: Opaque background (accent off) while dragging, the
/// transparent background plus blur accent afterwards. Returns the accent call's result.
pub fn set_drag_body(window: &mut Window, solid: bool) -> i32 {
    let Some(hwnd) = hwnd(window) else {
        return 0;
    };
    if solid {
        window.set_background_appearance(WindowBackgroundAppearance::Opaque);
        1
    } else {
        // GPUI writes accent 2 for Transparent, so the blur goes on afterwards.
        window.set_background_appearance(WindowBackgroundAppearance::Transparent);
        blur_accent(hwnd)
    }
}

/// The product main-window drag: cancel the pointer capture, then post SC_MOVE.
pub fn begin_system_move(hwnd: *mut c_void) {
    NEXT_VIA.with(|via| via.set("sc_move"));
    // SAFETY: `hwnd` is a live window of this thread; both messages are posted.
    unsafe {
        PostMessageW(hwnd, WM_CANCELMODE, 0, 0);
        PostMessageW(hwnd, WM_SYSCOMMAND, SC_MOVE_CAPTION, 0);
    }
}

fn window_rect(hwnd: *mut c_void) -> Rect {
    let mut rect = Rect::default();
    // SAFETY: `hwnd` is a live window of this thread; `rect` is frame-owned.
    unsafe { GetWindowRect(hwnd, &mut rect) };
    rect
}

fn scale(hwnd: *mut c_void) -> f32 {
    // SAFETY: `hwnd` is a live window of this thread.
    let dpi = unsafe { GetDpiForWindow(hwnd) };
    if dpi == 0 { 1.0 } else { dpi as f32 / 96.0 }
}

/// Rounded region over the whole window (minus any margin). Returns SetWindowRgn's result.
fn apply_region(hwnd: *mut c_void, lab: &Lab) -> i32 {
    // SAFETY: `hwnd` is a live window of this thread; the region handle is owned by the system
    // after a successful SetWindowRgn, and a null region removes the current one.
    unsafe {
        if !lab.config.corners.clips() || IsZoomed(hwnd) != 0 {
            return SetWindowRgn(hwnd, std::ptr::null_mut(), 1);
        }
        let rect = window_rect(hwnd);
        let (width, height) = (rect.right - rect.left, rect.bottom - rect.top);
        let diameter = lab.radius_px * 2;
        let region = CreateRoundRectRgn(0, 0, width + 1, height + 1, diameter, diameter);
        SetWindowRgn(hwnd, region, 1)
    }
}

/// Subclasses the window once so the lab sees hit tests, sizes and the move loop.
pub fn install(hwnd: *mut c_void, config: Config) {
    let key = hwnd as isize;
    if LABS.with(|labs| labs.borrow().contains_key(&key)) {
        return;
    }
    // SAFETY: replaces this window's procedure with `lab_proc`, which forwards every message
    // to the previous procedure stored below.
    let old_proc = unsafe { SetWindowLongPtrW(hwnd, GWLP_WNDPROC, lab_proc as WndProc as isize) };
    LABS.with(|labs| {
        labs.borrow_mut().insert(
            key,
            Lab {
                old_proc,
                config,
                margin_px: 0,
                radius_px: 0,
                drag: None,
                shadow: 0,
                shadow_rendered: None,
            },
        )
    });
}

/// Applies one configuration: GPUI background, accent, DWM shadow policy, margin geometry and
/// region. Returns a diagnostic receipt.
pub fn apply(window: &mut Window, config: Config) -> String {
    let Some(hwnd) = hwnd(window) else {
        return "hwnd=none".into();
    };
    let margin = config.corners == Corners::Margin;
    // GPUI rewrites the accent when the background changes, so it goes first. Solid square and
    // region windows stay Opaque and keep ClearType; the margin needs transparent pixels.
    window.set_background_appearance(if config.frosted || margin {
        WindowBackgroundAppearance::Transparent
    } else {
        WindowBackgroundAppearance::Opaque
    });
    let host_backdrop = config.frosted && config.backdrop == Backdrop::Host;
    let accent_ok = if host_backdrop {
        // The window opts into DWM's host backdrop; the composition visual paints it.
        set_accent(hwnd, ACCENT_HOST_BACKDROP, 0, 0)
    } else if config.frosted {
        blur_accent(hwnd)
    } else if margin {
        set_accent(hwnd, ACCENT_DISABLED, 0, 0)
    } else {
        1
    };
    let host = crate::host::show(hwnd, host_backdrop);
    // DWMNCRP_DISABLED removes the rectangular DWM shadow; 0 restores the window-style default.
    let shadow_hr = set_u32(hwnd, DWMWA_NCRENDERING_POLICY, if margin { 1 } else { 0 });
    // On Windows 11 the lab turns system rounding off to show the Windows 10 starting point,
    // and the documented host-backdrop switch replaces the accent state.
    let (corner_hr, backdrop_hr) = if windows_build() >= ROUNDED_CORNER_BUILD {
        (
            set_u32(hwnd, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_DONOTROUND),
            set_u32(hwnd, DWMWA_USE_HOSTBACKDROPBRUSH, u32::from(host_backdrop)),
        )
    } else {
        (0, 0)
    };
    let factor = scale(hwnd);
    let wanted_margin = if margin {
        (config.margin_dip * factor).round() as i32
    } else {
        0
    };
    let current = LABS.with(|labs| labs.borrow().get(&(hwnd as isize)).copied());
    let Some(mut lab) = current else {
        return "subclass=missing".into();
    };
    let delta = wanted_margin - lab.margin_px;
    lab.config = config;
    lab.margin_px = wanted_margin;
    lab.radius_px = (config.radius_dip * factor).round() as i32;
    if config.corners == Corners::RegionShadow && lab.shadow == 0 {
        lab.shadow = crate::shadow::create(hwnd) as isize;
    }
    LABS.with(|labs| labs.borrow_mut().insert(hwnd as isize, lab));
    sync_shadow(hwnd);
    // Geometry and region change from a helper thread: a synchronous SetWindowPos here would
    // deliver WM_SIZE to GPUI while this window is being updated. The system marshals the
    // calls to the window's thread, which handles them once the current update returns.
    let target = hwnd as usize;
    std::thread::spawn(move || {
        let hwnd = target as *mut c_void;
        if delta != 0 {
            let rect = window_rect(hwnd);
            // SAFETY: `hwnd` names a live window of this process; the call is marshalled.
            unsafe {
                SetWindowPos(
                    hwnd,
                    std::ptr::null_mut(),
                    rect.left - delta,
                    rect.top - delta,
                    rect.right - rect.left + 2 * delta,
                    rect.bottom - rect.top + 2 * delta,
                    SWP_NOZORDER | SWP_NOACTIVATE,
                );
            }
        }
        let region_ok = apply_region(hwnd, &lab);
        crate::log_line(format!(
            "lab.geometry hwnd={target} delta_px={delta} region_ok={region_ok}"
        ));
    });
    format!(
        "frosted={} backdrop={} {host} corners={} drag={} accent_ok={accent_ok} shadow_hr={shadow_hr:#x} corner_hr={corner_hr:#x} backdrop_hr={backdrop_hr:#x} margin_px={wanted_margin} radius_px={} scale={factor}",
        config.frosted,
        config.backdrop.tag(),
        config.corners.tag(),
        config.drag.tag(),
        lab.radius_px
    )
}

fn hit_test_margin(hwnd: *mut c_void, lparam: isize, lab: &Lab) -> Option<isize> {
    // SAFETY: `hwnd` is a live window of this thread.
    if lab.margin_px == 0 || unsafe { IsZoomed(hwnd) } != 0 {
        return None;
    }
    let x = (lparam & 0xFFFF) as i16 as i32;
    let y = ((lparam >> 16) & 0xFFFF) as i16 as i32;
    let rect = window_rect(hwnd);
    let band = lab.margin_px;
    let corner = band + lab.radius_px;
    let (left, right) = (x < rect.left + band, x >= rect.right - band);
    let (top, bottom) = (y < rect.top + band, y >= rect.bottom - band);
    if !(left || right || top || bottom) {
        return None;
    }
    if !lab.config.resizable {
        return Some(0); // HTNOWHERE
    }
    let near_left = x < rect.left + corner;
    let near_right = x >= rect.right - corner;
    let near_top = y < rect.top + corner;
    let near_bottom = y >= rect.bottom - corner;
    Some(match () {
        _ if near_top && near_left => 13,
        _ if near_top && near_right => 14,
        _ if near_bottom && near_left => 16,
        _ if near_bottom && near_right => 17,
        _ if left => 10,
        _ if right => 11,
        _ if top => 12,
        _ => 15,
    })
}

unsafe extern "system" fn lab_proc(hwnd: *mut c_void, msg: u32, wparam: usize, lparam: isize) -> isize {
    let key = hwnd as isize;
    let Some(lab) = LABS.with(|labs| labs.borrow().get(&key).copied()) else {
        // SAFETY: default processing for a window the lab never recorded.
        return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
    };
    match msg {
        WM_NCHITTEST => {
            if let Some(hit) = hit_test_margin(hwnd, lparam, &lab) {
                return hit;
            }
        }
        WM_ENTERSIZEMOVE => {
            // Drag-time switches rewrite the accent, so they apply to the accent-blur material only.
            let mode = if lab.config.frosted && lab.config.backdrop == Backdrop::Accent {
                lab.config.drag
            } else {
                DragMode::Blur
            };
            match mode {
                DragMode::Blur => {}
                DragMode::Tint => {
                    set_accent(hwnd, ACCENT_GRADIENT, 2, lab.config.tint);
                }
                DragMode::Solid => {
                    // The blur stops now; GPUI repaints the body opaque on its next timer turn.
                    set_accent(hwnd, ACCENT_DISABLED, 0, 0);
                    send_drag_event(hwnd, true);
                }
            }
            let now = Instant::now();
            let via = NEXT_VIA.with(|via| via.replace("caption"));
            let interval_ms = refresh_ms() * f64::from(lab.config.throttle.frames());
            let start_rect = window_rect(hwnd);
            LABS.with(|labs| {
                if let Some(entry) = labs.borrow_mut().get_mut(&key) {
                    entry.drag = Some(DragStats {
                        started: now,
                        last: now,
                        moves: 0,
                        max_gap_ms: 0.0,
                        slow_gaps: 0,
                        via,
                        interval_ms,
                        applied_at: None,
                        applied: start_rect,
                        pending: false,
                        origin: start_rect,
                        cursor: cursor_position(),
                        held: 0,
                        flushed: 0,
                    });
                }
            });
            let hidden = lab.config.drag_hide_shadow && lab.shadow != 0;
            if hidden {
                crate::shadow::hide(lab.shadow as *mut c_void);
            }
            crate::log_line(format!(
                "lab.drag.enter hwnd={key} via={via} frosted={} drag={} throttle={} interval_ms={interval_ms:.1} shadow_hidden={hidden}",
                lab.config.frosted,
                mode.tag(),
                lab.config.throttle.tag()
            ));
        }
        WM_MOVING => {
            let now = Instant::now();
            LABS.with(|labs| {
                if let Some(entry) = labs.borrow_mut().get_mut(&key)
                    && let Some(drag) = entry.drag.as_mut()
                {
                    let gap = now.duration_since(drag.last).as_secs_f64() * 1000.0;
                    if drag.moves > 0 {
                        drag.max_gap_ms = drag.max_gap_ms.max(gap);
                        // Longer than two 60 Hz frames between mouse steps: the thread stalled.
                        if gap > 33.0 {
                            drag.slow_gaps += 1;
                        }
                    }
                    drag.last = now;
                    drag.moves += 1;
                    if drag.interval_ms > 0.0 && lparam != 0 {
                        // SAFETY: WM_MOVING carries the proposed window rectangle, which the
                        // window may rewrite before the move loop applies it.
                        let proposed = unsafe { &mut *(lparam as *mut Rect) };
                        let since = drag
                            .applied_at
                            .map(|at| now.duration_since(at).as_secs_f64() * 1000.0);
                        match since {
                            Some(elapsed) if elapsed < drag.interval_ms => {
                                // Hold the window where it is; the cursor-derived target lands
                                // when the interval ends, by a later step or the flush timer.
                                *proposed = drag.applied;
                                drag.pending = true;
                                drag.held += 1;
                                let wait = (drag.interval_ms - elapsed).ceil().max(1.0) as u32;
                                // SAFETY: `hwnd` is this thread's window; the timer is ours.
                                unsafe { SetTimer(hwnd, THROTTLE_TIMER_ID, wait, std::ptr::null()) };
                            }
                            _ => {
                                // The loop proposes from the current position, so held steps
                                // would be lost; the target follows the cursor instead.
                                *proposed = drag_target(drag);
                                drag.applied = *proposed;
                                drag.applied_at = Some(now);
                                drag.pending = false;
                            }
                        }
                    }
                    }
            });
        }
        WM_TIMER if wparam == THROTTLE_TIMER_ID => {
            // SAFETY: stops this window's own one-shot throttle timer.
            unsafe { KillTimer(hwnd, THROTTLE_TIMER_ID) };
            let target = LABS.with(|labs| {
                let mut labs = labs.borrow_mut();
                let drag = labs.get_mut(&key)?.drag.as_mut()?;
                if !drag.pending {
                    return None;
                }
                drag.pending = false;
                drag.applied = drag_target(drag);
                drag.applied_at = Some(Instant::now());
                drag.flushed += 1;
                Some(drag.applied)
            });
            if let Some(rect) = target {
                move_window(hwnd, rect);
            }
            return 0;
        }
        WM_EXITSIZEMOVE => {
            // SAFETY: stops this window's own throttle timer if it is still armed.
            unsafe { KillTimer(hwnd, THROTTLE_TIMER_ID) };
            let drag = LABS.with(|labs| {
                labs.borrow_mut()
                    .get_mut(&key)
                    .and_then(|entry| entry.drag.take())
            });
            // On release the loop re-applies the last rectangle it proposed, which may be a held
            // step, so a throttled drag ends where the cursor says.
            if let Some(drag) = drag
                && drag.interval_ms > 0.0
            {
                move_window(hwnd, drag_target(&drag));
            }
            let mode = if lab.config.frosted && lab.config.backdrop == Backdrop::Accent {
                lab.config.drag
            } else {
                DragMode::Blur
            };
            match mode {
                DragMode::Blur => {}
                DragMode::Tint => {
                    blur_accent(hwnd);
                }
                DragMode::Solid => send_drag_event(hwnd, false),
            }
            // The drag record is gone, so the shadow returns at the final position.
            sync_shadow(hwnd);
            if let Some(drag) = drag {
                crate::log_line(format!(
                    "lab.drag.exit hwnd={key} via={} frosted={} corners={} drag={} throttle={} interval_ms={:.1} drag_hide_shadow={} elapsed_ms={} moving_messages={} held={} flushed={} max_gap_ms={:.1} slow_gaps={}",
                    drag.via,
                    lab.config.frosted,
                    lab.config.corners.tag(),
                    mode.tag(),
                    lab.config.throttle.tag(),
                    drag.interval_ms,
                    lab.config.drag_hide_shadow,
                    drag.started.elapsed().as_millis(),
                    drag.moves,
                    drag.held,
                    drag.flushed,
                    drag.max_gap_ms,
                    drag.slow_gaps
                ));
            }
        }
        _ => {}
    }
    // SAFETY: forwards to the procedure this window had before the lab subclassed it.
    let result = unsafe { CallWindowProcW(lab.old_proc, hwnd, msg, wparam, lparam) };
    if msg == WM_SIZE && lab.config.corners.clips() {
        apply_region(hwnd, &lab);
    }
    if msg == WM_SIZE || msg == WM_WINDOWPOSCHANGED {
        sync_shadow(hwnd);
    }
    result
}

/// Keeps the shadow window framing the body: re-rendered when the size or palette changes,
/// moved otherwise, hidden while the body is maximized, minimized or hidden.
fn sync_shadow(hwnd: *mut c_void) {
    let key = hwnd as isize;
    let Some(lab) = LABS.with(|labs| labs.borrow().get(&key).copied()) else {
        return;
    };
    if lab.shadow == 0 {
        return;
    }
    let shadow = lab.shadow as *mut c_void;
    // SAFETY: `hwnd` is a live window of this thread.
    let presented = unsafe {
        IsZoomed(hwnd) == 0 && IsIconic(hwnd) == 0 && IsWindowVisible(hwnd) != 0
    };
    let dragging_hidden = lab.config.drag_hide_shadow && lab.drag.is_some();
    if lab.config.corners != Corners::RegionShadow || !presented || dragging_hidden {
        crate::shadow::hide(shadow);
        return;
    }
    let factor = scale(hwnd);
    let margin = (crate::shadow::MARGIN_DIP * factor).round() as i32;
    let rect = window_rect(hwnd);
    let (width, height) = (rect.right - rect.left, rect.bottom - rect.top);
    let wanted = (width, height, lab.config.dark, lab.radius_px);
    if lab.shadow_rendered == Some(wanted) {
        crate::shadow::move_to(shadow, rect.left, rect.top, margin);
        return;
    }
    let rendered = crate::shadow::render(
        shadow,
        rect.left,
        rect.top,
        width,
        height,
        margin,
        lab.radius_px,
        factor,
        lab.config.dark,
    );
    LABS.with(|labs| {
        if let Some(entry) = labs.borrow_mut().get_mut(&key) {
            entry.shadow_rendered = rendered.then_some(wanted);
        }
    });
}
