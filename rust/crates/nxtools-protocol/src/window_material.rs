//! Session-qualified native material facts and receipts; no placement or visibility policy.
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const CAPABILITY: &str = "window_material_v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SystemMaterialCapabilitiesChanged {
    pub correlation_id: Uuid,
    pub session_id: Uuid,
    pub revision: u64,
    pub supported: bool,
    pub transparency_enabled: bool,
    pub high_contrast: bool,
    pub energy_saver: bool,
    pub remote_session: bool,
    pub native_error_code: i32,
    /// DWM system acrylic backdrop: Windows 11 22H2, build 22621.
    pub system_backdrop: bool,
    /// DWM window rounding and border colour: Windows 11, build 22000.
    pub rounded_corners: bool,
}

impl SystemMaterialCapabilitiesChanged {
    pub fn is_qualified(&self) -> bool {
        !self.correlation_id.is_nil() && !self.session_id.is_nil() && self.revision != 0
    }
}

/// Saved theme style. Acrylic is the system acrylic backdrop where Windows offers one and the
/// accent blur elsewhere (Windows 10).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WindowMaterialStyle {
    Solid,
    Acrylic,
}

impl WindowMaterialStyle {
    pub fn is_frosted(self) -> bool {
        self != Self::Solid
    }
}

/// What the window is for: a persistent window or a transient surface (popup, flyout) takes
/// acrylic, a transparent one draws its own shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MaterialSurface {
    Backdrop,
    Transient,
    Transparent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApplyWindowMaterialCommand {
    pub session_id: Uuid,
    pub facts_revision: u64,
    pub window_handle: i64,
    pub generation: u64,
    pub style: WindowMaterialStyle,
    pub surface: MaterialSurface,
    pub dark: bool,
}

