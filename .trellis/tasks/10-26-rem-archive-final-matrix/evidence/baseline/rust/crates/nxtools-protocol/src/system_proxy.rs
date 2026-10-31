//! Read-only Windows current-user proxy configuration; not a resolved request route.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SystemProxySnapshot {
    pub auto_detect: bool,
    pub pac_configured: bool,
    pub endpoints: Vec<SystemProxyEndpoint>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SystemProxyEndpoint {
    pub protocol: String,
    pub host: String,
    pub port: u16,
}
impl SystemProxySnapshot {
    pub fn is_bounded(&self) -> bool {
        self.endpoints.len() <= 8
            && self.endpoints.iter().all(|endpoint| {
                matches!(
                    endpoint.protocol.as_str(),
                    "all" | "http" | "https" | "socks"
                ) && !endpoint.host.is_empty()
                    && endpoint.host.len() <= 253
                    && endpoint.port > 0
                    && !endpoint.host.chars().any(|c| {
                        c.is_whitespace()
                            || c.is_control()
                            || matches!(c, '@' | '/' | '\\' | '?' | '#')
                    })
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn system_proxy_wire_and_endpoint_budgets() {
        let command = crate::PlatformCommand::ReadSystemProxy;
        let value = serde_json::to_value(&command).expect("serialize command");
        assert_eq!(value["command"], "read_system_proxy");
        assert_eq!(
            serde_json::from_value::<crate::PlatformCommand>(value).expect("roundtrip"),
            command
        );
        let mut snapshot = SystemProxySnapshot {
            auto_detect: true,
            pac_configured: true,
            endpoints: vec![SystemProxyEndpoint {
                protocol: "http".into(),
                host: "::1".into(),
                port: 8080,
            }],
        };
        assert!(snapshot.is_bounded());
        snapshot.endpoints[0].host = "credential@host".into();
        assert!(!snapshot.is_bounded());
        snapshot.endpoints[0].host = "host".into();
        snapshot.endpoints = vec![snapshot.endpoints[0].clone(); 9];
        assert!(!snapshot.is_bounded());
    }
}
