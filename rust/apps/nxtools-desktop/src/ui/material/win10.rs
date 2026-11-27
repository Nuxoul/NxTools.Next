//! Windows 10 window shape and frosted material for framed GPUI windows.
//!
//! Windows 10 has no DWM corner rounding (attribute 33, build 22000), and its Accent3 blur
//! trails window drags (1903 and later). Framed windows therefore get:
//!
//! - an 8 DIP rounded window region (`SetWindowRgn`), which also clips everything behind the
//!   client, and a click-through layered shadow window owned by the body that draws the soft
//!   shadow plus an antialiased rim over the region's staircase edge ([`shadow`]);
//! - for the frosted style, the host-backdrop acrylic visual ([`host`]) under GPUI's content, the
//!   Windows Terminal path, which follows drags without lag.
//!
//! All of it must run in the window's own process and thread: the composition target accepts
//! only own-process windows, and the shadow follows the body synchronously from the body's
//! `WM_WINDOWPOSCHANGED`. The GPUI Root plugin publishes the wanted state from render; the
//! native work runs from a posted message, outside GPUI's window update, because `SetWindowRgn`
//! sends position messages to the GPUI window procedure.
use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::OnceLock;

mod host;
pub(crate) use host::{BLEND_COLOR, BLEND_LUMINOSITY, Recipe};
mod shadow;

/// Exterior radius in DIP; the GPUI contour uses the same Component `window_radius`.
const RADIUS_DIP: f32 = crate::theme::material::WINDOW_RADIUS;
const SUBCLASS_ID: usize = 0x4E58_5753;

const WM_SIZE: u32 = 0x0005;
const WM_WINDOWPOSCHANGED: u32 = 0x0047;
const WM_NCDESTROY: u32 = 0x0082;
const WM_TIMER: u32 = 0x0113;
const WM_ENTERSIZEMOVE: u32 = 0x0231;
const WM_EXITSIZEMOVE: u32 = 0x0232;
const DWMWA_CLOAKED: u32 = 14;
/// Longer than two 60 Hz frames between move steps: the window thread stalled.
const SLOW_GAP_MS: f64 = 33.0;

/// The state the GPUI side wants for one window.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(crate) struct Wanted {
    /// Rounded region and companion shadow.
    pub round: bool,
    /// Host-backdrop acrylic under the GPUI content.
    pub acrylic: Option<Recipe>,
    /// Shadow and rim palette.
    pub dark: bool,
}

#[derive(Default)]
struct Shape {
    wanted: Wanted,
    applied: Option<Wanted>,
    /// Size the current region was built for; `None` without a region.
    region: Option<(i32, i32, i32)>,
    shadow: isize,
    /// Size, palette and radius the shadow bitmap was rendered for.
    shadow_rendered: Option<(i32, i32, bool, i32)>,
    /// Where the shadow was last placed; `None` while hidden.
    shadow_placed: Option<(i32, i32)>,
    drag: Option<Drag>,
}

/// One system move/size loop, reported once when it ends.
struct Drag {
    started: std::time::Instant,
    last_move: std::time::Instant,
    moves: u32,
    max_gap_ms: f64,
    slow_gaps: u32,
    /// GPUI's move-loop timer: queued application work and any frame it draws.
    ticks: u32,
    tick_total_ms: f64,
    tick_max_ms: f64,
}

thread_local! {
    static SHAPES: RefCell<HashMap<isize, Shape>> = RefCell::new(HashMap::new());
}

