use std::net::{Ipv4Addr, Ipv6Addr};

use url::{Host, Url};

const MAX_URL_INPUT_BYTES: usize = 2_048;

pub fn web_url_target(query: &str) -> Option<String> {
    let query = query.trim();
    if query.is_empty()
        || query.len() > MAX_URL_INPUT_BYTES
        || query.chars().any(char::is_whitespace)
    {
        return None;
    }
    let has_http_scheme = query
        .get(..7)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("http://"))
        || query
            .get(..8)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("https://"));
    let target = if has_http_scheme {
        query.to_owned()
    } else {
        if query.contains("://") {
            return None;
        }
        format!("https://{query}")
    };
    let parsed = Url::parse(&target).ok()?;
    let authority = target.split_once("://")?.1.split(['/', '?', '#']).next()?;
    if authority.contains('@') {
        return None;
    }
    let raw_host = authority_host(authority)?;
    if !matches!(parsed.scheme(), "http" | "https")
        || parsed.host().is_none()
        || !parsed.username().is_empty()
        || parsed.password().is_some()
    {
        return None;
    }
    if !valid_web_host(parsed.host()?, raw_host, !has_http_scheme) {
        return None;
    }
    Some(target)
}

fn authority_host(authority: &str) -> Option<&str> {
    if authority.starts_with('[') {
        let closing_bracket = authority.find(']')?;
        return Some(&authority[..=closing_bracket]);
    }
    if let Some((host, port)) = authority.rsplit_once(':')
        && !port.is_empty()
        && port.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Some(host);
    }
    Some(authority)
}

fn valid_web_host(host: Host<&str>, raw_host: &str, require_bare_domain_shape: bool) -> bool {
    match host {
        Host::Ipv4(_) => raw_host.parse::<Ipv4Addr>().is_ok(),
        Host::Ipv6(_) => raw_host
            .strip_prefix('[')
            .and_then(|host| host.strip_suffix(']'))
            .is_some_and(|host| host.parse::<Ipv6Addr>().is_ok()),
        Host::Domain(domain) => {
            let domain = domain.trim_end_matches('.');
            let labels_are_valid = !domain.is_empty()
                && domain.len() <= 253
                && domain.split('.').all(|label| {
                    !label.is_empty()
                        && label.len() <= 63
                        && label
                            .as_bytes()
                            .first()
                            .is_some_and(u8::is_ascii_alphanumeric)
                        && label
                            .as_bytes()
                            .last()
                            .is_some_and(u8::is_ascii_alphanumeric)
                        && label
                            .bytes()
                            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
                });
            labels_are_valid
                && (!require_bare_domain_shape
                    || domain
                        .rsplit_once('.')
                        .is_some_and(|(_, suffix)| suffix.len() >= 2))
        }
    }
}
