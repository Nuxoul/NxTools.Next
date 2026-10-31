//! Real command/result/terminal output over an owned synthetic pipe, captured in a subprocess.
use super::*;
use nxtools_download::DownloadCommandResult;
use nxtools_download_ipc::{DOWNLOAD_IPC_PROTOCOL_VERSION, DownloadIpcFrame};
use nxtools_ipc::{accept_current_user_pipe, read_bounded_sync_frame};
use std::io::Write;

const CHILD_CASE: &str = "NXTOOLS_CLI_REPORTED_FAULT_FIXTURE";
const RESPONSE_ID: &str = "12345678-1234-4abc-8def-1234567890ab";
const MESSAGE: &str = "synthetic rejection 单次报告";

#[test]
fn reported_fault_child() {
    let Ok(case) = std::env::var(CHILD_CASE) else {
        return;
    };
    let fields = case.split(':').collect::<Vec<_>>();
    assert_eq!(fields.len(), 3);
    let verb = fields[0].to_owned();
    let json_output = fields[1] == "json";
    let code: DownloadFaultCode =
        serde_json::from_str(&format!("\"{}\"", fields[2])).expect("fixture fault code");
    let pipe = format!(r"\\.\pipe\nxtools-cli-output-{}", Uuid::new_v4());
    let server_pipe = pipe.clone();
    let expected_verb = verb.clone();
    let server = std::thread::spawn(move || {
        let mut connection = accept_current_user_pipe(&server_pipe).expect("owned temporary pipe");
        connection
            .reader
            .get_mut()
            .set_initial_frame_timeout(Duration::from_secs(3));
        let hello = read_bounded_sync_frame(&mut connection.reader)
            .expect("bounded hello read")
            .expect("hello");
        assert!(matches!(
            serde_json::from_str::<DownloadIpcFrame>(&hello).expect("hello frame"),
            DownloadIpcFrame::Hello { .. }
        ));
        let ack = DownloadIpcFrame::HelloAck {
            protocol_version: DOWNLOAD_IPC_PROTOCOL_VERSION,
            server_version: "fixture".into(),
            host_snapshot: None,
        };
        writeln!(
            connection.writer,
            "{}",
            serde_json::to_string(&ack).expect("hello JSON")
        )
        .expect("ack write");
        connection.writer.flush().expect("ack flush");
        connection
            .reader
            .get_mut()
            .set_initial_frame_timeout(Duration::from_secs(3));
        let frame = read_bounded_sync_frame(&mut connection.reader)
            .expect("bounded command read")
            .expect("command");
        let DownloadIpcFrame::Command(command) =
            serde_json::from_str(&frame).expect("command frame")
        else {
            panic!("expected command");
        };
        assert!(
            matches!(
                (expected_verb.as_str(), &command.command),
                (
                    "schedule",
                    DownloadCommand::Queue {
                        action: DownloadQueueAction::SetSchedule { .. }
                    }
                ) | ("add", DownloadCommand::Submit { .. })
                    | ("list", DownloadCommand::Snapshot { .. })
            ),
            "real intended command reaches pipe"
        );
        let result = DownloadIpcFrame::Result(DownloadCommandResult {
            request_id: Uuid::parse_str(RESPONSE_ID).expect("fixed response identity"),
            outcome: DownloadCommandOutcome::Rejected {
                fault: DownloadFault {
                    code,
                    message: MESSAGE.into(),
                    retryable: true,
                },
            },
        });
        writeln!(
            connection.writer,
            "{}",
            serde_json::to_string(&result).expect("result JSON")
        )
        .expect("result write");
        connection.writer.flush().expect("result flush");
    });
    let mut client = DownloadIpcClient::connect(&pipe, Duration::from_secs(3))
        .expect("connect only owned fixture");
    let command = match verb.as_str() {
        "schedule" => CliCommand::Schedule {
            queue_id: "fixture".into(),
            schedule: None,
            json: json_output,
        },
        "add" => CliCommand::Add {
            url: "https://synthetic.invalid/file.bin".into(),
            directory: PathBuf::from("fixture"),
            directory_explicit: true,
            name: Some("file.bin".into()),
            headers: Vec::new(),
            idempotency_key: None,
            json: json_output,
        },
        "list" => CliCommand::List {
            filter: ListFilter::All,
            json: json_output,
        },
        _ => panic!("unexpected fixture verb"),
    };
    let error = run_connected_command(command, &mut client).expect_err("synthetic rejected result");
    server.join().expect("fixture server completes");
    // Same unchanged final reporting rule as src/main.rs; capture the actual process exit.
    if !error.message.is_empty() {
        eprintln!("{}", error.message);
    }
    std::process::exit(i32::from(error.exit_code));
}

fn captured_case(verb: &str, mode: &str, code: &str) -> std::process::Output {
    let mut child = Command::new(std::env::current_exe().expect("CLI test executable"))
        .args([
            "--exact",
            "output_tests::reported_fault_child",
            "--nocapture",
            "--format=terse",
        ])
        .env(CHILD_CASE, format!("{verb}:{mode}:{code}"))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("owned output fixture process");
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if child.try_wait().expect("fixture exit state").is_some() {
            break;
        }
        if Instant::now() >= deadline {
            child.kill().expect("kill only fixture child");
            let output = child.wait_with_output().expect("drain timed-out fixture");
            panic!(
                "output fixture timed out: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    child.wait_with_output().expect("captured process output")
}

#[test]
fn reported_fault_command_output_is_once_with_exact_identity_and_stable_exits() {
    for verb in ["schedule", "add", "list"] {
        for mode in ["text", "json"] {
            for (code, common_exit) in [
                ("not_found", 4),
                ("invalid_state", 5),
                ("protocol", 8),
                ("unavailable", 3),
                ("invalid_request", 7),
                ("network", 7),
            ] {
                let output = captured_case(verb, mode, code);
                let expected_exit =
                    if verb == "add" && matches!(code, "invalid_request" | "network") {
                        6
                    } else {
                        common_exit
                    };
                assert_eq!(
                    output.status.code(),
                    Some(expected_exit),
                    "{verb}/{mode}/{code} exit"
                );
                let stdout = String::from_utf8(output.stdout).expect("UTF8 stdout");
                let stderr = String::from_utf8(output.stderr).expect("UTF8 stderr");
                if mode == "json" {
                    assert!(
                        stderr.is_empty(),
                        "{verb}/{code} JSON must not also print terminal text: {stderr}"
                    );
                    let records = stdout
                        .lines()
                        .filter(|line| line.starts_with('{'))
                        .collect::<Vec<_>>();
                    assert_eq!(records.len(), 1, "{verb}/{code} one JSON record: {stdout}");
                    let value: Value = serde_json::from_str(records[0]).expect("fault envelope");
                    assert_eq!(
                        value,
                        json!({ "schema_version": 1, "request_id": RESPONSE_ID, "ok": false,
                        "error": { "code": code, "message": MESSAGE, "retryable": true } }),
                        "exact original JSON schema"
                    );
                    assert_eq!(stdout.matches(MESSAGE).count(), 1);
                } else {
                    assert_eq!(
                        stderr,
                        format!("{MESSAGE}\n"),
                        "{verb}/{code} text emitted exactly once"
                    );
                    assert!(
                        !stdout.contains(MESSAGE)
                            && !stdout.lines().any(|line| line.starts_with('{')),
                        "text mode no JSON output"
                    );
                }
            }
        }
    }
}
