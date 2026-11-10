use super::super::super::create_output_plan::plan_output_names;
use super::*;
use nxtools_protocol::archive_job::{
    ArchiveJobCreate, ArchiveJobLevel, ArchiveJobLimits, ArchiveJobMethod,
};
use nxtools_protocol::{
    Message, PlatformCommand, archive_create_destination::ArchiveCreateDestinationResult,
};

fn request(name: &str, format: ArchiveJobFormat, split: bool) -> ArchiveJobRequest {
    ArchiveJobRequest {
        kind: ArchiveJobKind::Create,
        paths: vec![r"C:\owned\source.bin".into()],
        destination: Some(r"C:\owned\output".into()),
        codepage: 65001,
        limits: ArchiveJobLimits::default(),
        extract: None,
        create: Some(ArchiveJobCreate {
            format,
            level: (format != ArchiveJobFormat::Tar).then_some(ArchiveJobLevel::Normal),
            method: (format == ArchiveJobFormat::Zip).then_some(ArchiveJobMethod::Deflate),
            password: None,
            solid: (format == ArchiveJobFormat::SevenZ).then_some(false),
            encrypt_header: (format == ArchiveJobFormat::SevenZ).then_some(false),
            strip_single_root: false,
            exclude_patterns: vec![],
            test_after_create: false,
            name: name.into(),
            volume_bytes: split.then_some(65536),
        }),
        modify: None,
        readmission: None,
    }
}

fn names(requests: &[ArchiveJobRequest]) -> Vec<&str> {
    requests
        .iter()
        .map(|request| request.create.as_ref().expect("create").name.as_str())
        .collect()
}

#[test]
fn complete_plan_reserves_disk_future_and_generated_names_before_assignment() {
    let mut requests = vec![
        request("report.gz", ArchiveJobFormat::Gz, false),
        request("report (2).gz", ArchiveJobFormat::Gz, false),
        request("report.gz", ArchiveJobFormat::Gz, false),
    ];
    plan_output_names(&mut requests, &["REPORT.GZ".into()]).expect("complete plan");
    assert_eq!(
        names(&requests),
        vec!["report (3).gz", "report (2).gz", "report (4).gz"]
    );
}

#[test]
fn split_family_observation_reserves_parts_without_claiming_family_atomicity() {
    let mut requests = vec![
        request("archive.zip", ArchiveJobFormat::Zip, true),
        request("archive (2).zip", ArchiveJobFormat::Zip, true),
    ];
    plan_output_names(
        &mut requests,
        &["archive.z01".into(), "unrelated.txt".into()],
    )
    .expect("ZIP family plan");
    assert_eq!(names(&requests), vec!["archive (3).zip", "archive (2).zip"]);

    let mut sevenz = vec![
        request("bundle.7z", ArchiveJobFormat::SevenZ, true),
        request("other.7z", ArchiveJobFormat::SevenZ, true),
    ];
    plan_output_names(&mut sevenz, &["BUNDLE.7Z.001".into()]).expect("7z family plan");
    assert_eq!(names(&sevenz), vec!["bundle (2).7z", "other.7z"]);
}

#[test]
fn compound_extensions_and_long_valid_names_keep_format_suffixes() {
    let mut compound = vec![
        request("report.tar.gz", ArchiveJobFormat::Tgz, false),
        request("report.tar.gz", ArchiveJobFormat::Tgz, false),
    ];
    plan_output_names(&mut compound, &[]).expect("compound plan");
    assert_eq!(names(&compound), vec!["report.tar.gz", "report (2).tar.gz"]);

    let stem = "界".repeat(80);
    let original = format!("{stem}.xz");
    assert!(nxtools_protocol::archive_job::valid_job_name(&original));
    let mut long = vec![
        request(&original, ArchiveJobFormat::Xz, false),
        request(&original, ArchiveJobFormat::Xz, false),
    ];
    plan_output_names(&mut long, &[]).expect("bounded long-name plan");
    assert!(
        long[1]
            .create
            .as_ref()
            .expect("renamed")
            .name
            .ends_with(" (2).xz")
    );
    assert!(nxtools_protocol::archive_job::valid_job_name(
        &long[1].create.as_ref().expect("renamed").name
    ));
}

#[test]
fn planner_rejects_mixed_destinations_before_original_batch_admission() {
    let mut requests = vec![
        request("first.gz", ArchiveJobFormat::Gz, false),
        request("second.gz", ArchiveJobFormat::Gz, false),
    ];
    requests[1].destination = Some(r"C:\other".into());
    assert!(plan_output_names(&mut requests, &[]).is_err());
    assert_eq!(names(&requests), vec!["first.gz", "second.gz"]);
}

