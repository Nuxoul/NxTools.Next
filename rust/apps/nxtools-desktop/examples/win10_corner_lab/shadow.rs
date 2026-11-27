//! Click-through layered window that draws a soft shadow and an antialiased rim around a
//! region-clipped body. Owned by the body window, so it follows its z-order; the body area of
//! the bitmap is transparent, so the shadow only shows outside the rounded shape.
#![allow(unsafe_code)]
use std::ffi::c_void;

const WS_POPUP: u32 = 0x8000_0000;
const WS_EX_LAYERED: u32 = 0x0008_0000;
const WS_EX_TRANSPARENT: u32 = 0x0000_0020;
const WS_EX_TOOLWINDOW: u32 = 0x0000_0080;
const WS_EX_NOACTIVATE: u32 = 0x0800_0000;
const SW_HIDE: i32 = 0;
const SW_SHOWNOACTIVATE: i32 = 4;
const ULW_ALPHA: u32 = 2;
const SWP_NOSIZE: u32 = 0x0001;
const SWP_NOZORDER: u32 = 0x0004;
const SWP_NOACTIVATE: u32 = 0x0010;
/// Room around the body for the key shadow (offset plus about three sigma), in DIP.
pub const MARGIN_DIP: f32 = 28.0;

#[repr(C)]
struct WndClassExW {
    size: u32,
    style: u32,
    proc: unsafe extern "system" fn(*mut c_void, u32, usize, isize) -> isize,
    class_extra: i32,
    window_extra: i32,
    instance: *mut c_void,
    icon: *mut c_void,
    cursor: *mut c_void,
    background: *mut c_void,
    menu_name: *const u16,
    class_name: *const u16,
    small_icon: *mut c_void,
}

#[repr(C)]
struct BitmapInfoHeader {
    size: u32,
    width: i32,
    height: i32,
    planes: u16,
    bit_count: u16,
    compression: u32,
    size_image: u32,
    x_ppm: i32,
    y_ppm: i32,
    used: u32,
    important: u32,
}

#[repr(C)]
struct Point {
    x: i32,
    y: i32,
}

#[repr(C)]
struct Size {
    cx: i32,
    cy: i32,
}

#[repr(C)]
struct BlendFunction {
    op: u8,
    flags: u8,
    alpha: u8,
    format: u8,
}

#[link(name = "user32")]
unsafe extern "system" {
    fn RegisterClassExW(class: *const WndClassExW) -> u16;
    fn CreateWindowExW(
        ex_style: u32,
        class: *const u16,
        name: *const u16,
        style: u32,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        parent: *mut c_void,
        menu: *mut c_void,
        instance: *mut c_void,
        param: *mut c_void,
    ) -> *mut c_void;
    fn DefWindowProcW(hwnd: *mut c_void, msg: u32, w: usize, l: isize) -> isize;
    fn ShowWindow(hwnd: *mut c_void, command: i32) -> i32;
    fn SetWindowPos(
        hwnd: *mut c_void,
        after: *mut c_void,
        x: i32,
        y: i32,
        cx: i32,
        cy: i32,
        flags: u32,
    ) -> i32;
    fn UpdateLayeredWindow(
        hwnd: *mut c_void,
        screen: *mut c_void,
        position: *const Point,
        size: *const Size,
        source: *mut c_void,
        source_origin: *const Point,
        key: u32,
        blend: *const BlendFunction,
        flags: u32,
    ) -> i32;
    fn GetDC(hwnd: *mut c_void) -> *mut c_void;
    fn ReleaseDC(hwnd: *mut c_void, dc: *mut c_void) -> i32;
}

#[link(name = "gdi32")]
unsafe extern "system" {
    fn CreateCompatibleDC(dc: *mut c_void) -> *mut c_void;
    fn CreateDIBSection(
        dc: *mut c_void,
        info: *const BitmapInfoHeader,
        usage: u32,
        bits: *mut *mut c_void,
        section: *mut c_void,
        offset: u32,
    ) -> *mut c_void;
    fn SelectObject(dc: *mut c_void, object: *mut c_void) -> *mut c_void;
    fn DeleteObject(object: *mut c_void) -> i32;
    fn DeleteDC(dc: *mut c_void) -> i32;
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetModuleHandleW(name: *const u16) -> *mut c_void;
}

unsafe extern "system" fn shadow_proc(hwnd: *mut c_void, msg: u32, w: usize, l: isize) -> isize {
    // SAFETY: plain default processing; the shadow window handles no messages itself.
    unsafe { DefWindowProcW(hwnd, msg, w, l) }
}

fn class_name() -> Vec<u16> {
    "NxToolsLabShadow\0".encode_utf16().collect()
}

/// Creates the hidden shadow window owned by `owner`.
pub fn create(owner: *mut c_void) -> *mut c_void {
    let name = class_name();
    // SAFETY: registers a class with a static procedure (repeat registration fails harmlessly)
    // and creates a hidden popup owned by a live window of this thread.
    unsafe {
        let instance = GetModuleHandleW(std::ptr::null());
        let class = WndClassExW {
            size: std::mem::size_of::<WndClassExW>() as u32,
            style: 0,
            proc: shadow_proc,
            class_extra: 0,
            window_extra: 0,
            instance,
            icon: std::ptr::null_mut(),
            cursor: std::ptr::null_mut(),
            background: std::ptr::null_mut(),
            menu_name: std::ptr::null(),
            class_name: name.as_ptr(),
            small_icon: std::ptr::null_mut(),
        };
        RegisterClassExW(&class);
        CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            name.as_ptr(),
            name.as_ptr(),
            WS_POPUP,
            0,
            0,
            1,
            1,
            owner,
            std::ptr::null_mut(),
            instance,
            std::ptr::null_mut(),
        )
    }
}

