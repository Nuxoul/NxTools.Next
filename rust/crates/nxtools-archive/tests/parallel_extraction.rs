use nxtools_archive::worker::{WorkerRequest, write_frame};
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    io::{Read, Write},
    process::{Child, Command, Stdio},
    sync::mpsc::{self, Receiver},
    thread::JoinHandle,
    time::Duration,
};
use uuid::Uuid;

struct Worker {
    child: Child,
    frames: Receiver<std::io::Result<Value>>,
    reader: Option<JoinHandle<()>>,
}
impl Worker {
    fn new(request: &WorkerRequest) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_nxtools-archive-worker"))
            .arg("--archive-worker")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("owned worker");
        write_frame(child.stdin.as_mut().expect("stdin"), request).expect("start");
        let mut stdout = child.stdout.take().expect("stdout");
        let (sender, frames) = mpsc::channel();
        let reader = std::thread::spawn(move || {
            loop {
                let frame = (|| {
                    let mut prefix = [0; 4];
                    stdout.read_exact(&mut prefix)?;
                    let length = u32::from_le_bytes(prefix);
                    let binary = length & 0x8000_0000 != 0;
                    let size = (length & 0x7fff_ffff) as usize;
                    if !(if binary { 76..=32844 } else { 1..=1048576 }).contains(&size) {
                        return Err(std::io::ErrorKind::InvalidData.into());
                    }
                    let mut body = vec![0; size];
                    stdout.read_exact(&mut body)?;
                    if !binary {
                        return serde_json::from_slice(&body).map_err(std::io::Error::other);
                    }
                    if body[..4] != [1, 1, 0, 0] {
                        return Err(std::io::ErrorKind::InvalidData.into());
                    }
                    let token = Uuid::from_slice(&body[36..52]).map_err(std::io::Error::other)?;
                    let sequence = u64::from_le_bytes(body[52..60].try_into().expect("sequence"));
                    let offset = u64::from_le_bytes(body[60..68].try_into().expect("offset"));
                    let length = u32::from_le_bytes(body[68..72].try_into().expect("length"));
                    let actual = u32::from_le_bytes(body[72..76].try_into().expect("actual"));
                    if length != actual || size != 76 + actual as usize {
                        return Err(std::io::ErrorKind::InvalidData.into());
                    }
                    Ok(json!({"op":"payload","token":token,"sequence":sequence,
                        "offset":offset,"data":&body[76..]}))
                })();
                let failed = frame.is_err();
                if sender.send(frame).is_err() || failed {
                    break;
                }
            }
        });
        Self {
            child,
            frames,
            reader: Some(reader),
        }
    }
    fn next(&self) -> Value {
        self.frames
            .recv_timeout(Duration::from_secs(5))
            .expect("parallel ZIP must admit a second member before the first begin reply")
            .expect("complete atomic private frame")
    }
    fn reply(&mut self, operation: Uuid, call: &Value, token: Option<Uuid>) {
        write_frame(
            self.child.stdin.as_mut().expect("stdin"),
            &json!({
                "version":1,"operation_id":operation,"op":"native_result",
                "call_id":call["call_id"],"success":true,"token":token
            }),
        )
        .expect("native reply");
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        drop(self.child.stdin.take());
        let _ = self.child.kill();
        let _ = self.child.wait();
        if let Some(reader) = self.reader.take() {
            reader.join().expect("owned stdout reader joined");
        }
    }
}

#[test]
fn ordinary_zip_members_overlap_before_first_begin_reply_and_keep_exact_payload_order() {
    verify_members(8, 512 << 20, true, None);
}

#[test]
fn working_credit_limits_real_worker_to_two_lanes() {
    verify_members(48, 9 << 20, true, Some(2));
}

#[test]
fn insufficient_parallel_credit_falls_back_to_original_serial_streaming() {
    verify_members(8, 4 << 20, true, Some(1));
}

#[test]
fn absent_parallel_request_retains_single_member_route() {
    verify_members(8, 512 << 20, false, Some(1));
}

