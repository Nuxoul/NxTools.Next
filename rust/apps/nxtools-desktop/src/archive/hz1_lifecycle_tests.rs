//! Actual application/form notification and retirement ownership, synthetic IPC only.
use super::*;
use gpui_kit::TestAppContext;
use nxtools_protocol::{
    Message, PlatformCommand, PlatformCommandResult, archive_space::ArchiveSpaceObservation,
};
fn open(
    cx: &mut TestAppContext,
) -> (
    Entity<ArchiveApplication>,
    nxtools_ipc::PlatformServerEndpoint,
    Uuid,
    AnyWindowHandle,
    WeakEntity<super::super::forms::OperationForm>,
) {
    let (owner, endpoint) = tests::setup(cx);
    owner.update(cx, |app, cx| {
        app.open_extract_form(r"C:\owned\hz1-lifecycle.zip".into(), None, cx)
    });
    cx.run_until_parked();
    let (id, window, view) = owner.read_with(cx, |app, _| {
        let (id, slot) = app.jobs.forms.iter().next().expect("original form");
        (*id, slot.window, slot.view.clone())
    });
    (owner, endpoint, id, window, view)
}
#[gpui_kit::test]
fn hz1_completed_plan_is_invalidated_by_actual_disconnect_notification(cx: &mut TestAppContext) {
    let (owner, endpoint, id, window, view) = open(cx);
    let wire = endpoint
        .try_receive_command_for_contract_test()
        .expect("original observation dispatch");
    let Message::PlatformCommand(PlatformCommand::ArchiveSpace(command)) = wire.message else {
        panic!("observation");
    };
    let result = PlatformCommandResult {
        success: true,
        error: None,
        native_error_code: None,
        data: Some(
            serde_json::to_string(&ArchiveSpaceObservation {
                version: 1,
                operation_id: command.operation_id,
                session_id: id,
                generation: command.generation,
                available_bytes: 8192,
                allocation_bytes: 4096,
                target_identity: "v1:target".into(),
                volume_identity: "v1:volume".into(),
                destination_missing: false,
                missing_destination_directories: Some(0),
                source_matches: true,
                source_set: None,
            })
            .expect("observation"),
        ),
    };
    owner.update(cx, |app, cx| {
        app.notification(
            PlatformNotification::CommandResult {
                correlation_id: wire.id,
                result,
            },
            cx,
        )
    });
    cx.run_until_parked();
    assert!(
        view.read_with(cx, |form, _| form.space_state_for_test().1.is_some())
            .expect("live form")
    );
    owner.update(cx, |app, cx| {
        app.notification(
            PlatformNotification::Disconnected {
                generation: 1,
                pending_command_ids: vec![],
            },
            cx,
        )
    });
    cx.run_until_parked();
    assert_eq!(
        view.read_with(cx, |form, _| form.space_state_for_test())
            .expect("retained form"),
        (false, None),
        "disconnect must retire the displayed completed observation, not only pending requests"
    );
    window
        .update(cx, |_, window, cx| {
            view.update(cx, |form, cx| form.close(window, cx))
        })
        .expect("window")
        .expect("form");
    cx.run_until_parked();
}
#[gpui_kit::test]
fn hz1_closed_form_releases_only_its_advisory_pending_record(cx: &mut TestAppContext) {
    let (owner, _endpoint, id, window, view) = open(cx);
    assert_eq!(owner.read_with(cx, |app, _| app.jobs.spaces.len()), 1);
    window
        .update(cx, |_, window, cx| {
            view.update(cx, |form, cx| form.close(window, cx))
        })
        .expect("window")
        .expect("form");
    cx.run_until_parked();
    assert!(owner.read_with(cx, |app, _| !app.jobs.forms.contains_key(&id)));
    assert!(
        owner.read_with(cx, |app, _| app.jobs.spaces.is_empty()),
        "closed form must release its UI-only observation record without waiting for timeout; native slots remain native-owned"
    );
}

fn result_for(
    command: &nxtools_protocol::archive_space::ArchiveSpaceCommand,
) -> PlatformCommandResult {
    PlatformCommandResult {
        success: true,
        error: None,
        native_error_code: None,
        data: Some(
            serde_json::to_string(&ArchiveSpaceObservation {
                version: 1,
                operation_id: command.operation_id,
                session_id: command.session_id,
                generation: command.generation,
                available_bytes: 8192,
                allocation_bytes: 4096,
                target_identity: "v1:target".into(),
                volume_identity: "v1:volume".into(),
                destination_missing: false,
                missing_destination_directories: Some(0),
                source_matches: true,
                source_set: None,
            })
            .expect("observation"),
        ),
    }
}

