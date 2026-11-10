//! Actual worker protocol, source reader closure and held-candidate readback boundary.
#[path = "common/bulk_peer.rs"]
mod bulk_peer;
use bulk_peer::{read_frame, write_frame};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    process::{Command, Stdio},
    time::UNIX_EPOCH,
};
use uuid::Uuid;
#[test]
fn original_zip_edit_worker_closes_source_before_handoff_and_rejects_bad_readback() {
    use base64::Engine as _;
    for corrupt in [false, true] {
        let root = tempfile::tempdir().expect("owned");
        let path = root.path().join("edit.zip");
        let mut zip = zip::ZipWriter::new(File::create(&path).expect("source"));
        zip.start_file("old.txt", zip::write::SimpleFileOptions::default())
            .expect("entry");
        zip.write_all(b"payload").expect("bytes");
        zip.finish().expect("finish");
        let original = std::fs::read(&path).expect("original");
        let digest: [u8; 32] = Sha256::digest(&original).into();
        let metadata = std::fs::metadata(&path).expect("metadata");
        let modified = metadata
            .modified()
            .expect("modified")
            .duration_since(UNIX_EPOCH)
            .expect("epoch")
            .as_nanos() as u64;
        let operation = Uuid::new_v4();
        let token = Uuid::new_v4();
        let mut partial = tempfile::tempfile().expect("partial");
        let mut child = Command::new(env!("CARGO_BIN_EXE_nxtools-archive-worker"))
            .arg("--archive-worker")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("worker");
        let request = json!({"schema":2,"bulk_transfer":1,"operation_id":operation,"op":"start","operation":"modify_zip","path":path.to_str().expect("path"),"job_execution":true,"job_modify":{"browser":{"snapshot_id":Uuid::new_v4(),"format":"Zip","comment":"","source":{"native":"v1:fixture","digest":digest,"size":metadata.len(),"modified_unix_ns":modified}},"action":"rename","ordinals":[0],"value":"renamed.txt"}});
        write_frame(child.stdin.as_mut().expect("stdin"), &request).expect("start");
        let mut handed_off = false;
        let mut committed = false;
        let mut readback = false;
        let mut aborted = false;
        let terminal = loop {
            let frame: Value = read_frame(child.stdout.as_mut().expect("stdout"))
                .expect("frame")
                .expect("terminal");
            if matches!(frame["op"].as_str(), Some("complete" | "failure")) {
                break frame;
            }
            if frame["op"] != "native_call" {
                continue;
            }
            let call = &frame["native"];
            let action = call["action"].as_str().expect("action");
            let mut reply = json!({"version":1,"operation_id":operation,"op":"native_result","call_id":call["call_id"],"success":true});
            match action {
                "source_acquire" | "edit_generating" | "edit_verifying" => {}
                "source_stamp" => reply["read_data"] = json!("v1:fixture"),
                "begin" => reply["token"] = json!(token),
                "write" => {
                    let data: Vec<u8> =
                        serde_json::from_value(call["data"].clone()).expect("bytes");
                    assert!(data.len() <= 65536);
                    partial.seek(SeekFrom::End(0)).expect("end");
                    partial.write_all(&data).expect("write");
                }
                "read" => {
                    readback = true;
                    let offset = call["offset"].as_u64().expect("offset");
                    let length = call["length"].as_u64().expect("length") as usize;
                    assert!(length <= 65536);
                    partial.seek(SeekFrom::Start(offset)).expect("seek");
                    let mut bytes = vec![0; length];
                    let count = partial.read(&mut bytes).expect("read");
                    bytes.truncate(count);
                    if corrupt && offset == 0 && !bytes.is_empty() {
                        bytes[0] ^= 1;
                    }
                    reply["read_data"] =
                        json!(base64::engine::general_purpose::STANDARD.encode(bytes));
                }
                "edit_reads_complete" => {
                    assert!(readback && !corrupt);
                    #[cfg(windows)]
                    {
                        use std::os::windows::fs::OpenOptionsExt;
                        let exclusive = OpenOptions::new()
                            .read(true)
                            .write(true)
                            .share_mode(0)
                            .open(&path)
                            .expect("all worker source handles actually closed");
                        drop(exclusive);
                    }
                    handed_off = true;
                }
                "commit" => {
                    assert!(handed_off && readback);
                    committed = true;
                }
                "abort" => aborted = true,
                other => panic!("unexpected {other}"),
            }
            write_frame(child.stdin.as_mut().expect("stdin"), &reply).expect("reply");
        };
        drop(child.stdin.take());
        let output = child.wait_with_output().expect("cleanup");
        assert!(output.status.success() && output.stderr.is_empty());
        assert_eq!(committed, !corrupt);
        assert_eq!(handed_off, !corrupt);
        assert_eq!(aborted, corrupt);
        assert_eq!(std::fs::read(&path).expect("source retained"), original);
        assert_eq!(terminal["op"], if corrupt { "failure" } else { "complete" });
        if !corrupt {
            assert_eq!(
                terminal["work"]["zip_local_checks"], 2,
                "one source admission plus one complete final candidate"
            );
            assert_eq!(
                terminal["work"]["zip_descriptor_checks"], 2,
                "worker source and final candidate each admit descriptors once"
            );
        }
        if corrupt {
            assert_eq!(terminal["error_code"], "CorruptArchive");
        }
    }
}
