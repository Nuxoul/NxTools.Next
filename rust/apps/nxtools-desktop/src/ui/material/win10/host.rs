//! Host-backdrop acrylic, the Windows Terminal / XAML `AcrylicBrush` path. PlatformHost opts the
//! window into DWM's host backdrop (Accent 5, plus DWM attribute 17 on Windows 11 before 22621);
//! a Windows.UI.Composition visual in the window's bottom layer paints it. GPUI's
//! DirectComposition content takes the topmost layer above it.
//!
//! The bare host backdrop is only the blurred desktop, so text over a translucent fill fights
//! every wallpaper's brightness. The brush runs the Fluent acrylic recipe XAML uses: a
//! luminosity layer pulls the backdrop's brightness toward the tint (keeping its hue), then a
//! tint layer colours it. The wallpaper's colour still shows; its light and dark do not.
//!
//! `ICompositorDesktopInterop::CreateDesktopWindowTarget` accepts only windows of the calling
//! process, so this lives in the GPUI process on the window's (GPUI's) thread, whose message
//! loop services the dispatcher queue.
use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::c_void;

use windows::Foundation::{IPropertyValue, PropertyValue};
use windows::Graphics::Effects::{
    IGraphicsEffect, IGraphicsEffect_Impl, IGraphicsEffectSource, IGraphicsEffectSource_Impl,
};
use windows::System::DispatcherQueueController;
use windows::UI::Composition::Desktop::DesktopWindowTarget;
use windows::UI::Composition::{
    CompositionBrush, CompositionEffectSourceParameter, Compositor, SpriteVisual,
};
use windows::Win32::Foundation::{E_BOUNDS, E_INVALIDARG, HWND};
use windows::Win32::System::WinRT::Composition::ICompositorDesktopInterop;
use windows::Win32::System::WinRT::Graphics::Direct2D::{
    GRAPHICS_EFFECT_PROPERTY_MAPPING, IGraphicsEffectD2D1Interop, IGraphicsEffectD2D1Interop_Impl,
};
use windows::Win32::System::WinRT::{
    CreateDispatcherQueueController, DQTAT_COM_NONE, DQTYPE_THREAD_CURRENT, DispatcherQueueOptions,
};
use windows::core::{GUID, HSTRING, Interface as _, PCWSTR, implement};
use windows_numerics::Vector2;

/// CLSID_D2D1Blend; property 0 is the blend mode, input 0 the backdrop, input 1 the layer.
const BLEND: GUID = GUID::from_u128(0x81c5b77b_13f8_4cdd_ad20_c890547ac65d);
/// CLSID_D2D1Flood; property 0 is the colour as a straight-alpha RGBA vector.
const FLOOD: GUID = GUID::from_u128(0x61c23c20_ae69_4d8e_94cf_50078df638f2);
/// D2D1_BLEND_MODE_COLOR and D2D1_BLEND_MODE_LUMINOSITY.
pub(crate) const BLEND_COLOR: u32 = 22;
pub(crate) const BLEND_LUMINOSITY: u32 = 23;
const BACKDROP: &str = "backdrop";

/// One acrylic brush: the tint's sRGB colour with the opacities of its luminosity and colour
/// layers in percent, and the blend mode of each layer.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct Recipe {
    pub tint: u32,
    pub luminosity_percent: u8,
    pub tint_percent: u8,
    pub luminosity_mode: u32,
    pub tint_mode: u32,
}

#[implement(IGraphicsEffect, IGraphicsEffectSource, IGraphicsEffectD2D1Interop)]
struct Effect {
    id: GUID,
    properties: Vec<IPropertyValue>,
    sources: Vec<IGraphicsEffectSource>,
    name: RefCell<HSTRING>,
}

impl IGraphicsEffect_Impl for Effect_Impl {
    fn Name(&self) -> windows::core::Result<HSTRING> {
        Ok(self.name.borrow().clone())
    }

    fn SetName(&self, name: &HSTRING) -> windows::core::Result<()> {
        *self.name.borrow_mut() = name.clone();
        Ok(())
    }
}

impl IGraphicsEffectSource_Impl for Effect_Impl {}

impl IGraphicsEffectD2D1Interop_Impl for Effect_Impl {
    fn GetEffectId(&self) -> windows::core::Result<GUID> {
        Ok(self.id)
    }

    /// No property is animated, so no name maps to one.
    fn GetNamedPropertyMapping(
        &self,
        _name: &PCWSTR,
        _index: *mut u32,
        _mapping: *mut GRAPHICS_EFFECT_PROPERTY_MAPPING,
    ) -> windows::core::Result<()> {
        Err(E_INVALIDARG.into())
    }

    fn GetPropertyCount(&self) -> windows::core::Result<u32> {
        Ok(self.properties.len() as u32)
    }

    fn GetProperty(&self, index: u32) -> windows::core::Result<IPropertyValue> {
        self.properties
            .get(index as usize)
            .cloned()
            .ok_or_else(|| E_BOUNDS.into())
    }

    fn GetSource(&self, index: u32) -> windows::core::Result<IGraphicsEffectSource> {
        self.sources
            .get(index as usize)
            .cloned()
            .ok_or_else(|| E_BOUNDS.into())
    }

    fn GetSourceCount(&self) -> windows::core::Result<u32> {
        Ok(self.sources.len() as u32)
    }
}

fn effect(
    id: GUID,
    properties: Vec<IPropertyValue>,
    sources: Vec<IGraphicsEffectSource>,
) -> Effect {
    Effect {
        id,
        properties,
        sources,
        name: RefCell::new(HSTRING::new()),
    }
}