fn verify_members(count: u8, working: u64, parallel: bool, cap: Option<usize>) {
    let root = tempfile::tempdir().expect("owned source");
    let path = root.path().join("parallel.zip");
    let mut zip = zip::ZipWriter::new(std::fs::File::create(&path).expect("source"));
    let mut expected = HashMap::new();
    for index in 0..count {
        let name = format!("member-{index}.bin");
        let data: Vec<_> = (0..70001)
            .map(|offset| index.wrapping_add(offset as u8))
            .collect();
        zip.start_file(
            &name,
            zip::write::SimpleFileOptions::default().compression_method(if index % 2 == 0 {
                zip::CompressionMethod::Stored
            } else {
                zip::CompressionMethod::Deflated
            }),
        )
        .expect("member");
        zip.write_all(&data).expect("member bytes");
        expected.insert(name, data);
    }
    zip.finish().expect("valid ZIP");
    let mut request = WorkerRequest::open(Uuid::new_v4(), path.to_string_lossy().into_owned());
    request.version = 2;
    request.operation = "extract".into();
    request.bulk_transfer = Some(1);
    request.parallel_extraction = parallel.then_some(true);
    request.limits.max_working_bytes = working;
    let mut worker = Worker::new(&request);
    let operation = request.operation_id;
    let mut held = Vec::new();
    let cpu_width = (std::thread::available_parallelism()
        .expect("CPU count")
        .get()
        .min(64)
        / 2)
    .max(1);
    let expected_width = cap.unwrap_or(cpu_width.min(count as usize).min(32));
    let scheduled = parallel && expected_width >= 2;
    assert!(
        cap.is_some() || cpu_width >= 2,
        "parallel fixture requires at least four logical CPUs"
    );
    let mut closed_tokens = std::collections::HashSet::new();
    let mut outputs: HashMap<String, (String, Vec<u8>)> = HashMap::new();
    let mut live = 0usize;
    let mut high_water = 0;
    let mut sequence = 0;
    let mut closed = 0;
    let mut accepted = false;
    loop {
        let frame = worker.next();
        match frame["op"].as_str().expect("op") {
            "accepted" => {
                assert!(!accepted, "single capability agreement");
                accepted = true;
                let capabilities = frame["capabilities"].as_array().expect("capabilities");
                assert_eq!(
                    capabilities
                        .iter()
                        .filter(|capability| capability.as_str() == Some("parallel_extraction_v1"))
                        .count(),
                    usize::from(parallel)
                );
            }
            "native_call" => {
                assert!(accepted, "capabilities precede native effects");
                let call = &frame["native"];
                match call["action"].as_str().expect("action") {
                    "begin" => {
                        let token = Uuid::new_v4();
                        outputs.insert(
                            token.to_string(),
                            (call["path"].as_str().expect("path").to_owned(), Vec::new()),
                        );
                        live += 1;
                        high_water = high_water.max(live);
                        if scheduled && outputs.len() <= expected_width {
                            held.push((call.clone(), token));
                            if held.len() == expected_width {
                                for (call, token) in held.drain(..).rev() {
                                    worker.reply(operation, &call, Some(token));
                                }
                            }
                        } else {
                            worker.reply(operation, call, Some(token));
                        }
                    }
                    "commit" => {
                        let token = call["token"].as_str().expect("token");
                        assert!(
                            closed_tokens.insert(token.to_owned()),
                            "each token closes once"
                        );
                        let (name, bytes) = outputs.get(token).expect("owned token");
                        assert_eq!(bytes, expected.get(name).expect("original member"));
                        live -= 1;
                        closed += 1;
                        worker.reply(operation, call, None);
                    }
                    "source_acquire" | "directory" => worker.reply(operation, call, None),
                    action => panic!("unexpected native action {action}"),
                }
            }
            "payload" => {
                sequence += 1;
                assert!(
                    !closed_tokens.contains(frame["token"].as_str().expect("token")),
                    "no payload after close"
                );
                assert_eq!(frame["sequence"], sequence, "global binary emission order");
                let (_, bytes) = outputs
                    .get_mut(frame["token"].as_str().expect("token"))
                    .expect("live output");
                assert_eq!(frame["offset"], bytes.len(), "per-token payload order");
                bytes.extend(
                    frame["data"]
                        .as_array()
                        .expect("bytes")
                        .iter()
                        .map(|value| value.as_u64().expect("byte") as u8),
                );
            }
            "complete" => {
                assert_eq!(frame["files_verified"], count);
                assert_eq!(frame["bytes_verified"], u64::from(count) * 70001);
                assert_eq!(
                    frame["work"]["parallel_workers_planned"],
                    if scheduled { expected_width } else { 0 }
                );
                assert_eq!(
                    frame["work"]["parallel_members_completed"],
                    if scheduled { count } else { 0 }
                );
                assert!(
                    frame["work"]["parallel_workers_high_water"]
                        .as_u64()
                        .expect("worker high water")
                        <= expected_width as u64
                );
                break;
            }
            "failure" => panic!("parallel extraction failed: {}", frame["error_code"]),
            _ => {}
        }
    }
    assert_eq!(
        high_water, expected_width,
        "controlled full-width admission and finite cap"
    );
    assert_eq!(
        (closed, live, outputs.len()),
        (count as usize, 0, count as usize)
    );
}

