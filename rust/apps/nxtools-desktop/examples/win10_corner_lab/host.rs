//! Host-backdrop material, the Windows Terminal / XAML `HostBackdrop` path: the window opts
//! into DWM's host backdrop and a Windows.UI.Composition visual in the window's bottom layer
//! paints it. GPUI's DirectComposition content sits in the topmost layer above it.
#![allow(unsafe_code)]
use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::c_void;

use windows::System::DispatcherQueueController;
use windows::UI::Composition::Compositor;
use windows::UI::Composition::Desktop::DesktopWindowTarget;
use windows::UI::Composition::SpriteVisual;
use windows::Win32::Foundation::HWND;
use windows::Win32::System::WinRT::Composition::ICompositorDesktopInterop;
use windows::Win32::System::WinRT::{
    CreateDispatcherQueueController, DQTAT_COM_NONE, DQTYPE_THREAD_CURRENT, DispatcherQueueOptions,
};
use windows::core::Interface as _;
use windows_numerics::Vector2;

struct Host {
    _target: DesktopWindowTarget,
    visual: SpriteVisual,
}

thread_local! {
    /// The composition thread's dispatcher queue and compositor; both live for the process.
    static QUEUE: RefCell<Option<DispatcherQueueController>> = const { RefCell::new(None) };
    static COMPOSITOR: RefCell<Option<Compositor>> = const { RefCell::new(None) };
    static HOSTS: RefCell<HashMap<isize, Host>> = RefCell::new(HashMap::new());
}

fn compositor() -> windows::core::Result<Compositor> {
    if let Some(compositor) = COMPOSITOR.with(|slot| slot.borrow().clone()) {
        return Ok(compositor);
    }
    let options = DispatcherQueueOptions {
        dwSize: std::mem::size_of::<DispatcherQueueOptions>() as u32,
        threadType: DQTYPE_THREAD_CURRENT,
        apartmentType: DQTAT_COM_NONE,
    };
    // SAFETY: creates the queue for this (GPUI's) thread, whose message loop services it.
    let queue = unsafe { CreateDispatcherQueueController(options) }?;
    QUEUE.with(|slot| *slot.borrow_mut() = Some(queue));
    let compositor = Compositor::new()?;
    COMPOSITOR.with(|slot| *slot.borrow_mut() = Some(compositor.clone()));
    Ok(compositor)
}

fn create(hwnd: *mut c_void) -> windows::core::Result<SpriteVisual> {
    let compositor = compositor()?;
    let interop = compositor.cast::<ICompositorDesktopInterop>()?;
    // SAFETY: `hwnd` is a live top-level window of this thread; the bottom (non-topmost) layer
    // is free because GPUI's content target is topmost.
    let target = unsafe { interop.CreateDesktopWindowTarget(HWND(hwnd), false) }?;
    let visual = compositor.CreateSpriteVisual()?;
    visual.SetBrush(&compositor.CreateHostBackdropBrush()?)?;
    // Fills the window and follows its size without per-resize updates.
    visual.SetRelativeSizeAdjustment(Vector2 { X: 1.0, Y: 1.0 })?;
    target.SetRoot(&visual)?;
    HOSTS.with(|hosts| {
        hosts.borrow_mut().insert(
            hwnd as isize,
            Host {
                _target: target,
                visual: visual.clone(),
            },
        )
    });
    Ok(visual)
}

/// Shows or hides the host-backdrop visual, creating it on first use. Returns a receipt.
pub fn show(hwnd: *mut c_void, visible: bool) -> String {
    let existing = HOSTS.with(|hosts| {
        hosts
            .borrow()
            .get(&(hwnd as isize))
            .map(|host| host.visual.clone())
    });
    let visual = match existing {
        Some(visual) => visual,
        None if !visible => return "host=none".into(),
        None => match create(hwnd) {
            Ok(visual) => visual,
            Err(error) => return format!("host=failed hr={:#x}", error.code().0),
        },
    };
    match visual.SetIsVisible(visible) {
        Ok(()) => format!("host={}", if visible { "shown" } else { "hidden" }),
        Err(error) => format!("host=failed hr={:#x}", error.code().0),
    }
}
