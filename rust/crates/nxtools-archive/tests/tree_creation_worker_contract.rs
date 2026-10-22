//! Actual worker rendezvous regression; C# fixtures separately prove protected native handles/Job.
use nxtools_archive::worker::{read_frame, write_frame};
use serde_json::{Value, json};
use std::{
    io::{Read, Seek, SeekFrom, Write},
    process::{Command, Stdio},
};
use uuid::Uuid;

#[test]
fn tree_worker_requires_complete_readback_and_preserves_first_typed_failure() {
    use base64::Engine as _;
    for format in ["tar", "tgz", "txz", "7z"] {
        if format == "7z" && !cfg!(feature = "isolated-codecs") {
            continue;
        }
        for mode in [
            "normal",
            "readback",
            "truncated",
            "source_io",
            "source_changed",
            "write_io",
        ] {
            let mut partial = tempfile::tempfile().expect("owned bounded partial");
            let mut child = Command::new(env!("CARGO_BIN_EXE_nxtools-archive-worker"))
                .arg("--archive-worker")
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("owned worker");
            let operation = Uuid::new_v4();
            let source = Uuid::new_v4();
            let token = Uuid::new_v4();
            let mut request = json!({"schema":2,"operation_id":operation,"op":"start","operation":format!("create_{format}"),"path":"authorized-source","create_name":format!("created.{format}"),"tree_options":{"strip_single_root":false,"exclude_patterns":[]}});
            if format != "tar" {
                request["compression_level"] = json!("fast");
            }
            if format == "7z" {
                request["sevenz_options"] =
                    json!({"solid":false,"encryption":"none","encrypt_header":false});
            }
            write_frame(child.stdin.as_mut().expect("stdin"), &request).expect("start");
            let mut committed = false;
            let mut aborted = false;
            let mut verified = false;
            let mut source_open = false;
            let terminal = loop {
                let frame: Value = read_frame(child.stdout.as_mut().expect("stdout"))
                    .expect("bounded frame")
                    .expect("terminal before EOF");
                if frame["op"] == "complete" || frame["op"] == "failure" {
                    break frame;
                }
                if frame["op"] != "native_call" {
                    continue;
                }
                let call = &frame["native"];
                let mut reply = json!({"version":1,"operation_id":operation,"op":"native_result","call_id":call["call_id"],"success":true});
                match call["action"].as_str().expect("action") {
                    "source_acquire" | "source_validated" | "source_precommit" | "creation_planning" | "creation_encoding" | "creation_verifying" | "source_check" => {},
                    "source_roots" => reply["read_data"] = json!(serde_json::to_string(&json!([{"id":source,"name":"data.bin","directory":false,"size":7,"dos_modified":null}])).expect("metadata")),
                    "source_open" => { assert!(!source_open); source_open = true; },
                    "source_close" => { assert!(source_open); source_open = false; },
                    "source_read" => {
                        assert!(source_open); let offset = call["offset"].as_u64().expect("offset") as usize;
                        let length = call["length"].as_u64().expect("bounded length") as usize; assert!(length <= 65536);
                        reply["read_data"] = json!(base64::engine::general_purpose::STANDARD.encode(&b"payload"[offset..(offset + length).min(7)]));
                        if mode == "source_io" || mode == "source_changed" { reply["success"] = json!(false); reply["error_code"] = json!(if mode == "source_io" { "source_unavailable" } else { "source_changed" }); }
                    },
                    "begin" => reply["token"] = json!(token),
                    "write" => {
                        let bytes: Vec<_> = call["data"].as_array().expect("chunk").iter().map(|byte| byte.as_u64().expect("byte") as u8).collect();
                        assert!(bytes.len() <= 65536);
                        if mode == "write_io" { reply["success"] = json!(false); reply["error_code"] = json!("write_file"); }
                        else { partial.seek(SeekFrom::End(0)).expect("end"); partial.write_all(&bytes).expect("write"); }
                    },
                    "read" => {
                        verified = true; let offset = call["offset"].as_u64().expect("offset");
                        let length = call["length"].as_u64().expect("length") as usize; assert!(length <= 65536);
                        let mut bytes = vec![0; length]; partial.seek(SeekFrom::Start(offset)).expect("seek");
                        let count = partial.read(&mut bytes).expect("read"); bytes.truncate(count);
                        if mode == "readback" && offset == 0 && !bytes.is_empty() { bytes[0] ^= 1; }
                        if mode == "truncated" { bytes.clear(); }
                        reply["read_data"] = json!(base64::engine::general_purpose::STANDARD.encode(bytes));
                    },
                    "commit" => { assert!(verified && !source_open); committed = true; },
                    "abort" => aborted = true,
                    other => panic!("unexpected native action {other}"),
                }
                write_frame(child.stdin.as_mut().expect("stdin"), &reply).expect("ack");
            };
            drop(child.stdin.take());
            let result = child.wait_with_output().expect("cleanup");
            assert!(
                result.status.success() && result.stderr.is_empty(),
                "{format}/{mode}"
            );
            assert!(!source_open, "source reader must close: {format}/{mode}");
            assert_eq!(committed, mode == "normal", "{format}/{mode}");
            assert_eq!(aborted, mode != "normal", "{format}/{mode}");
            assert_eq!(
                terminal["op"],
                if mode == "normal" {
                    "complete"
                } else {
                    "failure"
                },
                "{format}/{mode}"
            );
            if mode != "normal" {
                assert_eq!(
                    terminal["error_code"],
                    match mode {
                        "readback" | "truncated" => "CorruptArchive",
                        "source_changed" => "SourceChanged",
                        _ => "IoFailure",
                    },
                    "{format}/{mode}"
                );
            }
        }
    }
}
