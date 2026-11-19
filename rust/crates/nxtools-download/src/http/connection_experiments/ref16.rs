use super::*;
use reqwest::dns::{Addrs, Name, Resolve, Resolving};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

const SIZE: u64 = 1024 * 1024;

#[derive(Default)]
struct State {
    setups_us: Vec<u64>,
    resolutions: usize,
}
pub(crate) struct Observation {
    answers: Vec<SocketAddr>,
    state: Mutex<State>,
}
impl Observation {
    fn new(scenario: &str) -> Arc<Self> {
        let addresses: &[&str] = match scenario {
            "stable" | "proxy" => &["127.0.0.1:0"],
            "family6" => &["[::1]:0", "127.0.0.1:0"],
            "family4" => &["127.0.0.2:0", "[::1]:0"],
            "unreachable" => &["127.0.0.3:0", "127.0.0.1:0"],
            _ => &["127.0.0.2:0", "127.0.0.1:0"],
        };
        Arc::new(Self {
            answers: addresses
                .iter()
                .map(|value| value.parse().expect("fixed socket address"))
                .collect(),
            state: Mutex::new(State::default()),
        })
    }
    pub(crate) fn record(&self, _response: &Response, elapsed: Duration) {
        let mut state = self.state.lock().expect("observation lock");
        state.setups_us.push(elapsed.as_micros() as u64);
    }
}
impl Resolve for Observation {
    fn resolve(&self, _name: Name) -> Resolving {
        let answers = self.answers.clone();
        let mut state = self.state.lock().expect("resolver lock");
        state.resolutions += 1;
        Box::pin(async move { Ok(Box::new(answers.into_iter()) as Addrs) })
    }
}
fn client(observer: &Arc<Observation>, proxy_port: Option<u16>) -> HttpDownloadClient {
    let mut settings = NetworkSettings::default();
    settings.proxy.mode = if proxy_port.is_some() {
        NetworkProxyMode::ManualHttp
    } else {
        NetworkProxyMode::Direct
    };
    if let Some(port) = proxy_port {
        settings.proxy.host = "127.0.0.1".into();
        settings.proxy.port = port;
    }
    let policy = NetworkPolicy::new(settings).expect("owned policy");
    let resolver: Arc<dyn Resolve> = observer.clone();
    let mut client = HttpDownloadClient::build_for_snapshot(
        DEFAULT_STALL_TIMEOUT,
        &policy.snapshot().expect("snapshot"),
        DownloadCertificatePolicy::Strict,
        Some(resolver),
    )
    .expect("original connector");
    client.fixture_address_observer = Some(Arc::clone(observer));
    client
}
fn probe(request: &DownloadRequest) -> DownloadProbe {
    DownloadProbe {
        final_url: request.source_url.clone(),
        file_name: request.file_name.clone(),
        total_bytes: Some(SIZE),
        supports_ranges: false,
        content_type: None,
        validator: ResourceValidator::default(),
        media: None,
        resolution: None,
        server_observation: None,
    }
}
fn exact(path: &Path) {
    let mut file = std::fs::File::open(path).expect("completed file");
    assert_eq!(file.metadata().expect("file length").len(), SIZE);
    let mut buffer = [0_u8; 65536];
    let mut offset = 0_usize;
    loop {
        let count = file.read(&mut buffer).expect("outside-timing read");
        if count == 0 {
            break;
        }
        for (index, byte) in buffer[..count].iter().enumerate() {
            assert_eq!(
                *byte,
                ((offset + index) % 251) as u8,
                "exact offset {}",
                offset + index
            );
        }
        offset += count;
    }
    assert_eq!(offset as u64, SIZE);
}
fn server(scenario: &str) -> Server {
    Server::start_fixture(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/support/b6_address_server.py"
        ),
        scenario,
    )
}
async fn idle(server: &mut Server) -> serde_json::Value {
    let started = Instant::now();
    loop {
        let stats = server.command("stats");
        assert!(stats["failure"].is_null(), "owned fixture {stats}");
        if stats["pending_now"] == 0 && stats["open_sockets"] == 0 {
            return stats;
        }
        assert!(
            started.elapsed() < Duration::from_secs(1),
            "actual connects must retire {stats}"
        );
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}
async fn run(scenario: &str) -> serde_json::Value {
    let directory = tempfile::tempdir().expect("owned artifact root");
    let mut server = server(scenario);
    let observer = Observation::new(scenario);
    let client = client(&observer, (scenario == "proxy").then_some(server.port));
    let ports = server.command("stats")["ports"]
        .as_array()
        .expect("owned ports")
        .iter()
        .map(|value| u16::try_from(value.as_u64().expect("port")).expect("u16 port"))
        .collect::<Vec<_>>();
    let mut paths = Vec::new();
    let mut transfer_us = Vec::new();
    println!("B6_MEASUREMENT_START");
    let started = Instant::now();
    for index in 0..4 {
        let mut request = super::request(directory.path(), ports[index % ports.len()]);
        request.source_url = format!("http://b6.address.test:{}/file", ports[index % ports.len()]);
        request.file_name = Some(format!("owned-{index}.bin"));
        let id = DownloadTaskId::new();
        let probe = probe(&request);
        let (temporary, output) =
            HttpDownloadClient::artifact_paths(&request, &probe, id).expect("original paths");
        let transfer = Instant::now();
        let outcome = tokio::time::timeout(
            Duration::from_secs(120),
            client.download(
                &request,
                &probe,
                id,
                &DownloadTransferControl::new(),
                |_| {},
            ),
        )
        .await
        .expect("bounded transfer")
        .expect("original streamed download");
        transfer_us.push(transfer.elapsed().as_micros() as u64);
        assert!(matches!(
            outcome,
            DownloadTransferOutcome::Completed {
                downloaded_bytes: SIZE,
                ..
            }
        ));
        assert!(!temporary.exists());
        paths.push(output);
    }
    let elapsed_us = started.elapsed().as_micros() as u64;
    println!("B6_MEASUREMENT_END");
    for path in paths {
        exact(&path);
    }
    let client_errors = client.fixture_error_counts();
    let (setups_us, resolutions) = {
        let state = observer.state.lock().expect("completed observation");
        (state.setups_us.clone(), state.resolutions)
    };
    assert_eq!(setups_us.len(), 4);
    drop(client);
    idle(&mut server).await;
    let stats = server.finish();
    assert_eq!(stats["requests"], 4);
    assert_eq!(stats["accepted"], 4);
    assert_eq!(stats["served_bytes"], 4 * SIZE);
    if scenario == "primary" || scenario == "cross_port" || scenario.starts_with("family") {
        assert!(
            stats["pending_attempts"]
                .as_u64()
                .expect("actual TCP attempts")
                > 0,
            "must prove original pending TCP"
        );
    }
    if scenario == "proxy" {
        assert_eq!(stats["proxy_requests"], 4);
        assert_eq!(resolutions, 0);
    }
    serde_json::json!({"scenario": scenario, "version": "current",
        "elapsed_us": elapsed_us, "transfer_us": transfer_us, "setups_us": setups_us,
        "resolutions": resolutions, "exact_output": true,
        "server": stats, "server_joined": true, "reader_joined": true,
        "tcp_retransmitted_bytes": null, "client_errors": client_errors})
}
#[tokio::test]
#[ignore = "serial controlled multi-address measurement"]
async fn ref16_trial() {
    let scenario = std::env::var("B6_SCENARIO").expect("explicit scenario");
    assert!(
        [
            "primary",
            "cross_port",
            "stable",
            "family6",
            "family4",
            "unreachable",
            "proxy"
        ]
        .contains(&scenario.as_str())
    );
    println!("B6_RESULT={}", run(&scenario).await);
}
#[tokio::test]
async fn ref16_current_same_family_pending_connect_falls_back_and_preserves_exact_output() {
    println!("B6_REF16_CURRENT={}", run("primary").await);
}

#[tokio::test]
async fn ref16_pending_cancel_releases_original_connect_without_artifact_or_late_request() {
    let directory = tempfile::tempdir().expect("cancel root");
    let mut server = server("primary");
    let observer = Observation::new("primary");
    let client = client(&observer, None);
    let mut request = super::request(directory.path(), server.port);
    request.source_url = format!("http://b6.address.test:{}/file", server.port);
    let id = DownloadTaskId::new();
    let probe = probe(&request);
    let control = DownloadTransferControl::new();
    let future = client.download(&request, &probe, id, &control, |_| {});
    tokio::pin!(future);
    tokio::select! {
        _ = &mut future => panic!("owned first connect must remain pending"),
        () = tokio::time::sleep(Duration::from_millis(250)) => {}
    }
    assert!(
        server.command("stats")["pending_now"]
            .as_u64()
            .expect("native pending")
            > 0
    );
    let retirement = Instant::now();
    control.cancel();
    assert!(matches!(
        future.await.expect("cancel response"),
        DownloadTransferOutcome::Cancelled {
            downloaded_bytes: 0
        }
    ));
    let stats = idle(&mut server).await;
    assert_eq!(stats["requests"], 0);
    assert_eq!(
        std::fs::read_dir(directory.path()).expect("root").count(),
        0
    );
    assert!(retirement.elapsed() < Duration::from_secs(1));
    let stats = server.finish();
    println!(
        "B6_REF16_CANCEL={}",
        serde_json::json!({"retirement_us": retirement.elapsed().as_micros() as u64, "server": stats, "no_artifact": true, "actual_join": true})
    );
}

#[tokio::test]
async fn ref16_tls_policy_connect_sni_and_explicit_proxy_preserve_the_original_boundary() {
    for proxy in [false, true] {
        let directory = tempfile::tempdir().expect("TLS output root");
        let mut server = Server::start_fixture(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/support/b6_tls_server.py"
            ),
            if proxy { "proxy" } else { "direct" },
        );
        let observer = Observation::new("stable");
        let mut settings = NetworkSettings::default();
        settings.proxy.mode = if proxy {
            NetworkProxyMode::ManualHttp
        } else {
            NetworkProxyMode::Direct
        };
        settings.proxy.host = "127.0.0.1".into();
        settings.proxy.port = server.port;
        let policy = NetworkPolicy::new(settings).expect("TLS policy");
        let origin_port = if proxy {
            u16::try_from(
                server.command("stats")["origin_port"]
                    .as_u64()
                    .expect("canary port"),
            )
            .expect("u16 port")
        } else {
            server.port
        };
        for certificate in [
            DownloadCertificatePolicy::Strict,
            DownloadCertificatePolicy::AcceptInvalid,
        ] {
            let resolver: Arc<dyn Resolve> = observer.clone();
            let mut client = HttpDownloadClient::build_for_snapshot(
                DEFAULT_STALL_TIMEOUT,
                &policy.snapshot().expect("TLS snapshot"),
                certificate,
                Some(resolver),
            )
            .expect("original TLS client");
            client.fixture_address_observer = Some(Arc::clone(&observer));
            let mut request = super::request(directory.path(), origin_port);
            request.source_url = format!("https://b6.tls.test:{origin_port}/file");
            request.certificate_policy = certificate;
            let probe = probe(&request);
            let result = client
                .download(
                    &request,
                    &probe,
                    DownloadTaskId::new(),
                    &DownloadTransferControl::new(),
                    |_| {},
                )
                .await;
            if certificate == DownloadCertificatePolicy::Strict {
                assert!(
                    matches!(result, Err(HttpDownloadError::Client(_))),
                    "self-signed certificate must be rejected"
                );
                assert!(!directory.path().join("owned.bin").exists());
            } else {
                assert!(matches!(
                    result.expect("explicit TLS opt-in"),
                    DownloadTransferOutcome::Completed {
                        downloaded_bytes: SIZE,
                        ..
                    }
                ));
                exact(&directory.path().join("owned.bin"));
            }
            drop(client);
            server.idle();
        }
        let stats = server.finish();
        assert_eq!(stats["accepted"], 2);
        assert_eq!(stats["requests"], 1);
        assert_eq!(stats["served_bytes"], SIZE);
        assert_eq!(stats["sni_matches"], 2);
        assert_eq!(stats["tls_rejections"], 1);
        assert_eq!(stats["origin_canary_connections"], 0);
        assert_eq!(stats["connect_requests"], if proxy { 2 } else { 0 });
        if proxy {
            assert_eq!(
                observer
                    .state
                    .lock()
                    .expect("proxy observations")
                    .resolutions,
                0
            );
        }
        println!(
            "B6_REF16_TLS={}",
            serde_json::json!({"proxy": proxy, "server": stats, "exact_output": true, "actual_join": true})
        );
    }
}

#[tokio::test]
async fn ref16_current_cross_port_order_does_not_inherit_a_peer_from_another_port() {
    let result = run("cross_port").await;
    let setups = result["setups_us"]
        .as_array()
        .expect("actual setup durations");
    assert!(setups[1].as_u64().expect("second port setup") < 250_000);
    assert!(setups[3].as_u64().expect("second port repeated setup") < 250_000);
    assert_eq!(result["server"]["pending_attempts"], 2);
    println!("B6_REF16_CROSS_PORT={result}");
}