fn lifecycle_source() -> (tempfile::TempDir, std::path::PathBuf, WorkerRequest) {
    let root = tempfile::tempdir().expect("owned source");
    let path = root.path().join("lifecycle.zip");
    let mut zip = zip::ZipWriter::new(std::fs::File::create(&path).expect("source"));
    for index in 0..4u8 {
        zip.start_file(
            format!("member-{index}.bin"),
            zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored),
        )
        .expect("member");
        zip.write_all(&vec![index; 40001]).expect("bytes");
    }
    zip.finish().expect("ZIP");
    let mut request = WorkerRequest::open(Uuid::new_v4(), path.to_string_lossy().into_owned());
    request.version = 2;
    request.operation = "extract".into();
    request.bulk_transfer = Some(1);
    request.parallel_extraction = Some(true);
    request.limits.max_working_bytes = 9 << 20;
    (root, path, request)
}

#[test]
fn cancelled_parallel_worker_joins_multiple_pending_members_after_one_close() {
    interrupted_members("cancel");
}
#[test]
fn crc_failure_wakes_other_pending_member_and_survives_cancellation() {
    interrupted_members("crc");
}
#[test]
fn native_failure_precedes_cancel_and_wakes_other_pending_member() {
    interrupted_members("native");
}

fn interrupted_members(mode: &str) {
    use std::io::{Seek, SeekFrom};
    let (_root, path, request) = lifecycle_source();
    if mode == "crc" {
        let offset = {
            let mut zip =
                zip::ZipArchive::new(std::fs::File::open(&path).expect("source")).expect("ZIP");
            zip.by_index(1)
                .expect("member")
                .data_start()
                .expect("ordinary Stored payload offset")
        };
        let mut source = std::fs::OpenOptions::new()
            .write(true)
            .open(&path)
            .expect("owned corrupt source");
        source.seek(SeekFrom::Start(offset)).expect("payload");
        source
            .write_all(&[9])
            .expect("corrupt payload, not structural proof");
    }
    let operation = request.operation_id;
    let mut worker = Worker::new(&request);
    let mut pending = HashMap::new();
    let mut closed = 0;
    let mut triggered = false;
    let mut first_token = None;
    let mut first_bytes = Vec::new();
    loop {
        let frame = worker.next();
        match frame["op"].as_str().expect("op") {
            "native_call" => {
                let call = &frame["native"];
                match call["action"].as_str().expect("action") {
                    "source_acquire" | "directory" => worker.reply(operation, call, None),
                    "begin" => {
                        let name = call["path"].as_str().expect("path");
                        if name == "member-0.bin" {
                            let token = Uuid::new_v4();
                            first_token = Some(token.to_string());
                            worker.reply(operation, call, Some(token));
                        } else {
                            pending.insert(name.to_owned(), call.clone());
                            if pending.len() == 2 {
                                assert_eq!(closed, 1, "completed output precedes interruption");
                                assert!(
                                    pending.contains_key("member-1.bin")
                                        && pending.contains_key("member-2.bin")
                                );
                                triggered = true;
                                let damaged = &pending["member-1.bin"];
                                if mode == "crc" {
                                    worker.reply(operation, damaged, Some(Uuid::new_v4()));
                                } else {
                                    if mode == "native" {
                                        write_frame(worker.child.stdin.as_mut().expect("stdin"), &json!({"version":1,"operation_id":operation,"op":"native_result","call_id":damaged["call_id"],"success":false,"error_code":"resource_limit"})).expect("native error before cancel");
                                    }
                                    write_frame(worker.child.stdin.as_mut().expect("stdin"), &json!({"version":1,"operation_id":operation,"op":"cancel"})).expect("cancel");
                                }
                            }
                        }
                    }
                    "commit" => {
                        assert_eq!(call["token"], first_token.as_deref().expect("first token"));
                        assert_eq!(first_bytes, vec![0; 40001]);
                        closed += 1;
                        worker.reply(operation, call, None);
                    }
                    action => panic!("unexpected native action {action}"),
                }
            }
            "payload" if frame["token"].as_str() == first_token.as_deref() => {
                assert_eq!(frame["offset"], first_bytes.len());
                first_bytes.extend(
                    frame["data"]
                        .as_array()
                        .expect("payload")
                        .iter()
                        .map(|byte| byte.as_u64().expect("byte") as u8),
                );
            }
            "failure" => {
                assert!(triggered && closed == 1);
                assert_eq!(
                    frame["error_code"],
                    match mode {
                        "crc" => "CorruptArchive",
                        "native" => "ResourceLimit",
                        _ => "Cancelled",
                    }
                );
                assert_eq!(frame["work"]["parallel_workers_planned"], 2);
                assert_eq!(frame["work"]["parallel_workers_high_water"], 2);
                assert_eq!(frame["work"]["parallel_members_completed"], 1);
                break;
            }
            "complete" => panic!("interrupted operation cannot complete"),
            _ => {}
        }
    }
}

