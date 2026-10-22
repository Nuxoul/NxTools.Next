use nxtools_archive::{
    ArchiveError, OperationControl,
    interaction::{InteractionPort, Prompt, PromptResponse},
    output_readmission::{self, AttemptResult, OutputBudget, OutputShortfall, SourceBinding},
    worker::WorkerRequest,
};
use std::{sync::mpsc, time::Duration};

fn request(control: &OperationControl) -> WorkerRequest {
    let mut request = WorkerRequest::open(control.operation_id(), "synthetic.zip".into());
    request.version = 2;
    request.operation = "extract".into();
    request.limits.max_entry_bytes = 1024;
    request.limits.max_output_bytes = 8192;
    request
}
fn shortage(id: uuid::Uuid) -> AttemptResult {
    AttemptResult {
        operation_id: id,
        cleanup_complete: true,
        files_committed: 0,
        error_code: Some("ResourceLimit".into()),
        shortfall: Some(OutputShortfall {
            current: OutputBudget {
                entry_bytes: 1024,
                output_bytes: 8192,
            },
            required: OutputBudget {
                entry_bytes: 4096,
                output_bytes: 4096,
            },
            source: SourceBinding {
                native: "synthetic-held-source".into(),
                digest: [1; 32],
            },
        }),
    }
}
#[test]
fn incomplete_cleanup_commits_mismatched_result_and_hard_shortage_never_prompt() {
    for mode in [
        "cleanup",
        "committed",
        "identity",
        "current",
        "category",
        "hard_cap",
    ] {
        let control = OperationControl::default();
        let port = InteractionPort::new(control.operation_id(), 1);
        let mut calls = 0;
        let result =
            output_readmission::execute(request(&control), &control, &port, |request, _| {
                calls += 1;
                assert_eq!(calls, 1);
                let mut result = shortage(request.operation_id);
                match mode {
                    "cleanup" => result.cleanup_complete = false,
                    "committed" => result.files_committed = 1,
                    "identity" => result.operation_id = uuid::Uuid::new_v4(),
                    "current" => {
                        result
                            .shortfall
                            .as_mut()
                            .expect("shortfall")
                            .current
                            .entry_bytes = 2048
                    }
                    "category" => result.error_code = Some("CorruptArchive".into()),
                    "hard_cap" => {
                        result
                            .shortfall
                            .as_mut()
                            .expect("shortfall")
                            .required
                            .entry_bytes = output_readmission::MAX_ENTRY_BYTES + 1
                    }
                    _ => unreachable!(),
                }
                Ok(result)
            });
        assert!(matches!(
            result,
            Err(ArchiveError::IoFailure | ArchiveError::ResourceLimit)
        ));
        assert_eq!(calls, 1);
        assert!(port.pending().is_none());
    }
}
#[test]
fn ordinary_actual_failure_never_becomes_a_budget_prompt() {
    let control = OperationControl::default();
    let port = InteractionPort::new(control.operation_id(), 1);
    let result = output_readmission::execute(request(&control), &control, &port, |request, _| {
        let mut result = shortage(request.operation_id);
        result.shortfall = None;
        Ok(result)
    })
    .expect("original terminal");
    assert_eq!(result.error_code.as_deref(), Some("ResourceLimit"));
    assert!(port.pending().is_none());
}
#[test]
fn finite_approval_validation_retirement_and_shared_cancel_preserve_one_new_attempt() {
    let control = OperationControl::default();
    let port = InteractionPort::new(control.operation_id(), 4);
    let worker_control = control.clone();
    let worker_port = port.clone();
    let (done_tx, done_rx) = mpsc::sync_channel(1);
    let thread = std::thread::spawn(move || {
        let mut calls = 0;
        let original = worker_control.operation_id();
        let result = output_readmission::execute(
            request(&worker_control),
            &worker_control,
            &worker_port,
            |request, attempt| {
                calls += 1;
                assert_eq!(request.operation_id, attempt.operation_id());
                if calls == 1 {
                    return Ok(shortage(request.operation_id));
                }
                assert_eq!(calls, 2);
                assert_ne!(request.operation_id, original);
                let binding = request.output_readmission.as_ref().expect("bound approval");
                assert_eq!(binding.logical_id, original);
                assert_eq!(
                    binding.expected_source.as_ref().expect("source").digest,
                    [1; 32]
                );
                worker_control.cancel();
                assert!(matches!(attempt.checkpoint(), Err(ArchiveError::Cancelled)));
                let mut result = shortage(request.operation_id);
                result.error_code = Some("Cancelled".into());
                result.shortfall = None;
                Ok(result)
            },
        );
        done_tx.send((result, calls)).expect("bounded report");
    });
    let start = std::time::Instant::now();
    let pending = loop {
        if let Some(pending) = port.pending() {
            break pending;
        }
        assert!(start.elapsed() < Duration::from_secs(5));
        std::thread::sleep(Duration::from_millis(2));
    };
    assert!(matches!(pending.prompt, Prompt::OutputBudget { .. }));
    for approved in [
        OutputBudget {
            entry_bytes: 0,
            output_bytes: 8192,
        },
        OutputBudget {
            entry_bytes: 2048,
            output_bytes: 8192,
        },
        OutputBudget {
            entry_bytes: 4096,
            output_bytes: 4096,
        },
        OutputBudget {
            entry_bytes: u64::MAX,
            output_bytes: u64::MAX,
        },
        OutputBudget {
            entry_bytes: output_readmission::MAX_ENTRY_BYTES + 1,
            output_bytes: 8192,
        },
        OutputBudget {
            entry_bytes: 4096,
            output_bytes: output_readmission::MAX_OUTPUT_BYTES + 1,
        },
    ] {
        assert!(
            !port
                .respond(pending.ticket, PromptResponse::OutputBudget { approved })
                .expect("invalid approval ignored")
        );
        assert!(port.pending().is_some());
    }
    let approved = OutputBudget {
        entry_bytes: 4096,
        output_bytes: 8192,
    };
    let mut old = pending.ticket;
    old.generation -= 1;
    assert!(
        !port
            .respond(old, PromptResponse::OutputBudget { approved })
            .expect("old generation")
    );
    assert!(
        port.respond(pending.ticket, PromptResponse::OutputBudget { approved })
            .expect("valid approval")
    );
    assert!(
        !port
            .respond(pending.ticket, PromptResponse::OutputBudget { approved })
            .expect("duplicate")
    );
    let (result, calls) = done_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("terminates");
    thread.join().expect("worker");
    assert_eq!(calls, 2);
    assert_eq!(
        result.expect("terminal").error_code.as_deref(),
        Some("Cancelled")
    );
    assert!(port.pending().is_none());
}
