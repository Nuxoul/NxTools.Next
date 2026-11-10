use super::{NativeReply, ReplyPort};
use crate::{
    ArchiveError, OperationControl,
    worker_bulk::{Binding, SOURCE},
};
use uuid::Uuid;

#[test]
fn accepted_native_failure_survives_following_cancel_and_protocol_failure() {
    for protocol_failure in [false, true] {
        let operation = Uuid::new_v4();
        let control = OperationControl::with_operation_id(operation);
        let port = ReplyPort::new();
        port.configure_bulk(true);
        let call = Uuid::new_v4();
        let binding = port
            .install(
                call,
                Some(Binding {
                    kind: SOURCE,
                    operation,
                    call,
                    token: Uuid::new_v4(),
                    sequence: 0,
                    offset: 7,
                    length: 42,
                }),
            )
            .expect("install original rendezvous")
            .expect("bulk binding");
        assert!(port.deliver(NativeReply {
            call_id: call,
            success: false,
            token: Some(binding.token),
            error_code: Some("source_changed".into()),
            read_data: None,
            existing_bytes: None,
            sequence: Some(binding.sequence),
            offset: Some(binding.offset),
            length: Some(binding.length),
            binary: None,
            payload: None,
        }));
        if protocol_failure {
            port.fail(ArchiveError::IoFailure);
        }
        control.cancel();
        assert!(port.wait(call, &control).is_ok_and(
            |reply| !reply.success && reply.error_code.as_deref() == Some("source_changed")
        ));
    }
}

#[test]
fn retired_binding_cannot_deliver_late_or_duplicate_payload() {
    let operation = Uuid::new_v4();
    let port = ReplyPort::new();
    port.configure_bulk(true);
    let call = Uuid::new_v4();
    let binding = port
        .install(
            call,
            Some(Binding {
                kind: SOURCE,
                operation,
                call,
                token: Uuid::new_v4(),
                sequence: 0,
                offset: 0,
                length: 1,
            }),
        )
        .expect("install")
        .expect("binding");
    assert!(port.deliver_binary(binding, zeroize::Zeroizing::new(vec![9])));
    assert!(!port.deliver_binary(binding, zeroize::Zeroizing::new(vec![9])));
    port.retire(call);
    assert!(!port.deliver_binary(binding, zeroize::Zeroizing::new(vec![9])));
}
#[test]
fn output_call_timing_observes_the_original_ack_boundary() {
    use super::PrivateOutput;
    use std::io::Write;
    use std::sync::mpsc;
    struct Wire(mpsc::Sender<Vec<u8>>, Vec<u8>);
    impl Write for Wire {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.1.extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            self.0
                .send(std::mem::take(&mut self.1))
                .expect("wire receiver");
            Ok(())
        }
    }
    let control = OperationControl::default();
    let port = ReplyPort::new();
    port.configure_bulk(true);
    let token = Uuid::new_v4();
    port.output_token(token, true)
        .expect("register output token");
    let (send, receive) = mpsc::channel();
    let worker = std::thread::spawn({
        let control = control.clone();
        let port = port.clone();
        move || {
            let mut writer = Wire(send, Vec::new());
            let mut output = PrivateOutput::new(&mut writer, &control, port);
            output
                .call("write", None, Some(token), &[1, 2, 3])
                .expect("acknowledged native write");
        }
    });
    let frame = receive.recv().expect("original binary frame");
    let packet = crate::worker_bulk::read(&mut frame.as_slice())
        .expect("valid frame")
        .expect("binary packet");
    let binding = match packet {
        crate::worker_bulk::Packet::Binary(binding, payload) => {
            assert_eq!(&payload[..], &[1, 2, 3]);
            binding
        }
        crate::worker_bulk::Packet::Json(_) => panic!("expected bulk output"),
    };
    std::thread::sleep(std::time::Duration::from_millis(2));
    assert!(port.deliver(NativeReply {
        call_id: binding.call,
        success: true,
        token: Some(binding.token),
        error_code: None,
        read_data: None,
        existing_bytes: None,
        sequence: Some(binding.sequence),
        offset: Some(binding.offset),
        length: Some(binding.length),
        binary: None,
        payload: None,
    }));
    worker.join().expect("writer thread");
    let work = control.work_observation();
    assert_eq!(work.output_write_calls, 1);
    assert_eq!(work.output_write_bytes, 3);
    assert!(work.output_ipc_ack_us >= 1_000);
}