pub fn hide(shadow: *mut c_void) {
    // SAFETY: `shadow` is a live window of this thread.
    unsafe { ShowWindow(shadow, SW_HIDE) };
}

/// Moves the shadow so its bitmap frames a body whose top-left corner is at (`x`, `y`).
pub fn move_to(shadow: *mut c_void, x: i32, y: i32, margin: i32) {
    // SAFETY: `shadow` is a live window of this thread.
    unsafe {
        SetWindowPos(
            shadow,
            std::ptr::null_mut(),
            x - margin,
            y - margin,
            0,
            0,
            SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
        );
        ShowWindow(shadow, SW_SHOWNOACTIVATE);
    }
}

/// Signed distance from (`px`, `py`) to a rounded rectangle given by its center, half extents
/// and corner radius.
fn rounded_distance(px: f32, py: f32, cx: f32, cy: f32, hx: f32, hy: f32, radius: f32) -> f32 {
    let qx = (px - cx).abs() - (hx - radius);
    let qy = (py - cy).abs() - (hy - radius);
    let outside = (qx.max(0.0).powi(2) + qy.max(0.0).powi(2)).sqrt();
    outside + qx.max(qy).min(0.0) - radius
}

/// Logistic approximation of the complementary normal CDF: the coverage a gaussian blur of
/// `sigma` gives at signed distance `distance` outside an edge.
fn blurred(distance: f32, sigma: f32) -> f32 {
    1.0 / (1.0 + (1.702 * distance / sigma).exp())
}

/// Renders and shows the shadow for a body of `width` x `height` device pixels at (`x`, `y`).
pub fn render(
    shadow: *mut c_void,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    margin: i32,
    radius: i32,
    scale: f32,
    dark: bool,
) -> bool {
    let (total_w, total_h) = (width + 2 * margin, height + 2 * margin);
    if total_w <= 0 || total_h <= 0 {
        return false;
    }
    let (cx, cy) = (total_w as f32 / 2.0, total_h as f32 / 2.0);
    let (hx, hy) = (width as f32 / 2.0, height as f32 / 2.0);
    let radius = radius as f32;
    // Key light below the window and a tight ambient contact shadow.
    let (key_alpha, ambient_alpha) = if dark { (0.55, 0.35) } else { (0.30, 0.14) };
    let key_offset = 6.0 * scale;
    let key_sigma = 9.0 * scale;
    let ambient_sigma = 1.5 * scale;
    // The rim covers the region's 1-bit staircase with an antialiased contour.
    let (rim_rgb, rim_alpha) = if dark {
        (255.0, 0.14)
    } else {
        (0.0, 0.16)
    };
    let mut pixels = vec![0u32; (total_w * total_h) as usize];
    for row in 0..total_h {
        let py = row as f32 + 0.5;
        for column in 0..total_w {
            let px = column as f32 + 0.5;
            let body = rounded_distance(px, py, cx, cy, hx, hy, radius);
            let key = rounded_distance(px, py, cx, cy + key_offset, hx, hy, radius);
            let shadow_alpha = (key_alpha * blurred(key, key_sigma)
                + ambient_alpha * blurred(body, ambient_sigma))
            .min(1.0);
            // Transparent over the body: only what lies outside the rounded shape shows.
            let outside = (body + 0.5).clamp(0.0, 1.0);
            let shadow_alpha = shadow_alpha * outside;
            let rim = rim_alpha * (1.0 - (body - 0.25).abs()).clamp(0.0, 1.0);
            let alpha = rim + shadow_alpha * (1.0 - rim);
            if alpha <= 0.0 {
                continue;
            }
            let color = (rim * rim_rgb).round().clamp(0.0, 255.0) as u32;
            let a = (alpha * 255.0).round().clamp(0.0, 255.0) as u32;
            pixels[(row * total_w + column) as usize] = (a << 24) | (color << 16) | (color << 8) | color;
        }
    }
    // SAFETY: a top-down 32-bit DIB of the exact size receives the premultiplied pixels and is
    // handed to UpdateLayeredWindow; every GDI object created here is released before return.
    unsafe {
        let screen = GetDC(std::ptr::null_mut());
        let memory = CreateCompatibleDC(screen);
        let info = BitmapInfoHeader {
            size: std::mem::size_of::<BitmapInfoHeader>() as u32,
            width: total_w,
            height: -total_h,
            planes: 1,
            bit_count: 32,
            compression: 0,
            size_image: 0,
            x_ppm: 0,
            y_ppm: 0,
            used: 0,
            important: 0,
        };
        let mut bits: *mut c_void = std::ptr::null_mut();
        let bitmap = CreateDIBSection(memory, &info, 0, &mut bits, std::ptr::null_mut(), 0);
        let mut updated = 0;
        if !bitmap.is_null() && !bits.is_null() {
            std::ptr::copy_nonoverlapping(pixels.as_ptr(), bits.cast::<u32>(), pixels.len());
            let previous = SelectObject(memory, bitmap);
            let position = Point {
                x: x - margin,
                y: y - margin,
            };
            let size = Size {
                cx: total_w,
                cy: total_h,
            };
            let origin = Point { x: 0, y: 0 };
            let blend = BlendFunction {
                op: 0,
                flags: 0,
                alpha: 255,
                format: 1,
            };
            updated = UpdateLayeredWindow(
                shadow, screen, &position, &size, memory, &origin, 0, &blend, ULW_ALPHA,
            );
            SelectObject(memory, previous);
            DeleteObject(bitmap);
        }
        DeleteDC(memory);
        ReleaseDC(std::ptr::null_mut(), screen);
        ShowWindow(shadow, SW_SHOWNOACTIVATE);
        updated != 0
    }
}
