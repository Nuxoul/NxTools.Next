use std::sync::{Arc, RwLock};
use std::time::Duration;

use nxtools_core::{NetworkProxyMode, NetworkSettings};
use reqwest::{Client, ClientBuilder};
use thiserror::Error;

/// Product-wide outbound HTTP policy.
///
/// Consumers freeze an immutable snapshot at the start of an operation. Updating the policy only
/// changes snapshots created afterwards, so one operation cannot mix routes while it is running.
#[derive(Clone)]
pub struct NetworkPolicy {
    settings: Arc<RwLock<NetworkSettings>>,
    generation: Arc<std::sync::atomic::AtomicU64>,
}

impl NetworkPolicy {
    pub fn new(mut settings: NetworkSettings) -> Result<Self, NetworkPolicyError> {
        normalize_and_validate(&mut settings)?;
        Ok(Self {
            settings: Arc::new(RwLock::new(settings)),
            generation: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        })
    }

    pub fn update(&self, mut settings: NetworkSettings) -> Result<(), NetworkPolicyError> {
        normalize_and_validate(&mut settings)?;
        *self
            .settings
            .write()
            .map_err(|_| NetworkPolicyError::LockPoisoned)? = settings;
        // Bumped after the write so a snapshot that observes the new generation
        // also observes the new settings. Consumers that cache built clients use
        // it to drop clients made under the previous route.
        self.generation
            .fetch_add(1, std::sync::atomic::Ordering::AcqRel);
        Ok(())
    }

    /// Monotonic counter that changes whenever the policy is updated.
    pub fn generation(&self) -> u64 {
        self.generation.load(std::sync::atomic::Ordering::Acquire)
    }

    pub fn snapshot(&self) -> Result<NetworkOperationSnapshot, NetworkPolicyError> {
        let settings = self
            .settings
            .read()
            .map_err(|_| NetworkPolicyError::LockPoisoned)?
            .clone();
        NetworkOperationSnapshot::from_settings(settings)
    }
}

#[derive(Clone)]
pub struct NetworkOperationSnapshot {
    route: NetworkRoute,
}

#[derive(Clone)]
enum NetworkRoute {
    Direct,
    System,
    ManualHttp { endpoint: String },
}

impl NetworkOperationSnapshot {
    fn from_settings(settings: NetworkSettings) -> Result<Self, NetworkPolicyError> {
        let route = match settings.proxy.mode {
            NetworkProxyMode::Direct => NetworkRoute::Direct,
            NetworkProxyMode::System => NetworkRoute::System,
            NetworkProxyMode::ManualHttp => NetworkRoute::ManualHttp {
                endpoint: manual_proxy_endpoint(&settings.proxy.host, settings.proxy.port),
            },
        };
        Ok(Self { route })
    }

    pub const fn mode(&self) -> NetworkProxyMode {
        match self.route {
            NetworkRoute::Direct => NetworkProxyMode::Direct,
            NetworkRoute::System => NetworkProxyMode::System,
            NetworkRoute::ManualHttp { .. } => NetworkProxyMode::ManualHttp,
        }
    }

    pub const fn source(&self) -> &'static str {
        match self.route {
            NetworkRoute::Direct => "none",
            NetworkRoute::System => "system",
            NetworkRoute::ManualHttp { .. } => "configured",
        }
    }

    /// Stable identity of the outbound route, used to decide whether a built
    /// client can be reused. Two snapshots with equal keys build equivalent
    /// clients.
    pub fn route_key(&self) -> String {
        match &self.route {
            NetworkRoute::Direct => "direct".to_owned(),
            NetworkRoute::System => "system".to_owned(),
            NetworkRoute::ManualHttp { endpoint } => format!("manual:{endpoint}"),
        }
    }

    pub fn build_client(&self, builder: ClientBuilder) -> Result<Client, NetworkPolicyError> {
        let builder = match &self.route {
            NetworkRoute::Direct => builder.no_proxy(),
            NetworkRoute::System => builder,
            NetworkRoute::ManualHttp { endpoint } => builder
                .no_proxy()
                .proxy(reqwest::Proxy::all(endpoint).map_err(NetworkPolicyError::Client)?),
        };
        builder
            // Multi-segment transfers hammer one host concurrently; keep idle
            // connections alive so segment rebalances reuse TLS sessions
            // instead of re-handshaking.
            .pool_max_idle_per_host(16)
            .pool_idle_timeout(Duration::from_secs(90))
            .tcp_nodelay(true)
            .build()
            .map_err(NetworkPolicyError::Client)
    }
}

pub const fn proxy_mode_name(mode: NetworkProxyMode) -> &'static str {
    match mode {
        NetworkProxyMode::Direct => "direct",
        NetworkProxyMode::System => "system",
        NetworkProxyMode::ManualHttp => "manual_http",
    }
}

fn normalize_and_validate(settings: &mut NetworkSettings) -> Result<(), NetworkPolicyError> {
    settings.normalize();
    settings
        .validate()
        .map_err(NetworkPolicyError::InvalidSettings)
}

fn manual_proxy_endpoint(host: &str, port: u16) -> String {
    if host.contains(':') && !host.starts_with('[') {
        format!("http://[{host}]:{port}")
    } else {
        format!("http://{host}:{port}")
    }
}