// Compare production extraction writes with the unchanged direct native-call boundary.
#[test]
fn bounded_output_grouping_drains_tail_before_commit_and_keeps_exact_ack_sequence() {
    use crate::output::ArchiveOutput;
    use std::{io::Write, sync::mpsc};

    struct Wire(mpsc::Sender<Vec<u8>>, Vec<u8>);
    impl Write for Wire {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.1.extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            self.0
                .send(std::mem::take(&mut self.1))
                .expect("native receiver");
            Ok(())
        }
    }
    fn trial(coalesced: bool) -> (Vec<u8>, Vec<usize>) {
        let control = OperationControl::default();
        let port = ReplyPort::new();
        port.configure_bulk(true);
        let token = Uuid::new_v4();
        // The original begin reply installs the token before any payload.
        let (send, receive) = mpsc::channel();
        let input: Vec<u8> = (0..(65_536 + 7)).map(|index| (index % 251) as u8).collect();
        let expected = input.clone();
        let worker = std::thread::spawn({
            let control = control.clone();
            let port = port.clone();
            move || {
                let mut wire = Wire(send, Vec::new());
                let mut output = super::PrivateOutput::new(&mut wire, &control, port);
                let native_token = output
                    .begin_extracted_file("member.bin", Some(input.len() as u64))
                    .expect("member admission")
                    .expect("member output");
                assert_eq!(native_token, token);
                for part in input.chunks(8_192) {
                    if coalesced {
                        output
                            .write_file(token, part)
                            .expect("production extraction write");
                    } else {
                        output
                            .call("write", None, Some(token), part)
                            .expect("direct output control");
                    }
                }
                output
                    .call("source_precommit", None, None, &[])
                    .expect("verification follows drained output");
                output
                    .commit_file(token)
                    .expect("commit after all output ACKs");
            }
        });
        let mut result = Vec::new();
        let mut sizes = Vec::new();
        let mut sequence = 0;
        while let Ok(frame) = receive.recv() {
            match crate::worker_bulk::read(&mut frame.as_slice())
                .expect("valid worker frame")
                .expect("frame")
            {
                crate::worker_bulk::Packet::Binary(binding, payload) => {
                    assert_eq!(binding.kind, crate::worker_bulk::OUTPUT);
                    assert_eq!(binding.sequence, sequence + 1);
                    assert_eq!(binding.offset, result.len() as u64);
                    assert!(receive.try_recv().is_err(), "one pending ACK only");
                    sequence += 1;
                    sizes.push(payload.len());
                    result.extend_from_slice(&payload);
                    assert!(port.deliver(NativeReply {
                        call_id: binding.call,
                        success: true,
                        token: Some(token),
                        error_code: None,
                        read_data: None,
                        existing_bytes: None,
                        sequence: Some(binding.sequence),
                        offset: Some(binding.offset),
                        length: Some(binding.length),
                        binary: None,
                        payload: None,
                    }));
                }
                crate::worker_bulk::Packet::Json(bytes) => {
                    let frame: serde_json::Value =
                        serde_json::from_slice(&bytes).expect("native call");
                    let native = &frame["native"];
                    let action = native["action"].as_str().expect("action");
                    assert!(matches!(action, "begin" | "source_precommit" | "commit"));
                    if action != "begin" {
                        assert_eq!(
                            result, expected,
                            "verification/commit must follow the tail ACK"
                        );
                    }
                    let call = Uuid::parse_str(native["call_id"].as_str().expect("call ID"))
                        .expect("UUID");
                    assert!(port.deliver(NativeReply {
                        call_id: call,
                        success: true,
                        token: (action == "begin").then_some(token),
                        error_code: None,
                        read_data: None,
                        existing_bytes: None,
                        sequence: None,
                        offset: None,
                        length: None,
                        binary: None,
                        payload: None,
                    }));
                    if action == "commit" {
                        break;
                    }
                }
            }
        }
        worker.join().expect("output worker");
        let work = control.work_observation();
        assert_eq!(work.output_write_calls as usize, sizes.len());
        assert_eq!(work.output_write_bytes, expected.len() as u64);
        (result, sizes)
    }
    let (baseline, individual) = trial(false);
    let (grouped, groups) = trial(true);
    assert_eq!(baseline, grouped);
    assert_eq!(
        individual,
        vec![8192; 8].into_iter().chain([7]).collect::<Vec<_>>()
    );
    assert_eq!(groups, vec![32_768, 32_768, 7]);
}

