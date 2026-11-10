//! Real worker protocol and held candidate boundary; Python tarfile supplies the independent input.
#![cfg(feature = "isolated-codecs")]
use base64::Engine as _;
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

fn exercise(format: &str) {
    for corrupt in [false, true] {
        let root = tempfile::tempdir().expect("owned fixture");
        let path = root.path().join(match format {
            "TarGz" => "edit.tgz",
            "TarXz" => "edit.txz",
            _ => "edit.tar",
        });
        let generated = Command::new("python")
            .args(["-c", "import io,sys,tarfile; t=tarfile.open(sys.argv[1],{'Tar':'w','TarGz':'w:gz','TarXz':'w:xz'}[sys.argv[2]],format=tarfile.USTAR_FORMAT); h=tarfile.TarInfo('old.txt'); h.size=7; h.uid=123; h.gid=456; h.mode=0o640; h.mtime=1700000000; t.addfile(h,io.BytesIO(b'payload')); t.close()"])
            .arg(&path).arg(format).status().expect("independent Python fixture");
        assert!(generated.success());
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
        let mut partial = tempfile::tempfile().expect("candidate");
        let mut child = Command::new(env!("CARGO_BIN_EXE_nxtools-archive-worker"))
            .arg("--archive-worker")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("worker");
        let request = json!({"schema":2,"bulk_transfer":1,"operation_id":operation,"op":"start","operation":"modify_tar","path":path.to_str().expect("path"),"job_execution":true,"password_interaction":true,"job_modify":{"browser":{"snapshot_id":Uuid::new_v4(),"format":format,"comment":"","source":{"native":"v1:fixture","digest":digest,"size":metadata.len(),"modified_unix_ns":modified}},"action":"rename","ordinals":[0],"value":"renamed.txt"}});
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
                                .expect("ALL worker old source readers really closed"),
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
            }
            write_frame(child.stdin.as_mut().expect("stdin"), &reply).expect("reply");
        };
        drop(child.stdin.take());
        let output = child.wait_with_output().expect("cleanup");
        assert!(output.stderr.is_empty());
        assert_eq!(
            terminal["op"],
            if corrupt { "failure" } else { "complete" },
            "ordinary TAR must execute the actual edit candidate route: {terminal}"
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
                ]
            );
            partial.rewind().expect("rewind");
            let candidate = root.path().join("oracle.tar");
            std::io::copy(
                &mut partial,
                &mut File::create(&candidate).expect("oracle copy"),
            )
            .expect("copy");
            let verified = Command::new("python").args(["-c", "import sys,tarfile; t=tarfile.open(sys.argv[1]); a=t.getmembers(); assert [x.name for x in a]==['renamed.txt']; h=a[0]; assert (h.uid,h.gid,h.mode,h.mtime)==(123,456,0o640,1700000000); assert t.extractfile(h).read()==b'payload'"])
                .arg(candidate).status().expect("independent full content/metadata oracle");
            assert!(verified.success());
        }
    }
}
#[test]
fn tar_edit_real_worker_renames_and_closes_source_before_single_commit() {
    exercise("Tar");
}
#[test]
fn tgz_edit_real_worker_preserves_content_metadata_and_source_handoff() {
    exercise("TarGz");
}
#[test]
fn txz_edit_real_worker_preserves_content_metadata_and_source_handoff() {
    exercise("TarXz");
}