#[gpui_kit::test]
fn original_create_form_observes_destination_before_any_batch_job(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (owner, endpoint) = super::super::super::tests::setup(cx);
    owner.update(cx, |app, cx| {
        app.settings.create_format = nxtools_core::archive::ArchiveCreateFormat::Gz;
        app.open_create_form(
            vec![
                crate::archive::draft::DraftSource {
                    path: r"C:\one\report.txt".into(),
                    directory: false,
                    bytes: Some(1),
                },
                crate::archive::draft::DraftSource {
                    path: r"C:\two\report (2).txt".into(),
                    directory: false,
                    bytes: Some(1),
                },
                crate::archive::draft::DraftSource {
                    path: r"C:\three\report.txt".into(),
                    directory: false,
                    bytes: Some(1),
                },
            ],
            cx,
        );
    });
    cx.run_until_parked();
    let window = owner.read_with(cx, |app, _| {
        app.jobs.forms.values().next().expect("create form").window
    });
    let mut surface = gpui_kit::VisualTestContext::from_window(window, cx);
    super::object_scroll_to(&mut surface, "更多选项");
    super::object_click(&mut surface, "更多选项");
    super::object_scroll_to(&mut surface, "分别压缩每个来源");
    super::object_click(&mut surface, "分别压缩每个来源");
    super::object_click(&mut surface, "创建");

    let mut observation = None;
    while let Ok(envelope) = endpoint.try_receive_command_for_contract_test() {
        match envelope.message {
            Message::PlatformCommand(PlatformCommand::ArchiveCreateDestination(command)) => {
                observation = Some(command);
            }
            Message::PlatformCommand(PlatformCommand::ArchiveJob(_)) => {
                panic!("batch job started before destination observation")
            }
            _ => {}
        }
    }
    let command = observation.expect("destination observation command");
    assert_eq!(command.destination, r"C:\one");
    assert!(owner.read_with(cx, |app, _| {
        app.jobs.create_plans.contains_key(&command.operation_id)
    }));
    let result = ArchiveCreateDestinationResult {
        version: 1,
        operation_id: command.operation_id,
        session_id: command.session_id,
        generation: command.generation,
        destination_missing: false,
        existing_names: vec!["REPORT.GZ".into()],
        code: None,
        native_error_code: None,
    };
    owner.update(cx, |app, cx| {
        app.notification(
            nxtools_ipc::PlatformNotification::CommandResult {
                correlation_id: command.operation_id,
                result: nxtools_protocol::PlatformCommandResult {
                    success: true,
                    error: None,
                    native_error_code: None,
                    data: Some(serde_json::to_string(&result).expect("result wire")),
                },
            },
            cx,
        );
    });
    cx.run_until_parked();
    let mut planned = owner.read_with(cx, |app, _| {
        app.jobs
            .records
            .values()
            .filter_map(|record| {
                record
                    .command
                    .request
                    .as_ref()
                    .and_then(|request| request.create.as_ref())
                    .map(|create| create.name.clone())
            })
            .collect::<Vec<_>>()
    });
    planned.sort();
    assert_eq!(
        planned,
        vec![
            "report (2).gz".to_owned(),
            "report (3).gz".to_owned(),
            "report (4).gz".to_owned(),
        ]
    );
}
type TestOwner = gpui_kit::Entity<super::super::super::ArchiveApplication>;
type TestEndpoint = nxtools_ipc::PlatformServerEndpoint;

fn submitted_each_input(
    cx: &mut gpui_kit::TestAppContext,
) -> (
    TestOwner,
    TestEndpoint,
    nxtools_protocol::archive_create_destination::ArchiveCreateDestinationCommand,
) {
    let (owner, endpoint) = super::super::super::tests::setup(cx);
    owner.update(cx, |app, cx| {
        app.settings.create_format = nxtools_core::archive::ArchiveCreateFormat::Gz;
        app.open_create_form(
            ["one", "two", "three"]
                .into_iter()
                .map(|part| crate::archive::draft::DraftSource {
                    path: format!(r"C:\{part}\report.txt").into(),
                    directory: false,
                    bytes: Some(1),
                })
                .collect(),
            cx,
        );
    });
    cx.run_until_parked();
    let window = owner.read_with(cx, |app, _| {
        app.jobs
            .forms
            .values()
            .next()
            .expect("original create form")
            .window
    });
    let mut surface = gpui_kit::VisualTestContext::from_window(window, cx);
    super::object_scroll_to(&mut surface, "更多选项");
    super::object_click(&mut surface, "更多选项");
    super::object_scroll_to(&mut surface, "分别压缩每个来源");
    super::object_click(&mut surface, "分别压缩每个来源");
    super::object_click(&mut surface, "创建");
    let mut observation = None;
    while let Ok(envelope) = endpoint.try_receive_command_for_contract_test() {
        match envelope.message {
            Message::PlatformCommand(PlatformCommand::ArchiveCreateDestination(command)) => {
                assert!(observation.replace(command).is_none(), "one observation");
            }
            Message::PlatformCommand(PlatformCommand::ArchiveJob(_)) => {
                panic!("no job admitted before observation")
            }
            _ => {}
        }
    }
    let command = observation.expect("original form submitted one observation");
    assert!(owner.read_with(cx, |app, _| {
        app.jobs.create_plans.contains_key(&command.operation_id)
    }));
    (owner, endpoint, command)
}

