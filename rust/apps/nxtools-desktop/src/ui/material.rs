//! Shared native facts and request identity, not a placement or visibility owner.
use gpui_kit::{App, Global};
use nxtools_protocol::window_material::{
    AppliedWindowMaterial, ApplyWindowMaterialCommand, MaterialSurface,
    SystemMaterialCapabilitiesChanged, WindowMaterialStyle,
};

#[derive(Clone, Copy, Default)]
pub(crate) struct UiMaterialCapabilities {
    connected: bool,
    snapshot: Option<SystemMaterialCapabilitiesChanged>,
}
impl Global for UiMaterialCapabilities {}
impl UiMaterialCapabilities {
    pub(crate) fn connected(&mut self) {
        self.connected = true;
    }
    pub(crate) fn disconnected(&mut self) {
        *self = Self::default();
    }
    pub(crate) fn snapshot(self) -> Option<SystemMaterialCapabilitiesChanged> {
        self.snapshot
    }
    pub(crate) fn apply(&mut self, facts: SystemMaterialCapabilitiesChanged) -> bool {
        if !self.connected
            || !facts.is_qualified()
            || self.snapshot.is_some_and(|current| {
                facts.session_id != current.session_id || facts.revision <= current.revision
            })
        {
            return false;
        }
        self.snapshot = Some(facts);
        true
    }
}

#[derive(Default)]
struct MaterialGeneration(u64);
impl Global for MaterialGeneration {}

fn protocol_style(style: nxtools_core::MaterialStyle) -> WindowMaterialStyle {
    match style {
        nxtools_core::MaterialStyle::Solid => WindowMaterialStyle::Solid,
        nxtools_core::MaterialStyle::Acrylic => WindowMaterialStyle::Acrylic,
    }
}

fn facts(cx: &App) -> Option<SystemMaterialCapabilitiesChanged> {
    cx.try_global::<UiMaterialCapabilities>()
        .copied()
        .and_then(UiMaterialCapabilities::snapshot)
}

/// Windows have 8 DIP rounded corners once PlatformHost reports facts: Windows 11 rounds them
/// through DWM (attribute 33), Windows 10 through a window region with a companion shadow
/// window ([`win10`]). Unknown until facts arrive.
pub(crate) fn rounded_corners(cx: &App) -> bool {
    facts(cx).is_some()
}

/// Windows 10: no DWM rounding, so framed windows are rounded by a window region.
pub(crate) fn region_corners(cx: &App) -> bool {
    facts(cx).is_some_and(|facts| !facts.rounded_corners)
}

/// DWM system acrylic (Windows 11 22H2). Without it the frosted style is the host backdrop
/// (the Windows Terminal path), a bare blurred desktop without tint or noise. Assumed present
/// until facts arrive.
pub(crate) fn system_backdrop(cx: &App) -> bool {
    facts(cx).is_none_or(|facts| facts.system_backdrop)
}

pub(crate) fn command(
    window_handle: i64,
    surface: MaterialSurface,
    cx: &mut App,
) -> Option<ApplyWindowMaterialCommand> {
    let facts = cx
        .try_global::<UiMaterialCapabilities>()
        .copied()?
        .snapshot()?;
    if window_handle == 0 {
        return None;
    }
    native_appearance::observe(window_handle, cx);
    let appearance = cx
        .try_global::<crate::theme::UiThemeState>()
        .copied()
        .unwrap_or_default();
    let generation = cx.default_global::<MaterialGeneration>();
    generation.0 = generation
        .0
        .checked_add(1)
        .expect("material request generation exhausted");
    Some(ApplyWindowMaterialCommand {
        session_id: facts.session_id,
        facts_revision: facts.revision,
        window_handle,
        generation: generation.0,
        style: protocol_style(appearance.material_style),
        surface,
        dark: appearance.is_dark(),
    })
}

pub(crate) fn is_current(command: &ApplyWindowMaterialCommand, cx: &App) -> bool {
    let facts = cx
        .try_global::<UiMaterialCapabilities>()
        .copied()
        .and_then(|state| state.snapshot());
    let appearance = cx
        .try_global::<crate::theme::UiThemeState>()
        .copied()
        .unwrap_or_default();
    facts.is_some_and(|facts| {
        command.session_id == facts.session_id && command.facts_revision == facts.revision
    }) && command.style == protocol_style(appearance.material_style)
        && command.dark == appearance.is_dark()
        && native_appearance::current(command, cx)
}

/// Request/receipt state shared by existing presentation owners; never shows a window.
#[derive(Default)]
pub(crate) struct Preparation {
    pending: Option<PendingPreparation>,
}

struct PendingPreparation {
    id: uuid::Uuid,
    command: ApplyWindowMaterialCommand,
    applied: Option<AppliedWindowMaterial>,
}