#[test]
fn grouped_write_failure_stops_before_commit_even_after_cancel() {
    use crate::output::ArchiveOutput;
    use std::{io::Write, sync::mpsc};
    struct Wire(mpsc::Sender<Vec<u8>>, Vec<u8>);
    impl Write for Wire {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.1.extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            self.0
                .send(std::mem::take(&mut self.1))
                .expect("native receiver");
            Ok(())
        }
    }
    let control = OperationControl::default();
    let port = ReplyPort::new();
    port.configure_bulk(true);
    let token = Uuid::new_v4();
    port.output_token(token, true).expect("begin token");
    let (send, receive) = mpsc::channel();
    let worker = std::thread::spawn({
        let control = control.clone();
        let port = port.clone();
        move || {
            let mut wire = Wire(send, Vec::new());
            let mut output = super::PrivateOutput::new(&mut wire, &control, port);
            let failure = output.write_file(token, &[3; 32_768]);
            assert_eq!(failure.err(), Some(ArchiveError::IoFailure));
            // No metadata or commit after the failed acknowledged write.
        }
    });
    let frame = receive.recv().expect("grouped output");
    let packet = crate::worker_bulk::read(&mut frame.as_slice())
        .expect("frame")
        .expect("packet");
    let binding = match packet {
        crate::worker_bulk::Packet::Binary(binding, bytes) => {
            assert_eq!(bytes.len(), 32_768);
            binding
        }
        crate::worker_bulk::Packet::Json(_) => panic!("expected grouped data"),
    };
    assert!(port.deliver(NativeReply {
        call_id: binding.call,
        success: false,
        token: Some(token),
        error_code: Some("write_file".into()),
        read_data: None,
        existing_bytes: None,
        sequence: Some(binding.sequence),
        offset: Some(binding.offset),
        length: Some(binding.length),
        binary: None,
        payload: None,
    }));
    control.cancel();
    worker.join().expect("failed writer retired");
    assert!(
        receive.try_recv().is_err(),
        "no write or commit after first failure"
    );
    let work = control.work_observation();
    assert_eq!(work.output_write_calls, 1);
    assert_eq!(work.output_write_bytes, 0);
}

