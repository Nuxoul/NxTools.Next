//! Original coordinator guard; injected terminal facts, NOT native decoder overflow evidence.
use nxtools_archive::{
    OperationControl,
    interaction::{InteractionPort, Prompt, PromptResponse},
    output_readmission::{self, AttemptResult, OutputBudget, OutputShortfall, SourceBinding},
    worker::WorkerRequest,
};
use std::{
    sync::mpsc,
    time::{Duration, Instant},
};
#[test]
fn second_actual_resource_limit_facts_never_create_a_third_attempt() {
    let control = OperationControl::default();
    let port = InteractionPort::new(control.operation_id(), 1);
    let work_control = control.clone();
    let work_port = port.clone();
    let (tx, rx) = mpsc::sync_channel(1);
    let worker = std::thread::spawn(move || {
        let mut request =
            WorkerRequest::open(work_control.operation_id(), "owned-known-stream.xz".into());
        request.version = 2;
        request.operation = "extract".into();
        request.limits.max_entry_bytes = 1;
        request.limits.max_output_bytes = 8192;
        let mut attempts = 0;
        let result =
            output_readmission::execute(request, &work_control, &work_port, |request, attempt| {
                attempts += 1;
                assert!(attempts <= 2);
                assert_eq!(request.operation_id, attempt.operation_id());
                Ok(AttemptResult {
                    operation_id: request.operation_id,
                    cleanup_complete: true,
                    files_committed: 0,
                    error_code: Some("ResourceLimit".into()),
                    shortfall: if attempts == 1 {
                        Some(OutputShortfall {
                            current: OutputBudget {
                                entry_bytes: 1,
                                output_bytes: 8192,
                            },
                            required: OutputBudget {
                                entry_bytes: 4096,
                                output_bytes: 4096,
                            },
                            source: SourceBinding {
                                native: "injected-held-source".into(),
                                digest: [1; 32],
                            },
                        })
                    } else {
                        None
                    },
                })
            });
        tx.send((result, attempts)).expect("result");
    });
    let started = Instant::now();
    let pending = loop {
        if let Some(pending) = port.pending() {
            break pending;
        }
        assert!(started.elapsed() < Duration::from_secs(5));
        std::thread::sleep(Duration::from_millis(2));
    };
    assert!(matches!(pending.prompt, Prompt::OutputBudget { .. }));
    assert!(
        port.respond(
            pending.ticket,
            PromptResponse::OutputBudget {
                approved: OutputBudget {
                    entry_bytes: 4096,
                    output_bytes: 8192
                }
            }
        )
        .expect("finite approval")
    );
    let (result, attempts) = rx
        .recv_timeout(Duration::from_secs(5))
        .expect("bounded finish");
    worker.join().expect("worker");
    let result = result.expect("second terminal");
    assert_eq!(attempts, 2);
    assert_eq!(result.error_code.as_deref(), Some("ResourceLimit"));
    assert!(result.shortfall.is_none());
    assert!(port.pending().is_none());
}
