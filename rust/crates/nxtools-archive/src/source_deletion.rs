//! Internal lifecycle evidence, not Windows delete authority. No paths or payloads are exposed.
use serde::Serialize;
use std::sync::Arc;
use uuid::Uuid;

/// Private, non-cloneable lifecycle ownership. Pointer identity cannot be supplied by a caller.
pub(crate) struct Lifecycle(Arc<()>);

#[derive(Serialize)]
pub struct SourceDeleteEligibility {
    operation_id: Uuid,
    reason: &'static str,
    source_ids: Vec<Uuid>,
}
// A decoded worker frame is not a way to manufacture in-process lifecycle authority.
impl<'de> serde::Deserialize<'de> for SourceDeleteEligibility {
    fn deserialize<D: serde::Deserializer<'de>>(_deserializer: D) -> Result<Self, D::Error> {
        Err(serde::de::Error::custom(
            "lifecycle evidence is output only",
        ))
    }
}
impl SourceDeleteEligibility {
    pub fn operation_id(&self) -> Uuid {
        self.operation_id
    }
    pub fn reason_code(&self) -> &'static str {
        self.reason
    }
}
pub(crate) struct Evidence {
    reason: &'static str,
    sources: Vec<Uuid>,
    consumed: bool,
    current: Option<Arc<()>>,
}
impl Default for Evidence {
    fn default() -> Self {
        Self {
            reason: "lifecycle_incomplete",
            sources: Vec::new(),
            consumed: false,
            current: None,
        }
    }
}
impl Evidence {
    pub(crate) fn begin(&mut self) -> Lifecycle {
        self.reason = "lifecycle_incomplete";
        self.sources.clear();
        let identity = Arc::new(());
        self.current = Some(identity.clone());
        // Consumption is operation-wide, including reused control clones.
        Lifecycle(identity)
    }
    pub(crate) fn finish(
        &mut self,
        lifecycle: &Lifecycle,
        reason: &'static str,
        sources: Vec<Uuid>,
    ) {
        if !self
            .current
            .as_ref()
            .is_some_and(|current| Arc::ptr_eq(current, &lifecycle.0))
        {
            return;
        }
        self.current = None;
        self.reason = reason;
        self.sources = sources;
    }
    pub(crate) fn take(
        &mut self,
        operation_id: Uuid,
        consent: bool,
        cancelled: bool,
    ) -> SourceDeleteEligibility {
        let reason = if !consent {
            "consent_missing"
        } else if self.consumed {
            "already_consumed"
        } else if cancelled {
            "cancelled"
        } else {
            self.reason
        };
        let source_ids = if reason == "eligible" {
            self.consumed = true;
            std::mem::take(&mut self.sources)
        } else {
            Vec::new()
        };
        SourceDeleteEligibility {
            operation_id,
            reason,
            source_ids,
        }
    }
}
