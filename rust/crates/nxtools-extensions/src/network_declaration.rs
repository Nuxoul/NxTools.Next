//! Host-neutral destination declaration. Parsing never resolves DNS or performs IO.
use crate::PackageError;
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use std::collections::HashSet;
use url::{Host, Url};

pub const MAX_NETWORK_ORIGINS: usize = 32;
const MAX_ORIGIN_BYTES: usize = 8192;

/// Canonical HTTP(S) origin, without request path, query, credentials or connection state.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NetworkOrigin(String);
impl NetworkOrigin {
    pub fn parse(value: &str) -> Result<Self, PackageError> {
        if value.is_empty()
            || value.len() > MAX_ORIGIN_BYTES
            || value
                .chars()
                .any(|ch| ch.is_control() || ch.is_whitespace())
            || value.contains(['\\', '?', '#', '@', '*', '%'])
        {
            return Err(PackageError::InvalidManifest);
        }
        let (scheme, remainder) = value
            .split_once("://")
            .ok_or(PackageError::InvalidManifest)?;
        if !scheme.eq_ignore_ascii_case("http") && !scheme.eq_ignore_ascii_case("https") {
            return Err(PackageError::InvalidManifest);
        }
        let (authority, path) = remainder
            .split_once('/')
            .map_or((remainder, ""), |(authority, path)| (authority, path));
        if authority.is_empty() || !path.is_empty() {
            return Err(PackageError::InvalidManifest);
        }
        let raw_host = if authority.starts_with('[') {
            let end = authority.find(']').ok_or(PackageError::InvalidManifest)?;
            validate_port_suffix(&authority[end + 1..])?;
            &authority[..=end]
        } else {
            match authority.rsplit_once(':') {
                Some((host, port)) => {
                    if host.contains(':') {
                        return Err(PackageError::InvalidManifest);
                    }
                    validate_port_suffix(&format!(":{port}"))?;
                    host
                }
                None => authority,
            }
        };
        let parsed = Url::parse(value).map_err(|_| PackageError::InvalidManifest)?;
        if parsed.host().is_none()
            || parsed.port_or_known_default().is_none_or(|port| port == 0)
            || !parsed.username().is_empty()
            || parsed.password().is_some()
            || parsed.path() != "/"
            || parsed.query().is_some()
            || parsed.fragment().is_some()
        {
            return Err(PackageError::InvalidManifest);
        }
        // WHATWG accepts shortened/octal/hex IPv4 forms. Declarations must be unambiguous.
        if let Some(Host::Ipv4(address)) = parsed.host()
            && raw_host != address.to_string()
        {
            return Err(PackageError::InvalidManifest);
        }
        Ok(Self(parsed.origin().ascii_serialization()))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
    /// Exact origin match only. Request syntax, current approval and redirects remain host gates.
    pub fn matches_url(&self, url: &Url) -> bool {
        matches!(url.scheme(), "http" | "https") && self.0 == url.origin().ascii_serialization()
    }
}
fn validate_port_suffix(suffix: &str) -> Result<(), PackageError> {
    if suffix.is_empty() {
        return Ok(());
    }
    let port = suffix
        .strip_prefix(':')
        .ok_or(PackageError::InvalidManifest)?;
    if port.is_empty()
        || !port.bytes().all(|byte| byte.is_ascii_digit())
        || port.parse::<u16>().ok().is_none_or(|port| port == 0)
    {
        return Err(PackageError::InvalidManifest);
    }
    Ok(())
}
impl Serialize for NetworkOrigin {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}
impl<'de> Deserialize<'de> for NetworkOrigin {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = String::deserialize(deserializer)?;
        Self::parse(&value).map_err(|_| de::Error::custom("invalid exact HTTP origin"))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct NetworkDeclaration {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub origins: Vec<NetworkOrigin>,
}
impl NetworkDeclaration {
    pub fn is_empty(&self) -> bool {
        self.origins.is_empty()
    }
    pub(crate) fn validate(&self) -> Result<(), PackageError> {
        let mut unique = HashSet::new();
        if self.origins.len() > MAX_NETWORK_ORIGINS
            || self.origins.iter().any(|origin| !unique.insert(origin))
        {
            return Err(PackageError::InvalidManifest);
        }
        Ok(())
    }
    pub fn allows_url(&self, url: &Url) -> bool {
        self.origins.iter().any(|origin| origin.matches_url(url))
    }
}
impl<'de> Deserialize<'de> for NetworkDeclaration {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Declaration {
            #[serde(default)]
            origins: Vec<NetworkOrigin>,
        }
        let crate::strict_json::StrictValue(value) = Deserialize::deserialize(deserializer)?;
        if !value.is_object() {
            return Err(de::Error::custom("network declaration must be an object"));
        }
        let declaration: Declaration = serde_json::from_value(value)
            .map_err(|_| de::Error::custom("invalid network declaration fields"))?;
        let declaration = Self {
            origins: declaration.origins,
        };
        declaration
            .validate()
            .map_err(|_| de::Error::custom("duplicate or excessive network origins"))?;
        Ok(declaration)
    }
}
