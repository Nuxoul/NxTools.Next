use uuid::Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Phase {
    Prepare,
    Frame,
    Show,
    Update,
    Hide,
    Retire,
}

#[derive(Default)]
pub(super) struct Lifecycle {
    pub(super) pending: Option<(Uuid, Phase)>,
    pub(super) visible: bool,
    pub(super) prepared: bool,
    pub(super) revision: u64,
}
impl Lifecycle {
    pub(super) fn begin(&mut self, id: Uuid, phase: Phase) {
        self.pending = Some((id, phase));
    }
    pub(super) fn retire(&mut self) {
        self.pending = None;
        self.visible = false;
        self.prepared = false;
        self.revision += 1;
    }
    pub(super) fn take(&mut self, id: Uuid) -> Option<Phase> {
        if self.pending.is_some_and(|(current, _)| current == id) {
            self.pending.take().map(|(_, phase)| phase)
        } else {
            None
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn retired_native_reply_cannot_reveal_replacement() {
        let mut state = Lifecycle::default();
        let old = Uuid::new_v4();
        state.begin(old, Phase::Prepare);
        state.retire();
        state.begin(Uuid::new_v4(), Phase::Prepare);
        assert_eq!(state.take(old), None);
        assert!(!state.visible);
        assert!(state.pending.is_some());
    }
    #[test]
    fn pending_is_once_only_and_frame_is_explicit() {
        let mut state = Lifecycle::default();
        let id = Uuid::new_v4();
        state.begin(id, Phase::Frame);
        assert_eq!(state.take(id), Some(Phase::Frame));
        assert_eq!(state.take(id), None);
        assert!(!state.visible);
    }
}
