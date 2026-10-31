use nxtools_archive::worker::{WorkerRequest, read_frame, write_frame};
use serde_json::{Value, json};
use std::{
    io::{Read, Seek, SeekFrom, Write},
    process::{Command, Stdio},
};
use uuid::Uuid;

#[test]
fn actual_creation_worker_requires_full_partial_readback_before_any_publication() {
    use base64::Engine as _;
    for operation in ["create_gzip", "create_xz"] {
        for damage in [false, true] {
            let owned = tempfile::tempdir().expect("owned tree");
            let source = owned.path().join("source.bin");
            std::fs::write(&source, b"known independent source bytes\n").expect("bounded source");
            let mut spool = tempfile::tempfile().expect("owned spool");
            let mut child = Command::new(env!("CARGO_BIN_EXE_nxtools-archive-worker"))
                .arg("--archive-worker")
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("owned worker");
            let id = Uuid::new_v4();
            let mut request = WorkerRequest::open(id, source.to_string_lossy().into_owned());
            request.version = 2;
            request.operation = operation.into();
            request.create_name = Some("created.archive".into());
            request.compression_level = Some("normal".into());
            write_frame(child.stdin.as_mut().expect("stdin"), &request).expect("start");
            let token = Uuid::new_v4();
            let mut committed = false;
            let mut aborted = false;
            let mut readback = false;
            let terminal = loop {
                let frame: Value = read_frame(child.stdout.as_mut().expect("stdout"))
                    .expect("bounded frame")
                    .expect("terminal before EOF");
                if frame["op"] == "accepted" {
                    assert!(
                        frame["capabilities"]
                            .as_array()
                            .expect("capabilities")
                            .iter()
                            .any(|value| value == "partial_read_v1")
                    );
                }
                if frame["op"] == "failure" || frame["op"] == "complete" {
                    break frame;
                }
                if frame["op"] != "native_call" {
                    continue;
                }
                let call = &frame["native"];
                let mut reply = json!({"version":1,"operation_id":id,"op":"native_result","call_id":call["call_id"],"success":true});
                match call["action"].as_str().expect("action") {
                    "source_acquire" | "source_validated" | "source_precommit"
                    | "creation_encoding" | "creation_verifying" => {}
                    "begin" => {
                        reply["token"] = json!(token);
                    }
                    "write" => {
                        assert_eq!(call["token"], token.to_string());
                        let data: Vec<u8> = call["data"]
                            .as_array()
                            .expect("bounded bytes")
                            .iter()
                            .map(|value| value.as_u64().expect("byte") as u8)
                            .collect();
                        assert!(data.len() <= 65536);
                        spool.seek(SeekFrom::End(0)).expect("spool end");
                        spool.write_all(&data).expect("spool write");
                    }
                    "read" => {
                        readback = true;
                        assert_eq!(call["token"], token.to_string());
                        let length = call["length"].as_u64().expect("read size") as usize;
                        assert!(length <= 65536);
                        let offset = call["offset"].as_u64().expect("offset");
                        spool.seek(SeekFrom::Start(offset)).expect("spool seek");
                        let mut bytes = vec![0; length];
                        let count = spool.read(&mut bytes).expect("spool read");
                        bytes.truncate(count);
                        if damage && offset == 0 && bytes.len() > 10 {
                            bytes[10] ^= 0x80;
                        }
                        reply["read_data"] =
                            json!(base64::engine::general_purpose::STANDARD.encode(bytes));
                    }
                    "commit" => {
                        assert!(readback, "commit before full verification");
                        committed = true;
                    }
                    "abort" => {
                        aborted = true;
                    }
                    action => panic!("unexpected fixed action {action}"),
                }
                write_frame(child.stdin.as_mut().expect("stdin"), &reply).expect("ack");
            };
            assert_eq!(committed, !damage);
            assert_eq!(aborted, damage);
            assert!(readback);
            if damage {
                assert_eq!(terminal["error_code"], "CorruptArchive");
            } else {
                assert_eq!(terminal["op"], "complete");
            }
            drop(child.stdin.take());
            let result = child.wait_with_output().expect("worker cleanup");
            assert!(result.status.success());
            assert!(result.stderr.is_empty());
        }
    }
}
