use nxtools_extensions::{Invocation, InvocationError, InvocationIdentity, InvocationNotice};
use uuid::Uuid;

fn identity() -> InvocationIdentity {
    InvocationIdentity {
        run_id: Uuid::new_v4(),
        correlation_id: Uuid::new_v4(),
        generation: 7,
        package_id: "example.script".into(),
        handler_id: "run".into(),
        content_hash: "a".repeat(64),
    }
}
fn frame(identity: &InvocationIdentity, body: &str) -> Vec<u8> {
    format!(
        r#"{{"protocol_version":1,"run_id":"{}",{body}}}"#,
        identity.run_id
    )
    .into_bytes()
}

#[test]
fn real_invocation_terminal_is_once_current_and_exit_without_completion_is_failure() {
    let id = identity();
    let mut run = Invocation::new(id.clone());
    let complete = frame(
        &id,
        r#""op":"complete","status":"success","value":{"text":"done"}"#,
    );
    assert!(matches!(
        run.accept(&complete, &id),
        Ok(InvocationNotice::Completed { .. })
    ));
    assert_eq!(run.accept(&complete, &id), Err(InvocationError::Retired));
    assert!(run.process_exit(true, true).is_ok());
    let mut missing = Invocation::new(identity());
    assert_eq!(
        missing.process_exit(true, true),
        Err(InvocationError::MissingCompletion)
    );
    let mut stale = Invocation::new(id.clone());
    let mut new_identity = id.clone();
    new_identity.generation += 1;
    assert_eq!(
        stale.accept(&complete, &new_identity),
        Err(InvocationError::Stale)
    );
    stale.cancel();
    assert_eq!(stale.accept(&complete, &id), Err(InvocationError::Retired));
}

#[test]
fn host_calls_are_bounded_unique_and_duplex_replies_bind_current_run() {
    let id = identity();
    let mut run = Invocation::new(id.clone());
    for call in 0..8 {
        let call_id = format!("call-{call}");
        let bytes = frame(
            &id,
            &format!(
                r#""op":"host_call","call_id":"{call_id}","method":"host.describe","params":{{}}"#
            ),
        );
        assert!(matches!(
            run.accept(&bytes, &id),
            Ok(InvocationNotice::HostCall { .. })
        ));
    }
    let extra = frame(
        &id,
        r#""op":"host_call","call_id":"overflow","method":"host.describe","params":{}"#,
    );
    assert_eq!(run.accept(&extra, &id), Err(InvocationError::HostCallLimit));
    let mut run = Invocation::new(id.clone());
    let call = frame(
        &id,
        r#""op":"host_call","call_id":"one","method":"clipboard.copy","params":{"text":"fixture"}"#,
    );
    run.accept(&call, &id).expect("admit call");
    let reply = run
        .host_result("one", true, serde_json::json!({}), &id)
        .expect("duplex result");
    let json: serde_json::Value = serde_json::from_str(&reply).expect("reply JSON");
    assert_eq!(json["run_id"], id.run_id.to_string());
    assert_eq!(json["op"], "host_result");
    assert_eq!(
        run.host_result("one", true, serde_json::json!({}), &id),
        Err(InvocationError::UnknownCall)
    );
    assert_eq!(run.accept(&call, &id), Err(InvocationError::DuplicateCall));
    assert_eq!(
        run.host_result("one", true, serde_json::json!({}), &id),
        Err(InvocationError::Retired)
    );
}

#[test]
fn protocol_rejects_duplicate_unknown_wrong_run_and_overlong_frames() {
    let id = identity();
    for bytes in [
        frame(
            &id,
            r#""op":"complete","status":"success","value":{},"value":{}"#,
        ),
        frame(
            &id,
            r#""op":"complete","status":"success","value":{},"unknown":true"#,
        ),
        frame(
            &id,
            r#""op":"complete","status":"success","value":{"nested":{"a":1,"a":2}}"#,
        ),
        vec![b'x'; 256 * 1024 + 1],
        vec![0xc3, 0x28],
    ] {
        assert!(Invocation::new(id.clone()).accept(&bytes, &id).is_err());
    }
    let other = identity();
    assert_eq!(
        Invocation::new(id.clone()).accept(
            &frame(&other, r#""op":"complete","status":"success","value":{}"#),
            &id
        ),
        Err(InvocationError::WrongRun)
    );
    let mut unknown = Invocation::new(id.clone());
    let request = frame(
        &id,
        r#""op":"host_call","call_id":"x","method":"arbitrary.reflect","params":{}"#,
    );
    assert!(matches!(
        unknown.accept(&request, &id),
        Ok(InvocationNotice::HostCall { .. })
    ));
    let reply = unknown
        .host_result("x", false, serde_json::json!({"code":"unsupported"}), &id)
        .expect("typed unsupported result");
    assert!(reply.contains("unsupported"));
}
