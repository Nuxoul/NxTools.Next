use super::*;

fn native_create(name: &str) -> ArchiveJobTerminal {
    let root = std::env::var_os("NXTOOLS_OPT03_CREATE_WIRE")
        .expect("run effects-audited OPT03 native fixture and set NXTOOLS_OPT03_CREATE_WIRE");
    let bytes = std::fs::read(PathBuf::from(root).join(name)).expect("actual C# terminal export");
    let terminal: ArchiveJobTerminal = serde_json::from_slice(&bytes).expect("actual C# wire");
    assert_eq!(terminal.output_published, Some(true));
    assert!(terminal.cleanup_complete && terminal.result_location.is_some());
    terminal
}
fn post_test_case(cx: &mut gpui_kit::TestAppContext, check_target: bool, test_cleanup: bool) {
    for (file, format, split) in [
        ("ordinary.zip.json", ArchiveJobFormat::Zip, false),
        ("split.zip.json", ArchiveJobFormat::Zip, true),
        ("split.7z.json", ArchiveJobFormat::SevenZ, true),
    ] {
        for outcome in [
            ArchiveJobOutcome::Succeeded,
            ArchiveJobOutcome::Failed,
            ArchiveJobOutcome::Cancelled,
        ] {
            let native = native_create(file);
            let target = native
                .result_location
                .as_ref()
                .expect("native location")
                .target
                .clone();
            let (owner, endpoint) = super::super::super::tests::setup(cx);
            let mut input = request(1);
            input.kind = ArchiveJobKind::Create;
            input.destination = Some(r"C:\owned\request-destination".into());
            input.create = Some(ArchiveJobCreate {
                format,
                level: Some(ArchiveJobLevel::Normal),
                method: (format == ArchiveJobFormat::Zip).then_some(ArchiveJobMethod::Deflate),
                password: None,
                solid: None,
                encrypt_header: None,
                strip_single_root: false,
                exclude_patterns: vec![],
                test_after_create: true,
                name: if format == ArchiveJobFormat::SevenZ {
                    "request.7z"
                } else {
                    "request.zip"
                }
                .into(),
                volume_bytes: split.then_some(1_048_576),
            });
            let id = owner.update(cx, |app, cx| {
                app.submit_job(input, completion(), cx)
                    .expect("original Create")
            });
            while endpoint.try_receive_command_for_contract_test().is_ok() {}
            let mut event = terminal(id, true);
            event.terminal = Some(native.clone());
            assert!(
                event.validate().is_ok(),
                "actual C# Create terminal is valid"
            );
            deliver(&owner, &endpoint, event, cx);
            let mut test = None;
            while let Ok(wire) = endpoint.try_receive_command_for_contract_test() {
                if let Message::PlatformCommand(PlatformCommand::ArchiveJob(command)) = wire.message
                    && command.action == ArchiveJobAction::Start
                {
                    test = Some(command);
                }
            }
            let test = test.expect("original post-Test dispatch");
            if check_target {
                assert_eq!(
                    test.request.as_ref().expect("Test request").paths,
                    vec![target],
                    "post-Test uses native committed primary, not request-name guess"
                );
            }
            let mut event = terminal(test.operation_id, test_cleanup);
            event.session_id = id;
            let result = event.terminal.as_mut().expect("Test terminal");
            result.outcome = outcome;
            result.code = match outcome {
                ArchiveJobOutcome::Succeeded => None,
                ArchiveJobOutcome::Failed => Some("corrupt_archive".into()),
                _ => Some("cancelled".into()),
            };
            result.files_committed = 0;
            result.bytes_committed = 0;
            result.files_verified = 0;
            result.bytes_verified = 0;
            result.files_skipped = 0;
            result.commit_state = None;
            result.journal.clear();
            result.output_published = Some(false);
            result.result_location = None;
            let expected_code = result.code.clone();
            assert!(
                event.validate().is_ok(),
                "Test cleanup terminal reaches the business boundary"
            );
            deliver(&owner, &endpoint, event, cx);
            let facts = owner.read_with(cx, |app, _| {
                app.jobs.records[&id]
                    .terminal
                    .clone()
                    .expect("logical terminal")
            });
            assert_eq!(facts.outcome, outcome);
            assert_eq!(
                facts.code, expected_code,
                "Test keeps its own outcome/error"
            );
            assert_eq!(
                facts.output_published, native.output_published,
                "Test cannot erase Create publication"
            );
            assert_eq!(facts.result_location, native.result_location);
            assert_eq!(facts.commit_state, native.commit_state);
            assert_eq!(facts.journal, native.journal);
            assert_eq!(
                (
                    facts.files_committed,
                    facts.bytes_committed,
                    facts.files_verified,
                    facts.bytes_verified,
                    facts.files_skipped
                ),
                (
                    native.files_committed,
                    native.bytes_committed,
                    native.files_verified,
                    native.bytes_verified,
                    native.files_skipped
                )
            );
            assert_eq!(
                facts.cleanup_complete, test_cleanup,
                "Create cleanup cannot hide Test cleanup failure"
            );
            if !test_cleanup {
                owner.read_with(cx, |app, _| {
                    let record = &app.jobs.records[&id];
                    assert!(
                        app.jobs.queue.is_running(id),
                        "incomplete Test cleanup holds the original running reservation"
                    );
                    assert!(
                        record.created.is_none(),
                        "Create facts merged once into the logical terminal"
                    );
                    assert!(record.location_pending.is_none() && !record.auto_location_waiting);
                    assert!(!record.retry_pending && !record.can_continue_reclamation());
                });
                owner.update(cx, |app, cx| {
                    app.locate_job_result(id, cx);
                    app.retry_job(id, cx);
                });
                cx.run_until_parked();
                while let Ok(wire) = endpoint.try_receive_command_for_contract_test() {
                    assert!(
                        !matches!(
                            wire.message,
                            Message::PlatformCommand(
                                PlatformCommand::ArchiveJob(_)
                                    | PlatformCommand::ArchiveSpace(_)
                                    | PlatformCommand::OpenShellTarget(_)
                            )
                        ),
                        "no new attempt, reacquisition or location after cleanup failure"
                    );
                }
                assert!(owner.read_with(cx, |app, _| app.jobs.queue.is_running(id)));
            }
        }
    }
}
#[gpui_kit::test]
fn opt03_post_test_uses_actual_native_primary(cx: &mut gpui_kit::TestAppContext) {
    post_test_case(cx, true, true);
}
#[gpui_kit::test]
fn opt03_post_test_preserves_native_publication_and_location(cx: &mut gpui_kit::TestAppContext) {
    post_test_case(cx, false, true);
}

