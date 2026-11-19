//! ZIP-SFX completion on the original JobCenter and TestPlatform queue.
use super::*;

fn post_test(cx: &mut gpui_kit::TestAppContext, clean: bool) {
    for split in [false, true] {
        for outcome in [
            ArchiveJobOutcome::Succeeded,
            ArchiveJobOutcome::Failed,
            ArchiveJobOutcome::Cancelled,
        ] {
            let (owner, endpoint) = super::super::super::tests::setup(cx);
            let mut input = request(1);
            input.kind = ArchiveJobKind::Create;
            input.destination = Some(r"C:\owned\requested".into());
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
                name: "requested.exe".into(),
                volume_bytes: split.then_some(1_048_576),
                self_extracting: true,
            });
            let id = owner.update(cx, |app, cx| {
                app.submit_job(input, completion(), cx).expect("SFX Create")
            });
            while endpoint.try_receive_command_for_contract_test().is_ok() {}
            let target = if split {
                r"C:\owned\published\actual.zip"
            } else {
                r"C:\owned\published\actual.exe"
            };
            let mut created = terminal(id, true);
            let facts = created.terminal.as_mut().expect("Create facts");
            facts.outcome = ArchiveJobOutcome::Succeeded;
            facts.code = None;
            facts.output_published = Some(true);
            facts.result_location = Some(ArchiveJobResultLocation {
                target: target.into(),
                action: nxtools_protocol::ShellOpenAction::SelectInExplorer,
            });
            let native = facts.clone();
            assert!(created.validate().is_ok());
            deliver(&owner, &endpoint, created, cx);
            let mut tests = vec![];
            while let Ok(wire) = endpoint.try_receive_command_for_contract_test() {
                match wire.message {
                    Message::PlatformCommand(PlatformCommand::ArchiveJob(command))
                        if command.action == ArchiveJobAction::Start =>
                    {
                        tests.push(command)
                    }
                    Message::PlatformCommand(PlatformCommand::OpenShellTarget(_)) => {
                        panic!("Create must not execute or locate before post-Test")
                    }
                    _ => {}
                }
            }
            assert_eq!(tests.len(), 1);
            let test = tests.remove(0);
            let captured = test.request.as_ref().expect("immutable Test request");
            assert_eq!(captured.kind, ArchiveJobKind::Test);
            assert_eq!(captured.paths, [target]);
            assert!(captured.create.is_none() && captured.destination.is_none());
            assert_ne!(test.operation_id, id);
            assert!(owner.read_with(cx, |app, _| app.jobs.records[&id].terminal.is_none()));
            let mut done = terminal(test.operation_id, clean);
            done.session_id = id;
            let facts = done.terminal.as_mut().expect("Test facts");
            facts.outcome = outcome;
            facts.code = match outcome {
                ArchiveJobOutcome::Succeeded => None,
                ArchiveJobOutcome::Failed => Some("corrupt_archive".into()),
                _ => Some("cancelled".into()),
            };
            facts.files_committed = 0;
            facts.bytes_committed = 0;
            facts.files_verified = 0;
            facts.bytes_verified = 0;
            facts.commit_state = None;
            facts.journal.clear();
            facts.output_published = Some(false);
            facts.result_location = None;
            assert!(done.validate().is_ok());
            deliver(&owner, &endpoint, done, cx);
            cx.run_until_parked();
            owner.read_with(cx, |app, _| {
                let record = &app.jobs.records[&id];
                let facts = record.terminal.as_ref().expect("logical terminal");
                assert_eq!(facts.outcome, outcome);
                assert_eq!(facts.cleanup_complete, clean);
                assert_eq!(facts.output_published, native.output_published);
                assert_eq!(facts.result_location, native.result_location);
                assert_eq!(facts.journal, native.journal);
                assert_eq!(facts.files_committed, native.files_committed);
                assert_eq!(facts.bytes_committed, native.bytes_committed);
                assert_eq!(app.jobs.queue.is_running(id), !clean);
            });
            while let Ok(wire) = endpoint.try_receive_command_for_contract_test() {
                assert!(
                    !matches!(
                        wire.message,
                        Message::PlatformCommand(PlatformCommand::OpenShellTarget(_))
                    ),
                    "no automatic executable/member launch or unrequested folder opening"
                );
            }
        }
    }
}

#[gpui_kit::test]
fn ref20_sfx_post_test_uses_committed_primary_and_retains_publication(
    cx: &mut gpui_kit::TestAppContext,
) {
    post_test(cx, true);
}

#[gpui_kit::test]
fn ref20_sfx_post_test_incomplete_cleanup_keeps_original_running_slot(
    cx: &mut gpui_kit::TestAppContext,
) {
    post_test(cx, false);
}
