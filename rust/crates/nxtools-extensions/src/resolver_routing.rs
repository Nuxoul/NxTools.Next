use crate::{PackageError, PackageRegistry};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HandlerReference {
    pub package_id: String,
    pub handler_id: String,
}
impl HandlerReference {
    pub(crate) fn validate(&self, registry: &PackageRegistry) -> Result<(), PackageError> {
        if !crate::valid_id(&self.package_id)
            || !crate::valid_id(&self.handler_id)
            || !registry.packages.iter().any(|package| {
                package.manifest.id == self.package_id
                    && package.manifest.handlers.iter().any(|handler| {
                        handler.id == self.handler_id
                            && handler.feature == "downloads"
                            && handler.entry == "resolve"
                    })
            })
        {
            return Err(PackageError::InvalidManifest);
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ResolverRoutingError {
    #[error("download extension routing conflict; choose one handler")]
    Conflict,
    #[error("configured download handler is unavailable or untrusted")]
    Unavailable,
    #[error("download extension returned a processing cycle")]
    Cycle,
}
impl PackageRegistry {
    pub fn resolver_route(
        &self,
        url: &url::Url,
    ) -> Result<Vec<HandlerReference>, ResolverRoutingError> {
        let host = url
            .host_str()
            .ok_or(ResolverRoutingError::Unavailable)?
            .to_ascii_lowercase();
        let configured = self.resolver_chains.get(&host).cloned().or_else(|| {
            self.resolver_preferences
                .get(&host)
                .cloned()
                .map(|handler| vec![handler])
        });
        if let Some(route) = configured {
            if route.is_empty()
                || route.len() > 8
                || route.iter().any(|reference| !self.handler_ready(reference))
            {
                return Err(ResolverRoutingError::Unavailable);
            }
            return Ok(route);
        }
        let mut candidates = Vec::new();
        for package in self
            .packages
            .iter()
            .filter(|package| package.enabled && package.trust.is_some())
        {
            for handler in package
                .manifest
                .handlers
                .iter()
                .filter(|handler| handler.feature == "downloads" && handler.entry == "resolve")
            {
                let best = handler
                    .url_matches
                    .iter()
                    .filter(|rule| rule.matches(url))
                    .map(|rule| {
                        (
                            rule.priority,
                            usize::from(!rule.include_subdomains),
                            rule.host.len(),
                            rule.path
                                .bytes()
                                .filter(|byte| !matches!(byte, b'*' | b'?'))
                                .count(),
                        )
                    })
                    .max();
                if let Some(score) = best {
                    candidates.push((
                        score,
                        HandlerReference {
                            package_id: package.manifest.id.clone(),
                            handler_id: handler.id.clone(),
                        },
                    ));
                }
            }
        }
        candidates.sort_by_key(|candidate| std::cmp::Reverse(candidate.0));
        if candidates.len() > 1 && candidates[0].0 == candidates[1].0 {
            return Err(ResolverRoutingError::Conflict);
        }
        Ok(candidates
            .into_iter()
            .take(1)
            .map(|(_, reference)| reference)
            .collect())
    }
    fn handler_ready(&self, reference: &HandlerReference) -> bool {
        reference.validate(self).is_ok()
            && self.packages.iter().any(|package| {
                package.manifest.id == reference.package_id
                    && package.enabled
                    && package.trust.is_some()
            })
    }
    pub fn set_resolver_chain(
        &mut self,
        host: String,
        route: Vec<HandlerReference>,
    ) -> Result<(), PackageError> {
        let mut candidate = self.clone();
        if route.is_empty() {
            candidate.resolver_chains.remove(&host);
        } else {
            candidate.resolver_chains.insert(host, route);
        }
        candidate.validate()?;
        *self = candidate;
        Ok(())
    }
    pub(crate) fn validate_resolver_routes(&self) -> Result<(), PackageError> {
        if self.resolver_preferences.len() > 256 || self.resolver_chains.len() > 256 {
            return Err(PackageError::InvalidManifest);
        }
        let valid_host = |host: &str| {
            !host.is_empty()
                && host.len() <= 253
                && host == host.to_ascii_lowercase()
                && url::Host::parse(host).is_ok()
        };
        for (host, reference) in &self.resolver_preferences {
            if !valid_host(host) {
                return Err(PackageError::InvalidManifest);
            }
            reference.validate(self)?;
        }
        for (host, route) in &self.resolver_chains {
            let mut identities = HashSet::new();
            if !valid_host(host) || route.is_empty() || route.len() > 8 {
                return Err(PackageError::InvalidManifest);
            }
            for reference in route {
                reference.validate(self)?;
                if !identities.insert(reference) {
                    return Err(PackageError::InvalidManifest);
                }
            }
        }
        Ok(())
    }
}
/// Unchanged pass-through is allowed; revisiting a changed normalized URL is not.
pub struct ResolverCycleGuard {
    seen: HashSet<String>,
    current: String,
}
impl ResolverCycleGuard {
    pub fn new(url: &url::Url) -> Self {
        let current = normalized(url);
        Self {
            seen: HashSet::from([current.clone()]),
            current,
        }
    }
    pub fn accept(&mut self, url: &url::Url) -> Result<(), ResolverRoutingError> {
        let next = normalized(url);
        if next == self.current {
            return Ok(());
        }
        if !self.seen.insert(next.clone()) {
            return Err(ResolverRoutingError::Cycle);
        }
        self.current = next;
        Ok(())
    }
}
fn normalized(url: &url::Url) -> String {
    let mut url = url.clone();
    url.set_fragment(None);
    url.into()
}