fn observation_result(
    command: &nxtools_protocol::archive_create_destination::ArchiveCreateDestinationCommand,
    existing: Vec<String>,
) -> ArchiveCreateDestinationResult {
    ArchiveCreateDestinationResult {
        version: command.version,
        operation_id: command.operation_id,
        session_id: command.session_id,
        generation: command.generation,
        destination_missing: false,
        existing_names: existing,
        code: None,
        native_error_code: None,
    }
}

fn notify_observation(
    owner: &TestOwner,
    cx: &mut gpui_kit::TestAppContext,
    operation: uuid::Uuid,
    result: nxtools_protocol::PlatformCommandResult,
) {
    owner.update(cx, |app, cx| {
        app.notification(
            nxtools_ipc::PlatformNotification::CommandResult {
                correlation_id: operation,
                result,
            },
            cx,
        );
    });
}

fn result_wire(result: &ArchiveCreateDestinationResult) -> nxtools_protocol::PlatformCommandResult {
    nxtools_protocol::PlatformCommandResult {
        success: true,
        error: None,
        native_error_code: None,
        data: Some(serde_json::to_string(result).expect("native result wire")),
    }
}

fn assert_form_retryable_without_jobs(owner: &TestOwner, cx: &mut gpui_kit::TestAppContext) {
    cx.run_until_parked();
    owner.read_with(cx, |app, cx| {
        assert!(
            app.jobs.create_plans.is_empty(),
            "observation ownership retired"
        );
        assert!(app.jobs.records.is_empty(), "no partial batch admission");
        let slot = app.jobs.forms.values().next().expect("form");
        assert!(
            slot.view
                .read_with(cx, |form, _| !form.busy())
                .unwrap_or(false),
            "original form is retryable"
        );
    });
}

#[gpui_kit::test]
fn each_input_observation_timeout_retires_pending_and_reenables_original_form(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (owner, endpoint, command) = submitted_each_input(cx);
    cx.executor()
        .advance_clock(std::time::Duration::from_secs(8));
    assert_form_retryable_without_jobs(&owner, cx);
    notify_observation(
        &owner,
        cx,
        command.operation_id,
        result_wire(&observation_result(&command, vec![])),
    );
    cx.run_until_parked();
    assert!(owner.read_with(cx, |app, _| app.jobs.records.is_empty()));
    assert!(
        !std::iter::from_fn(|| endpoint.try_receive_command_for_contract_test().ok()).any(
            |envelope| matches!(
                envelope.message,
                Message::PlatformCommand(PlatformCommand::ArchiveJob(_))
            )
        )
    );
}

fn failed_observation_has_retryable_form_and_zero_jobs(
    cx: &mut gpui_kit::TestAppContext,
    variant: u8,
) {
    let (owner, endpoint, command) = submitted_each_input(cx);
    let result = match variant {
        0 => nxtools_protocol::PlatformCommandResult {
            success: false,
            error: Some("destination_unavailable".into()),
            native_error_code: Some(5),
            data: None,
        },
        1 => nxtools_protocol::PlatformCommandResult {
            success: true,
            error: None,
            native_error_code: None,
            data: Some("{not-json".into()),
        },
        2 => result_wire(&observation_result(&command, vec!["x".into(); 16_385])),
        _ => nxtools_protocol::PlatformCommandResult {
            success: true,
            error: None,
            native_error_code: None,
            data: Some("x".repeat(nxtools_protocol::MAX_FRAME_BYTES + 1)),
        },
    };
    notify_observation(&owner, cx, command.operation_id, result);
    assert_form_retryable_without_jobs(&owner, cx);
    assert!(
        !std::iter::from_fn(|| endpoint.try_receive_command_for_contract_test().ok()).any(
            |envelope| matches!(
                envelope.message,
                Message::PlatformCommand(PlatformCommand::ArchiveJob(_))
            )
        )
    );
}

#[gpui_kit::test]
fn each_input_native_failure_keeps_original_form_retryable(cx: &mut gpui_kit::TestAppContext) {
    failed_observation_has_retryable_form_and_zero_jobs(cx, 0);
}

#[gpui_kit::test]
fn each_input_malformed_receipt_keeps_original_form_retryable(cx: &mut gpui_kit::TestAppContext) {
    failed_observation_has_retryable_form_and_zero_jobs(cx, 1);
}

