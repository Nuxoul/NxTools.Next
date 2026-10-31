//! Isolated Phase F pointer seam. Deliberately not mounted by ArchiveBrowser before spike acceptance.
use gpui_kit::{
    AppContext, Context, InteractiveElement, MouseButton, ParentElement, Render, Styled, Window,
    div,
};
use nxtools_protocol::{PlatformCommand, archive_virtual_drag::ArchiveVirtualDragCommand};
use std::rc::Rc;

pub(crate) struct VirtualDragSpike {
    command: ArchiveVirtualDragCommand,
    platform: Rc<nxtools_ipc::PlatformBridge>,
    pending: bool,
}
impl VirtualDragSpike {
    pub(crate) fn new(
        command: ArchiveVirtualDragCommand,
        platform: Rc<nxtools_ipc::PlatformBridge>,
    ) -> Self {
        Self {
            command,
            platform,
            pending: false,
        }
    }
    fn begin(&mut self, cx: &mut Context<Self>) {
        if self.pending || self.command.validate().is_err() {
            return;
        }
        let command_id = uuid::Uuid::new_v4();
        self.pending = true;
        // No await/defer/GPUI drag layer: original still-pressed callback owns immediate enqueue.
        if self
            .platform
            .send_command_with_id_correlated(
                PlatformCommand::ArchiveVirtualDragSpike(Box::new(self.command.clone())),
                command_id,
                self.command.operation_id,
            )
            .is_err()
        {
            self.pending = false;
        }
        cx.notify();
    }
}
impl Render for VirtualDragSpike {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl gpui_kit::IntoElement {
        div()
            .id("archive-virtual-drag-spike")
            .debug_selector(|| "archive-virtual-drag-spike".into())
            .size_full()
            .child("Virtual-file spike (not production)")
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    this.begin(cx);
                    cx.stop_propagation();
                }),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nxtools_protocol::{Message, archive_browser::*, archive_virtual_drag::*};
    use uuid::Uuid;
    #[gpui_kit::test]
    fn pressed_callback_enqueues_original_snapshot_without_materializing_and_is_once(
        cx: &mut gpui_kit::TestAppContext,
    ) {
        cx.update(gpui_kit::init);
        cx.executor().allow_parking();
        let (platform, endpoint) = nxtools_ipc::platform_bridge(8);
        endpoint.set_connected_for_contract_test(true);
        let platform = Rc::new(platform);
        let session = Uuid::new_v4();
        let snapshot = Uuid::new_v4();
        let command = ArchiveVirtualDragCommand {
            version: 1,
            authorization_id: None,
            operation_id: Uuid::new_v4(),
            session_id: session,
            generation: 7,
            action: "start".into(),
            source_window_handle: 42,
            items: Some(vec![ArchiveVirtualDragItem {
                name: "original.txt".into(),
                size: Some(3),
                directory: false,
                entry: ArchiveEntryCommand {
                    version: 1,
                    operation_id: Uuid::new_v4(),
                    session_id: session,
                    generation: 7,
                    action: ArchiveEntryAction::Materialize,
                    path: Some(r"C:\owned\source.zip".into()),
                    codepage: 0,
                    ordinal: Some(9),
                    browser: Some(ArchiveBrowserContext {
                        snapshot_id: snapshot,
                        format: "Zip".into(),
                        comment: String::new(),
                        source: ArchiveSourceBinding {
                            native: "v1:fixture".into(),
                            digest: [1; 32],
                            size: 99,
                            modified_unix_ns: 1,
                        },
                    }),
                    lease_id: None,
                    depth: 0,
                    copy_motw: Some(true),
                    password_interaction: false,
                    temp_policy: None,
                },
            }]),
        };
        let expected = command.clone();
        let (_, surface) = cx.add_window_view(|window, cx| {
            let view = cx.new(|_| VirtualDragSpike::new(command, platform));
            gpui_kit::base::Root::new(view, window, cx)
        });
        surface.simulate_resize(gpui_kit::size(gpui_kit::px(200.), gpui_kit::px(80.)));
        surface.refresh().expect("spike render");
        let bounds = surface
            .debug_bounds("archive-virtual-drag-spike")
            .expect("pointer seam");
        surface.simulate_event(gpui_kit::MouseDownEvent {
            position: bounds.center(),
            button: MouseButton::Left,
            ..Default::default()
        });
        let wire = endpoint
            .try_receive_command_for_contract_test()
            .expect("command queued before mouse-up or deferred turn");
        let Message::PlatformCommand(PlatformCommand::ArchiveVirtualDragSpike(actual)) =
            wire.message
        else {
            panic!("not virtual drag")
        };
        assert_eq!(*actual, expected);
        assert_eq!(
            actual.items.expect("frozen selection")[0]
                .entry
                .browser
                .as_ref()
                .expect("source")
                .snapshot_id,
            snapshot
        );
        assert!(
            endpoint.try_receive_command_for_contract_test().is_err(),
            "no eager materialization command"
        );
        surface.simulate_event(gpui_kit::MouseDownEvent {
            position: bounds.center(),
            button: MouseButton::Left,
            ..Default::default()
        });
        assert!(
            endpoint.try_receive_command_for_contract_test().is_err(),
            "held callback cannot start second gesture"
        );
    }
}