impl Preparation {
    pub(crate) fn confirmed(
        id: uuid::Uuid,
        command: ApplyWindowMaterialCommand,
        receipt: AppliedWindowMaterial,
        cx: &App,
    ) -> Option<Self> {
        (is_current(&command, cx) && receipt.matches(&command)).then_some(Self {
            pending: Some(PendingPreparation {
                id,
                command,
                applied: Some(receipt),
            }),
        })
    }

    pub(crate) fn ensure(
        &mut self,
        platform: &nxtools_ipc::PlatformBridge,
        window_handle: i64,
        surface: MaterialSurface,
        cx: &mut App,
    ) -> Result<bool, &'static str> {
        if let Some(pending) = &self.pending
            && pending.command.window_handle == window_handle
            && pending.command.surface == surface
            && is_current(&pending.command, cx)
        {
            let receipt = pending
                .applied
                .filter(|receipt| receipt.matches(&pending.command));
            if let Some(receipt) = receipt {
                paint::publish(pending.command, receipt, cx);
            }
            return Ok(receipt.is_some());
        }
        self.retire(platform);
        let Some(command) = command(window_handle, surface, cx) else {
            return Ok(false);
        };
        let id = uuid::Uuid::new_v4();
        self.pending = Some(PendingPreparation {
            id,
            command,
            applied: None,
        });
        if platform
            .send_command_with_id(
                nxtools_protocol::PlatformCommand::ApplyWindowMaterial(command),
                id,
            )
            .is_err()
        {
            self.pending = None;
            return Err("material_enqueue");
        }
        Ok(false)
    }

    pub(crate) fn accept(
        &mut self,
        id: uuid::Uuid,
        result: &nxtools_protocol::PlatformCommandResult,
        cx: &mut App,
    ) -> Option<Result<bool, &'static str>> {
        let pending = self.pending.as_mut().filter(|pending| pending.id == id)?;
        if !is_current(&pending.command, cx)
            || (!result.success && result.error.as_deref() == Some("stale_material_facts"))
        {
            return Some(Ok(false));
        }
        let receipt = result
            .success
            .then_some(result.data.as_deref())
            .flatten()
            .filter(|data| data.len() <= 2048)
            .and_then(|data| serde_json::from_str::<AppliedWindowMaterial>(data).ok())
            .filter(|receipt| receipt.matches(&pending.command));
        let Some(receipt) = receipt else {
            return Some(Err("native_material_result"));
        };
        pending.applied = Some(receipt);
        paint::publish(pending.command, receipt, cx);
        Some(Ok(true))
    }

    pub(crate) fn retire(&mut self, platform: &nxtools_ipc::PlatformBridge) {
        if let Some(pending) = self.pending.take() {
            platform.retire_queued_window_material(pending.id);
        }
    }

    pub(crate) fn receipt(&self, cx: &App) -> Option<AppliedWindowMaterial> {
        let pending = self
            .pending
            .as_ref()
            .filter(|pending| is_current(&pending.command, cx))?;
        pending
            .applied
            .filter(|receipt| receipt.matches(&pending.command))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;
    #[test]
    fn material_facts_require_connection_and_monotonic_current_session() {
        let mut policy = UiMaterialCapabilities::default();
        let facts = SystemMaterialCapabilitiesChanged {
            correlation_id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            revision: 1,
            supported: true,
            transparency_enabled: true,
            high_contrast: false,
            energy_saver: false,
            remote_session: false,
            native_error_code: 0,
            system_backdrop: true,
            rounded_corners: true,
        };
        assert!(!policy.apply(facts));
        policy.connected();
        assert!(policy.snapshot().is_none());
        assert!(policy.apply(facts));
        assert!(!policy.apply(facts));
        assert!(!policy.apply(SystemMaterialCapabilitiesChanged {
            session_id: Uuid::new_v4(),
            revision: 2,
            ..facts
        }));
        assert!(policy.apply(SystemMaterialCapabilitiesChanged {
            revision: 2,
            transparency_enabled: false,
            ..facts
        }));
        policy.connected();
        assert_eq!(
            policy.snapshot().expect("retained current facts").revision,
            2
        );
        policy.disconnected();
        assert!(policy.snapshot().is_none());
        assert!(!policy.apply(facts));
        policy.connected();
        assert!(policy.apply(SystemMaterialCapabilitiesChanged {
            session_id: Uuid::new_v4(),
            ..facts
        }));
    }
}

#[cfg(test)]
mod preparation_tests;

pub(crate) mod runtime;

mod native_appearance;

pub(crate) mod paint;

/// Windows 10 window shape and host-backdrop material, owned by the window's own process.
#[allow(unsafe_code)]
mod win10;

#[cfg(test)]
mod corner_gpu_tests;
