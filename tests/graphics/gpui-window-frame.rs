//! Owned Win32 HWNDs exercise the production NCCALCSIZE function and border cache.
//! Run from gpui-pre-windows' lib tests with `gpui_window_frame --test-threads=1`.
//! These are native single-monitor lifecycle tests, not mixed-DPI or visual acceptance.

use super::*;
use crate::events::{
    calculate_frame_client_size, custom_frame_resize_hit_test, maximized_custom_frame_rect,
};

#[derive(Default)]
struct FrameFixture {
    custom: bool,
    fullscreen: Cell<bool>,
    border: WindowBorderOffset,
    callbacks: Cell<usize>,
    stale_callbacks: Cell<usize>,
    native_errors: Cell<usize>,
}

unsafe extern "system" fn frame_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg == WM_NCCREATE {
        let create = unsafe { &*(lparam.0 as *const CREATESTRUCTW) };
        unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, create.lpCreateParams as isize) };
    }
    let ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) } as *const FrameFixture;
    if !ptr.is_null() {
        // The owning test keeps the boxed fixture alive until DestroyWindow completes.
        let fixture = unsafe { &*ptr };
        match msg {
            WM_NCCALCSIZE => {
                if let Some(result) = calculate_frame_client_size(
                    hwnd,
                    wparam,
                    lparam,
                    fixture.custom,
                    fixture.fullscreen.get(),
                ) {
                    return LRESULT(result);
                }
            }
            WM_WINDOWPOSCHANGED => {
                if fixture
                    .border
                    .update(hwnd, fixture.fullscreen.get())
                    .is_err()
                {
                    fixture.native_errors.set(fixture.native_errors.get() + 1);
                }
                // DefWindowProc sends WM_MOVE/WM_SIZE after the real frame has settled.
            }
            WM_NCHITTEST
                if fixture.custom
                    && !fixture.fullscreen.get()
                    && !unsafe { IsZoomed(hwnd) }.as_bool() =>
            {
                if let Some(hit) = custom_frame_resize_hit_test(hwnd, lparam) {
                    return LRESULT(hit);
                }
            }
            WM_MOVE | WM_SIZE => {
                if !fixture.fullscreen.get()
                    && !unsafe { IsIconic(hwnd) }.as_bool()
                    && !unsafe { IsZoomed(hwnd) }.as_bool()
                {
                    fixture.callbacks.set(fixture.callbacks.get() + 1);
                    if let Ok((outer, client)) = native_rects(hwnd) {
                        let cached = calculate_client_rect(outer, &fixture.border, 1.0)
                            .to_device_pixels(1.0);
                        if rect_tuple(calculate_window_rect(
                            cached,
                            &WindowBorderOffset::default(),
                        )) != rect_tuple(client)
                        {
                            fixture
                                .stale_callbacks
                                .set(fixture.stale_callbacks.get() + 1);
                        }
                    } else {
                        fixture.native_errors.set(fixture.native_errors.get() + 1);
                    }
                }
            }
            _ => {}
        }
    }
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

struct ThreadDpiContext(DPI_AWARENESS_CONTEXT);

impl Drop for ThreadDpiContext {
    fn drop(&mut self) {
        let _ = unsafe { SetThreadDpiAwarenessContext(self.0) };
    }
}

struct OwnedFrame {
    hwnd: HWND,
    fixture: Box<FrameFixture>,
    _dpi_context: ThreadDpiContext,
}

