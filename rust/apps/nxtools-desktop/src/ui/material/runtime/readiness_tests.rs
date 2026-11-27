use super::tests::*;
use super::*;
use gpui_kit::TestAppContext;
use nxtools_protocol::PlatformCommandResult;
use std::{cell::RefCell, time::Duration};
use uuid::Uuid;

type Completions = Rc<RefCell<Vec<Result<(), &'static str>>>>;

fn request(
    window: AnyWindowHandle,
    bridge: Rc<PlatformBridge>,
    completions: Completions,
    cx: &mut App,
) {
    when_ready_with_handle(
        window,
        bridge,
        42,
        MaterialSurface::Backdrop,
        Duration::from_secs(3),
        move |result, _| {
            completions.borrow_mut().push(result);
        },
        cx,
    );
}
fn terminal(id: Uuid, command: ApplyWindowMaterialCommand) -> PlatformNotification {
    PlatformNotification::CommandResult {
        correlation_id: id,
        result: PlatformCommandResult {
            success: true,
            error: None,
            native_error_code: None,
            data: Some(serde_json::to_string(&applied(command)).expect("receipt")),
        },
    }
}

#[gpui_kit::test]
fn material_readiness_requires_receipt_and_rechecks_facts_before_deferred_callback(
    cx: &mut TestAppContext,
) {
    let (window, bridge, endpoint, session) = fixture(cx);
    let completions = Rc::new(RefCell::new(Vec::new()));
    cx.update(|cx| {
        let mut old = take_preparation(window, cx).expect("first record");
        old.retire(&bridge);
        request(window, bridge.clone(), completions.clone(), cx);
    });
    cx.run_until_parked();
    assert!(completions.borrow().is_empty());
    let (id, command) = next(&endpoint);
    let notification = terminal(id, command);
    if let PlatformNotification::CommandResult { result, .. } = &notification {
        endpoint
            .route_command_result_for_contract_test(id, result.clone())
            .expect("terminal");
    }
    cx.update(|cx| {
        assert!(handle_notification(&notification, cx));
        sample(session, 2, cx);
    });
    cx.run_until_parked();
    assert!(
        completions.borrow().is_empty(),
        "obsolete receipt cannot release reveal"
    );
    let (latest_id, latest) = next(&endpoint);
    assert_eq!(latest.facts_revision, 2);
    let notification = terminal(latest_id, latest);
    if let PlatformNotification::CommandResult { result, .. } = &notification {
        endpoint
            .route_command_result_for_contract_test(latest_id, result.clone())
            .expect("terminal");
    }
    cx.update(|cx| assert!(handle_notification(&notification, cx)));
    cx.run_until_parked();
    assert_eq!(&*completions.borrow(), &[Ok(())]);
    cx.run_until_parked();
    assert_eq!(completions.borrow().len(), 1);
}

#[gpui_kit::test]
fn material_readiness_close_cannot_release_old_reveal(cx: &mut TestAppContext) {
    let (window, bridge, endpoint, _) = fixture(cx);
    let completions = Rc::new(RefCell::new(Vec::new()));
    cx.update(|cx| {
        take_preparation(window, cx);
        request(window, bridge, completions.clone(), cx);
    });
    cx.run_until_parked();
    let (id, command) = next(&endpoint);
    cx.update(|cx| {
        window
            .update(cx, |_, window, _| window.remove_window())
            .expect("close");
    });
    cx.run_until_parked();
    cx.update(|cx| assert!(!handle_notification(&terminal(id, command), cx)));
    cx.run_until_parked();
    assert!(completions.borrow().is_empty());
}

#[gpui_kit::test]
fn material_readiness_disconnect_cannot_release_old_reveal(cx: &mut TestAppContext) {
    let completions = Rc::new(RefCell::new(Vec::new()));
    let (window, bridge, endpoint, _) = fixture(cx);
    cx.update(|cx| {
        take_preparation(window, cx);
        request(window, bridge, completions.clone(), cx);
    });
    cx.run_until_parked();
    let (id, command) = next(&endpoint);
    endpoint.set_connected_for_contract_test(false);
    cx.update(|cx| {
        let mut facts = *cx.global::<UiMaterialCapabilities>();
        facts.disconnected();
        cx.set_global(facts);
    });
    cx.run_until_parked();
    assert_eq!(&*completions.borrow(), &[Err("material_disconnected")]);
    cx.update(|cx| assert!(!handle_notification(&terminal(id, command), cx)));
    cx.run_until_parked();
    assert_eq!(completions.borrow().len(), 1);
}

#[gpui_kit::test]
fn material_readiness_timeout_cancels_unsent_and_never_calls_success(cx: &mut TestAppContext) {
    let (window, bridge, endpoint, _) = fixture(cx);
    let completions = Rc::new(RefCell::new(Vec::new()));
    let mut blocking = Preparation::default();
    cx.update(|cx| {
        blocking
            .ensure(&bridge, 43, MaterialSurface::Backdrop, cx)
            .expect("enqueue");
    });
    let (active, _) = next(&endpoint);
    cx.update(|cx| {
        take_preparation(window, cx);
        request(window, bridge, completions.clone(), cx);
    });
    cx.run_until_parked();
    let (token, queued) = cx.update(|cx| {
        let record = &cx.global::<MaterialRuntime>().windows[0];
        (
            record.readiness.as_ref().expect("pending").token,
            record.preparation.pending.as_ref().expect("queued").id,
        )
    });
    cx.update(|cx| expire_readiness(window, token, cx));
    cx.run_until_parked();
    assert_eq!(&*completions.borrow(), &[Err("material_timeout")]);
    assert!(!endpoint.command_pending_for_contract_test(queued));
    assert!(endpoint.command_pending_for_contract_test(active));
    assert!(endpoint.try_receive_command_for_contract_test().is_err());
}

#[gpui_kit::test]
fn material_readiness_native_failure_notifies_once_and_rejects_late_success(
    cx: &mut TestAppContext,
) {
    let (window, bridge, endpoint, _) = fixture(cx);
    let completions = Rc::new(RefCell::new(Vec::new()));
    cx.update(|cx| {
        take_preparation(window, cx);
        request(window, bridge.clone(), completions.clone(), cx);
    });
    cx.run_until_parked();
    let (id, command) = next(&endpoint);
    endpoint
        .route_command_result_for_contract_test(
            id,
            PlatformCommandResult {
                success: false,
                error: Some("window_material_failed".into()),
                native_error_code: Some(5),
                data: None,
            },
        )
        .expect("terminal failure");
    let notification = bridge
        .notification_receiver()
        .try_recv()
        .expect("original result route");
    cx.update(|cx| assert!(handle_notification(&notification, cx)));
    cx.run_until_parked();
    assert_eq!(&*completions.borrow(), &[Err("native_material_result")]);
    cx.update(|cx| assert!(!handle_notification(&terminal(id, command), cx)));
    cx.run_until_parked();
    assert_eq!(completions.borrow().len(), 1);
    assert!(endpoint.try_receive_command_for_contract_test().is_err());
}
