use nxtools_extensions::{
    InvocationError, InvocationIdentity, InvocationNotice, InvocationSession, SessionNotice,
};
use serde_json::{Value, json};
use uuid::Uuid;
fn identity() -> InvocationIdentity {
    InvocationIdentity {
        run_id: Uuid::new_v4(),
        correlation_id: Uuid::new_v4(),
        generation: 1,
        package_id: "fixture.session".into(),
        handler_id: "run".into(),
        content_hash: "a".repeat(64),
    }
}
fn frame(session: Uuid, id: &InvocationIdentity, op: &str) -> Value {
    json!({"protocol_version":2,"op":op,"session_id":session,"run_id":id.run_id})
}
fn active(id: &InvocationIdentity) -> InvocationSession {
    let mut session = InvocationSession::new(id.clone());
    let init: Value =
        serde_json::from_str(&session.initialize_frame().expect("init")).expect("JSON");
    assert_eq!(init["protocol_version"], 2);
    assert_eq!(init["limits"]["frame_bytes"], 262144);
    assert_eq!(
        session.accept(
            &serde_json::to_vec(
                &json!({"protocol_version":2,"op":"ready","session_id":session.session_id()})
            )
            .expect("ready"),
            id
        ),
        Ok(SessionNotice::Ready)
    );
    let invoke: Value = serde_json::from_str(
        &session
            .invoke_frame("tools", json!({}), json!({}), json!({}))
            .expect("invoke"),
    )
    .expect("JSON");
    assert_eq!(invoke["run_id"], id.run_id.to_string());
    assert_eq!(invoke["correlation_id"], id.correlation_id.to_string());
    session
}
#[test]
fn expansion_session_completion_requires_finished_then_actual_native_cleanup() {
    for cleanup in [true, false] {
        let id = identity();
        let mut session = active(&id);
        let mut complete = frame(session.session_id(), &id, "complete");
        complete["status"] = json!("success");
        complete["value"] = json!({});
        assert!(matches!(
            session.accept(&serde_json::to_vec(&complete).expect("complete"), &id),
            Ok(SessionNotice::Invocation(
                InvocationNotice::Completed { .. }
            ))
        ));
        let finished = frame(session.session_id(), &id, "request_finished");
        assert_eq!(
            session.accept(&serde_json::to_vec(&finished).expect("finished"), &id),
            Ok(SessionNotice::RequestFinished)
        );
        assert_eq!(
            session.accept(&serde_json::to_vec(&finished).expect("duplicate"), &id),
            Err(InvocationError::Retired)
        );
        assert_eq!(
            session.process_exit(true, cleanup),
            if cleanup {
                Ok(())
            } else {
                Err(InvocationError::ProcessFailure)
            }
        );
    }
    let id = identity();
    let mut session = active(&id);
    assert_eq!(
        session.process_exit(true, true),
        Err(InvocationError::MissingCompletion)
    );
}
#[test]
fn expansion_session_strict_identity_and_duplicate_handshake_fail_closed() {
    let id = identity();
    for invalid in [
        json!({"protocol_version":2,"op":"ready","session_id":Uuid::new_v4()}),
        json!({"protocol_version":1,"op":"ready","session_id":Uuid::new_v4()}),
        json!({"protocol_version":2,"op":"ready","session_id":Uuid::new_v4(),"ignored":true}),
    ] {
        let mut session = InvocationSession::new(id.clone());
        session.initialize_frame().expect("init");
        assert!(
            session
                .accept(&serde_json::to_vec(&invalid).expect("invalid"), &id)
                .is_err()
        );
        assert_eq!(session.accept(b"{}", &id), Err(InvocationError::Retired));
    }
    let mut session = active(&id);
    let duplicate = json!({"protocol_version":2,"op":"ready","session_id":session.session_id()});
    assert!(
        session
            .accept(&serde_json::to_vec(&duplicate).expect("duplicate"), &id)
            .is_err()
    );
    let mut session = active(&id);
    let mut other = id.clone();
    other.generation += 1;
    assert_eq!(session.accept(b"{}", &other), Err(InvocationError::Stale));
    session.cancel();
    assert_eq!(session.accept(b"{}", &id), Err(InvocationError::Retired));
}
#[test]
fn expansion_session_unknown_duplicate_keys_wrong_run_and_early_complete_are_rejected() {
    let id = identity();
    for mode in 0..5 {
        let mut session = active(&id);
        let bytes=match mode {
            0=>format!(r#"{{"protocol_version":2,"op":"complete","session_id":"{}","run_id":"{}","status":"success","value":{{}},"value":{{}}}}"#,session.session_id(),id.run_id).into_bytes(),
            1=>serde_json::to_vec(&frame(session.session_id(),&identity(),"request_finished")).expect("wrong"),
            2=>vec![b'x';262144],
            3=>serde_json::to_vec(&json!({"protocol_version":2,"op":"host_call","session_id":session.session_id(),"run_id":id.run_id,"call_id":"x","method":"fixture.echo","params":{},"ignored":true})).expect("unknown"),
            _=>vec![0xc3,0x28],
        };
        assert!(session.accept(&bytes, &id).is_err());
        assert_eq!(session.accept(b"{}", &id), Err(InvocationError::Retired));
    }
    let mut session = active(&id);
    let call = json!({"protocol_version":2,"op":"host_call","session_id":session.session_id(),"run_id":id.run_id,"call_id":"pending","method":"fixture.echo","params":{}});
    session
        .accept(&serde_json::to_vec(&call).expect("call"), &id)
        .expect("call admitted");
    let complete = json!({"protocol_version":2,"op":"complete","session_id":session.session_id(),"run_id":id.run_id,"status":"success","value":{}});
    assert!(
        session
            .accept(&serde_json::to_vec(&complete).expect("complete"), &id)
            .is_err(),
        "pending host call cannot be discarded by completion"
    );
}
#[test]
fn expansion_session_duplex_reply_keeps_session_and_run_and_rejects_duplicate_call() {
    let id = identity();
    let mut session = active(&id);
    let call = json!({"protocol_version":2,"op":"host_call","session_id":session.session_id(),"run_id":id.run_id,"call_id":"echo","method":"fixture.echo","params":{}});
    let bytes = serde_json::to_vec(&call).expect("call");
    session.accept(&bytes, &id).expect("call");
    let reply: Value = serde_json::from_str(
        &session
            .host_result("echo", true, json!({"result":1}), &id)
            .expect("reply"),
    )
    .expect("JSON");
    assert_eq!(reply["protocol_version"], 2);
    assert_eq!(reply["session_id"], session.session_id().to_string());
    assert_eq!(reply["run_id"], id.run_id.to_string());
    assert_eq!(
        session.accept(&bytes, &id),
        Err(InvocationError::DuplicateCall)
    );
}

fn host_call(
    session: &InvocationSession,
    id: &InvocationIdentity,
    index: usize,
    payload: usize,
) -> Vec<u8> {
    serde_json::to_vec(&json!({"protocol_version":2,"op":"host_call","session_id":session.session_id(),"run_id":id.run_id,
        "call_id":format!("call-{index}"),"method":"fixture.echo","params":{"padding":"x".repeat(payload)}})).expect("call frame")
}

#[test]
fn expansion_session_exact_frame_includes_newline_and_future_data_frames_are_not_exempt() {
    let id = identity();
    for extra in [0, 1] {
        let mut session = active(&id);
        let base = host_call(&session, &id, 0, 0).len();
        let bytes = host_call(&session, &id, 0, 262144 - 1 - base + extra);
        assert_eq!(bytes.len() + 1, 262144 + extra);
        assert_eq!(session.accept(&bytes, &id).is_ok(), extra == 0);
    }
    for op in ["progress", "data", "data_ack", "blob_chunk"] {
        let mut session = active(&id);
        assert!(
            session
                .accept(
                    &serde_json::to_vec(&frame(session.session_id(), &id, op))
                        .expect("future frame"),
                    &id
                )
                .is_err()
        );
    }
}

#[test]
fn expansion_session_active_and_total_host_call_limits_survive_v2_wrapping() {
    let id = identity();
    let mut session = active(&id);
    for index in 0..8 {
        session
            .accept(&host_call(&session, &id, index, 0), &id)
            .expect("eight active");
    }
    assert_eq!(
        session.accept(&host_call(&session, &id, 8, 0), &id),
        Err(InvocationError::HostCallLimit)
    );
    let mut session = active(&id);
    for index in 0..64 {
        session
            .accept(&host_call(&session, &id, index, 0), &id)
            .expect("bounded call");
        session
            .host_result(&format!("call-{index}"), true, json!({}), &id)
            .expect("bounded result");
    }
    assert_eq!(
        session.accept(&host_call(&session, &id, 64, 0), &id),
        Err(InvocationError::HostCallLimit)
    );
}

#[test]
fn expansion_session_ordinary_output_budget_counts_exact_wire_bytes_not_normalized_payload() {
    let id = identity();
    let mut session = InvocationSession::new(id.clone());
    session.initialize_frame().expect("init");
    let ready = serde_json::to_vec(
        &json!({"protocol_version":2,"op":"ready","session_id":session.session_id()}),
    )
    .expect("ready");
    session.accept(&ready, &id).expect("ready accepted");
    session
        .invoke_frame("tools", json!({}), json!({}), json!({}))
        .expect("invoke");
    let mut used = ready.len() + 1;
    for index in 0..16 {
        let target = (4 * 1024 * 1024 - used).min(262144);
        let base = host_call(&session, &id, index, 0).len() + 1;
        let bytes = host_call(&session, &id, index, target - base);
        used += bytes.len() + 1;
        session
            .accept(&bytes, &id)
            .expect("within exact output budget");
        session
            .host_result(&format!("call-{index}"), true, json!({}), &id)
            .expect("result");
    }
    assert_eq!(used, 4 * 1024 * 1024);
    assert_eq!(
        session.accept(&host_call(&session, &id, 16, 0), &id),
        Err(InvocationError::InvalidFrame)
    );
}

#[test]
fn expansion_session_input_budget_includes_handshake_invoke_and_each_v2_reply() {
    let id = identity();
    let mut session = InvocationSession::new(id.clone());
    let mut used = session.initialize_frame().expect("init").len() + 1;
    session
        .accept(
            &serde_json::to_vec(
                &json!({"protocol_version":2,"op":"ready","session_id":session.session_id()}),
            )
            .expect("ready"),
            &id,
        )
        .expect("ready accepted");
    used += session
        .invoke_frame("tools", json!({}), json!({}), json!({}))
        .expect("invoke")
        .len()
        + 1;
    for index in 0..4 {
        session
            .accept(&host_call(&session, &id, index, 0), &id)
            .expect("call");
        let call_id = format!("call-{index}");
        let base = serde_json::to_string(&json!({"protocol_version":2,"op":"host_result","session_id":session.session_id(),"run_id":id.run_id,"call_id":call_id,"success":true,"value":{"padding":""}})).expect("reply base").len() + 1;
        let target = (1024 * 1024 - used).min(262144);
        let reply = session
            .host_result(
                &call_id,
                true,
                json!({"padding":"x".repeat(target-base)}),
                &id,
            )
            .expect("within exact input budget");
        assert_eq!(reply.len() + 1, target);
        used += reply.len() + 1;
    }
    assert_eq!(used, 1024 * 1024);
    session
        .accept(&host_call(&session, &id, 4, 0), &id)
        .expect("next call");
    assert_eq!(
        session.host_result("call-4", true, json!({}), &id),
        Err(InvocationError::InvalidFrame)
    );
    let mut complete = frame(session.session_id(), &id, "complete");
    complete["status"] = json!("success");
    complete["value"] = json!({});
    assert_eq!(
        session.accept(
            &serde_json::to_vec(&complete).expect("completion after failed send"),
            &id
        ),
        Err(InvocationError::Retired),
        "a reply that could not be sent must retire the session"
    );
}

#[test]
fn expansion_session_registered_data_plane_preserves_identity_and_one_ack_per_chunk() {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    use nxtools_extensions::blobs::BlobChunk;
    let id = identity();
    let mut session = active(&id);
    let upload = Uuid::new_v4();
    session
        .register_upload(upload, 65536)
        .expect("host upload registration");
    let raw=serde_json::to_vec(&json!({"protocol_version":2,"op":"data","session_id":session.session_id(),"run_id":id.run_id,"transfer_id":upload,"sequence":1,"offset":0,"data":STANDARD.encode(vec![7;65536])})).expect("data");
    assert!(matches!(
        session.accept(&raw, &id),
        Ok(SessionNotice::Data(_))
    ));
    let ack: Value =
        serde_json::from_str(&session.data_ack_frame(upload, 1, 65536).expect("host ack"))
            .expect("ack JSON");
    assert_eq!(ack["op"], "data_ack");
    let download = Uuid::new_v4();
    session.register_download(download, 0, 3).expect("download");
    let chunk = BlobChunk {
        transfer_id: download,
        sequence: 1,
        offset: 0,
        data: STANDARD.encode(b"abc"),
    };
    let sent: Value =
        serde_json::from_str(&session.data_frame(&chunk).expect("data frame")).expect("JSON");
    assert_eq!(sent["session_id"], session.session_id().to_string());
    assert!(
        session.data_frame(&chunk).is_err(),
        "no second block before ACK"
    );
    let mut session = active(&id);
    session.register_download(download, 0, 3).expect("download");
    session.data_frame(&chunk).expect("one block");
    let ack=serde_json::to_vec(&json!({"protocol_version":2,"op":"data_ack","session_id":session.session_id(),"run_id":id.run_id,"transfer_id":download,"sequence":1,"offset":3})).expect("ACK");
    assert!(matches!(
        session.accept(&ack, &id),
        Ok(SessionNotice::DataAck(_))
    ));
    assert!(
        session.accept(&ack, &id).is_err(),
        "duplicate ACK fails closed"
    );
}

#[test]
fn expansion_progress_session_known_unknown_sequence_limits_and_terminal_are_strict() {
    let id = identity();
    let mut session = active(&id);
    for sequence in 1..=256 {
        let mut value = frame(session.session_id(), &id, "progress");
        value["sequence"] = json!(sequence);
        if sequence % 2 == 0 {
            value["completed"] = json!(sequence);
            value["total"] = json!(256);
        }
        value["message"] = json!("插件报告进度");
        session
            .accept(&serde_json::to_vec(&value).expect("progress JSON"), &id)
            .expect("valid progress including coalesced flood");
    }
    let mut overflow = frame(session.session_id(), &id, "progress");
    overflow["sequence"] = json!(257);
    assert!(
        session
            .accept(&serde_json::to_vec(&overflow).expect("overflow"), &id)
            .is_err()
    );
    for extra in [
        json!({"sequence":0}),
        json!({"sequence":1,"completed":1}),
        json!({"sequence":1,"completed":2,"total":1}),
        json!({"sequence":1,"completed":0,"total":0}),
        json!({"sequence":1,"completed":0,"total":9007199254740992u64}),
        json!({"sequence":1,"completed":0.5,"total":1}),
        json!({"sequence":1,"message":"x".repeat(513)}),
        json!({"sequence":1,"message":"\u{1b}"}),
        json!({"sequence":1,"unknown":true}),
    ] {
        let mut session = active(&id);
        let mut value = frame(session.session_id(), &id, "progress");
        value
            .as_object_mut()
            .expect("object")
            .extend(extra.as_object().expect("fields").clone());
        assert!(
            session
                .accept(&serde_json::to_vec(&value).expect("invalid"), &id)
                .is_err(),
            "{extra}"
        );
    }
    let mut session = active(&id);
    let mut value = frame(session.session_id(), &id, "progress");
    value["sequence"] = json!(1);
    let raw = serde_json::to_vec(&value).expect("sequence");
    session.accept(&raw, &id).expect("first");
    assert!(
        session.accept(&raw, &id).is_err(),
        "duplicate progress fails closed"
    );
}

#[test]
fn expansion_progress_after_complete_and_foreign_session_never_revives_run() {
    let id = identity();
    for mode in ["complete", "cancel", "foreign"] {
        let mut session = active(&id);
        let mut progress = frame(session.session_id(), &id, "progress");
        progress["sequence"] = json!(1);
        if mode == "complete" {
            let mut complete = frame(session.session_id(), &id, "complete");
            complete["status"] = json!("success");
            complete["value"] = json!({});
            session
                .accept(&serde_json::to_vec(&complete).expect("complete"), &id)
                .expect("complete");
        } else if mode == "cancel" {
            session.cancel();
        } else {
            progress["session_id"] = json!(Uuid::new_v4());
        }
        assert!(
            session
                .accept(&serde_json::to_vec(&progress).expect("progress"), &id)
                .is_err()
        );
    }
}