#[gpui_kit::test]
fn hz1_ac01_submit_retires_only_its_observation_and_late_receipt_is_quiet(cx: &mut TestAppContext) {
    let (owner, endpoint, first, window, _) = open(cx);
    let first_wire = endpoint
        .try_receive_command_for_contract_test()
        .expect("first observation");
    let Message::PlatformCommand(PlatformCommand::ArchiveSpace(first_command)) = first_wire.message
    else {
        panic!("space");
    };
    owner.update(cx, |app, cx| {
        app.open_extract_form(r"C:\owned\other.zip".into(), None, cx)
    });
    cx.run_until_parked();
    let other_wire = endpoint
        .try_receive_command_for_contract_test()
        .expect("other observation");
    let Message::PlatformCommand(PlatformCommand::ArchiveSpace(other_command)) = other_wire.message
    else {
        panic!("space");
    };
    let mut surface = gpui_kit::VisualTestContext::from_window(window, cx);
    surface.update(|window, cx| {
        window.activate_window();
        window.draw(cx).clear(cx);
    });
    let position = surface
        .debug_bounds("archive-form-submit")
        .expect("original Start Button")
        .center();
    surface.simulate_event(gpui_kit::MouseDownEvent {
        position,
        button: gpui_kit::MouseButton::Left,
        ..Default::default()
    });
    surface.simulate_event(gpui_kit::MouseUpEvent {
        position,
        button: gpui_kit::MouseButton::Left,
        modifiers: Default::default(),
        click_count: 1,
    });
    surface.run_until_parked();
    assert!(
        owner.read_with(&surface, |app, _| !app.jobs.forms.contains_key(&first)
            && app.jobs.records.len() == 1),
        "actual Start accepted and closes only first form"
    );
    assert_eq!(
        owner.read_with(&surface, |app, _| app.jobs.spaces.len()),
        1,
        "accepted submit must immediately retire only its UI observation"
    );
    owner.update(&mut surface, |app, cx| {
        assert!(
            !app.space_result(first_wire.id, &result_for(&first_command), cx),
            "late first receipt has no retired owner"
        );
        assert!(
            app.space_result(other_wire.id, &result_for(&other_command), cx),
            "other form keeps current observation"
        );
    });
    surface.run_until_parked();
    let (other_window, other_view) = owner.read_with(&surface, |app, _| {
        let slot = &app.jobs.forms[&other_command.session_id];
        (slot.window, slot.view.clone())
    });
    assert_eq!(
        other_view
            .read_with(&surface, |form, _| form.space_state_for_test())
            .expect("other form"),
        (false, Some((8192, None)))
    );
    other_window
        .update(&mut surface, |_, window, cx| {
            other_view.update(cx, |form, cx| form.close(window, cx))
        })
        .expect("window")
        .expect("form");
    surface.run_until_parked();
}