impl OwnedFrame {
    fn new(custom: bool) -> Self {
        let previous =
            unsafe { SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
        assert_ne!(
            previous,
            DPI_AWARENESS_CONTEXT::default(),
            "set fixture thread DPI context"
        );
        let dpi_context = ThreadDpiContext(previous);
        static REGISTER: Once = Once::new();
        REGISTER.call_once(|| {
            let class = WNDCLASSW {
                lpfnWndProc: Some(frame_proc),
                hInstance: get_module_handle().into(),
                lpszClassName: w!("NxTools.GpuiWindowFrame.Regression"),
                ..Default::default()
            };
            assert_ne!(unsafe { RegisterClassW(&class) }, 0);
        });
        let fixture = Box::new(FrameFixture {
            custom,
            ..Default::default()
        });
        let hwnd = unsafe {
            CreateWindowExW(
                WS_EX_NOACTIVATE | WS_EX_APPWINDOW,
                w!("NxTools.GpuiWindowFrame.Regression"),
                w!("Owned frame regression"),
                WS_OVERLAPPEDWINDOW,
                100,
                120,
                640,
                480,
                None,
                None,
                Some(get_module_handle().into()),
                Some(fixture.as_ref() as *const _ as *const _),
            )
        }
        .expect("create owned frame HWND");
        fixture
            .border
            .update(hwnd, false)
            .expect("initial native margins");
        fixture.callbacks.set(0);
        fixture.stale_callbacks.set(0);
        Self {
            hwnd,
            fixture,
            _dpi_context: dpi_context,
        }
    }

    fn position(&self, x: i32, y: i32, width: i32, height: i32) {
        unsafe {
            SetWindowPos(
                self.hwnd,
                None,
                x,
                y,
                width,
                height,
                SWP_FRAMECHANGED | SWP_NOACTIVATE | SWP_NOZORDER,
            )
        }
        .expect("position owned HWND");
    }

    fn assert_roundtrip(&self) -> (RECT, RECT) {
        let (outer, client) = native_rects(self.hwnd).expect("read native frame");
        let cached = calculate_client_rect(outer, &self.fixture.border, 1.0).to_device_pixels(1.0);
        assert_eq!(
            cached.origin,
            point(DevicePixels(client.left), DevicePixels(client.top))
        );
        assert_eq!(
            cached.size,
            size(
                DevicePixels(client.right - client.left),
                DevicePixels(client.bottom - client.top)
            )
        );
        assert_eq!(
            rect_tuple(calculate_window_rect(cached, &self.fixture.border)),
            rect_tuple(outer)
        );
        assert_eq!(self.fixture.native_errors.get(), 0);
        (outer, client)
    }
}

impl Drop for OwnedFrame {
    fn drop(&mut self) {
        unsafe { DestroyWindow(self.hwnd) }.expect("destroy owned HWND");
    }
}

fn rect_tuple(rect: RECT) -> (i32, i32, i32, i32) {
    (rect.left, rect.top, rect.right, rect.bottom)
}

fn native_rects(hwnd: HWND) -> windows::core::Result<(RECT, RECT)> {
    let mut outer = RECT::default();
    let mut client = RECT::default();
    let mut origin = POINT::default();
    unsafe {
        GetWindowRect(hwnd, &mut outer)?;
        GetClientRect(hwnd, &mut client)?;
        ClientToScreen(hwnd, &mut origin).ok()?;
    }
    client.left += origin.x;
    client.right += origin.x;
    client.top += origin.y;
    client.bottom += origin.y;
    Ok((outer, client))
}

#[::core::prelude::v1::test]
fn gpui_window_frame_native_caption_asymmetric_actual_origin_and_cache_roundtrip() {
    let window = OwnedFrame::new(false);
    let (outer, client) = window.assert_roundtrip();
    assert!(
        client.top - outer.top > outer.bottom - client.bottom,
        "native caption is not a symmetric vertical margin"
    );
    window.position(180, 160, 720, 540);
    window.assert_roundtrip();
    // A real FRAMECHANGED alters the top margin; callbacks must see the new cache.
    let style = unsafe { GetWindowLongPtrW(window.hwnd, GWL_STYLE) };
    unsafe { SetWindowLongPtrW(window.hwnd, GWL_STYLE, style & !(WS_CAPTION.0 as isize)) };
    window.position(180, 160, 720, 540);
    window.assert_roundtrip();
    unsafe { SetWindowLongPtrW(window.hwnd, GWL_STYLE, style) };
    window.position(220, 190, 800, 600);
    window.assert_roundtrip();
    assert!(window.fixture.callbacks.get() > 0);
    assert_eq!(
        window.fixture.stale_callbacks.get(),
        0,
        "cache must precede move/resize callbacks"
    );
}

#[::core::prelude::v1::test]
fn gpui_window_frame_custom_hidden_passive_resize_covers_all_edges() {
    let active = unsafe { GetActiveWindow() };
    let foreground = unsafe { GetForegroundWindow() };
    let window = OwnedFrame::new(true);
    for (x, y, width, height) in [(100, 120, 640, 480), (190, 150, 820, 560)] {
        window.position(x, y, width, height);
        let (outer, client) = window.assert_roundtrip();
        assert_eq!(
            rect_tuple(client),
            rect_tuple(outer),
            "no custom-frame glass collar"
        );
        assert!(!unsafe { IsWindowVisible(window.hwnd) }.as_bool());
        let style = unsafe { GetWindowLongPtrW(window.hwnd, GWL_STYLE) } as u32;
        assert_eq!(style & WS_OVERLAPPEDWINDOW.0, WS_OVERLAPPEDWINDOW.0);
        assert_ne!(
            unsafe { GetWindowLongPtrW(window.hwnd, GWL_EXSTYLE) } & WS_EX_NOACTIVATE.0 as isize,
            0
        );
        assert_eq!(unsafe { GetActiveWindow() }, active);
        assert_eq!(unsafe { GetForegroundWindow() }, foreground);
    }
    assert!(window.fixture.callbacks.get() > 0);
    assert_eq!(window.fixture.stale_callbacks.get(), 0);
}

#[::core::prelude::v1::test]
fn gpui_window_frame_maximize_minimize_restore_preserves_normal_margins() {
    for custom in [false, true] {
        let window = OwnedFrame::new(custom);
        window.position(180, 160, 720, 540);
        let (normal_outer, normal_client) = window.assert_roundtrip();
        let _ = unsafe { ShowWindow(window.hwnd, SW_SHOWMAXIMIZED) };
        assert!(unsafe { IsZoomed(window.hwnd) }.as_bool());
        let mut placement = WINDOWPLACEMENT {
            length: std::mem::size_of::<WINDOWPLACEMENT>() as u32,
            ..Default::default()
        };
        unsafe { GetWindowPlacement(window.hwnd, &mut placement) }.expect("maximized placement");
        assert_eq!(
            rect_tuple(placement.rcNormalPosition),
            rect_tuple(normal_outer)
        );
        if custom {
            let (_, client) = native_rects(window.hwnd).expect("maximized native client");
            let monitor = unsafe { MonitorFromWindow(window.hwnd, MONITOR_DEFAULTTONEAREST) };
            let mut info = MONITORINFO {
                cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                ..Default::default()
            };
            unsafe { GetMonitorInfoW(monitor, &mut info) }
                .ok()
                .expect("monitor work area");
            assert_eq!(rect_tuple(client), rect_tuple(info.rcWork));
        }
        let saved_client =
            calculate_client_rect(placement.rcNormalPosition, &window.fixture.border, 1.0)
                .to_device_pixels(1.0);
        assert_eq!(
            saved_client.origin,
            point(
                DevicePixels(normal_client.left),
                DevicePixels(normal_client.top)
            ),
            "maximized margins must not reinterpret restored placement"
        );
        assert_eq!(
            saved_client.size,
            size(
                DevicePixels(normal_client.right - normal_client.left),
                DevicePixels(normal_client.bottom - normal_client.top)
            )
        );
        let _ = unsafe { ShowWindow(window.hwnd, SW_RESTORE) };
        assert!(!unsafe { IsZoomed(window.hwnd) }.as_bool());
        let (restored_outer, _) = window.assert_roundtrip();
        assert_eq!(rect_tuple(restored_outer), rect_tuple(normal_outer));
        let _ = unsafe { ShowWindow(window.hwnd, SW_MINIMIZE) };
        unsafe { GetWindowPlacement(window.hwnd, &mut placement) }.expect("minimized placement");
        assert_eq!(
            rect_tuple(placement.rcNormalPosition),
            rect_tuple(normal_outer)
        );
        assert!(unsafe { IsIconic(window.hwnd) }.as_bool());
        assert_eq!(
            calculate_client_rect(placement.rcNormalPosition, &window.fixture.border, 1.0)
                .to_device_pixels(1.0),
            saved_client
        );
        let _ = unsafe { ShowWindow(window.hwnd, SW_RESTORE) };
        window.assert_roundtrip();
        let _ = unsafe { ShowWindow(window.hwnd, SW_HIDE) };
        window.position(210, 180, 760, 580);
        window.assert_roundtrip();
    }
}

#[::core::prelude::v1::test]
fn gpui_window_frame_fullscreen_does_not_replace_restored_caption_margins() {
    let window = OwnedFrame::new(false);
    let (normal_outer, _) = window.assert_roundtrip();
    let normal = calculate_client_rect(normal_outer, &window.fixture.border, 1.0);
    let style = unsafe { GetWindowLongPtrW(window.hwnd, GWL_STYLE) };
    window.fixture.fullscreen.set(true);
    unsafe {
        SetWindowLongPtrW(
            window.hwnd,
            GWL_STYLE,
            style & !((WS_THICKFRAME | WS_CAPTION).0 as isize),
        )
    };
    window.position(0, 0, 1024, 768);
    let (outer, client) = native_rects(window.hwnd).expect("fullscreen client");
    assert_eq!(rect_tuple(outer), rect_tuple(client));
    assert_eq!(
        calculate_client_rect(normal_outer, &window.fixture.border, 1.0),
        normal
    );
    window.fixture.fullscreen.set(false);
    unsafe { SetWindowLongPtrW(window.hwnd, GWL_STYLE, style) };
    window.position(
        normal_outer.left,
        normal_outer.top,
        normal_outer.right - normal_outer.left,
        normal_outer.bottom - normal_outer.top,
    );
    window.assert_roundtrip();
}

#[::core::prelude::v1::test]
fn gpui_window_frame_dpi_math_roundtrip_uses_actual_asymmetric_margins() {
    let window = OwnedFrame::new(false);
    let (outer, client) = window.assert_roundtrip();
    for scale in [1.0, 1.25, 1.5, 2.0] {
        let logical = calculate_client_rect(outer, &window.fixture.border, scale);
        let device = logical.to_device_pixels(scale);
        assert_eq!(
            device.origin,
            point(DevicePixels(client.left), DevicePixels(client.top))
        );
        assert_eq!(
            rect_tuple(calculate_window_rect(device, &window.fixture.border)),
            rect_tuple(outer)
        );
    }
}

#[::core::prelude::v1::test]
fn gpui_window_frame_custom_native_hit_test_retains_eight_resize_regions() {
    let window = OwnedFrame::new(true);
    window.position(180, 160, 720, 540);
    let (_, client) = window.assert_roundtrip();
    let mid_x = (client.left + client.right) / 2;
    let mid_y = (client.top + client.bottom) / 2;
    for (x, y, expected) in [
        (client.left + 1, client.top + 1, HTTOPLEFT),
        (mid_x, client.top + 1, HTTOP),
        (client.right - 2, client.top + 1, HTTOPRIGHT),
        (client.left + 1, mid_y, HTLEFT),
        (client.right - 2, mid_y, HTRIGHT),
        (client.left + 1, client.bottom - 2, HTBOTTOMLEFT),
        (mid_x, client.bottom - 2, HTBOTTOM),
        (client.right - 2, client.bottom - 2, HTBOTTOMRIGHT),
        (mid_x, mid_y, HTCLIENT),
    ] {
        let coordinates = LPARAM(((x as u16 as u32) | ((y as u16 as u32) << 16)) as isize);
        let hit = unsafe {
            SendMessageW(
                window.hwnd,
                WM_NCHITTEST,
                Some(WPARAM(0)),
                Some(coordinates),
            )
        };
        assert_eq!(hit.0, expected as isize, "resize region at {x}, {y}");
    }
}

#[::core::prelude::v1::test]
fn gpui_window_frame_maximized_dpi_rect_math_keeps_sizing_frame_outside_work_area() {
    let work = RECT {
        left: -1600,
        top: 40,
        right: 0,
        bottom: 1000,
    };
    for dpi in [96, 120, 144, 192] {
        let rect = maximized_custom_frame_rect(work, dpi);
        let x = unsafe { GetSystemMetricsForDpi(SM_CXSIZEFRAME, dpi) }
            + unsafe { GetSystemMetricsForDpi(SM_CXPADDEDBORDER, dpi) };
        let y = unsafe { GetSystemMetricsForDpi(SM_CYSIZEFRAME, dpi) }
            + unsafe { GetSystemMetricsForDpi(SM_CXPADDEDBORDER, dpi) };
        assert_eq!(
            (rect.left + x, rect.top + y, rect.right - x, rect.bottom - y),
            rect_tuple(work)
        );
    }
}

#[::core::prelude::v1::test]
fn gpui_window_frame_production_windowpos_refresh_precedes_default_callbacks() {
    let source = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/events.rs"));
    let branch = source
        .split("WM_WINDOWPOSCHANGED => {")
        .nth(1)
        .expect("production windowpos frame boundary")
        .split("WM_MOVE =>")
        .next()
        .expect("move dispatch follows frame boundary");
    assert!(branch.contains(".update(handle, self.state.is_fullscreen())"));
    assert!(
        branch.contains("None"),
        "DefWindowProc must deliver native move/size callbacks"
    );
}