#[repr(C)]
#[derive(Default, Clone, Copy)]
struct Rect {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

type SubclassProc =
    unsafe extern "system" fn(*mut c_void, u32, usize, isize, usize, usize) -> isize;

#[link(name = "user32")]
unsafe extern "system" {
    fn RegisterWindowMessageW(name: *const u16) -> u32;
    fn PostMessageW(hwnd: *mut c_void, msg: u32, w: usize, l: isize) -> i32;
    fn GetWindowRect(hwnd: *mut c_void, rect: *mut Rect) -> i32;
    fn SetWindowRgn(hwnd: *mut c_void, region: *mut c_void, redraw: i32) -> i32;
    fn GetDpiForWindow(hwnd: *mut c_void) -> u32;
    fn IsZoomed(hwnd: *mut c_void) -> i32;
    fn IsIconic(hwnd: *mut c_void) -> i32;
    fn IsWindowVisible(hwnd: *mut c_void) -> i32;
}

#[link(name = "gdi32")]
unsafe extern "system" {
    fn CreateRoundRectRgn(l: i32, t: i32, r: i32, b: i32, w: i32, h: i32) -> *mut c_void;
    fn DeleteObject(object: *mut c_void) -> i32;
}

#[link(name = "comctl32")]
unsafe extern "system" {
    fn SetWindowSubclass(hwnd: *mut c_void, proc: SubclassProc, id: usize, data: usize) -> i32;
    fn RemoveWindowSubclass(hwnd: *mut c_void, proc: SubclassProc, id: usize) -> i32;
    fn DefSubclassProc(hwnd: *mut c_void, msg: u32, w: usize, l: isize) -> isize;
}

#[link(name = "dwmapi")]
unsafe extern "system" {
    fn DwmGetWindowAttribute(hwnd: *mut c_void, attr: u32, value: *mut c_void, size: u32) -> i32;
}

fn sync_message() -> u32 {
    static MESSAGE: OnceLock<u32> = OnceLock::new();
    *MESSAGE.get_or_init(|| {
        let name: Vec<u16> = "NxTools.Win10WindowShape.Sync\0".encode_utf16().collect();
        // SAFETY: `name` is a NUL-terminated UTF-16 string that outlives the call.
        unsafe { RegisterWindowMessageW(name.as_ptr()) }
    })
}

/// Records the wanted state for `hwnd` and schedules its native application when it changed.
/// Called from render on the window's thread; does no native work beyond subclassing once and
/// posting one message.
pub(crate) fn sync(hwnd: i64, wanted: Wanted) {
    let key = hwnd as isize;
    let known = SHAPES.with(|shapes| shapes.borrow().get(&key).map(|shape| shape.wanted));
    match known {
        Some(current) if current == wanted => return,
        // Nothing to undo on a window this module never shaped.
        None if !wanted.round && wanted.acrylic.is_none() => return,
        _ => {}
    }
    let window = hwnd as *mut c_void;
    if known.is_none() {
        // SAFETY: `window` is a live top-level window of this thread; the subclass forwards every
        // message and removes itself on WM_NCDESTROY.
        if unsafe { SetWindowSubclass(window, shape_proc, SUBCLASS_ID, 0) } == 0 {
            tracing::warn!(
                event = "product.material.win10_shape.subclass_failed",
                window_handle = hwnd
            );
            return;
        }
    }
    SHAPES.with(|shapes| shapes.borrow_mut().entry(key).or_default().wanted = wanted);
    // SAFETY: posting to a live window of this thread; the handler runs from its message loop.
    unsafe { PostMessageW(window, sync_message(), 0, 0) };
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

fn radius_px(hwnd: *mut c_void) -> i32 {
    (RADIUS_DIP * scale(hwnd)).round() as i32
}

fn with_shape<R>(hwnd: *mut c_void, update: impl FnOnce(&mut Shape) -> R) -> Option<R> {
    SHAPES.with(|shapes| shapes.borrow_mut().get_mut(&(hwnd as isize)).map(update))
}

/// Rounds the whole window rectangle, or removes the region while maximized or unwanted.
/// Returns false when a region call failed.
fn sync_region(hwnd: *mut c_void) -> bool {
    let Some((round, current)) = with_shape(hwnd, |shape| (shape.wanted.round, shape.region))
    else {
        return true;
    };
    // SAFETY: `hwnd` is a live window of this thread.
    let maximized = unsafe { IsZoomed(hwnd) } != 0;
    let wanted = (round && !maximized).then(|| {
        let rect = window_rect(hwnd);
        (
            rect.right - rect.left,
            rect.bottom - rect.top,
            radius_px(hwnd),
        )
    });
    if wanted == current {
        return true;
    }
    // SAFETY: the system owns a region after a successful SetWindowRgn; a failed call leaves it
    // with the caller, which deletes it. A null region removes the current one.
    let applied = unsafe {
        match wanted {
            Some((width, height, radius)) => {
                let diameter = radius * 2;
                let region = CreateRoundRectRgn(0, 0, width + 1, height + 1, diameter, diameter);
                if region.is_null() {
                    false
                } else if SetWindowRgn(hwnd, region, 1) == 0 {
                    DeleteObject(region);
                    false
                } else {
                    true
                }
            }
            None => SetWindowRgn(hwnd, std::ptr::null_mut(), 1) != 0,
        }
    };
    if applied {
        with_shape(hwnd, |shape| shape.region = wanted);
    }
    applied
}

fn cloaked(hwnd: *mut c_void) -> bool {
    let mut value = 0u32;
    // SAFETY: DWMWA_CLOAKED writes one DWORD into the frame-owned value.
    let result = unsafe {
        DwmGetWindowAttribute(
            hwnd,
            DWMWA_CLOAKED,
            (&mut value as *mut u32).cast(),
            std::mem::size_of::<u32>() as u32,
        )
    };
    result == 0 && value != 0
}

/// Keeps the shadow framing the body: re-rendered when size, palette or DPI change, moved
/// otherwise, hidden while the body is maximized, minimized, hidden or still cloaked for its
/// first frame. The cloak is a cross-process DWM query, so it is asked only before the shadow
/// is first placed; a placed shadow follows moves with window-local calls only.
fn sync_shadow(hwnd: *mut c_void) {
    let Some((wanted, shadow, rendered, placed)) = with_shape(hwnd, |shape| {
        (
            shape.wanted,
            shape.shadow,
            shape.shadow_rendered,
            shape.shadow_placed,
        )
    }) else {
        return;
    };
    if shadow == 0 {
        return;
    }
    let shadow = shadow as *mut c_void;
    // SAFETY: `hwnd` is a live window of this thread.
    let presented =
        unsafe { IsZoomed(hwnd) == 0 && IsIconic(hwnd) == 0 && IsWindowVisible(hwnd) != 0 }
            && (placed.is_some() || !cloaked(hwnd));
    if !wanted.round || !presented {
        if placed.is_some() {
            shadow::hide(shadow);
            with_shape(hwnd, |shape| shape.shadow_placed = None);
        }
        return;
    }
    let factor = scale(hwnd);
    let margin = (shadow::MARGIN_DIP * factor).round() as i32;
    let radius = radius_px(hwnd);
    let rect = window_rect(hwnd);
    let (width, height) = (rect.right - rect.left, rect.bottom - rect.top);
    let key = (width, height, wanted.dark, radius);
    if rendered == Some(key) {
        if placed != Some((rect.left, rect.top)) {
            shadow::move_to(shadow, rect.left, rect.top, margin);
            with_shape(hwnd, |shape| {
                shape.shadow_placed = Some((rect.left, rect.top))
            });
        }
        return;
    }
    let ok = shadow::render(
        shadow,
        rect.left,
        rect.top,
        width,
        height,
        margin,
        radius,
        factor,
        wanted.dark,
    );
    with_shape(hwnd, |shape| {
        shape.shadow_rendered = ok.then_some(key);
        shape.shadow_placed = Some((rect.left, rect.top));
    });
    if !ok {
        tracing::warn!(
            event = "product.material.win10_shape.shadow_failed",
            window_handle = hwnd as i64,
            width,
            height
        );
    }
}

/// Applies the recorded wanted state: region, host backdrop and shadow.
fn apply(hwnd: *mut c_void) {
    let Some((wanted, previous, shadow)) =
        with_shape(hwnd, |shape| (shape.wanted, shape.applied, shape.shadow))
    else {
        return;
    };
    if previous == Some(wanted) {
        return;
    }
    let started = std::time::Instant::now();
    let region_ok = sync_region(hwnd);
    let host = host::show(hwnd, wanted.acrylic);
    if wanted.round && shadow == 0 {
        let created = shadow::create(hwnd);
        with_shape(hwnd, |shape| shape.shadow = created as isize);
    }
    if previous.is_some_and(|previous| previous.dark != wanted.dark) {
        with_shape(hwnd, |shape| shape.shadow_rendered = None);
    }
    sync_shadow(hwnd);
    with_shape(hwnd, |shape| shape.applied = Some(wanted));
    let shadow_ok = with_shape(hwnd, |shape| shape.shadow != 0 || !wanted.round).unwrap_or(false);
    let host_error = host.err().unwrap_or(0);
    if region_ok && shadow_ok && host_error == 0 {
        tracing::info!(
            event = "product.material.win10_shape.applied",
            window_handle = hwnd as i64,
            round = wanted.round,
            host_backdrop = wanted.acrylic.is_some(),
            dark = wanted.dark,
            elapsed_ms = started.elapsed().as_millis() as u64
        );
    } else {
        tracing::warn!(
            event = "product.material.win10_shape.failed",
            window_handle = hwnd as i64,
            round = wanted.round,
            host_backdrop = wanted.acrylic.is_some(),
            region_ok,
            shadow_ok,
            native_error_code = host_error,
            elapsed_ms = started.elapsed().as_millis() as u64
        );
    }
}

fn release(hwnd: *mut c_void) {
    let shape = SHAPES.with(|shapes| shapes.borrow_mut().remove(&(hwnd as isize)));
    if let Some(shape) = shape
        && shape.shadow != 0
    {
        shadow::destroy(shape.shadow as *mut c_void);
    }
    host::release(hwnd);
    // SAFETY: removes this module's subclass from the window being destroyed.
    unsafe { RemoveWindowSubclass(hwnd, shape_proc, SUBCLASS_ID) };
}

/// Starts the per-drag record on `WM_ENTERSIZEMOVE`.
fn drag_started(hwnd: *mut c_void) {
    let now = std::time::Instant::now();
    with_shape(hwnd, |shape| {
        shape.drag = Some(Drag {
            started: now,
            last_move: now,
            moves: 0,
            max_gap_ms: 0.0,
            slow_gaps: 0,
            ticks: 0,
            tick_total_ms: 0.0,
            tick_max_ms: 0.0,
        })
    });
}

fn drag_moved(hwnd: *mut c_void) {
    let now = std::time::Instant::now();
    with_shape(hwnd, |shape| {
        if let Some(drag) = shape.drag.as_mut() {
            if drag.moves > 0 {
                let gap = now.duration_since(drag.last_move).as_secs_f64() * 1000.0;
                drag.max_gap_ms = drag.max_gap_ms.max(gap);
                if gap > SLOW_GAP_MS {
                    drag.slow_gaps += 1;
                }
            }
            drag.last_move = now;
            drag.moves += 1;
        }
    });
}

fn drag_ticked(hwnd: *mut c_void, elapsed_ms: f64) {
    with_shape(hwnd, |shape| {
        if let Some(drag) = shape.drag.as_mut() {
            drag.ticks += 1;
            drag.tick_total_ms += elapsed_ms;
            drag.tick_max_ms = drag.tick_max_ms.max(elapsed_ms);
        }
    });
}

/// Reports the finished move/size loop once: whether the window thread kept up with the
/// pointer, and how long GPUI's move-loop timer spent on queued work and frames.
fn drag_finished(hwnd: *mut c_void) {
    let Some(Some(drag)) = with_shape(hwnd, |shape| shape.drag.take()) else {
        return;
    };
    let wanted = with_shape(hwnd, |shape| shape.wanted).unwrap_or_default();
    tracing::info!(
        event = "product.material.win10_shape.drag",
        window_handle = hwnd as i64,
        round = wanted.round,
        host_backdrop = wanted.acrylic.is_some(),
        elapsed_ms = drag.started.elapsed().as_millis() as u64,
        moves = drag.moves,
        max_gap_ms = drag.max_gap_ms.round() as u64,
        slow_gaps = drag.slow_gaps,
        timer_ticks = drag.ticks,
        timer_total_ms = drag.tick_total_ms.round() as u64,
        timer_max_ms = drag.tick_max_ms.round() as u64
    );
}

unsafe extern "system" fn shape_proc(
    hwnd: *mut c_void,
    msg: u32,
    wparam: usize,
    lparam: isize,
    _id: usize,
    _data: usize,
) -> isize {
    if msg == sync_message() {
        apply(hwnd);
        return 0;
    }
    let dragging =
        msg == WM_TIMER && with_shape(hwnd, |shape| shape.drag.is_some()).unwrap_or(false);
    let started = dragging.then(std::time::Instant::now);
    // SAFETY: forwards to the procedure below this subclass (GPUI's window procedure).
    let result = unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) };
    match msg {
        WM_SIZE => {
            sync_region(hwnd);
            sync_shadow(hwnd);
        }
        WM_WINDOWPOSCHANGED => {
            drag_moved(hwnd);
            sync_shadow(hwnd);
        }
        WM_TIMER => {
            if let Some(started) = started {
                drag_ticked(hwnd, started.elapsed().as_secs_f64() * 1000.0);
            }
            // GPUI's first-frame uncloak sends no position message; a placed shadow needs
            // nothing from timer ticks.
            if with_shape(hwnd, |shape| shape.shadow_placed.is_none()).unwrap_or(false) {
                sync_shadow(hwnd);
            }
        }
        WM_ENTERSIZEMOVE => drag_started(hwnd),
        WM_EXITSIZEMOVE => drag_finished(hwnd),
        WM_NCDESTROY => release(hwnd),
        _ => {}
    }
    result
}