#[gpui_kit::test]
fn each_input_overflowed_names_keep_original_form_retryable(cx: &mut gpui_kit::TestAppContext) {
    failed_observation_has_retryable_form_and_zero_jobs(cx, 2);
}

#[gpui_kit::test]
fn each_input_overflowed_frame_keeps_original_form_retryable(cx: &mut gpui_kit::TestAppContext) {
    failed_observation_has_retryable_form_and_zero_jobs(cx, 3);
}

#[gpui_kit::test]
fn each_input_form_close_retires_observation_and_late_result_cannot_admit(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (owner, endpoint, command) = submitted_each_input(cx);
    let (window, view) = owner.read_with(cx, |app, _| {
        let slot = &app.jobs.forms[&command.session_id];
        (slot.window, slot.view.clone())
    });
    window
        .update(cx, |_, window, cx| {
            view.update(cx, |form, cx| form.close(window, cx))
                .expect("live form");
        })
        .expect("live window");
    cx.run_until_parked();
    assert!(owner.read_with(
        cx,
        |app, _| !app.jobs.forms.contains_key(&command.session_id)
            && app.jobs.create_plans.is_empty()
    ));
    notify_observation(
        &owner,
        cx,
        command.operation_id,
        result_wire(&observation_result(&command, vec![])),
    );
    cx.run_until_parked();
    assert!(owner.read_with(cx, |app, _| app.jobs.records.is_empty()));
    assert!(
        !std::iter::from_fn(|| endpoint.try_receive_command_for_contract_test().ok()).any(
            |envelope| matches!(
                envelope.message,
                Message::PlatformCommand(PlatformCommand::ArchiveJob(_))
            )
        )
    );
}

#[gpui_kit::test]
fn each_input_observation_receipt_then_disconnect_before_deferred_admission(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (owner, endpoint, command) = submitted_each_input(cx);
    owner.update(cx, |app, cx| {
        app.notification(
            nxtools_ipc::PlatformNotification::CommandResult {
                correlation_id: command.operation_id,
                result: result_wire(&observation_result(&command, vec![])),
            },
            cx,
        );
        endpoint.set_connected_for_contract_test(false);
    });
    cx.run_until_parked();
    assert_form_retryable_without_jobs(&owner, cx);
}

fn original_queue_capacity_prevents_partial_each_input_admission(
    cx: &mut gpui_kit::TestAppContext,
    occupied: usize,
) {
    let (owner, endpoint, command) = submitted_each_input(cx);
    owner.update(cx, |app, _| {
        for index in 0..occupied {
            app.jobs
                .queue
                .enqueue(
                    uuid::Uuid::new_v4(),
                    crate::archive::queue::Resources {
                        sources: vec![format!(r"C:\queue\{index}.bin")],
                        output: Some(r"C:\occupied".into()),
                        memory: 128 << 20,
                    },
                )
                .expect("original queue prefilled");
        }
        assert_eq!(app.jobs.queue.queued_count(), occupied);
    });
    notify_observation(
        &owner,
        cx,
        command.operation_id,
        result_wire(&observation_result(&command, vec![])),
    );
    assert_form_retryable_without_jobs(&owner, cx);
    assert!(
        !std::iter::from_fn(|| endpoint.try_receive_command_for_contract_test().ok()).any(
            |envelope| matches!(
                envelope.message,
                Message::PlatformCommand(PlatformCommand::ArchiveJob(_))
            )
        )
    );
}

#[gpui_kit::test]
fn each_input_full_original_queue_refuses_whole_planned_batch(cx: &mut gpui_kit::TestAppContext) {
    original_queue_capacity_prevents_partial_each_input_admission(cx, 32);
}

#[gpui_kit::test]
fn each_input_near_full_queue_refuses_three_members_not_two_of_three(
    cx: &mut gpui_kit::TestAppContext,
) {
    original_queue_capacity_prevents_partial_each_input_admission(cx, 30);
}

#[test]
fn supported_large_destination_numbered_collisions_do_not_block_output_planning() {
    let mut requests = vec![
        request("report.gz", ArchiveJobFormat::Gz, false),
        request("other.gz", ArchiveJobFormat::Gz, false),
    ];
    let mut existing = vec!["report.gz".to_owned()];
    existing.extend((2..=8193).map(|number| format!("report ({number}).gz")));
    let started = std::time::Instant::now();
    plan_output_names(&mut requests, &existing)
        .expect("supported destination below 16,384 entries");
    assert!(
        started.elapsed() < std::time::Duration::from_secs(5),
        "one supported destination must not stall the application turn"
    );
    assert_eq!(names(&requests), vec!["report (8194).gz", "other.gz"]);
}