#[test]
fn parallel_collision_prompts_share_one_apply_all_decision() {
    let (_root, _path, request) = lifecycle_source();
    let mut request_json = serde_json::to_value(&request).expect("request");
    request_json["conflict_policy"] = json!("ask");
    let request = serde_json::from_value::<WorkerRequest>(request_json).expect("Ask request");
    let operation = request.operation_id;
    let mut worker = Worker::new(&request);
    let mut initial = Vec::new();
    let mut prompts = 0;
    let mut outputs: HashMap<String, (u8, Vec<u8>)> = HashMap::new();
    let mut closed = 0;
    loop {
        let frame = worker.next();
        match frame["op"].as_str().expect("op") {
            "native_call" => {
                let call = &frame["native"];
                match call["action"].as_str().expect("action") {
                    "source_acquire" | "directory" => worker.reply(operation, call, None),
                    "begin" => {
                        initial.push(call.clone());
                        if initial.len() >= 2 {
                            for call in initial.drain(..) {
                                write_frame(worker.child.stdin.as_mut().expect("stdin"), &json!({"version":1,"operation_id":operation,"op":"native_result","call_id":call["call_id"],"success":false,"error_code":"target_exists","existing_bytes":7})).expect("CREATE_NEW collision");
                            }
                        }
                    }
                    "begin_overwrite" => {
                        let token = Uuid::new_v4();
                        let name = call["path"].as_str().expect("path");
                        let index = name
                            .strip_prefix("member-")
                            .and_then(|name| name.strip_suffix(".bin"))
                            .expect("member")
                            .parse()
                            .expect("index");
                        outputs.insert(token.to_string(), (index, Vec::new()));
                        worker.reply(operation, call, Some(token));
                    }
                    "commit" => {
                        let (index, bytes) = &outputs[call["token"].as_str().expect("token")];
                        assert_eq!(bytes, &vec![*index; 40001]);
                        closed += 1;
                        worker.reply(operation, call, None);
                    }
                    action => panic!("unexpected native action {action}"),
                }
            }
            "conflict_prompt" => {
                prompts += 1;
                assert_eq!(prompts, 1, "one serialized Apply All prompt");
                let mut stale = frame["conflict_prompt"]["ticket"].clone();
                stale["operation_id"] = json!(Uuid::new_v4());
                for ticket in [stale, frame["conflict_prompt"]["ticket"].clone()] {
                    write_frame(worker.child.stdin.as_mut().expect("stdin"), &json!({"version":1,"operation_id":operation,"op":"conflict_reply","conflict_reply":{"ticket":ticket,"decision":"overwrite","apply_to_all":true}})).expect("ticket response");
                }
            }
            "payload" => {
                let (_, bytes) = outputs
                    .get_mut(frame["token"].as_str().expect("token"))
                    .expect("output");
                assert_eq!(frame["offset"], bytes.len());
                bytes.extend(
                    frame["data"]
                        .as_array()
                        .expect("payload")
                        .iter()
                        .map(|byte| byte.as_u64().expect("byte") as u8),
                );
            }
            "complete" => {
                assert_eq!(frame["files_verified"], 4);
                break;
            }
            "failure" => panic!("conflict operation failed: {}", frame["error_code"]),
            _ => {}
        }
    }
    assert_eq!((prompts, closed), (1, 4));
}

#[test]
fn parallel_request_is_private_opt_in_and_only_admitted_for_ordinary_bulk_extraction() {
    let mut request = WorkerRequest::open(Uuid::new_v4(), "owned.zip".into());
    assert!(
        serde_json::to_value(&request)
            .expect("legacy request")
            .get("parallel_extraction")
            .is_none()
    );
    request.parallel_extraction = Some(false);
    assert!(request.validate().is_ok());
    request.parallel_extraction = Some(true);
    assert!(request.validate().is_err());
    request.version = 2;
    request.operation = "extract".into();
    request.bulk_transfer = Some(1);
    assert!(request.validate().is_ok());
    request.bulk_transfer = None;
    assert!(request.validate().is_err());
}
