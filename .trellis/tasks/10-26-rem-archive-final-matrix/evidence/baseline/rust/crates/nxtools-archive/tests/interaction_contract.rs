use nxtools_archive::{
    ArchiveError, OperationControl,
    interaction::{InteractionPort, Prompt, PromptResponse, SecretPassword},
};
use std::{
    thread,
    time::{Duration, Instant},
};

#[test]
fn generation_scoped_password_answer_resumes_once_and_old_ticket_cannot_apply() {
    let control = OperationControl::default();
    let port = InteractionPort::new(control.operation_id(), 7);
    let worker_port = port.clone();
    let worker_control = control.clone();
    let worker = thread::spawn(move || {
        worker_port.request(Prompt::Password { retry: false }, &worker_control)
    });
    let pending = wait_prompt(&port);
    let mut stale = pending.ticket;
    stale.generation = 6;
    assert!(
        !port
            .respond(stale, PromptResponse::Password(SecretPassword::new("old")))
            .expect("old answer rejected")
    );
    assert!(
        port.respond(
            pending.ticket,
            PromptResponse::Password(SecretPassword::new("synthetic"))
        )
        .expect("current answer")
    );
    assert!(
        !port
            .respond(pending.ticket, PromptResponse::Cancel)
            .expect("duplicate answer rejected")
    );
    let response = worker
        .join()
        .expect("worker exit")
        .expect("password response");
    match response {
        PromptResponse::Password(password) => assert_eq!(password.expose(), "synthetic"),
        _ => panic!("exact response type"),
    }
}

#[test]
fn cancellation_wakes_prompt_wait_and_busy_prompt_never_overwrites_current() {
    let control = OperationControl::default();
    let port = InteractionPort::new(control.operation_id(), 1);
    let worker_port = port.clone();
    let worker_control = control.clone();
    let worker = thread::spawn(move || {
        worker_port.request(Prompt::Password { retry: true }, &worker_control)
    });
    let pending = wait_prompt(&port);
    assert!(matches!(
        port.request(
            Prompt::Risk {
                declared_bytes: 500,
                limit_bytes: 100
            },
            &control
        ),
        Err(ArchiveError::Busy)
    ));
    assert_eq!(
        port.pending().expect("original survives").ticket,
        pending.ticket
    );
    let before = Instant::now();
    control.cancel();
    assert!(matches!(
        worker.join().expect("worker exit"),
        Err(ArchiveError::Cancelled)
    ));
    assert!(before.elapsed() < Duration::from_millis(200));
    assert!(port.pending().is_none());
    assert!(
        !port
            .respond(pending.ticket, PromptResponse::Cancel)
            .expect("retired prompt")
    );
}

#[test]
fn wrong_response_type_preserves_pending_and_close_is_explicit_cancel() {
    let control = OperationControl::default();
    let port = InteractionPort::new(control.operation_id(), 1);
    let worker_port = port.clone();
    let worker_control = control.clone();
    let worker = thread::spawn(move || {
        worker_port.request(
            Prompt::Conflict {
                existing_bytes: Some(3),
                incoming_bytes: Some(5),
            },
            &worker_control,
        )
    });
    let pending = wait_prompt(&port);
    assert!(
        !port
            .respond(
                pending.ticket,
                PromptResponse::Password(SecretPassword::new("ignored"))
            )
            .expect("type mismatch rejects")
    );
    assert!(port.pending().is_some());
    port.close();
    assert!(matches!(
        worker.join().expect("worker exit"),
        Err(ArchiveError::Cancelled)
    ));
}

fn wait_prompt(port: &InteractionPort) -> nxtools_archive::interaction::PendingPrompt {
    let before = Instant::now();
    loop {
        if let Some(pending) = port.pending() {
            return pending;
        }
        assert!(before.elapsed() < Duration::from_secs(2), "prompt deadline");
        thread::sleep(Duration::from_millis(1));
    }
}

#[test]
fn risk_confirmation_accepts_only_the_original_finite_limit_and_current_ticket() {
    let control = OperationControl::default();
    let port = InteractionPort::new(control.operation_id(), 3);
    let wait_port = port.clone();
    let wait_control = control.clone();
    let worker = thread::spawn(move || {
        wait_port.request(
            Prompt::Risk {
                declared_bytes: 4096,
                limit_bytes: 8192,
            },
            &wait_control,
        )
    });
    let pending = wait_prompt(&port);
    for approved_limit in [0, 4095, 4096, 8193, u64::MAX] {
        assert!(
            !port
                .respond(pending.ticket, PromptResponse::Risk { approved_limit })
                .expect("reject budget change")
        );
        assert_eq!(
            port.pending().expect("still pending").ticket,
            pending.ticket
        );
    }
    let mut stale = pending.ticket;
    stale.generation = 2;
    assert!(
        !port
            .respond(
                stale,
                PromptResponse::Risk {
                    approved_limit: 8192
                }
            )
            .expect("stale rejected")
    );
    assert!(
        port.respond(
            pending.ticket,
            PromptResponse::Risk {
                approved_limit: 8192
            }
        )
        .expect("same finite admission")
    );
    assert!(
        !port
            .respond(
                pending.ticket,
                PromptResponse::Risk {
                    approved_limit: 8192
                }
            )
            .expect("duplicate rejected")
    );
    assert!(matches!(
        worker.join().expect("wait exits").expect("answer"),
        PromptResponse::Risk {
            approved_limit: 8192
        }
    ));
}