fn flood(rgb: u32, alpha: f32) -> windows::core::Result<IGraphicsEffectSource> {
    let channel = |shift: u32| ((rgb >> shift) & 0xFF) as f32 / 255.0;
    let color = PropertyValue::CreateSingleArray(&[channel(16), channel(8), channel(0), alpha])?
        .cast::<IPropertyValue>()?;
    Ok(effect(FLOOD, vec![color], Vec::new()).into())
}

fn blend(
    mode: u32,
    backdrop: IGraphicsEffectSource,
    layer: IGraphicsEffectSource,
) -> windows::core::Result<Effect> {
    let mode = PropertyValue::CreateUInt32(mode)?.cast::<IPropertyValue>()?;
    Ok(effect(BLEND, vec![mode], vec![backdrop, layer]))
}

fn acrylic_brush(
    compositor: &Compositor,
    recipe: Recipe,
) -> windows::core::Result<CompositionBrush> {
    let percent = |value: u8| f32::from(value.min(100)) / 100.0;
    let backdrop = CompositionEffectSourceParameter::Create(&HSTRING::from(BACKDROP))?
        .cast::<IGraphicsEffectSource>()?;
    let luminosity = blend(
        recipe.luminosity_mode,
        backdrop,
        flood(recipe.tint, percent(recipe.luminosity_percent))?,
    )?;
    let tinted: IGraphicsEffect = blend(
        recipe.tint_mode,
        luminosity.into(),
        flood(recipe.tint, percent(recipe.tint_percent))?,
    )?
    .into();
    let brush = compositor.CreateEffectFactory(&tinted)?.CreateBrush()?;
    brush.SetSourceParameter(
        &HSTRING::from(BACKDROP),
        &compositor.CreateHostBackdropBrush()?,
    )?;
    brush.cast()
}

struct Host {
    _target: DesktopWindowTarget,
    visual: SpriteVisual,
    recipe: Option<Recipe>,
}

thread_local! {
    /// The GPUI thread's dispatcher queue and compositor; both live for the process.
    static QUEUE: RefCell<Option<DispatcherQueueController>> = const { RefCell::new(None) };
    static COMPOSITOR: RefCell<Option<Compositor>> = const { RefCell::new(None) };
    static HOSTS: RefCell<HashMap<isize, Host>> = RefCell::new(HashMap::new());
}

fn compositor() -> windows::core::Result<Compositor> {
    if let Some(compositor) = COMPOSITOR.with(|slot| slot.borrow().clone()) {
        return Ok(compositor);
    }
    if QUEUE.with(|slot| slot.borrow().is_none()) {
        let options = DispatcherQueueOptions {
            dwSize: std::mem::size_of::<DispatcherQueueOptions>() as u32,
            threadType: DQTYPE_THREAD_CURRENT,
            apartmentType: DQTAT_COM_NONE,
        };
        // SAFETY: creates the queue for this (GPUI's) thread, whose message loop services it.
        let queue = unsafe { CreateDispatcherQueueController(options) }?;
        QUEUE.with(|slot| *slot.borrow_mut() = Some(queue));
    }
    let compositor = Compositor::new()?;
    COMPOSITOR.with(|slot| *slot.borrow_mut() = Some(compositor.clone()));
    Ok(compositor)
}

fn create(hwnd: *mut c_void, compositor: &Compositor) -> windows::core::Result<SpriteVisual> {
    let interop = compositor.cast::<ICompositorDesktopInterop>()?;
    // SAFETY: `hwnd` is a live top-level window of this thread; the bottom (non-topmost) layer
    // is free because GPUI's content target is topmost.
    let target = unsafe { interop.CreateDesktopWindowTarget(HWND(hwnd), false) }?;
    let visual = compositor.CreateSpriteVisual()?;
    // Fills the window and follows its size without per-resize updates.
    visual.SetRelativeSizeAdjustment(Vector2 { X: 1.0, Y: 1.0 })?;
    target.SetRoot(&visual)?;
    HOSTS.with(|hosts| {
        hosts.borrow_mut().insert(
            hwnd as isize,
            Host {
                _target: target,
                visual: visual.clone(),
                recipe: None,
            },
        )
    });
    Ok(visual)
}

/// Shows the acrylic visual with `recipe`, creating it on first show, or hides it for `None`.
/// Errors carry the HRESULT.
pub(super) fn show(hwnd: *mut c_void, recipe: Option<Recipe>) -> Result<(), i32> {
    let code = |error: windows::core::Error| error.code().0;
    let existing = HOSTS.with(|hosts| {
        hosts
            .borrow()
            .get(&(hwnd as isize))
            .map(|host| (host.visual.clone(), host.recipe))
    });
    let Some(recipe) = recipe else {
        if let Some((visual, _)) = existing {
            visual.SetIsVisible(false).map_err(code)?;
        }
        return Ok(());
    };
    let compositor = compositor().map_err(code)?;
    let (visual, current) = match existing {
        Some(found) => found,
        None => (create(hwnd, &compositor).map_err(code)?, None),
    };
    if current != Some(recipe) {
        visual
            .SetBrush(&acrylic_brush(&compositor, recipe).map_err(code)?)
            .map_err(code)?;
        HOSTS.with(|hosts| {
            if let Some(host) = hosts.borrow_mut().get_mut(&(hwnd as isize)) {
                host.recipe = Some(recipe);
            }
        });
    }
    visual.SetIsVisible(true).map_err(code)
}

/// Releases the window's composition target; called once the window is being destroyed.
pub(super) fn release(hwnd: *mut c_void) {
    HOSTS.with(|hosts| hosts.borrow_mut().remove(&(hwnd as isize)));
}