#[derive(Debug, Error)]
pub enum NetworkPolicyError {
    #[error("invalid network settings: {0}")]
    InvalidSettings(String),
    #[error("network policy lock is poisoned")]
    LockPoisoned,
    #[error("could not build HTTP client: {0}")]
    Client(#[source] reqwest::Error),
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::time::{Duration, Instant};

    use nxtools_core::{NetworkProxySettings, NetworkSettings};

    use super::*;

    fn accept_bounded(listener: TcpListener) -> std::net::TcpStream {
        listener.set_nonblocking(true).expect("bounded listener");
        let deadline = Instant::now() + Duration::from_secs(3);
        let stream = loop {
            match listener.accept() {
                Ok((stream, _)) => break stream,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    assert!(Instant::now() < deadline, "route fixture accept deadline");
                    std::thread::sleep(Duration::from_millis(5));
                }
                Err(error) => panic!("route fixture accept failed: {error}"),
            }
        };
        stream
            .set_nonblocking(false)
            .expect("blocking fixture socket");
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .expect("read deadline");
        stream
            .set_write_timeout(Some(Duration::from_secs(2)))
            .expect("write deadline");
        stream
    }

    #[test]
    fn operation_snapshot_is_immutable_across_policy_updates() {
        let policy = NetworkPolicy::new(NetworkSettings::default()).expect("direct policy");
        let active = policy.snapshot().expect("active snapshot");

        policy
            .update(NetworkSettings {
                proxy: NetworkProxySettings {
                    mode: NetworkProxyMode::ManualHttp,
                    host: "127.0.0.1".into(),
                    port: 3128,
                },
            })
            .expect("update policy");
        let next = policy.snapshot().expect("next snapshot");

        assert_eq!(active.mode(), NetworkProxyMode::Direct);
        assert_eq!(next.mode(), NetworkProxyMode::ManualHttp);
    }

    #[tokio::test]
    async fn manual_snapshot_routes_a_real_http_request_through_the_proxy() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind proxy fixture");
        let address = listener.local_addr().expect("proxy fixture address");
        let fixture = std::thread::spawn(move || {
            let mut stream = accept_bounded(listener);
            let mut request = [0_u8; 4096];
            let read = stream.read(&mut request).expect("read proxied request");
            let request = String::from_utf8_lossy(&request[..read]);
            assert!(
                request.starts_with("GET http://example.invalid/network-policy HTTP/1.1"),
                "proxy must receive the absolute request target: {request}"
            );
            stream
                .write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Length: 7\r\nConnection: close\r\n\r\nproxied",
                )
                .expect("write proxy response");
        });
        let policy = NetworkPolicy::new(NetworkSettings {
            proxy: NetworkProxySettings {
                mode: NetworkProxyMode::ManualHttp,
                host: address.ip().to_string(),
                port: address.port(),
            },
        })
        .expect("manual policy");
        let response = policy
            .snapshot()
            .expect("manual snapshot")
            .build_client(
                reqwest::Client::builder()
                    .connect_timeout(Duration::from_secs(2))
                    .read_timeout(Duration::from_secs(2)),
            )
            .expect("proxied client")
            .get("http://example.invalid/network-policy")
            .send()
            .await
            .expect("proxied request");
        assert_eq!(response.bytes().await.expect("response body"), "proxied");
        fixture.join().expect("join proxy fixture");
    }
    #[tokio::test]
    async fn direct_and_system_routes_use_real_requests_with_an_injected_resolver() {
        for mode in [NetworkProxyMode::Direct, NetworkProxyMode::System] {
            let listener = TcpListener::bind("127.0.0.1:0").expect("route fixture");
            let address = listener.local_addr().expect("route address");
            let fixture = std::thread::spawn(move || {
                let mut stream = accept_bounded(listener);
                let mut bytes = [0u8; 4096];
                let count = stream.read(&mut bytes).expect("actual request");
                let request = String::from_utf8_lossy(&bytes[..count]);
                match mode {
                    NetworkProxyMode::Direct => {
                        assert!(request.starts_with("GET /route-check HTTP/1.1"))
                    }
                    NetworkProxyMode::System => {
                        assert!(request.starts_with("GET http://example.invalid:"))
                    }
                    _ => unreachable!(),
                }
                stream
                    .write_all(
                        b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok",
                    )
                    .expect("response");
            });
            let policy = NetworkPolicy::new(NetworkSettings {
                proxy: NetworkProxySettings {
                    mode,
                    ..Default::default()
                },
            })
            .expect("policy");
            let builder = Client::builder()
                .resolve("example.invalid", address)
                .proxy(reqwest::Proxy::all(format!("http://{address}")).expect("injected resolver"))
                .timeout(Duration::from_secs(2));
            let response = policy
                .snapshot()
                .expect("snapshot")
                .build_client(builder)
                .expect("client")
                .get(format!(
                    "http://example.invalid:{}/route-check",
                    address.port()
                ))
                .send()
                .await
                .expect("routed request");
            assert_eq!(response.bytes().await.expect("body"), "ok");
            fixture.join().expect("join bounded route fixture");
        }
    }
}