#[gpui_kit::test]
fn opt03_post_test_missing_location_cleanup_and_disconnect(cx: &mut gpui_kit::TestAppContext) {
    for boundary in ["missing", "unclean", "cancelled", "disconnect"] {
        let mut native = native_create("ordinary.zip.json");
        let (owner, endpoint) = super::super::super::tests::setup(cx);
        let mut input = request(1);
        input.kind = ArchiveJobKind::Create;
        input.destination = Some(r"C:\owned\out".into());
        input.create = Some(ArchiveJobCreate {
            format: ArchiveJobFormat::Zip,
            level: Some(ArchiveJobLevel::Normal),
            method: Some(ArchiveJobMethod::Deflate),
            password: None,
            solid: None,
            encrypt_header: None,
            strip_single_root: false,
            exclude_patterns: vec![],
            test_after_create: true,
            name: "request.zip".into(),
            volume_bytes: None,
        });
        let id = owner.update(cx, |app, cx| {
            app.submit_job(input, completion(), cx).expect("Create")
        });
        while endpoint.try_receive_command_for_contract_test().is_ok() {}
        match boundary {
            "missing" => native.result_location = None,
            "unclean" => native.cleanup_complete = false,
            "cancelled" => {
                native.outcome = ArchiveJobOutcome::Cancelled;
                native.code = Some("cancelled".into());
            }
            _ => {}
        }
        let mut event = terminal(id, native.cleanup_complete);
        event.terminal = Some(native.clone());
        deliver(&owner, &endpoint, event, cx);
        let mut tests = 0;
        while let Ok(wire) = endpoint.try_receive_command_for_contract_test() {
            if let Message::PlatformCommand(PlatformCommand::ArchiveJob(command)) = wire.message
                && command.action == ArchiveJobAction::Start
            {
                tests += 1;
            }
        }
        if boundary == "disconnect" {
            assert_eq!(tests, 1);
            owner.update(cx, |app, cx| {
                let generation = app.platform.connection_generation().expect("connection");
                endpoint.set_connected_for_contract_test(false);
                app.notification(
                    PlatformNotification::Disconnected {
                        generation,
                        pending_command_ids: vec![],
                    },
                    cx,
                );
            });
            owner.read_with(cx, |app, _| {
                let record = &app.jobs.records[&id];
                assert!(
                    record.terminal.is_none(),
                    "unknown Test cleanup retains the original running slot"
                );
                let created = record
                    .created
                    .as_ref()
                    .expect("Create facts survive disconnect");
                assert_eq!(created.output_published, Some(true));
                assert_eq!(created.result_location, native.result_location);
                assert_eq!(created.journal, native.journal);
                assert!(app.jobs.queue.is_running(id));
            });
            continue;
        } else {
            assert_eq!(
                tests, 0,
                "no Test without completed successful creation and location"
            );
        }
        let facts = owner.read_with(cx, |app, _| {
            app.jobs.records[&id]
                .terminal
                .clone()
                .expect("logical terminal")
        });
        assert_eq!(facts.output_published, Some(true));
        assert_eq!(facts.result_location, native.result_location);
        assert_eq!(facts.journal, native.journal);
        assert_eq!(facts.files_committed, native.files_committed);
        match boundary {
            "missing" => assert_eq!(
                facts.code.as_deref(),
                Some("created_result_location_unavailable")
            ),
            "unclean" | "disconnect" => assert!(!facts.cleanup_complete),
            "cancelled" => assert_eq!(facts.outcome, ArchiveJobOutcome::Cancelled),
            _ => unreachable!(),
        }
    }
}

#[gpui_kit::test]
fn opt03_ac03_post_test_cleanup_failure_preserves_create_facts_and_running_slot(
    cx: &mut gpui_kit::TestAppContext,
) {
    post_test_case(cx, true, false);
}
