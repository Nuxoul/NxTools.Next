use super::*;
use crate::archive::ordinary_material_test_support as support;
use gpui_kit::TestAppContext;

fn stage(
    cx: &mut TestAppContext,
) -> (
    Entity<ArchiveApplication>,
    nxtools_ipc::PlatformServerEndpoint,
    Uuid,
) {
    let (owner, endpoint) = super::super::tests::setup_unprepared(cx);
    let operation = Uuid::new_v4();
    let event = ArchiveVirtualDragProgress {
        version: 1,
        operation_id: operation,
        session_id: Uuid::new_v4(),
        generation: 1,
        phase: "materializing".into(),
        bytes_done: 0,
        bytes_total: Some(10),
        files_done: 0,
        files_total: 1,
        current_index: None,
        current_member: None,
        current_request_id: None,
        current_bytes_done: None,
        current_bytes_total: None,
        bytes_processed: Some(0),
        sequence: Some(1),
        cleanup_complete: None,
        code: None,
        native_error_code: None,
    };
    let capture = Rc::new(std::cell::RefCell::new(None));
    let captured = capture.clone();
    let weak = owner.downgrade();
    let initial = event.clone();
    let window: AnyWindowHandle = cx.update(|cx| {
        cx.open_window(
            WindowOptions {
                show: false,
                focus: false,
                ..crate::ui::components::titlebar::caption_window_options()
            },
            move |window, cx| {
                let view = cx.new(|cx| JobProgressView::new_drag(initial, weak, cx));
                *captured.borrow_mut() = Some(view.downgrade());
                cx.new(|cx| gpui_kit::base::Root::new(view, window, cx))
            },
        )
        .expect("original progress Root")
        .into()
    });
    owner.update(cx, |app, cx| {
        support::facts(Uuid::new_v4(), 1, cx);
        crate::ui::material::runtime::set_test_window_handle(window, 48, cx);
        let receipt = Uuid::new_v4();
        app.jobs.drag_progress = Some(DragPresentation {
            operation,
            generation: 1,
            session: event.session_id,
            ended: false,
            latest: event,
            members: Rc::from([None]),
            started: Instant::now(),
            window,
            view: capture.borrow_mut().take().expect("progress view"),
            hidden: false,
            prepare: Some((receipt, 1)),
            material_pending: false,
            hwnd: 48,
            bounds: SetWindowBoundsCommand {
                window_handle: 48,
                x: 10,
                y: 20,
                width: 560,
                height: 300,
            },
        });
        assert!(app.drag_presentation_result(
            receipt,
            &PlatformCommandResult {
                success: true,
                error: None,
                native_error_code: None,
                data: None,
            },
            cx
        ));
    });
    cx.run_until_parked();
    (owner, endpoint, operation)
}

#[gpui_kit::test]
fn material_drag_bounds_ack_alone_cannot_show_then_receipt_uses_original_visibility(
    cx: &mut TestAppContext,
) {
    let (owner, endpoint, _) = stage(cx);
    let (id, command) = support::next(&endpoint);
    assert!(
        endpoint.try_receive_command_for_contract_test().is_err(),
        "no early reveal"
    );
    cx.update(|cx| support::accept(&endpoint, id, command, cx));
    cx.run_until_parked();
    let envelope = endpoint
        .try_receive_command_for_contract_test()
        .expect("original reveal");
    assert!(matches!(
        envelope.message,
        nxtools_protocol::Message::PlatformCommand(PlatformCommand::SetWindowVisibility(
            SetWindowVisibilityCommand {
                visible: true,
                window_handle: 48
            }
        ))
    ));
    owner.read_with(cx, |app, _| {
        assert!(
            !app.jobs
                .drag_progress
                .as_ref()
                .expect("retained drag")
                .material_pending
        )
    });
}

#[gpui_kit::test]
fn material_drag_background_during_wait_cannot_release_late_reveal(cx: &mut TestAppContext) {
    let (owner, endpoint, operation) = stage(cx);
    let (id, command) = support::next(&endpoint);
    owner.update(cx, |app, cx| app.background_drag_progress(operation, cx));
    cx.update(|cx| support::accept(&endpoint, id, command, cx));
    cx.run_until_parked();
    owner.read_with(cx, |app, _| {
        assert!(
            app.jobs
                .drag_progress
                .as_ref()
                .expect("retained hidden drag")
                .hidden
        )
    });
    while let Ok(envelope) = endpoint.try_receive_command_for_contract_test() {
        assert!(!matches!(
            envelope.message,
            nxtools_protocol::Message::PlatformCommand(PlatformCommand::SetWindowVisibility(
                SetWindowVisibilityCommand { visible: true, .. }
            ))
        ));
    }
}
