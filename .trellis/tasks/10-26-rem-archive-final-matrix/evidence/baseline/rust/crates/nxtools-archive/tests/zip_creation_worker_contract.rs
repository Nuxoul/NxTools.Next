use nxtools_archive::worker::{read_frame, write_frame};
use serde_json::{Value, json};
use std::{
    io::{Read, Seek, SeekFrom, Write},
    process::{Command, Stdio},
};
use uuid::Uuid;

#[test]
fn actual_zip_creation_worker_verifies_native_source_and_partial_before_publication() {
    use base64::Engine as _;
    for directory in [false, true] {
        for mode in ["normal", "readback", "finish", "source_io", "zip64"] {
            if directory && matches!(mode, "source_io" | "zip64") {
                continue;
            }
            let damage = mode != "normal";
            let mut spool = tempfile::tempfile().expect("bounded native partial stand-in");
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
            let request = json!({"schema":2,"operation_id":operation,"op":"start","operation":"create_zip","path":"authorized-source","create_name":"created.zip","compression_level":"normal","delete_source":true,"zip_options":{"method":"deflate","encryption":"none","strip_single_root":false,"exclude_patterns":[]}});
            write_frame(child.stdin.as_mut().expect("stdin"), &request).expect("start ZIP");
            let mut committed = false;
            let mut aborted = false;
            let mut verified = false;
            let terminal = loop {
                let frame: Value = read_frame(child.stdout.as_mut().expect("stdout"))
                    .expect("ZIP frame")
                    .expect("ZIP creation accepted, not EOF");
                if frame["op"] == "complete" || frame["op"] == "failure" {
                    break frame;
                }
                if frame["op"] != "native_call" {
                    continue;
                }
                let call = &frame["native"];
                let mut reply = json!({"version":1,"operation_id":operation,"op":"native_result","call_id":call["call_id"],"success":true});
                match call["action"].as_str().expect("action") {
                "source_acquire" | "source_validated" | "source_precommit" | "creation_planning" | "creation_encoding" | "creation_verifying" | "source_open" | "source_close" | "source_check" => {},
                "source_roots" => reply["read_data"] = json!(serde_json::to_string(&json!([{"id":source,"name":if directory {"空目录"} else {"资料.bin"},"directory":directory,"size":if mode=="zip64" {u32::MAX as u64+1}else if directory {0}else{7},"dos_modified":null}])).expect("metadata")),
                "source_next" => reply["read_data"] = json!("null"),
                "source_read" => {
                    let offset = call["offset"].as_u64().expect("source offset") as usize;
                    let length = call["length"].as_u64().expect("length") as usize;
                    let bytes = if mode=="zip64" { &b""[..] } else { &b"payload"[offset..(offset+length).min(7)] };
                    reply["read_data"] = json!(base64::engine::general_purpose::STANDARD.encode(bytes));
                    if mode == "source_io" { reply["success"] = json!(false); reply["error_code"] = json!("source_unavailable"); }
                },
                "begin" => reply["token"] = json!(token),
                "write" => {
                    let bytes: Vec<_> = call["data"].as_array().expect("data").iter().map(|b| b.as_u64().expect("byte") as u8).collect();
                    if mode == "finish" && bytes.starts_with(b"PK\x01\x02") { reply["success"] = json!(false); reply["error_code"] = json!("write_file"); }
                    else { spool.seek(SeekFrom::End(0)).expect("end"); spool.write_all(&bytes).expect("write"); }
                },
                "read" => {
                    verified = true;
                    let offset=call["offset"].as_u64().expect("partial offset");
                    let mut bytes=vec![0; call["length"].as_u64().expect("bounded length") as usize];
                    spool.seek(SeekFrom::Start(offset)).expect("seek"); let count=spool.read(&mut bytes).expect("read"); bytes.truncate(count);
                    if mode == "readback" && offset == 0 && !bytes.is_empty() { bytes[0] ^= 1; }
                    reply["read_data"] = json!(base64::engine::general_purpose::STANDARD.encode(bytes));
                },
                "commit" => { assert!(verified); committed=true; },
                "abort" => aborted=true,
                other => panic!("unexpected native action {other}"),
            }
                write_frame(child.stdin.as_mut().expect("stdin"), &reply).expect("ack");
            };
            drop(child.stdin.take());
            let result = child.wait_with_output().expect("worker cleanup");
            assert!(result.status.success());
            assert!(result.stderr.is_empty());
            if mode == "zip64" {
                spool.rewind().expect("ZIP64 rewind");
                let mut header = [0u8; 30];
                spool.read_exact(&mut header).expect("ZIP64 local header");
                assert_eq!(&header[..4], b"PK\x03\x04");
                assert!(u16::from_le_bytes([header[4], header[5]]) >= 45);
                assert_ne!(u16::from_le_bytes([header[6], header[7]]) & 8, 0);
                let name = u16::from_le_bytes([header[26], header[27]]) as u64;
                spool
                    .seek(SeekFrom::Current(name as i64))
                    .expect("ZIP64 extra");
                let mut extra = [0u8; 4];
                spool.read_exact(&mut extra).expect("ZIP64 field");
                assert_eq!(extra, [1, 0, 16, 0]);
            }
            assert_eq!(committed, !damage);
            assert_eq!(aborted, damage);
            assert_eq!(terminal["op"], if damage { "failure" } else { "complete" });
            assert_eq!(
                terminal["source_delete"]["reason"],
                if damage {
                    "lifecycle_incomplete"
                } else if directory {
                    "directory_coverage_unproven"
                } else {
                    "eligible"
                }
            );
            if !damage && !directory {
                assert_eq!(terminal["source_delete"]["source_ids"], json!([source]));
            }
            if damage {
                assert_eq!(
                    terminal["error_code"],
                    if mode == "readback" {
                        "CorruptArchive"
                    } else if mode == "zip64" {
                        "SourceChanged"
                    } else {
                        "IoFailure"
                    }
                );
            }
        }
    }
}
