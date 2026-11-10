//! Original worker rendezvous: encoded candidate readback and actual old-source reader closure.
#![cfg(feature = "isolated-codecs")]
#[path = "common/bulk_peer.rs"]
mod bulk_peer;
use bulk_peer::{read_frame, write_frame};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs::{File, OpenOptions},
    io::{Cursor, Read, Seek, SeekFrom, Write},
    process::{Command, Stdio},
    time::UNIX_EPOCH,
};
use uuid::Uuid;
#[test]
fn sevenz_edit_real_worker_renames_and_closes_source_before_single_commit() {
    use base64::Engine as _;
    for corrupt in [false, true] {
        let root = tempfile::tempdir().expect("owned");
        let path = root.path().join("edit.7z");
        let mut archive =
            sevenz_rust2::ArchiveWriter::new(File::create(&path).expect("source")).expect("writer");
        archive.set_encrypt_header(false);
        archive.set_content_methods(vec![
            sevenz_rust2::encoder_options::Lzma2Options::from_level(1).into(),
        ]);
        archive
            .push_archive_entry(
                sevenz_rust2::ArchiveEntry::new_file("old.txt"),
                Some(Cursor::new(b"payload")),
            )
            .expect("entry");
        archive.finish().expect("finish");
        let original = std::fs::read(&path).expect("original");
        let digest: [u8; 32] = Sha256::digest(&original).into();
        let metadata = std::fs::metadata(&path).expect("metadata");
        let modified = metadata
            .modified()
            .expect("mtime")
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
        let request = json!({"schema":2,"bulk_transfer":1,"operation_id":operation,"op":"start","operation":"modify_7z","path":path.to_str().expect("path"),"job_execution":true,"job_modify":{"browser":{"snapshot_id":Uuid::new_v4(),"format":"SevenZ","comment":"","source":{"native":"v1:fixture","digest":digest,"size":metadata.len(),"modified_unix_ns":modified}},"action":"rename","ordinals":[0],"value":"renamed.txt"}});
        write_frame(child.stdin.as_mut().expect("stdin"), &request).expect("start");
        let mut handed = false;
        let mut committed = 0;
        let mut readback = false;
        let mut aborted = false;
        let mut phases = Vec::new();
        let terminal = loop {
            let Some(frame) =
                read_frame::<_, Value>(child.stdout.as_mut().expect("stdout")).expect("frame")
            else {
                break json!({"op":"eof"});
            };
            if matches!(frame["op"].as_str(), Some("failure" | "complete")) {
                break frame;
            }
            if frame["op"] == "progress"
                && let Some(phase) = frame["progress"]["phase"].as_str()
                && phases.last().map(String::as_str) != Some(phase)
            {
                phases.push(phase.to_owned());
            }
            if frame["op"] != "native_call" {
                continue;
            }
            let call = &frame["native"];
            let mut reply = json!({"version":1,"operation_id":operation,"op":"native_result","call_id":call["call_id"],"success":true});
            match call["action"].as_str().expect("action") {
                "source_acquire" | "edit_planning" | "edit_reading" | "edit_reencoding"
                | "edit_verifying" | "creation_verifying" | "source_precommit" => {}
                "source_stamp" => reply["read_data"] = json!("v1:fixture"),
                "begin" => reply["token"] = json!(token),
                "write" => {
                    let bytes: Vec<u8> =
                        serde_json::from_value(call["data"].clone()).expect("bytes");
                    assert!(bytes.len() <= 65536);
                    partial.seek(SeekFrom::End(0)).expect("end");
                    partial.write_all(&bytes).expect("write");
                }
                "read" => {
                    readback = true;
                    let offset = call["offset"].as_u64().expect("offset");
                    let length = call["length"].as_u64().expect("length") as usize;
                    assert!(length <= 65536);
                    partial.seek(SeekFrom::Start(offset)).expect("seek");
                    let mut bytes = vec![0; length];
                    let n = partial.read(&mut bytes).expect("read");
                    bytes.truncate(n);
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
                        drop(
                            OpenOptions::new()
                                .read(true)
                                .write(true)
                                .share_mode(0)
                                .open(&path)
                                .expect("all worker source readers closed"),
                        );
                    }
                    handed = true;
                }
                "commit" => {
                    assert!(handed);
                    committed += 1;
                }
                "abort" => aborted = true,
                other => panic!("unexpected {other}"),
            };
            write_frame(child.stdin.as_mut().expect("stdin"), &reply).expect("reply");
        };
        drop(child.stdin.take());
        let output = child.wait_with_output().expect("cleanup");
        assert!(output.stderr.is_empty());
        assert_eq!(
            terminal["op"],
            if corrupt { "failure" } else { "complete" },
            "7z request must run actual edit candidate path"
        );
        assert_eq!(committed, usize::from(!corrupt));
        assert_eq!(handed, !corrupt);
        assert_eq!(aborted, corrupt);
        assert_eq!(std::fs::read(&path).expect("source retained"), original);
        if !corrupt {
            assert_eq!(
                phases,
                [
                    "edit_planning",
                    "edit_reading",
                    "edit_reencoding",
                    "edit_verifying",
                    "edit_committing"
                ],
                "actual stage transition reaches original progress wire"
            );
        }
        if !corrupt {
            partial.rewind().expect("rewind");
            let mut reader =
                sevenz_rust2::ArchiveReader::new(partial, sevenz_rust2::Password::empty())
                    .expect("complete candidate");
            assert_eq!(reader.archive().files[0].name, "renamed.txt");
            let mut bytes = Vec::new();
            reader
                .for_each_entries(|_, r| {
                    r.read_to_end(&mut bytes)?;
                    Ok(true)
                })
                .expect("full decode");
            assert_eq!(bytes, b"payload");
        }
    }
}
