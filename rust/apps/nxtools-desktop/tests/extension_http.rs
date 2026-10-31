//! Owned loopback fixtures only. No product/GPUI launch, system proxy lookup or external network.
#[path = "../src/extensions/http_client.rs"]
mod http_client;
use http_client::ExtensionHttpClient;
use nxtools_core::{NetworkProxyMode, NetworkProxySettings, NetworkSettings};
use nxtools_extensions::{
    InvocationIdentity, NetworkDeclaration,
    blobs::{BlobStore, MAX_GLOBAL_BLOB_BYTES},
    http::HttpRun,
};
use nxtools_network::NetworkPolicy;
use serde_json::{Value, json};
use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    thread,
    time::{Duration, Instant},
};
use tokio::sync::watch;
use uuid::Uuid;
struct Server {
    address: SocketAddr,
    calls: Arc<AtomicUsize>,
    requests: Arc<Mutex<Vec<Vec<u8>>>>,
    stop: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<()>>,
}
impl Server {
    fn new(reply: impl Fn(usize, &[u8], &mut TcpStream) + Send + 'static) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("owned listener");
        listener.set_nonblocking(true).expect("bounded accept");
        let address = listener.local_addr().expect("local endpoint");
        let calls = Arc::new(AtomicUsize::new(0));
        let requests = Arc::new(Mutex::new(Vec::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let counted = calls.clone();
        let captured = requests.clone();
        let done = stop.clone();
        let worker = thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(8);
            while !done.load(Ordering::Acquire) && Instant::now() < deadline {
                match listener.accept() {
                    Ok((mut socket, _)) => {
                        socket.set_nonblocking(false).expect("accepted blocking IO");
                        socket
                            .set_read_timeout(Some(Duration::from_secs(2)))
                            .expect("read bound");
                        socket
                            .set_write_timeout(Some(Duration::from_secs(2)))
                            .expect("write bound");
                        let mut bytes = Vec::new();
                        let mut buffer = [0u8; 4096];
                        let mut expected = None;
                        loop {
                            match socket.read(&mut buffer) {
                                Ok(0) => break,
                                Ok(length) => bytes.extend_from_slice(&buffer[..length]),
                                Err(_) => break,
                            }
                            if bytes.len() > 3 * 1024 * 1024 {
                                break;
                            }
                            if expected.is_none()
                                && let Some(end) =
                                    bytes.windows(4).position(|part| part == b"\r\n\r\n")
                            {
                                let header =
                                    String::from_utf8_lossy(&bytes[..end]).to_ascii_lowercase();
                                let body = header
                                    .lines()
                                    .find_map(|line| {
                                        line.strip_prefix("content-length:")
                                            .and_then(|n| n.trim().parse::<usize>().ok())
                                    })
                                    .unwrap_or(0);
                                expected = Some(end + 4 + body);
                            }
                            if expected.is_some_and(|length| bytes.len() >= length) {
                                break;
                            }
                        }
                        let number = counted.fetch_add(1, Ordering::AcqRel);
                        captured
                            .lock()
                            .expect("synthetic captures")
                            .push(bytes.clone());
                        reply(number, &bytes, &mut socket);
                    }
                    Err(failure) if failure.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(2))
                    }
                    Err(_) => break,
                }
            }
        });
        Self {
            address,
            calls,
            requests,
            stop,
            worker: Some(worker),
        }
    }
    fn url(&self, path: &str) -> String {
        format!("http://{}{path}", self.address)
    }
    fn count(&self) -> usize {
        self.calls.load(Ordering::Acquire)
    }
    fn request(&self, index: usize) -> String {
        String::from_utf8(self.requests.lock().expect("captures")[index].clone())
            .expect("synthetic ASCII request")
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            worker.join().expect("bounded server exited");
        }
    }
}
fn reply(socket: &mut TcpStream, status: u16, headers: &str, body: &[u8]) {
    let header = format!(
        "HTTP/1.1 {status} Fixture\r\nConnection: close\r\nContent-Length: {}\r\n{headers}\r\n",
        body.len()
    );
    let _ = socket.write_all(header.as_bytes());
    let _ = socket.write_all(body);
}
fn direct() -> NetworkPolicy {
    NetworkPolicy::new(NetworkSettings {
        proxy: NetworkProxySettings {
            mode: NetworkProxyMode::Direct,
            ..Default::default()
        },
    })
    .expect("synthetic direct")
}
fn manual(address: SocketAddr) -> NetworkSettings {
    NetworkSettings {
        proxy: NetworkProxySettings {
            mode: NetworkProxyMode::ManualHttp,
            host: address.ip().to_string(),
            port: address.port(),
        },
    }
}
struct Call {
    _root: tempfile::TempDir,
    store: BlobStore,
    blobs: nxtools_extensions::blobs::BlobRun,
    identity: InvocationIdentity,
    run: HttpRun,
    cancel: watch::Sender<bool>,
    cancellation: watch::Receiver<bool>,
}
impl Call {
    fn new(origins: Vec<String>) -> Self {
        let root = tempfile::tempdir().expect("owned body backing");
        let store = BlobStore::new(root.path().join("backing"));
        let identity = InvocationIdentity {
            run_id: Uuid::new_v4(),
            correlation_id: Uuid::new_v4(),
            generation: 1,
            package_id: "fixture.http".into(),
            handler_id: "run".into(),
            content_hash: "a".repeat(64),
        };
        let blobs = store.invocation(identity.clone(), Uuid::new_v4());
        let declaration: NetworkDeclaration =
            serde_json::from_value(json!({"origins":origins})).expect("declared loopback origins");
        let run = HttpRun::new(
            identity.clone(),
            declaration,
            tokio::time::Instant::now() + Duration::from_secs(5),
        );
        let (cancel, cancellation) = watch::channel(false);
        Self {
            _root: root,
            store,
            blobs,
            identity,
            run,
            cancel,
            cancellation,
        }
    }
    async fn request(
        &self,
        client: &ExtensionHttpClient,
        params: Value,
    ) -> nxtools_extensions::HostReply {
        client
            .request(
                &self.identity,
                &self.run,
                &self.blobs,
                self.cancellation.clone(),
                params,
            )
            .await
    }
    async fn read(&self, result: &Value) -> Vec<u8> {
        let id =
            serde_json::from_value(result["body_blob_id"].clone()).expect("opaque response blob");
        let body = self
            .blobs
            .host_read_sealed(id, 8 * 1024 * 1024)
            .await
            .expect("original sealed backing");
        self.blobs
            .release(id)
            .await
            .expect("original physical release");
        body
    }
    async fn cleanup(&self) {
        self.blobs.retire();
        tokio::time::timeout(Duration::from_secs(3), async {
            loop {
                let _ = self.store.cleanup().await;
                if self.store.available_bytes() == MAX_GLOBAL_BLOB_BYTES {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("actual resource release");
    }
}
#[tokio::test]
async fn expansion_http_client_direct_valid_http_errors_no_cookie_retention_and_head() {
    let server = Server::new(|_, request, socket| {
        if request.starts_with(b"HEAD ") {
            let _ = socket
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 999\r\nConnection: close\r\n\r\n");
        } else {
            reply(
                socket,
                418,
                "Set-Cookie: synthetic=not-retained\r\nX-Test: visible\r\n",
                b"synthetic error body",
            );
        }
    });
    let client = ExtensionHttpClient::new(direct());
    let call = Call::new(vec![server.url("")]);
    for _ in 0..2 {
        let result = call
            .request(&client, json!({"url":server.url("/error"),"method":"GET"}))
            .await;
        assert!(
            result.success,
            "fixed code only: {}",
            result.value.get("code").unwrap_or(&Value::Null)
        );
        assert_eq!(result.value["status"], 418);
        assert_eq!(call.read(&result.value).await, b"synthetic error body");
        assert!(
            result.value["headers"]
                .as_array()
                .expect("headers")
                .iter()
                .all(|header| header["name"] != "set-cookie")
        );
    }
    call.cleanup().await;
    let call = Call::new(vec![server.url("")]);
    assert!(!server.request(1).to_ascii_lowercase().contains("cookie:"));
    let result = call
        .request(&client, json!({"url":server.url("/head"),"method":"HEAD"}))
        .await;
    assert!(result.success);
    assert_eq!(result.value["body_bytes"], 0);
    assert!(call.read(&result.value).await.is_empty());
    call.cleanup().await;
}
#[tokio::test]
async fn expansion_http_client_undeclared_and_invalid_requests_have_zero_contact() {
    let server = Server::new(|_, _, socket| reply(socket, 200, "", b"unreachable"));
    let client = ExtensionHttpClient::new(direct());
    let call = Call::new(vec![]);
    let result = call
        .request(&client, json!({"url":server.url("/"),"method":"GET"}))
        .await;
    assert!(!result.success);
    assert_eq!(result.value["code"], "origin_denied");
    let declared = Call::new(vec![server.url("")]);
    let result=declared.request(&client,json!({"url":server.url("/"),"method":"GET","headers":[{"name":"Host","value":"synthetic"}]})).await;
    assert!(!result.success);
    assert_eq!(result.value["code"], "invalid_params");
    assert_eq!(server.count(), 0);
    assert_eq!(declared.store.available_bytes(), MAX_GLOBAL_BLOB_BYTES);
    call.cleanup().await;
    declared.cleanup().await;
}
#[tokio::test]
async fn expansion_http_client_redirect_exact_origin_and_all_caller_headers_drop() {
    let target = Server::new(|_, _, socket| reply(socket, 200, "", b"target"));
    let location = target.url("/final");
    let source = Server::new(move |_, request, socket| {
        if request.starts_with(b"GET /same ") {
            reply(socket, 302, "Location: /local\r\n", b"");
        } else if request.starts_with(b"GET /local ") {
            reply(socket, 200, "", b"same");
        } else {
            reply(socket, 302, &format!("Location: {location}\r\n"), b"");
        }
    });
    let client = ExtensionHttpClient::new(direct());
    let call = Call::new(vec![source.url(""), target.url("")]);
    let headers = json!([{"name":"Authorization","value":"synthetic-token"},{"name":"Cookie","value":"synthetic=value"},{"name":"X-Private","value":"synthetic"},{"name":"Referer","value":"https://synthetic.test/"}]);
    let result = call
        .request(
            &client,
            json!({"url":source.url("/cross"),"method":"GET","headers":headers,"max_redirects":3}),
        )
        .await;
    assert!(result.success);
    assert_eq!(result.value["redirect_count"], 1);
    assert_eq!(call.read(&result.value).await, b"target");
    let received = target.request(0).to_ascii_lowercase();
    for forbidden in ["authorization:", "cookie:", "x-private:", "referer:"] {
        assert!(
            !received.contains(forbidden),
            "all caller headers must drop"
        );
    }
    let result=call.request(&client,json!({"url":source.url("/same"),"method":"GET","headers":[{"name":"X-Private","value":"synthetic"}],"max_redirects":3})).await;
    assert!(result.success);
    assert_eq!(call.read(&result.value).await, b"same");
    assert!(source.request(2).contains("x-private: synthetic"));
    call.cleanup().await;
}
#[tokio::test]
async fn expansion_http_client_default_no_redirect_undeclared_destination_and_post_no_replay() {
    let target = Server::new(|_, _, socket| reply(socket, 200, "", b"must not contact"));
    let location = target.url("/other");
    let source = Server::new(move |_, _, socket| {
        reply(
            socket,
            307,
            &format!("Location: {location}\r\n"),
            b"redirect",
        )
    });
    let client = ExtensionHttpClient::new(direct());
    let call = Call::new(vec![source.url("")]);
    for params in [
        json!({"url":source.url("/"),"method":"GET"}),
        json!({"url":source.url("/"),"method":"POST","body_text":"synthetic side effect","max_redirects":3}),
    ] {
        let result = call.request(&client, params).await;
        assert!(result.success);
        assert_eq!(result.value["status"], 307);
        assert_eq!(result.value["redirect_count"], 0);
        let _ = call.read(&result.value).await;
    }
    call.cleanup().await;
    let call = Call::new(vec![source.url("")]);
    let result = call
        .request(
            &client,
            json!({"url":source.url("/"),"method":"GET","max_redirects":3}),
        )
        .await;
    assert!(!result.success);
    assert_eq!(result.value["code"], "origin_denied");
    assert_eq!(target.count(), 0);
    assert_eq!(source.count(), 3);
    call.cleanup().await;
}
#[tokio::test]
async fn expansion_http_client_manual_proxy_immutable_snapshot_and_no_direct_fallback() {
    let proxy = Server::new(|_, request, socket| {
        assert!(
            request.starts_with(b"GET http://127.0.0.1:"),
            "proxy absolute-form request"
        );
        reply(socket, 200, "", b"proxy");
    });
    let origin = Server::new(|_, _, socket| reply(socket, 200, "", b"direct"));
    let policy = NetworkPolicy::new(manual(proxy.address)).expect("owned manual proxy");
    let client = ExtensionHttpClient::new(policy.clone());
    let call = Call::new(vec![origin.url("")]);
    let result = call
        .request(&client, json!({"url":origin.url("/"),"method":"GET"}))
        .await;
    assert!(result.success);
    assert_eq!(call.read(&result.value).await, b"proxy");
    assert_eq!(origin.count(), 0);
    policy
        .update(NetworkSettings {
            proxy: NetworkProxySettings {
                mode: NetworkProxyMode::Direct,
                ..Default::default()
            },
        })
        .expect("later route update");
    let result = call
        .request(&client, json!({"url":origin.url("/"),"method":"GET"}))
        .await;
    assert!(result.success);
    assert_eq!(call.read(&result.value).await, b"direct");
    let unavailable = TcpListener::bind("127.0.0.1:0").expect("reserve unavailable proxy");
    let address = unavailable.local_addr().expect("endpoint");
    drop(unavailable);
    policy
        .update(manual(address))
        .expect("explicit unavailable proxy");
    call.cleanup().await;
    let call = Call::new(vec![origin.url("")]);
    let result = call
        .request(
            &client,
            json!({"url":origin.url("/"),"method":"POST","body_text":"never replay"}),
        )
        .await;
    assert!(!result.success);
    assert_eq!(origin.count(), 1, "manual failure cannot fall back direct");
    call.cleanup().await;
}
#[tokio::test]
async fn expansion_http_client_body_limit_missing_length_gzip_and_corruption_have_no_partial_blob()
{
    use flate2::{Compression, write::GzEncoder};
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder
        .write_all(&vec![b'a'; 65537])
        .expect("synthetic gzip");
    let compressed = encoder.finish().expect("gzip fixture");
    let server = Server::new(move |_, request, socket| {
        if request.starts_with(b"GET /gzip ") {
            reply(socket, 200, "Content-Encoding: gzip\r\n", &compressed);
        } else if request.starts_with(b"GET /bad ") {
            reply(socket, 200, "Content-Encoding: gzip\r\n", b"bad gzip");
        } else {
            let _ = socket.write_all(b"HTTP/1.1 200 OK\r\nConnection: close\r\n\r\n12345");
        }
    });
    let client = ExtensionHttpClient::new(direct());
    for (path, maximum, code) in [
        ("/plain", 4, "content_too_large"),
        ("/gzip", 65536, "content_too_large"),
        ("/bad", 65536, "body_decode_failed"),
    ] {
        let call = Call::new(vec![server.url("")]);
        let result = call
            .request(
                &client,
                json!({"url":server.url(path),"method":"GET","response_max_bytes":maximum}),
            )
            .await;
        assert!(!result.success);
        assert_eq!(result.value["code"], code);
        assert!(result.value.get("body_blob_id").is_none());
        call.cleanup().await;
    }
    let call = Call::new(vec![server.url("")]);
    let result = call
        .request(
            &client,
            json!({"url":server.url("/gzip"),"method":"GET","response_max_bytes":65537}),
        )
        .await;
    assert!(result.success);
    assert_eq!(call.read(&result.value).await, vec![b'a'; 65537]);
    call.cleanup().await;
}
#[tokio::test]
async fn expansion_http_client_timeout_cancel_and_tls_failure_never_publish_partial_success() {
    let server = Server::new(|_, _, socket| {
        let _ = socket
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 100\r\nConnection: close\r\n\r\na");
        thread::sleep(Duration::from_millis(250));
    });
    let client = ExtensionHttpClient::new(direct());
    let call = Call::new(vec![server.url("")]);
    let result = call
        .request(
            &client,
            json!({"url":server.url("/slow"),"method":"GET","timeout_ms":40}),
        )
        .await;
    assert!(!result.success);
    assert_eq!(result.value["code"], "timeout");
    assert!(result.value.get("body_blob_id").is_none());
    call.cleanup().await;
    let call = Call::new(vec![server.url("")]);
    let cancelled = call.cancel.clone();
    let operation = call.request(&client, json!({"url":server.url("/cancel"),"method":"GET"}));
    let (result, ()) = tokio::join!(operation, async move {
        tokio::time::sleep(Duration::from_millis(30)).await;
        cancelled.send(true).expect("owned cancellation");
    });
    assert!(!result.success);
    assert_eq!(result.value["code"], "cancelled");
    call.cleanup().await;
    let tls = Server::new(|_, _, socket| reply(socket, 200, "", b"plaintext is not TLS"));
    let url = format!("https://{}", tls.address);
    let call = Call::new(vec![url.clone()]);
    let result = call
        .request(&client, json!({"url":url,"method":"GET","timeout_ms":200}))
        .await;
    assert!(!result.success);
    assert!(result.value.get("body_blob_id").is_none());
    call.cleanup().await;
}

#[tokio::test]
async fn expansion_http_client_policy_update_during_redirect_uses_original_snapshot() {
    let origin = Server::new(|_, _, socket| reply(socket, 200, "", b"wrong direct route"));
    let entered = Arc::new(AtomicBool::new(false));
    let proceed = Arc::new(AtomicBool::new(false));
    let arrived = entered.clone();
    let released = proceed.clone();
    let proxy = Server::new(move |number, _, socket| {
        if number == 0 {
            arrived.store(true, Ordering::Release);
            let until = Instant::now() + Duration::from_secs(2);
            while !released.load(Ordering::Acquire) && Instant::now() < until {
                thread::sleep(Duration::from_millis(2));
            }
            reply(socket, 302, "Location: /second\r\n", b"");
        } else {
            reply(socket, 200, "", b"original snapshot");
        }
    });
    let policy = NetworkPolicy::new(manual(proxy.address)).expect("owned proxy");
    let client = ExtensionHttpClient::new(policy.clone());
    let call = Call::new(vec![origin.url("")]);
    let operation = call.request(
        &client,
        json!({"url":origin.url("/first"),"method":"GET","max_redirects":3}),
    );
    let (result, ()) = tokio::join!(operation, async {
        tokio::time::timeout(Duration::from_secs(2), async {
            while !entered.load(Ordering::Acquire) {
                tokio::time::sleep(Duration::from_millis(2)).await;
            }
        })
        .await
        .expect("actual proxy accepted first request");
        policy
            .update(NetworkSettings {
                proxy: NetworkProxySettings {
                    mode: NetworkProxyMode::Direct,
                    ..Default::default()
                },
            })
            .expect("publish later route");
        proceed.store(true, Ordering::Release);
    });
    assert!(result.success);
    assert_eq!(call.read(&result.value).await, b"original snapshot");
    assert_eq!(proxy.count(), 2);
    assert_eq!(origin.count(), 0);
    call.cleanup().await;
}

#[tokio::test]
async fn expansion_http_client_finite_run_and_process_admission_before_any_extra_contact() {
    let proceed = Arc::new(AtomicBool::new(false));
    let released = proceed.clone();
    let server = Server::new(move |_, _, socket| {
        let _ =
            socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 1\r\nConnection: close\r\n\r\n");
        let until = Instant::now() + Duration::from_secs(3);
        while !released.load(Ordering::Acquire) && Instant::now() < until {
            thread::sleep(Duration::from_millis(2));
        }
        let _ = socket.write_all(b"a");
    });
    let client = ExtensionHttpClient::new(direct());
    let first = Call::new(vec![server.url("")]);
    let second = Call::new(vec![server.url("")]);
    let extra = Call::new(vec![server.url("")]);
    let url = server.url("/held");
    let mut tasks = Vec::new();
    for call in [&first, &first, &second, &second] {
        let client = client.clone();
        let id = call.identity.clone();
        let run = call.run.clone();
        let blobs = call.blobs.clone();
        let cancellation = call.cancellation.clone();
        let url = url.clone();
        tasks.push(tokio::spawn(async move {
            client
                .request(
                    &id,
                    &run,
                    &blobs,
                    cancellation,
                    json!({"url":url,"method":"GET"}),
                )
                .await
        }));
    }
    tokio::time::timeout(Duration::from_secs(2), async {
        while first.store.available_bytes() != MAX_GLOBAL_BLOB_BYTES - 4 * 1024 * 1024
            || second.store.available_bytes() != MAX_GLOBAL_BLOB_BYTES - 4 * 1024 * 1024
        {
            tokio::time::sleep(Duration::from_millis(2)).await;
        }
    })
    .await
    .expect("four real response reservations admitted");
    let result = first
        .request(&client, json!({"url":url,"method":"GET"}))
        .await;
    assert!(!result.success);
    assert_eq!(result.value["code"], "resource_limit");
    let result = extra
        .request(&client, json!({"url":url,"method":"GET"}))
        .await;
    assert!(!result.success);
    assert_eq!(result.value["code"], "resource_limit");
    assert_eq!(extra.store.available_bytes(), MAX_GLOBAL_BLOB_BYTES);
    first.cancel.send(true).expect("retire first calls");
    second.cancel.send(true).expect("retire second calls");
    proceed.store(true, Ordering::Release);
    for task in tasks {
        let _ = task.await.expect("bounded request terminated");
    }
    first.cleanup().await;
    second.cleanup().await;
    extra.cleanup().await;
}

#[tokio::test]
async fn expansion_http_client_retired_body_cancelled_or_expired_calls_never_contact() {
    let server = Server::new(|_, _, socket| reply(socket, 200, "", b"must not contact"));
    let client = ExtensionHttpClient::new(direct());
    let call = Call::new(vec![server.url("")]);
    let result = call
        .request(
            &client,
            json!({"url":server.url("/"),"method":"POST","body_blob_id":Uuid::new_v4()}),
        )
        .await;
    assert!(!result.success);
    assert_eq!(result.value["code"], "blob_stale");
    assert_eq!(server.count(), 0);
    call.cancel.send(true).expect("pre-admission cancel");
    let result = call
        .request(&client, json!({"url":server.url("/"),"method":"GET"}))
        .await;
    assert!(!result.success);
    assert_eq!(result.value["code"], "cancelled");
    assert_eq!(server.count(), 0);
    call.cleanup().await;
    let mut call = Call::new(vec![server.url("")]);
    let declaration = serde_json::from_value(json!({"origins":[server.url("")]})).expect("origin");
    call.run = HttpRun::new(
        call.identity.clone(),
        declaration,
        tokio::time::Instant::now() - Duration::from_millis(1),
    );
    let result = call
        .request(&client, json!({"url":server.url("/"),"method":"GET"}))
        .await;
    assert!(!result.success);
    assert_eq!(result.value["code"], "timeout");
    assert_eq!(server.count(), 0);
    assert_eq!(call.store.available_bytes(), MAX_GLOBAL_BLOB_BYTES);
    call.cleanup().await;
}