#[gpui_kit::test]
fn hz1_ac02_missing_target_zero_member_never_reports_definite_zero_demand(cx: &mut TestAppContext) {
    let (owner, endpoint) = tests::setup(cx);
    owner.update(cx, |app, cx| {
        app.open_canonical(r"C:\owned\zero.zip".into(), 0, vec![], cx)
    });
    cx.run_until_parked();
    let (session, generation, snapshot) = owner.update(cx, |app, cx| {
        let (&id, slot) = app.windows.iter_mut().next().expect("browser");
        let snapshot = Uuid::new_v4();
        slot.browser = Some(nxtools_protocol::archive_browser::ArchiveBrowserContext {
            snapshot_id: snapshot, format: "Zip".into(), comment: String::new(),
            source: nxtools_protocol::archive_browser::ArchiveSourceBinding { native: "v1:zero-source".into(), digest: [0;32], size: 500, modified_unix_ns: 0 },
            sevenz_edit: None,
            tar_edit: None,
        });
        let entry = serde_json::from_value(serde_json::json!({"ordinal":0,"name":"zero.txt","kind":"File","size":0,"packed_size":0,"encrypted":false,"warning_codes":[],"provenance":"Utf8Flag","codepage":65001})).expect("zero metadata");
        slot.window.update(cx, |_, window, cx| slot.view.update(cx, |view, cx| view.install(snapshot, vec![entry], "Zip".into(), window, cx))).expect("window").expect("browser");
        (id, slot.generation, snapshot)
    });
    while endpoint.try_receive_command_for_contract_test().is_ok() {}
    owner.update(cx, |app, cx| {
        app.settings.default_destination = nxtools_core::archive::ArchiveDestination::Ask;
        app.settings.extract_custom_destination = Some(r"C:\owned\missing\child".into());
        app.extract_browser(session, generation, snapshot, None, cx);
    });
    cx.run_until_parked();
    let wire = endpoint
        .try_receive_command_for_contract_test()
        .expect("space dispatch");
    let Message::PlatformCommand(PlatformCommand::ArchiveSpace(command)) = wire.message else {
        panic!("space");
    };
    let mut result = result_for(&command);
    let mut value: serde_json::Value =
        serde_json::from_str(result.data.as_deref().expect("data")).expect("observation");
    value["available_bytes"] = 0.into();
    value["destination_missing"] = true.into();
    value["missing_destination_directories"] = 2.into();
    result.data = Some(serde_json::to_string(&value).expect("wire"));
    owner.update(cx, |app, cx| {
        app.notification(
            PlatformNotification::CommandResult {
                correlation_id: wire.id,
                result,
            },
            cx,
        )
    });
    cx.run_until_parked();
    let (window, view) = owner.read_with(cx, |app, _| {
        let slot = &app.jobs.forms[&command.session_id];
        (slot.window, slot.view.clone())
    });
    window
        .update(cx, |_, window, cx| {
            window.draw(cx).clear(cx);
        })
        .expect("real form draw");
    let (_, plan) = view
        .read_with(cx, |form, _| form.space_state_for_test())
        .expect("form");
    let (available, allocation) = plan.expect("current observation");
    assert_eq!(available, 0);
    assert_eq!(
        allocation,
        Some(8192),
        "two observed target components add exactly two allocation allowances"
    );
    assert!(
        allocation.is_none_or(|bytes| bytes >= 8192),
        "missing target components cannot display definite zero demand/fit"
    );
    window
        .update(cx, |_, window, cx| {
            view.update(cx, |form, cx| form.close(window, cx))
        })
        .expect("window")
        .expect("form");
    let host = owner.read_with(cx, |app, _| app.host.as_ref().expect("host").id);
    owner.update(cx, |app, cx| app.close_host(host, cx));
    cx.run_until_parked();
}

#[gpui_kit::test]
fn hz1_ac01_failed_submit_keeps_other_and_current_observations(cx: &mut TestAppContext) {
    let (owner, endpoint, first, window, view) = open(cx);
    let first_wire = endpoint
        .try_receive_command_for_contract_test()
        .expect("first space");
    let Message::PlatformCommand(PlatformCommand::ArchiveSpace(command)) = first_wire.message
    else {
        panic!("space");
    };
    owner.update(cx, |app, cx| {
        app.open_extract_form(r"C:\owned\other-failure.zip".into(), None, cx);
        let invalid = nxtools_protocol::archive_job::ArchiveJobRequest {
            kind: nxtools_protocol::archive_job::ArchiveJobKind::Extract,
            paths: vec![],
            destination: None,
            codepage: 0,
            limits: Default::default(),
            extract: None,
            create: None,
            modify: None,
            readmission: None,
        };
        app.submit_form(first, command.generation, invalid, Default::default(), cx);
    });
    cx.run_until_parked();
    assert_eq!(
        owner.read_with(cx, |app, _| app.jobs.spaces.len()),
        2,
        "rejected submit does not retire either valid observation"
    );
    assert!(
        owner.read_with(cx, |app, _| app.jobs.forms.contains_key(&first)
            && app.jobs.records.is_empty())
    );
    owner.update(cx, |app, cx| {
        assert!(app.space_result(first_wire.id, &result_for(&command), cx));
    });
    cx.run_until_parked();
    assert_eq!(
        view.read_with(cx, |form, _| form.space_state_for_test())
            .expect("first draft"),
        (false, Some((8192, None)))
    );
    window
        .update(cx, |_, window, cx| {
            view.update(cx, |form, cx| form.close(window, cx))
        })
        .expect("window")
        .expect("form");
    cx.run_until_parked();
    assert_eq!(
        owner.read_with(cx, |app, _| app.jobs.spaces.len()),
        1,
        "closing first preserves only other pending observation"
    );
    let (other_window, other_view) = owner.read_with(cx, |app, _| {
        let slot = app.jobs.forms.values().next().expect("other form");
        (slot.window, slot.view.clone())
    });
    other_window
        .update(cx, |_, window, cx| {
            other_view.update(cx, |form, cx| form.close(window, cx))
        })
        .expect("window")
        .expect("form");
    cx.run_until_parked();
    assert!(owner.read_with(cx, |app, _| app.jobs.spaces.is_empty()));
}