impl ApplyWindowMaterialCommand {
    pub fn is_qualified(&self) -> bool {
        !self.session_id.is_nil()
            && self.facts_revision != 0
            && self.window_handle != 0
            && self.generation != 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EffectiveWindowMaterial {
    Solid,
    Acrylic,
    AcrylicOpaque,
    Transparent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MaterialFallbackReason {
    None,
    Unsupported,
    TransparencyDisabled,
    HighContrast,
    EnergySaver,
    RemoteSession,
    CapabilityReadFailed,
    NativeApplyFailed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AppliedWindowMaterial {
    pub session_id: Uuid,
    pub facts_revision: u64,
    pub window_handle: i64,
    pub generation: u64,
    pub style: WindowMaterialStyle,
    pub surface: MaterialSurface,
    pub dark: bool,
    pub effective: EffectiveWindowMaterial,
    pub fallback_reason: MaterialFallbackReason,
    pub native_error_code: i32,
}

impl AppliedWindowMaterial {
    /// A qualified setter/reset receipt, not proof of compositor pixels.
    pub fn matches(&self, command: &ApplyWindowMaterialCommand) -> bool {
        command.is_qualified()
            && self.session_id == command.session_id
            && self.facts_revision == command.facts_revision
            && self.window_handle == command.window_handle
            && self.generation == command.generation
            && self.style == command.style
            && self.surface == command.surface
            && self.dark == command.dark
            && match (self.surface, self.effective) {
                (MaterialSurface::Transparent, EffectiveWindowMaterial::Transparent) => {
                    self.fallback_reason == MaterialFallbackReason::None
                        && self.native_error_code == 0
                }
                (MaterialSurface::Transparent, _) => false,
                (_, EffectiveWindowMaterial::Solid) => {
                    !self.style.is_frosted()
                        && self.fallback_reason == MaterialFallbackReason::None
                        && self.native_error_code == 0
                }
                (_, EffectiveWindowMaterial::Acrylic) => {
                    self.style.is_frosted()
                        && self.fallback_reason == MaterialFallbackReason::None
                        && self.native_error_code == 0
                }
                (_, EffectiveWindowMaterial::AcrylicOpaque) => {
                    self.style.is_frosted()
                        && self.fallback_reason != MaterialFallbackReason::None
                        && match self.fallback_reason {
                            MaterialFallbackReason::NativeApplyFailed
                            | MaterialFallbackReason::CapabilityReadFailed => {
                                self.native_error_code != 0
                            }
                            _ => self.native_error_code == 0,
                        }
                }
                (_, EffectiveWindowMaterial::Transparent) => false,
            }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Envelope, Message, PlatformCommand, PlatformEvent};

    fn command() -> ApplyWindowMaterialCommand {
        ApplyWindowMaterialCommand {
            session_id: Uuid::new_v4(),
            facts_revision: 3,
            window_handle: -42,
            generation: 7,
            style: WindowMaterialStyle::Acrylic,
            surface: MaterialSurface::Backdrop,
            dark: false,
        }
    }

    fn receipt(command: ApplyWindowMaterialCommand) -> AppliedWindowMaterial {
        AppliedWindowMaterial {
            session_id: command.session_id,
            facts_revision: command.facts_revision,
            window_handle: command.window_handle,
            generation: command.generation,
            style: command.style,
            surface: command.surface,
            dark: command.dark,
            effective: EffectiveWindowMaterial::Acrylic,
            fallback_reason: MaterialFallbackReason::None,
            native_error_code: 0,
        }
    }

    #[test]
    fn material_wire_roundtrip_and_strict_values() {
        let command = command();
        let envelope = Envelope::new(Message::PlatformCommand(
            PlatformCommand::ApplyWindowMaterial(command),
        ));
        let wire = envelope.encode_line().expect("encode material request");
        assert!(wire.contains("\"command\":\"apply_window_material\""));
        assert!(wire.contains("\"surface\":\"backdrop\""));
        assert_eq!(
            Envelope::decode_line(&wire).expect("decode material request"),
            envelope
        );
        for (style, wire) in [
            (WindowMaterialStyle::Solid, "solid"),
            (WindowMaterialStyle::Acrylic, "acrylic"),
        ] {
            let value = serde_json::to_value(ApplyWindowMaterialCommand { style, ..command })
                .expect("material style value");
            assert_eq!(value["style"], wire);
            assert_eq!(style.is_frosted(), style != WindowMaterialStyle::Solid);
        }
        let transient = serde_json::to_value(ApplyWindowMaterialCommand {
            surface: MaterialSurface::Transient,
            ..command
        })
        .expect("transient value");
        assert_eq!(transient["surface"], "transient");
        for (key, value) in [("style", "mica"), ("surface", "overlay")] {
            let mut invalid = serde_json::to_value(command).expect("material request value");
            invalid[key] = value.into();
            assert!(serde_json::from_value::<ApplyWindowMaterialCommand>(invalid).is_err());
        }
        let mut extra = serde_json::to_value(command).expect("material request value");
        extra["process_id"] = 123.into();
        assert!(serde_json::from_value::<ApplyWindowMaterialCommand>(extra).is_err());
        let facts = SystemMaterialCapabilitiesChanged {
            correlation_id: Uuid::new_v4(),
            session_id: command.session_id,
            revision: 3,
            supported: true,
            transparency_enabled: true,
            high_contrast: false,
            energy_saver: false,
            remote_session: false,
            native_error_code: 0,
            system_backdrop: true,
            rounded_corners: true,
        };
        assert!(facts.is_qualified());
        let event = Envelope::new(Message::PlatformEvent(
            PlatformEvent::SystemMaterialCapabilitiesChanged(facts),
        ));
        let wire = event.encode_line().expect("encode facts");
        assert!(wire.contains("\"system_backdrop\":true"));
        assert!(wire.contains("\"rounded_corners\":true"));
        assert_eq!(Envelope::decode_line(&wire).expect("decode facts"), event);
        assert!(
            !SystemMaterialCapabilitiesChanged {
                revision: 0,
                ..facts
            }
            .is_qualified()
        );
    }

    #[test]
    fn material_receipt_rejects_every_stale_identity_and_invalid_effect() {
        let command = command();
        let applied = receipt(command);
        assert!(applied.matches(&command));
        let wrong = [
            ApplyWindowMaterialCommand {
                session_id: Uuid::new_v4(),
                ..command
            },
            ApplyWindowMaterialCommand {
                facts_revision: 4,
                ..command
            },
            ApplyWindowMaterialCommand {
                window_handle: 43,
                ..command
            },
            ApplyWindowMaterialCommand {
                generation: 8,
                ..command
            },
            ApplyWindowMaterialCommand {
                style: WindowMaterialStyle::Solid,
                ..command
            },
            ApplyWindowMaterialCommand {
                surface: MaterialSurface::Transparent,
                ..command
            },
            ApplyWindowMaterialCommand {
                dark: true,
                ..command
            },
            ApplyWindowMaterialCommand {
                generation: 0,
                ..command
            },
        ];
        for stale in wrong {
            assert!(!applied.matches(&stale));
        }
        assert!(
            !AppliedWindowMaterial {
                native_error_code: 5,
                ..applied
            }
            .matches(&command)
        );
        for reason in [
            MaterialFallbackReason::Unsupported,
            MaterialFallbackReason::TransparencyDisabled,
            MaterialFallbackReason::HighContrast,
            MaterialFallbackReason::EnergySaver,
            MaterialFallbackReason::RemoteSession,
            MaterialFallbackReason::CapabilityReadFailed,
            MaterialFallbackReason::NativeApplyFailed,
        ] {
            let fallback = AppliedWindowMaterial {
                effective: EffectiveWindowMaterial::AcrylicOpaque,
                fallback_reason: reason,
                native_error_code: if matches!(
                    reason,
                    MaterialFallbackReason::CapabilityReadFailed
                        | MaterialFallbackReason::NativeApplyFailed
                ) {
                    5
                } else {
                    0
                },
                ..applied
            };
            assert!(fallback.matches(&command));
            assert_eq!(
                serde_json::from_str::<AppliedWindowMaterial>(
                    &serde_json::to_string(&fallback).expect("encode material receipt")
                )
                .expect("decode material receipt"),
                fallback
            );
        }
        assert!(
            !AppliedWindowMaterial {
                effective: EffectiveWindowMaterial::AcrylicOpaque,
                ..applied
            }
            .matches(&command)
        );
        let transparent = ApplyWindowMaterialCommand {
            surface: MaterialSurface::Transparent,
            ..command
        };
        assert!(
            AppliedWindowMaterial {
                surface: MaterialSurface::Transparent,
                effective: EffectiveWindowMaterial::Transparent,
                ..applied
            }
            .matches(&transparent)
        );
        for surface in [MaterialSurface::Backdrop, MaterialSurface::Transient] {
            let frosted = ApplyWindowMaterialCommand {
                surface,
                ..command
            };
            assert!(
                AppliedWindowMaterial {
                    surface,
                    ..applied
                }
                .matches(&frosted)
            );
            assert!(
                !AppliedWindowMaterial {
                    surface,
                    effective: EffectiveWindowMaterial::Solid,
                    ..applied
                }
                .matches(&frosted)
            );
        }
        let solid_transient = ApplyWindowMaterialCommand {
            style: WindowMaterialStyle::Solid,
            surface: MaterialSurface::Transient,
            ..command
        };
        assert!(
            !AppliedWindowMaterial {
                style: WindowMaterialStyle::Solid,
                surface: MaterialSurface::Transient,
                ..applied
            }
            .matches(&solid_transient),
            "solid never reports a frosted effect"
        );
        assert!(
            !AppliedWindowMaterial {
                surface: MaterialSurface::Transient,
                effective: EffectiveWindowMaterial::Transparent,
                ..applied
            }
            .matches(&ApplyWindowMaterialCommand {
                surface: MaterialSurface::Transient,
                ..command
            }),
            "only transparent surfaces report a transparent effect"
        );
    }
}
