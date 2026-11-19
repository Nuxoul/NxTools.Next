use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub const CONTENT: &[u8] = &[b'x'; 256 * 1024];

pub struct UrlServer {
    pub address: std::net::SocketAddr,
    pub requests: Arc<Mutex<Vec<String>>>,
    pub held: Arc<AtomicUsize>,
    pub hold: Arc<AtomicBool>,
    shutdown: tokio::sync::oneshot::Sender<()>,
    task: tokio::task::JoinHandle<()>,
}

struct Held(Arc<AtomicUsize>);
impl Drop for Held {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}

impl UrlServer {
    pub async fn start() -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("owned loopback listener");
        let address = listener.local_addr().expect("local address");
        let requests = Arc::new(Mutex::new(Vec::new()));
        let held = Arc::new(AtomicUsize::new(0));
        let hold = Arc::new(AtomicBool::new(true));
        let fail = Arc::new(AtomicBool::new(true));
        let (shutdown, mut stopping) = tokio::sync::oneshot::channel();
        let observed = requests.clone();
        let active = held.clone();
        let holding = hold.clone();
        let failing = fail.clone();
        let task = tokio::spawn(async move {
            let mut connections = tokio::task::JoinSet::new();
            loop {
                tokio::select! {
                    _ = &mut stopping => break,
                    Some(result) = connections.join_next(), if !connections.is_empty() => { result.expect("HTTP connection fixture"); },
                    accepted = listener.accept() => {
                        let (mut stream, _) = accepted.expect("accept loopback connection");
                        let observed = observed.clone();
                        let active = active.clone();
                        let holding = holding.clone();
                        let failing = failing.clone();
                        connections.spawn(async move {
                            let mut bytes = Vec::new();
                            while !bytes.ends_with(b"\r\n\r\n") {
                                let mut byte = [0_u8; 1];
                                if stream.read_exact(&mut byte).await.is_err() { return; }
                                bytes.push(byte[0]);
                                assert!(bytes.len() < 16 * 1024, "bounded request headers");
                            }
                            let request = String::from_utf8(bytes).expect("fixture ASCII headers");
                            let mut words = request.split_whitespace();
                            let head = words.next() == Some("HEAD");
                            let path = words.next().expect("path").to_owned();
                            let range = request.lines().find_map(|line| {
                                let (name, value) = line.split_once(':')?;
                                if !name.eq_ignore_ascii_case("range") { return None; }
                                let (start,end) = value.trim().strip_prefix("bytes=")?.split_once('-')?;
                                Some((start.parse::<usize>().ok()?,end.parse::<usize>().ok()?))
                            });
                            let if_range = request.lines().find_map(|line| {
                                let (name, value) = line.split_once(':')?;
                                name.eq_ignore_ascii_case("if-range").then(|| value.trim().to_owned())
                            });
                            observed.lock().expect("request facts").push(request);
                            if path == "/held" {
                                active.fetch_add(1, Ordering::AcqRel);
                                let _held = Held(active);
                                while holding.load(Ordering::Acquire) {
                                    let mut byte = [0_u8; 1];
                                    tokio::select! {
                                        _ = tokio::time::sleep(Duration::from_millis(2)) => {},
                                        _ = stream.read(&mut byte) => return,
                                    }
                                }
                            }
                            if path == "/recover-failed" {
                                failing.store(false, Ordering::Release);
                                let _ = stream.write_all(b"HTTP/1.1 204 No Content\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").await;
                                return;
                            }
                            if path == "/failed" && failing.load(Ordering::Acquire) {
                                let _ = stream.write_all(b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").await;
                                return;
                            }
                            let identity = if path.contains("blank-last") {
                                "ETag: \r\nLast-Modified: Mon, 27 Jul 2026 00:00:00 GMT\r\n"
                            } else if path.contains("space-last") {
                                "ETag: \t \r\nLast-Modified: Mon, 27 Jul 2026 00:00:00 GMT\r\n"
                            } else if path.contains("quoted-empty") {
                                "ETag: \"\"\r\nLast-Modified: Mon, 27 Jul 2026 00:00:00 GMT\r\n"
                            } else if path.contains("weak-last") {
                                "ETag: W/\"resume-static\"\r\nLast-Modified: Mon, 27 Jul 2026 00:00:00 GMT\r\n"
                            } else if path.contains("missing") {
                                ""
                            } else if path.contains("weak") {
                                "ETag: W/\"resume-static\"\r\n"
                            } else if path.contains("last") {
                                "Last-Modified: Mon, 27 Jul 2026 00:00:00 GMT\r\n"
                            } else if path == "/changed" {
                                "ETag: \"different-resource\"\r\nLast-Modified: Mon, 27 Jul 2026 00:00:00 GMT\r\n"
                            } else {
                                "ETag: \"resume-static\"\r\nLast-Modified: Mon, 27 Jul 2026 00:00:00 GMT\r\n"
                            };
                            let range = range.filter(|range| {
                                if path == "/no-range" { return false; }
                                if path.ends_with("-new") && *range != (0, 0) {
                                    let expected = if path.contains("quoted-empty") { "\"\"" } else { "Mon, 27 Jul 2026 00:00:00 GMT" };
                                    return if_range.as_deref() == Some(expected);
                                }
                                true
                            });
                            let body: &[u8] = if path == "/changed" { &[b'y'; 256 * 1024] } else { CONTENT };
                            let (status, range_header, body) = if let Some((start,end)) = range {
                                assert!(start <= end && end < body.len());
                                ("206 Partial Content", format!("Content-Range: bytes {start}-{end}/{}\r\n", body.len()), &body[start..=end])
                            } else { ("200 OK", String::new(), body) };
                            let supports = if path == "/no-range" { "" } else { "Accept-Ranges: bytes\r\n" };
                            let headers = format!("HTTP/1.1 {status}\r\n{range_header}{identity}{supports}Content-Length: {}\r\nConnection: close\r\n\r\n", body.len());
                            if stream.write_all(headers.as_bytes()).await.is_err() || head { return; }
                            if path == "/partial-old" && range != Some((0,0)) {
                                let prefix = body.len().min(64 * 1024);
                                if stream.write_all(&body[..prefix]).await.is_err() { return; }
                                if stream.flush().await.is_err() { return; }
                                while holding.load(Ordering::Acquire) {
                                    let mut byte = [0_u8; 1];
                                    tokio::select! {
                                        _ = tokio::time::sleep(Duration::from_millis(2)) => {},
                                        _ = stream.read(&mut byte) => return,
                                    }
                                }
                                let _ = stream.write_all(&body[prefix..]).await;
                                return;
                            }
                            if path.contains("old") && range != Some((0,0)) {
                                let _ = stream.write_all(&body[..1]).await;
                                let _ = stream.flush().await;
                                tokio::time::sleep(Duration::from_millis(250)).await;
                                let _ = stream.write_all(&body[1..]).await;
                            } else { let _ = stream.write_all(body).await; }
                        });
                    }
                }
            }
            connections.abort_all();
            while let Some(result) = connections.join_next().await {
                assert!(result.is_ok() || result.expect_err("cancelled fixture").is_cancelled());
            }
        });
        Self {
            address,
            requests,
            held,
            hold,
            shutdown,
            task,
        }
    }

    pub fn url(&self, path: &str) -> String {
        format!("http://{}{path}", self.address)
    }

    pub async fn wait_held(&self) {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
        while self.held.load(Ordering::Acquire) == 0 {
            assert!(
                tokio::time::Instant::now() < deadline,
                "candidate probe must reach HTTP server"
            );
            tokio::time::sleep(Duration::from_millis(2)).await;
        }
    }

    pub async fn wait_drained(&self) {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
        while self.held.load(Ordering::Acquire) != 0 {
            assert!(
                tokio::time::Instant::now() < deadline,
                "cancelled candidate sockets must drain"
            );
            tokio::time::sleep(Duration::from_millis(2)).await;
        }
    }

    pub async fn close(self) -> Vec<String> {
        self.shutdown.send(()).expect("stop owned listener");
        self.task
            .await
            .expect("all HTTP connection workers drained");
        assert_eq!(self.held.load(Ordering::Acquire), 0);
        Arc::try_unwrap(self.requests)
            .expect("request owners drained")
            .into_inner()
            .expect("request facts")
    }
}
