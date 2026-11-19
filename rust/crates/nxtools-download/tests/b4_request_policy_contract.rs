use std::sync::{Arc, Mutex};
use std::time::Duration;

use nxtools_core::{DownloadSettings, NetworkSettings};
use nxtools_download::{
    DOWNLOAD_COMMAND_PROTOCOL_VERSION, DownloadCommand, DownloadCommandEnvelope,
    DownloadCommandOutcome, DownloadDuplicateStrategy, DownloadEngine, DownloadHttpPolicy,
    DownloadRequest, DownloadRequestHeader, DownloadStatus, DownloadSubmitMode, NetworkPolicy,
    RequestOrigin,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const EXPECTED_AGENT: &str = "Mozilla/5.0 (Windows NT 6.1; Trident/7.0; rv:11.0) like Gecko";
const CONTENT: &[u8] = b"B4 written bytes";

struct HeaderServer {
    address: std::net::SocketAddr,
    requests: Arc<Mutex<Vec<String>>>,
    task: tokio::task::JoinHandle<()>,
}

impl HeaderServer {
    async fn start() -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind local HTTP fixture");
        let address = listener.local_addr().expect("fixture address");
        let requests = Arc::new(Mutex::new(Vec::new()));
        let observed = Arc::clone(&requests);
        let task = tokio::spawn(async move {
            while let Ok((mut stream, _)) = listener.accept().await {
                let mut bytes = Vec::new();
                loop {
                    let mut byte = [0_u8; 1];
                    if stream.read_exact(&mut byte).await.is_err() {
                        break;
                    }
                    bytes.push(byte[0]);
                    if bytes.ends_with(b"\r\n\r\n") {
                        break;
                    }
                }
                let request = String::from_utf8(bytes).expect("HTTP fixture request");
                let redirect = if request.starts_with("HEAD /same-redirect ")
                    || request.starts_with("GET /same-redirect ")
                {
                    Some("/ordinary.bin".to_owned())
                } else if request.starts_with("HEAD /cross-redirect ")
                    || request.starts_with("GET /cross-redirect ")
                {
                    Some(format!("http://localhost:{}/ordinary.bin", address.port()))
                } else {
                    None
                };
                if let Some(location) = redirect {
                    observed
                        .lock()
                        .expect("observed request lock")
                        .push(request);
                    let _ = stream.write_all(format!("HTTP/1.1 302 Found\r\nLocation: {location}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").as_bytes()).await;
                    let _ = stream.shutdown().await;
                    continue;
                }
                let head = request.starts_with("HEAD ");
                let range = request.lines().find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    if !name.eq_ignore_ascii_case("range") {
                        return None;
                    }
                    let (start, end) = value.trim().strip_prefix("bytes=")?.split_once('-')?;
                    Some((start.parse::<usize>().ok()?, end.parse::<usize>().ok()?))
                });
                observed
                    .lock()
                    .expect("observed request lock")
                    .push(request);
                let (status, extra, body) = if let Some((start, end)) = range {
                    (
                        "206 Partial Content",
                        format!("Content-Range: bytes {start}-{end}/{}\r\n", CONTENT.len()),
                        &CONTENT[start..=end],
                    )
                } else {
                    ("200 OK", String::new(), CONTENT)
                };
                let headers = format!(
                    "HTTP/1.1 {status}\r\n{extra}Content-Length: {}\r\nAccept-Ranges: bytes\r\nETag: \"b4-headers\"\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                let _ = stream.write_all(headers.as_bytes()).await;
                if !head {
                    let _ = stream.write_all(body).await;
                }
                let _ = stream.shutdown().await;
            }
        });
        Self {
            address,
            requests,
            task,
        }
    }

    async fn close(self) -> Vec<String> {
        self.task.abort();
        assert!(
            self.task
                .await
                .expect_err("fixture stops on abort")
                .is_cancelled()
        );
        Arc::try_unwrap(self.requests)
            .expect("all fixture request owners drained")
            .into_inner()
            .expect("observed requests")
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn b4_default_headers_reach_actual_probe_and_transfer() {
    let server = HeaderServer::start().await;
    let directory = tempfile::tempdir().expect("owned download directory");
    let settings = DownloadSettings::default();
    let engine = DownloadEngine::open_with_policies(
        directory.path().join("downloads.sqlite3"),
        NetworkPolicy::new(NetworkSettings::default()).expect("network policy"),
        DownloadHttpPolicy::from_download_settings(&settings).expect("download HTTP policy"),
    )
    .expect("original SQLite engine");
    let request = DownloadRequest {
        source_url: format!("http://{}/ordinary.bin", server.address),
        save_directory: directory.path().to_owned(),
        save_directory_explicit: false,
        file_name: Some("ordinary.bin".into()),
        headers: Vec::new(),
        cookie: None,
        referrer: None,
        source_page: None,
        certificate_policy: Default::default(),
        connection_limit: Some(1),
        speed_limit_bytes_per_second: None,
        expected_total_bytes: None,
        scheduled_at_unix_ms: None,
        adaptive_segment_policy: None,
        media: None,
        category_id: None,
        category_name: None,
        duplicate_strategy: DownloadDuplicateStrategy::UseDefault,
    };
    let task_id = engine
        .enqueue(request, RequestOrigin::Ui)
        .expect("original admission");
    engine.resume(task_id).expect("start original transfer");
    let terminal = engine
        .wait_for_terminal(task_id, Duration::from_secs(5))
        .await
        .expect("actual terminal");
    assert_eq!(terminal.task.status(), DownloadStatus::Completed);
    assert_eq!(
        std::fs::read(directory.path().join("ordinary.bin")).expect("written output"),
        CONTENT
    );
    drop(engine);
    let requests = server.close().await;
    assert!(
        requests.iter().any(|request| request.starts_with("HEAD ")),
        "actual probe"
    );
    assert!(
        requests
            .iter()
            .any(|request| request.starts_with("GET ") && !request.contains("bytes=0-0\r\n")),
        "actual content transfer"
    );
    for request in requests {
        let agents: Vec<_> = request
            .lines()
            .filter_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("user-agent")
                    .then(|| value.trim())
            })
            .collect();
        assert_eq!(
            agents,
            vec![EXPECTED_AGENT],
            "one exact UA on probe and transfer"
        );
    }
}