struct ExtractionPeer {
    replies: ReplyPort,
    token: Uuid,
    pending: Vec<u8>,
    actions: Vec<String>,
    payload: Vec<u8>,
    fail_write: bool,
    cancel_after_failure: Option<OperationControl>,
}
impl ExtractionPeer {
    fn new(replies: ReplyPort) -> Self {
        Self {
            replies,
            token: Uuid::new_v4(),
            pending: Vec::new(),
            actions: Vec::new(),
            payload: Vec::new(),
            fail_write: false,
            cancel_after_failure: None,
        }
    }
}
impl std::io::Write for ExtractionPeer {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.pending.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        let frame = std::mem::take(&mut self.pending);
        let reply = match crate::worker_bulk::read(&mut frame.as_slice())
            .expect("original frame")
            .expect("one frame")
        {
            crate::worker_bulk::Packet::Binary(binding, bytes) => {
                assert_eq!(binding.token, self.token);
                assert_eq!(binding.offset, self.payload.len() as u64);
                self.actions.push("write".into());
                self.payload.extend_from_slice(&bytes);
                NativeReply {
                    call_id: binding.call,
                    success: !self.fail_write,
                    token: Some(binding.token),
                    error_code: self.fail_write.then(|| "write_file".into()),
                    sequence: Some(binding.sequence),
                    offset: Some(binding.offset),
                    length: Some(binding.length),
                    read_data: None,
                    existing_bytes: None,
                    binary: None,
                    payload: None,
                }
            }
            crate::worker_bulk::Packet::Json(bytes) => {
                let frame: serde_json::Value =
                    serde_json::from_slice(&bytes).expect("native frame");
                let call = &frame["native"];
                let action = call["action"].as_str().expect("native action");
                self.actions.push(action.into());
                NativeReply {
                    call_id: Uuid::parse_str(call["call_id"].as_str().expect("ID")).expect("UUID"),
                    success: true,
                    token: (action == "begin").then_some(self.token),
                    error_code: None,
                    sequence: None,
                    offset: None,
                    length: None,
                    read_data: None,
                    existing_bytes: None,
                    binary: None,
                    payload: None,
                }
            }
        };
        let failed = !reply.success;
        assert!(self.replies.deliver(reply));
        if failed && let Some(control) = &self.cancel_after_failure {
            control.cancel();
        }
        Ok(())
    }
}

#[test]
fn extracted_tail_is_drained_before_metadata_verification_and_commit() {
    use crate::output::ArchiveOutput;
    for barrier in ["file_metadata", "source_precommit", "commit"] {
        let control = OperationControl::default();
        let replies = ReplyPort::new();
        replies.configure_bulk(true);
        let mut peer = ExtractionPeer::new(replies.clone());
        let mut output =
            super::PrivateOutput::new(&mut peer, &control, replies).extraction_metadata(true);
        let token = output
            .begin_extracted_file("tail.bin", Some(13))
            .expect("begin")
            .expect("token");
        output
            .write_file(token, &[5; 13])
            .expect("buffered decoder bytes");
        assert_eq!(
            output.writer.direct().actions,
            ["begin"],
            "tail remains within its member"
        );
        if barrier == "file_metadata" {
            output
                .set_file_metadata(token, crate::metadata::OutputMetadata::default())
                .expect("metadata");
        } else if barrier == "commit" {
            output.commit_file(token).expect("commit");
        } else {
            output.source_checkpoint(barrier).expect("verification");
        }
        assert_eq!(output.writer.direct().actions, ["begin", "write", barrier]);
        assert_eq!(output.writer.direct().payload, [5; 13]);
        assert!(output.write_buffer.is_empty());
        assert!(output.write_buffer.capacity() <= 32_768);
    }
}

#[test]
fn buffered_cancel_abort_and_failed_tail_never_publish_or_replay() {
    use crate::output::ArchiveOutput;
    for mode in ["cancel", "abort", "write_failure"] {
        let control = OperationControl::default();
        let replies = ReplyPort::new();
        replies.configure_bulk(true);
        let mut peer = ExtractionPeer::new(replies.clone());
        if mode == "write_failure" {
            peer.fail_write = true;
            peer.cancel_after_failure = Some(control.clone());
        }
        {
            let mut output = super::PrivateOutput::new(&mut peer, &control, replies);
            let token = output
                .begin_extracted_file("tail.bin", Some(13))
                .expect("begin")
                .expect("token");
            output.write_file(token, &[9; 13]).expect("buffered tail");
            if mode == "cancel" {
                control.cancel();
            }
            if mode == "abort" {
                output.abort_file(token);
            } else {
                let expected = if mode == "cancel" {
                    ArchiveError::Cancelled
                } else {
                    ArchiveError::IoFailure
                };
                assert_eq!(output.commit_file(token).err(), Some(expected));
                assert_eq!(
                    output.commit_file(token).err(),
                    Some(expected),
                    "first failure retained, never retry pending bytes"
                );
            }
        }
        let expected: &[&str] = match mode {
            "cancel" => &["begin"],
            "abort" => &["begin", "abort"],
            _ => &["begin", "write"],
        };
        assert_eq!(peer.actions, expected);
        assert_eq!(control.work_observation().output_write_bytes, 0);
    }
}

