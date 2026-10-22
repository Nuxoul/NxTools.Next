use nxtools_archive::worker::{WorkerRequest, read_frame, write_frame};
use serde_json::{Value, json};
use std::{
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::mpsc,
    time::{Duration, Instant},
};
use uuid::Uuid;

struct OwnedWorker(Child);
impl Drop for OwnedWorker {
    fn drop(&mut self) {
        if self.0.try_wait().ok().flatten().is_none() {
            let _ = self.0.kill();
        }
        let _ = self.0.wait();
    }
}

#[test]
fn actual_worker_credit_wait_records_received_then_observed_without_an_ack() {
    let mut child = OwnedWorker(
        Command::new(env!("CARGO_BIN_EXE_nxtools-archive-worker"))
            .arg("--archive-worker")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("actual owned Rust worker"),
    );
    let id = Uuid::new_v4();
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/stored.zip");
    let mut request = WorkerRequest::open(id, source.to_str().expect("fixture Unicode").to_owned());
    request.version = 2;
    request.read_contract = Some(1);
    write_frame(child.0.stdin.as_mut().expect("stdin"), &request)
        .expect("original private request");
    let mut output = child.0.stdout.take().expect("one stdout reader");
    let (send, receive) = mpsc::sync_channel(8);
    let reader = std::thread::spawn(move || {
        while let Some(frame) =
            read_frame::<_, Value>(&mut output).expect("actual bounded private frame")
        {
            send.send(frame).expect("bounded test observer");
        }
    });
    loop {
        let frame = receive
            .recv_timeout(Duration::from_secs(5))
            .expect("worker frame before credit barrier");
        assert_eq!(frame["operation_id"], id.to_string());
        if frame["op"] == "native_call" {
            assert_eq!(frame["native"]["action"], "source_acquire");
            // Tiny trusted checked-in input only: this is a worker control test, NOT native source/Job proof.
            write_frame(child.0.stdin.as_mut().expect("stdin"), &json!({"version":1,"operation_id":id,"op":"native_result","call_id":frame["native"]["call_id"],"success":true})).expect("existing source acquire rendezvous");
        } else if frame["op"] == "entry_batch" {
            assert_eq!(frame["batch_sequence"], 0);
            break;
        } else {
            assert_eq!(frame["op"], "accepted");
        }
    }
    let caller = Instant::now();
    write_frame(
        child.0.stdin.as_mut().expect("stdin"),
        &json!({"version":1,"operation_id":id,"op":"cancel"}),
    )
    .expect("original input cancel, no metadata ACK");
    let terminal = receive
        .recv_timeout(Duration::from_secs(2))
        .expect("real worker cancellation terminal");
    assert_eq!(terminal["op"], "failure");
    assert_eq!(terminal["error_code"], "Cancelled");
    let observation = &terminal["cancellation"];
    assert_eq!(observation["boundary"], "metadata_credit");
    let received_us = observation["received_us"]
        .as_u64()
        .expect("reader actually received");
    let observed_us = observation["observed_us"]
        .as_u64()
        .expect("work thread actually observed");
    assert!(observed_us >= received_us && observed_us - received_us <= 200000);
    drop(child.0.stdin.take());
    let deadline = Instant::now();
    while child.0.try_wait().expect("worker wait").is_none() {
        assert!(deadline.elapsed() < Duration::from_secs(2));
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(child.0.try_wait().expect("wait").expect("exited").success());
    reader.join().expect("actual stdout EOF");
    assert!(
        receive.try_recv().is_err(),
        "no business/native frame after observation terminal"
    );
    let caller_terminal_us = caller.elapsed().as_micros();
    let root = PathBuf::from(std::env::var_os("PI_SCRATCH_DIR").expect("session scratch"))
        .join(format!("archive-cancel-credit-{id}"));
    std::fs::create_dir(&root).expect("new owned evidence");
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(root.join("evidence.json"))
        .expect("create evidence");
    serde_json::to_writer_pretty(file, &json!({"operation_id":id,"worker_terminal":terminal,"caller_terminal_us":caller_terminal_us,"no_ack":true,"no_cross_clock_subtraction":true,"native_cleanup_measured_here":false})).expect("fixed diagnostic evidence");
}