fn request(server: &HeaderServer, directory: &std::path::Path, path: &str) -> DownloadRequest {
    DownloadRequest {
        source_url: format!("http://{}/{path}", server.address),
        save_directory: directory.to_owned(),
        save_directory_explicit: false,
        file_name: Some("ordinary.bin".into()),
        headers: Vec::new(),
        cookie: None,
        referrer: None,
        source_page: None,
        certificate_policy: Default::default(),
        connection_limit: Some(1),
        speed_limit_bytes_per_second: None,
        expected_total_bytes: None,
        scheduled_at_unix_ms: None,
        adaptive_segment_policy: None,
        media: None,
        category_id: None,
        category_name: None,
        duplicate_strategy: DownloadDuplicateStrategy::UseDefault,
    }
}

fn header_values<'a>(request: &'a str, wanted: &str) -> Vec<&'a str> {
    request
        .lines()
        .filter_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case(wanted).then(|| value.trim())
        })
        .collect()
}

fn assert_actual_transfer(requests: &[String], expected_agent: &str) {
    assert!(
        requests.iter().any(|request| request.starts_with("HEAD ")),
        "actual probe"
    );
    assert!(
        requests
            .iter()
            .any(|request| request.starts_with("GET ") && !request.contains("bytes=0-0\r\n")),
        "actual content transfer"
    );
    for request in requests {
        assert_eq!(
            header_values(request, "user-agent"),
            vec![expected_agent],
            "one exact UA on every probe and transfer hop"
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn b4_custom_and_explicit_agents_freeze_across_policy_update_and_sqlite_reopen() {
    for variant in ["global", "task", "browser", "legacy-whitespace"] {
        let server = HeaderServer::start().await;
        let directory = tempfile::tempdir().expect("owned download directory");
        let database = directory.path().join("downloads.sqlite3");
        let settings = DownloadSettings {
            default_request_headers: if variant == "legacy-whitespace" {
                " \r\n\t".into()
            } else {
                "User-Agent: global-custom\nX-Global: retained".into()
            },
            default_cookie: "global-cookie=1".into(),
            ..DownloadSettings::default()
        };
        let engine = DownloadEngine::open_with_policies(
            &database,
            NetworkPolicy::new(NetworkSettings::default()).expect("network policy"),
            DownloadHttpPolicy::from_download_settings(&settings).expect("HTTP policy"),
        )
        .expect("original engine");
        let mut request = request(&server, directory.path(), "ordinary.bin");
        request.cookie = Some("task-cookie=1".into());
        request.referrer = Some("https://captured.example/source".into());
        let expected_agent = match variant {
            "task" => "explicit-task",
            "browser" => "explicit-browser",
            "legacy-whitespace" => EXPECTED_AGENT,
            _ => "global-custom",
        };
        if matches!(variant, "task" | "browser") {
            request.headers.push(DownloadRequestHeader {
                name: "uSeR-aGeNt".into(),
                value: expected_agent.into(),
            });
        }
        let task_id = if variant == "browser" {
            let reply = engine.handle_command(DownloadCommandEnvelope {
                protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
                request_id: uuid::Uuid::new_v4(),
                origin: RequestOrigin::Browser,
                command: DownloadCommand::BrowserSubmit {
                    handoff_id: uuid::Uuid::new_v4(),
                    request,
                    mode: DownloadSubmitMode::QueueOnly,
                },
            });
            let DownloadCommandOutcome::BrowserAccepted {
                task_id,
                duplicate: false,
            } = reply.outcome
            else {
                panic!("original browser admission: {:?}", reply.outcome);
            };
            task_id
        } else {
            engine
                .enqueue(request, RequestOrigin::Ui)
                .expect("ordinary admission")
        };
        let frozen = engine
            .task(task_id)
            .expect("query")
            .expect("task")
            .task
            .request()
            .clone();
        let changed = DownloadSettings {
            default_request_headers: "User-Agent: changed-after-acceptance".into(),
            default_cookie: "changed-cookie=1".into(),
            ..DownloadSettings::default()
        };
        engine
            .update_download_http_policy(&changed)
            .expect("update original policy");
        assert_eq!(
            engine
                .task(task_id)
                .expect("query")
                .expect("task")
                .task
                .request(),
            &frozen
        );
        drop(engine);
        let reopened = DownloadEngine::open_with_policies(
            &database,
            NetworkPolicy::new(NetworkSettings::default()).expect("network policy"),
            DownloadHttpPolicy::from_download_settings(&changed).expect("later policy"),
        )
        .expect("reopen original SQLite");
        assert_eq!(
            reopened
                .task(task_id)
                .expect("query")
                .expect("task")
                .task
                .request(),
            &frozen
        );
        reopened.resume(task_id).expect("resume original writer");
        let terminal = reopened
            .wait_for_terminal(task_id, Duration::from_secs(5))
            .await
            .expect("terminal");
        assert_eq!(terminal.task.status(), DownloadStatus::Completed);
        assert_eq!(
            std::fs::read(directory.path().join("ordinary.bin")).expect("written bytes"),
            CONTENT
        );
        assert_eq!(terminal.task.request(), &frozen);
        drop(reopened);
        let requests = server.close().await;
        assert_actual_transfer(&requests, expected_agent);
        for request in requests {
            assert_eq!(header_values(&request, "cookie"), vec!["task-cookie=1"]);
            assert_eq!(
                header_values(&request, "referer"),
                vec!["https://captured.example/source"]
            );
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn b4_default_agent_keeps_original_redirect_cookie_and_referrer_policy() {
    for cross_site in [false, true] {
        let server = HeaderServer::start().await;
        let directory = tempfile::tempdir().expect("owned download directory");
        let engine = DownloadEngine::open(directory.path().join("downloads.sqlite3"))
            .expect("original SQLite engine");
        let mut request = request(
            &server,
            directory.path(),
            if cross_site {
                "cross-redirect"
            } else {
                "same-redirect"
            },
        );
        request.cookie = Some("captured-cookie=1".into());
        request.referrer = Some("https://captured.example/source".into());
        let task_id = engine
            .enqueue(request, RequestOrigin::Ui)
            .expect("original admission");
        engine.resume(task_id).expect("start transfer");
        assert_eq!(
            engine
                .wait_for_terminal(task_id, Duration::from_secs(5))
                .await
                .expect("terminal")
                .task
                .status(),
            DownloadStatus::Completed
        );
        assert_eq!(
            std::fs::read(directory.path().join("ordinary.bin")).expect("written bytes"),
            CONTENT
        );
        drop(engine);
        let requests = server.close().await;
        assert_actual_transfer(&requests, EXPECTED_AGENT);
        assert!(
            requests
                .iter()
                .any(|request| request.contains(" /ordinary.bin HTTP/")),
            "actual redirect target"
        );
        for request in requests {
            let target = request.contains(" /ordinary.bin HTTP/");
            let direct_transfer = target
                && request.starts_with("GET ")
                && header_values(&request, "range")
                    .iter()
                    .any(|range| *range != "bytes=0-0");
            assert_eq!(
                header_values(&request, "cookie"),
                if cross_site && target && !direct_transfer {
                    vec![]
                } else {
                    vec!["captured-cookie=1"]
                },
                "redirect hops strip cross-site Cookie; direct final-range requests retain accepted task context"
            );
            assert_eq!(
                header_values(&request, "referer"),
                vec!["https://captured.example/source"],
                "no automatic Referer rewrite"
            );
        }
    }
}