struct MultiOutputPeer {
    replies: ReplyPort,
    tokens: [Uuid; 2],
    begun: usize,
    pending: Vec<u8>,
    writes: Vec<(Uuid, Vec<u8>)>,
    aborted: Vec<Uuid>,
    reject_second_write: bool,
}
impl std::io::Write for MultiOutputPeer {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.pending.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        let frame = std::mem::take(&mut self.pending);
        let reply = match crate::worker_bulk::read(&mut frame.as_slice())
            .expect("original frame")
            .expect("one frame")
        {
            crate::worker_bulk::Packet::Binary(binding, bytes) => {
                assert_eq!(binding.kind, crate::worker_bulk::OUTPUT);
                assert_eq!(binding.sequence, self.writes.len() as u64 + 1);
                assert_eq!(binding.offset, 0, "first write for each member");
                assert_eq!(binding.token, self.tokens[self.begun - 1]);
                let failed = self.reject_second_write && self.begun == 2;
                self.writes.push((binding.token, bytes.to_vec()));
                NativeReply {
                    call_id: binding.call,
                    success: !failed,
                    token: Some(binding.token),
                    error_code: failed.then(|| "write_file".into()),
                    sequence: Some(binding.sequence),
                    offset: Some(binding.offset),
                    length: Some(binding.length),
                    read_data: None,
                    existing_bytes: None,
                    binary: None,
                    payload: None,
                }
            }
            crate::worker_bulk::Packet::Json(bytes) => {
                let frame: serde_json::Value =
                    serde_json::from_slice(&bytes).expect("native frame");
                let call = &frame["native"];
                let action = call["action"].as_str().expect("action");
                let token = match action {
                    "begin" => {
                        let token = self.tokens[self.begun];
                        self.begun += 1;
                        Some(token)
                    }
                    "abort" => {
                        self.aborted.push(
                            Uuid::parse_str(call["token"].as_str().expect("abort token"))
                                .expect("UUID"),
                        );
                        None
                    }
                    _ => panic!("unexpected publication or replay: {action}"),
                };
                NativeReply {
                    call_id: Uuid::parse_str(call["call_id"].as_str().expect("ID")).expect("UUID"),
                    success: true,
                    token,
                    error_code: None,
                    sequence: None,
                    offset: None,
                    length: None,
                    read_data: None,
                    existing_bytes: None,
                    binary: None,
                    payload: None,
                }
            }
        };
        assert!(self.replies.deliver(reply), "original matching ACK");
        Ok(())
    }
}

fn multi_output_cleanup(reject_second_write: bool, fail_before_cleanup: bool) -> MultiOutputPeer {
    use crate::output::ArchiveOutput;
    let control = OperationControl::default();
    let replies = ReplyPort::new();
    replies.configure_bulk(true);
    let tokens = [Uuid::new_v4(), Uuid::new_v4()];
    let mut peer = MultiOutputPeer {
        replies: replies.clone(),
        tokens,
        begun: 0,
        pending: Vec::new(),
        writes: Vec::new(),
        aborted: Vec::new(),
        reject_second_write,
    };
    {
        let mut output = super::PrivateOutput::new(&mut peer, &control, replies);
        assert_eq!(
            output
                .begin_extracted_file("first.bin", Some(3))
                .expect("first begin"),
            Some(tokens[0])
        );
        output.write_file(tokens[0], &[3; 3]).expect("first member");
        assert_eq!(
            output
                .begin_extracted_file("second.bin", Some(32_768))
                .expect("second begin"),
            Some(tokens[1])
        );
        // The successful next-member barrier drains A, but neither member is committed yet.
        assert_eq!(output.writer.direct().writes, [(tokens[0], vec![3; 3])]);
        if fail_before_cleanup {
            assert_eq!(
                output.write_file(tokens[1], &[7; 32_768]).err(),
                Some(ArchiveError::IoFailure)
            );
        } else {
            output
                .write_file(tokens[1], &[7; 13])
                .expect("pending tail");
        }
        // run_sevenz aborts all staged tokens in creation order after decode/integrity failure.
        for token in tokens {
            output.abort_file(token);
        }
        if fail_before_cleanup {
            assert_eq!(
                output.commit_file(tokens[1]).err(),
                Some(ArchiveError::IoFailure),
                "successful aborts must not clear the first write failure"
            );
        }
    }
    peer
}

