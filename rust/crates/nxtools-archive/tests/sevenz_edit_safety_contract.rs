//! Actual isolated-worker faults; native handle/publication semantics have separate C# fixtures.
#![cfg(feature = "isolated-codecs")]
use base64::Engine as _;
#[path = "common/bulk_peer.rs"]
mod bulk_peer;
use bulk_peer::{read_frame, write_frame};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::{Cursor, Read, Seek, SeekFrom, Write},
    process::{Command, Stdio},
    time::UNIX_EPOCH,
};
use uuid::Uuid;

struct Outcome {
    terminal: Value,
    candidate: File,
    commits: usize,
    prompts: usize,
    summary: Option<Value>,
    handed: bool,
}
fn run(mode: &str) -> Outcome {
    let root = tempfile::tempdir().expect("owned source");
    let path = root.path().join("source.7z");
    let mut writer =
        sevenz_rust2::ArchiveWriter::new(File::create(&path).expect("source")).expect("writer");
    writer.set_encrypt_header(false);
    writer.set_content_methods(vec![
        sevenz_rust2::encoder_options::Lzma2Options::from_level(1).into(),
    ]);
    for name in ["a.txt", "b.txt"] {
        let mut entry = sevenz_rust2::ArchiveEntry::new_file(name);
        entry.has_windows_attributes = true;
        entry.windows_attributes = 0x21;
        writer
            .push_archive_entry(entry, Some(Cursor::new(b"original")))
            .expect("member");
    }
    if mode == "rename_collision" {
        writer
            .push_archive_entry::<std::io::Empty>(
                sevenz_rust2::ArchiveEntry::new_directory("a (2).txt/"),
                None,
            )
            .expect("reserved explicit directory");
        writer
            .push_archive_entry(
                sevenz_rust2::ArchiveEntry::new_file("a (3).txt/leaf"),
                Some(Cursor::new(b"leaf")),
            )
            .expect("reserved implicit directory");
    }
    writer.finish().expect("finish");
    let original = std::fs::read(&path).expect("bytes");
    let metadata = std::fs::metadata(&path).expect("metadata");
    let digest: [u8; 32] = Sha256::digest(&original).into();
    let modified = metadata
        .modified()
        .expect("mtime")
        .duration_since(UNIX_EPOCH)
        .expect("epoch")
        .as_nanos() as u64;
    let operation = Uuid::new_v4();
    let token = Uuid::new_v4();
    let source_ids = [Uuid::new_v4(), Uuid::new_v4()];
    let replacement = mode == "empty_replace";
    let data: &[u8] = if replacement { b"" } else { b"new-content" };
    let mut request = json!({"schema":2,"bulk_transfer":1,"operation_id":operation,"op":"start","operation":"modify_7z","path":path.to_str().expect("path"),"job_execution":true,"conflict_policy":"ask","job_modify":{"browser":{"snapshot_id":Uuid::new_v4(),"format":"SevenZ","comment":"","source":{"native":"v1:fixture","digest":digest,"size":metadata.len(),"modified_unix_ns":modified}},"action":if replacement {"replace_content"} else {"add"},"ordinals":if replacement {vec![0]} else {vec![]},"value":null}});
    if mode == "budget" {
        request["limits"] = json!({"max_entries":100,"max_metadata_bytes":33554432u64,"max_working_bytes":536870912u64,"max_entry_bytes":1048576u64,"max_output_bytes":40});
    }
    let mut child = Command::new(env!("CARGO_BIN_EXE_nxtools-archive-worker"))
        .arg("--archive-worker")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("worker");
    write_frame(child.stdin.as_mut().expect("stdin"), &request).expect("start");
    let mut partial = tempfile::tempfile().expect("candidate");
    let mut open = None;
    let mut commits = 0;
    let mut prompts = 0;
    let mut summary = None;
    let mut handed = false;
    let terminal = loop {
        let Some(frame) =
            read_frame::<_, Value>(child.stdout.as_mut().expect("stdout")).expect("frame")
        else {
            break json!({"op":"eof"});
        };
        match frame["op"].as_str().expect("op") {
            "failure" | "complete" => break frame,
            "extraction_summary" => {
                summary = Some(frame["extraction_summary"].clone());
                continue;
            }
            "conflict_prompt" => {
                prompts += 1;
                assert!(prompts <= 2, "finite original tickets");
                assert_eq!(
                    frame["conflict_prompt"]["display"]["incoming_member"], "a.txt",
                    "exact incoming archive member in original prompt"
                );
                assert_eq!(
                    frame["conflict_prompt"]["display"]["target"], "a.txt",
                    "exact existing archive member, not a fabricated disk target"
                );
                let decision = match mode {
                    "skip_all" => "skip",
                    "rename_all" | "rename_collision" => "auto_rename",
                    _ => "overwrite",
                };
                write_frame(child.stdin.as_mut().expect("stdin"), &json!({"version":1,"operation_id":operation,"op":"conflict_reply","conflict_reply":{"ticket":frame["conflict_prompt"]["ticket"],"decision":decision,"apply_to_all":true}})).expect("single-use response");
                continue;
            }
            "native_call" => {}
            _ => continue,
        }
        let call = &frame["native"];
        let mut reply = json!({"version":1,"operation_id":operation,"op":"native_result","call_id":call["call_id"],"success":true});
        match call["action"].as_str().expect("action") {
            "source_acquire" | "edit_planning" | "edit_reading" | "edit_reencoding"
            | "edit_verifying" | "creation_verifying" | "source_precommit" | "source_check" => {}
            "source_stamp" => reply["read_data"] = json!("v1:fixture"),
            "source_roots" => {
                let count = if replacement { 1 } else { 2 };
                let entries: Vec<_> = source_ids.iter().take(count).enumerate().map(|(i,id)| json!({"id":id,"name":if i==0 {"a.txt"} else {"b.txt"},"directory":false,"size":data.len(),"dos_modified":null})).collect();
                reply["read_data"] = json!(serde_json::to_string(&entries).expect("roots"));
            }
            "source_open" => {
                assert!(open.is_none());
                open = Some(call["token"].clone());
            }
            "source_close" => {
                assert_eq!(open.take(), Some(call["token"].clone()));
            }
            "source_read" => {
                assert_eq!(open, Some(call["token"].clone()));
                let offset = call["offset"].as_u64().expect("offset") as usize;
                let length = call["length"].as_u64().expect("length") as usize;
                assert!(length <= 65536);
                reply["read_data"] = json!(
                    base64::engine::general_purpose::STANDARD
                        .encode(&data[offset..(offset + length).min(data.len())])
                );
                if mode == "source_changed" {
                    reply["success"] = json!(false);
                    reply["error_code"] = json!("source_changed");
                }
            }
            "begin" => {
                reply["token"] = json!(token);
                if mode == "low_space" {
                    reply["success"] = json!(false);
                    reply["error_code"] = json!("write_limit");
                }
            }
            "write" => {
                if mode == "write_io" {
                    reply["success"] = json!(false);
                    reply["error_code"] = json!("write_file");
                } else {
                    let bytes: Vec<u8> =
                        serde_json::from_value(call["data"].clone()).expect("bytes");
                    partial.seek(SeekFrom::End(0)).expect("end");
                    partial.write_all(&bytes).expect("write");
                }
            }
            "read" => {
                let offset = call["offset"].as_u64().expect("offset");
                let mut bytes = vec![0; call["length"].as_u64().expect("length") as usize];
                partial.seek(SeekFrom::Start(offset)).expect("seek");
                let n = partial.read(&mut bytes).expect("read");
                bytes.truncate(n);
                if mode == "readback" && offset == 0 && !bytes.is_empty() {
                    bytes[0] ^= 1;
                }
                if mode == "truncated" {
                    bytes.clear();
                }
                reply["read_data"] = json!(base64::engine::general_purpose::STANDARD.encode(bytes));
            }
            "edit_reads_complete" => {
                assert!(open.is_none());
                handed = true;
            }
            "commit" => {
                assert!(handed);
                commits += 1;
            }
            "abort" => {}
            other => panic!("unexpected {other}"),
        }
        write_frame(child.stdin.as_mut().expect("stdin"), &reply).expect("ack");
    };
    drop(child.stdin.take());
    let result = child.wait_with_output().expect("cleanup");
    assert!(
        result.status.success() && result.stderr.is_empty(),
        "{mode}: {:?}",
        result.stderr
    );
    assert!(
        open.is_none(),
        "{mode}: source reader closed even on codec failure"
    );
    assert_eq!(
        std::fs::read(path).expect("retained"),
        original,
        "fixture never publishes into original"
    );
    Outcome {
        terminal,
        candidate: partial,
        commits,
        prompts,
        summary,
        handed,
    }
}
#[test]
fn sevenz_edit_worker_conflict_apply_all_and_skip_summary() {
    for mode in ["skip_all", "overwrite_all", "rename_all"] {
        let result = run(mode);
        assert_eq!(
            result.terminal["op"], "complete",
            "{mode}: {}",
            result.terminal
        );
        assert_eq!(result.prompts, 1, "{mode}: apply-to-all must not reprompt");
        assert_eq!(
            result.commits,
            usize::from(mode != "skip_all"),
            "all-skipped add does not rewrite original"
        );
        assert_eq!(
            result.summary,
            Some(
                json!({"files_skipped":if mode=="skip_all" {2} else {0},"all_members_written":mode!="skip_all"})
            ),
            "truthful skipped-addition outcome"
        );
    }
}
#[test]
fn sevenz_edit_worker_empty_replacement_retains_metadata_and_checks_eof() {
    let mut result = run("empty_replace");
    assert_eq!(result.terminal["op"], "complete", "{}", result.terminal);
    result.candidate.rewind().expect("rewind");
    let mut reader =
        sevenz_rust2::ArchiveReader::new(result.candidate, sevenz_rust2::Password::empty())
            .expect("candidate");
    let entry = reader
        .archive()
        .files
        .iter()
        .find(|e| e.name == "a.txt")
        .expect("target");
    assert_eq!(entry.size, 0);
    assert!(entry.has_windows_attributes);
    assert_eq!(entry.windows_attributes, 0x21);
    reader
        .for_each_entries(|entry, r| {
            let mut bytes = Vec::new();
            r.read_to_end(&mut bytes)?;
            assert_eq!(
                bytes,
                if entry.name == "a.txt" {
                    b"".as_slice()
                } else {
                    b"original".as_slice()
                }
            );
            Ok(true)
        })
        .expect("full integrity");
    assert_eq!(result.commits, 1);
}
#[test]
fn sevenz_edit_worker_faults_keep_first_error_and_never_handoff() {
    for (mode, code) in [
        ("source_changed", "SourceChanged"),
        ("write_io", "IoFailure"),
        ("readback", "CorruptArchive"),
        ("truncated", "CorruptArchive"),
        ("low_space", "ResourceLimit"),
        ("budget", "ResourceLimit"),
    ] {
        let result = run(mode);
        assert_eq!(
            result.terminal["op"], "failure",
            "{mode}: {}",
            result.terminal
        );
        assert_eq!(
            result.terminal["error_code"], code,
            "{mode}: first typed boundary failure"
        );
        assert_eq!(result.commits, 0);
        assert!(!result.handed, "{mode}");
    }
}

#[test]
fn sevenz_edit_worker_auto_rename_skips_explicit_and_implicit_directory_candidates() {
    let mut result = run("rename_collision");
    assert_eq!(result.terminal["op"], "complete", "{}", result.terminal);
    assert_eq!(result.commits, 1);
    result.candidate.rewind().expect("rewind");
    let reader =
        sevenz_rust2::ArchiveReader::new(result.candidate, sevenz_rust2::Password::empty())
            .expect("candidate");
    assert!(
        reader
            .archive()
            .files
            .iter()
            .any(|entry| entry.name == "a (4).txt" && !entry.is_directory)
    );
    assert!(
        reader
            .archive()
            .files
            .iter()
            .any(|entry| entry.name == "a (2).txt/" && entry.is_directory)
    );
    assert!(
        reader
            .archive()
            .files
            .iter()
            .any(|entry| entry.name == "a (3).txt/leaf")
    );
}
