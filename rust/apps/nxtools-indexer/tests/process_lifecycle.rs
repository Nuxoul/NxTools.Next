use std::io::{BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};

use nxtools_ipc::{read_bounded_sync_frame, write_envelope_sync};
use nxtools_protocol::{Envelope, Hello, IndexerEvent, MAX_FRAME_BYTES, Message};

static PROCESS_TEST_LOCK: Mutex<()> = Mutex::new(());

fn spawn_indexer() -> (tempfile::TempDir, Child, ChildStdin, BufReader<ChildStdout>) {
    let data_root = tempfile::tempdir().expect("isolated indexer data root");
    let mut child = Command::new(env!("CARGO_BIN_EXE_nxtools-indexer"))
        .env("LOCALAPPDATA", data_root.path())
        .env("APPDATA", data_root.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn indexer test process");
    let stdin = child.stdin.take().expect("indexer stdin");
    let stdout = BufReader::new(child.stdout.take().expect("indexer stdout"));
    (data_root, child, stdin, stdout)
}

fn handshake(stdin: &mut ChildStdin, stdout: &mut BufReader<ChildStdout>) {
    let hello = Envelope::new(Message::Hello(Hello {
        client_name: "process_lifecycle_test".into(),
        client_version: "0.1.0".into(),
        capabilities: vec!["index_status".into(), "file_search".into()],
    }));
    write_envelope_sync(stdin, &hello).expect("write hello");

    let ack = read_envelope(stdout);
    let status = read_envelope(stdout);
    assert!(matches!(ack.message, Message::HelloAck(_)));
    assert_eq!(ack.correlation_id, Some(hello.id));
    assert!(matches!(
        status.message,
        Message::IndexerEvent(IndexerEvent::Status(_))
    ));
}

fn read_envelope(stdout: &mut BufReader<ChildStdout>) -> Envelope {
    let frame = read_bounded_sync_frame(stdout)
        .expect("read indexer frame")
        .expect("indexer remained connected");
    Envelope::decode_line(&frame).expect("decode indexer frame")
}

fn wait_for_exit(child: &mut Child) -> std::process::ExitStatus {
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        if let Some(status) = child.try_wait().expect("inspect indexer process") {
            return status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("indexer did not exit after its desktop pipe closed");
        }
        thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn closing_the_desktop_pipe_stops_the_real_indexer_process() {
    let _serial = PROCESS_TEST_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let (_data_root, mut child, mut stdin, mut stdout) = spawn_indexer();
    handshake(&mut stdin, &mut stdout);

    drop(stdin);

    assert!(wait_for_exit(&mut child).success());
}

#[test]
fn an_oversized_real_process_frame_is_rejected_and_terminates_the_session() {
    let _serial = PROCESS_TEST_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let (_data_root, mut child, mut stdin, mut stdout) = spawn_indexer();
    handshake(&mut stdin, &mut stdout);

    let oversized = vec![b'x'; MAX_FRAME_BYTES + 1];
    let _ = stdin.write_all(&oversized);
    let _ = stdin.write_all(b"\n");
    let _ = stdin.flush();
    drop(stdin);

    assert!(!wait_for_exit(&mut child).success());
}