#[test]
fn cross_member_abort_discards_unverified_tail_without_new_write() {
    let peer = multi_output_cleanup(false, false);
    assert_eq!(peer.writes, [(peer.tokens[0], vec![3; 3])]);
    assert_eq!(peer.aborted, peer.tokens);
}

#[test]
fn cross_member_abort_write_failure_does_not_skip_earlier_abort() {
    let peer = multi_output_cleanup(true, false);
    assert_eq!(peer.aborted, peer.tokens, "both staged tokens reach abort");
    assert_eq!(peer.writes, [(peer.tokens[0], vec![3; 3])]);
}

#[test]
fn cross_member_abort_preserves_sticky_failure_without_suppressing_cleanup() {
    let peer = multi_output_cleanup(true, true);
    assert_eq!(
        peer.aborted, peer.tokens,
        "sticky failure cannot suppress abort"
    );
    assert_eq!(
        peer.writes,
        [
            (peer.tokens[0], vec![3; 3]),
            (peer.tokens[1], vec![7; 32_768])
        ],
        "no replay during abort or after the original failed write"
    );
}

fn control_reply(call_id: Uuid) -> NativeReply {
    NativeReply {
        call_id,
        success: true,
        token: None,
        error_code: None,
        read_data: None,
        existing_bytes: None,
        sequence: None,
        offset: None,
        length: None,
        binary: None,
        payload: None,
    }
}

#[test]
fn parallel_rendezvous_has_32_slots_routes_reverse_replies_and_rejects_stale_duplicates() {
    let port = ReplyPort::new();
    port.configure_parallel(true);
    let control = OperationControl::default();
    let calls = (0..32).map(|_| Uuid::new_v4()).collect::<Vec<_>>();
    for call in &calls {
        assert!(port.install(*call, None).is_ok());
    }
    assert_eq!(
        port.install(Uuid::new_v4(), None).err(),
        Some(ArchiveError::Busy)
    );
    for call in calls.iter().rev() {
        assert!(port.deliver(control_reply(*call)));
        assert!(!port.deliver(control_reply(*call)));
    }
    for call in calls {
        assert_eq!(
            port.wait(call, &control).expect("routed reply").call_id,
            call
        );
        assert!(!port.deliver(control_reply(call)));
    }
    assert!(port.0.state.lock().expect("state").pending.is_empty());
}

#[test]
fn parallel_reply_failure_wakes_and_joins_every_waiter_without_replacing_first_error() {
    let port = ReplyPort::new();
    port.configure_parallel(true);
    let control = OperationControl::default();
    let calls = (0..32).map(|_| Uuid::new_v4()).collect::<Vec<_>>();
    for call in &calls {
        port.install(*call, None).expect("slot");
    }
    std::thread::scope(|scope| {
        let threads = calls
            .iter()
            .map(|call| {
                let port = port.clone();
                let control = control.clone();
                scope.spawn(move || port.wait(*call, &control).err())
            })
            .collect::<Vec<_>>();
        port.fail(ArchiveError::SourceChanged);
        port.fail(ArchiveError::IoFailure);
        control.cancel();
        for thread in threads {
            assert_eq!(
                thread.join().expect("joined waiter"),
                Some(ArchiveError::SourceChanged)
            );
        }
    });
    assert!(port.0.state.lock().expect("state").pending.is_empty());
}
