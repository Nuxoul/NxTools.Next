use super::*;
use crate::http::HttpConnectionScope;

const CEILING_TTL: Duration = Duration::from_secs(600);
const MAX_CEILINGS: usize = 256;
type CeilingKey = (HttpConnectionScope, String);
pub(super) type Ceiling = Arc<Mutex<ReceivingWidth>>;

#[derive(Default)]
pub(super) struct ConnectionCeilings(VecDeque<(CeilingKey, Ceiling)>);
impl ConnectionCeilings {
    pub(super) fn for_resource(&mut self, scope: HttpConnectionScope, url: &str) -> Ceiling {
        let now = Instant::now();
        self.0
            .retain(|(_, entry)| entry.lock().is_ok_and(|value| value.expires > now));
        let host = url::Url::parse(url)
            .expect("validated final HTTP URL")
            .host_str()
            .expect("HTTP host")
            .to_owned();
        let key = (scope, host);
        if let Some((_, entry)) = self.0.iter().find(|(candidate, _)| *candidate == key) {
            return Arc::clone(entry);
        }
        if self.0.len() == MAX_CEILINGS {
            self.0.pop_front();
        }
        let entry = Arc::new(Mutex::new(ReceivingWidth::new(now)));
        self.0.push_back((key, Arc::clone(&entry)));
        entry
    }
}

pub(super) struct ReceivingWidth {
    expires: Instant,
    receiving: u16,
    proven_width: u16,
    refusals: u16,
    ceiling: Option<u16>,
}
impl ReceivingWidth {
    fn new(now: Instant) -> Self {
        Self {
            expires: now + CEILING_TTL,
            receiving: 0,
            proven_width: 0,
            refusals: 0,
            ceiling: None,
        }
    }
    fn expire(&mut self, now: Instant) {
        if now >= self.expires {
            self.expires = now + CEILING_TTL;
            self.proven_width = 0;
            self.refusals = 0;
            self.ceiling = None;
        }
    }
    #[cfg(test)]
    fn refused(&mut self, clear_refusal: bool, now: Instant) -> bool {
        self.expire(now);
        if !clear_refusal || self.receiving == 0 {
            return false;
        }
        self.refused_at_width(self.receiving, now)
    }
    fn refused_at_width(&mut self, receiving: u16, now: Instant) -> bool {
        self.expire(now);
        let previous = self.ceiling;
        let width = receiving.max(1);
        self.proven_width = if self.refusals == 0 {
            width
        } else {
            self.proven_width.max(width)
        };
        self.refusals = self.refusals.saturating_add(1);
        self.expires = now + CEILING_TTL;
        if self.refusals >= 2 {
            self.ceiling = Some(self.proven_width);
        }
        self.ceiling.is_some() && self.ceiling != previous
    }
}

pub(super) fn limit(ceiling: &Ceiling, requested: u16) -> u16 {
    ceiling
        .lock()
        .map(|mut value| {
            value.expire(Instant::now());
            requested.min(value.ceiling.unwrap_or(requested)).max(1)
        })
        .unwrap_or(requested)
}

pub(super) struct ReceivingLease {
    ceiling: Ceiling,
    receiving: bool,
}
impl ReceivingLease {
    pub(super) fn new(ceiling: Ceiling) -> Self {
        Self {
            ceiling,
            receiving: false,
        }
    }
    pub(super) fn progress(&mut self, receiving_width: u16) {
        if self.receiving {
            return;
        }
        if let Ok(mut value) = self.ceiling.lock() {
            value.expire(Instant::now());
            value.receiving = value.receiving.saturating_add(1);
            if value.refusals > 0 {
                value.proven_width = value.proven_width.max(receiving_width);
            }
            if let Some(cap) = value.ceiling {
                value.ceiling = Some(cap.max(receiving_width));
            }
            self.receiving = true;
        }
    }
}
impl Drop for ReceivingLease {
    fn drop(&mut self) {
        if self.receiving
            && let Ok(mut value) = self.ceiling.lock()
        {
            value.receiving = value.receiving.saturating_sub(1);
        }
    }
}

pub(super) fn observe_run_refusal(
    ceiling: &Ceiling,
    error: &HttpDownloadError,
    working: usize,
    receiving: usize,
) -> Option<u16> {
    if working == 0 || !error.is_refused_connection() {
        return None;
    }
    let mut value = ceiling.lock().ok()?;
    let width = u16::try_from(receiving).unwrap_or(u16::MAX);
    value
        .refused_at_width(width, Instant::now())
        .then_some(value.proven_width)
}

#[cfg(test)]
pub(super) fn observe_refusal(ceiling: &Ceiling, error: &HttpDownloadError) -> Option<u16> {
    let mut value = ceiling.lock().ok()?;
    value
        .refused(error.is_refused_connection(), Instant::now())
        .then_some(value.proven_width)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_repeated_clear_refusal_with_receiving_peers_learns_and_expires() {
        let now = Instant::now();
        let mut value = ReceivingWidth::new(now);
        assert!(!value.refused(true, now));
        assert_eq!(value.refusals, 0);
        value.receiving = 2;
        for _ in 0..5 {
            assert!(!value.refused(false, now));
        }
        assert_eq!(value.ceiling, None);
        assert!(!value.refused(true, now));
        assert_eq!(value.ceiling, None);
        assert!(value.refused(true, now));
        assert_eq!(value.ceiling, Some(2));
        value.expire(now + CEILING_TTL - Duration::from_millis(1));
        assert_eq!(value.ceiling, Some(2));
        value.expire(now + CEILING_TTL);
        assert_eq!(value.ceiling, None);
    }
    #[test]
    fn proven_progress_not_worker_count_and_lower_limits_win() {
        let ceiling = Arc::new(Mutex::new(ReceivingWidth::new(Instant::now())));
        let mut first = ReceivingLease::new(Arc::clone(&ceiling));
        let mut second = ReceivingLease::new(Arc::clone(&ceiling));
        first.progress(1);
        first.progress(1);
        second.progress(2);
        {
            let mut value = ceiling.lock().expect("observation");
            assert_eq!(value.receiving, 2);
            assert!(!value.refused(true, Instant::now()));
            assert!(value.refused(true, Instant::now()));
        }
        assert_eq!(limit(&ceiling, 4), 2);
        assert_eq!(limit(&ceiling, 1), 1);
        drop(first);
        drop(second);
        assert_eq!(ceiling.lock().expect("retired").receiving, 0);
    }
    #[test]
    fn host_policy_certificate_and_system_client_scope_are_independent() {
        let mut table = ConnectionCeilings::default();
        let scope = HttpConnectionScope {
            generation: 0,
            route: "direct".into(),
            certificate_invalid: false,
            system_client: None,
        };
        let first = table.for_resource(scope.clone(), "http://owned.test:100/file");
        assert!(Arc::ptr_eq(
            &first,
            &table.for_resource(scope.clone(), "http://owned.test:100/other")
        ));
        for url in ["http://owned.test:101/file", "https://owned.test:100/file"] {
            assert!(Arc::ptr_eq(&first, &table.for_resource(scope.clone(), url)));
        }
        assert!(!Arc::ptr_eq(
            &first,
            &table.for_resource(scope.clone(), "http://other.test:100/file")
        ));
        let changed = [
            HttpConnectionScope {
                generation: 1,
                ..scope.clone()
            },
            HttpConnectionScope {
                route: "manual:owned".into(),
                ..scope.clone()
            },
            HttpConnectionScope {
                certificate_invalid: true,
                ..scope.clone()
            },
            HttpConnectionScope {
                system_client: Some(uuid::Uuid::new_v4()),
                ..scope.clone()
            },
        ];
        for other in changed {
            assert!(!Arc::ptr_eq(
                &first,
                &table.for_resource(other, "http://owned.test:100/file")
            ));
        }
        for ordinal in 0..300 {
            table.for_resource(scope.clone(), &format!("http://owned-{ordinal}.test/file"));
        }
        assert_eq!(table.0.len(), MAX_CEILINGS);
    }

    #[test]
    fn synced_response_lease_is_not_an_extra_receiving_connection() {
        let ceiling = Arc::new(Mutex::new(ReceivingWidth::new(Instant::now())));
        let mut finished_but_syncing = ReceivingLease::new(Arc::clone(&ceiling));
        let mut peer = ReceivingLease::new(Arc::clone(&ceiling));
        finished_but_syncing.progress(1);
        peer.progress(2);
        let refusal = HttpDownloadError::Throttled {
            status: 503,
            retry_after: None,
        };
        assert_eq!(observe_run_refusal(&ceiling, &refusal, 2, 2), None);
        assert_eq!(observe_run_refusal(&ceiling, &refusal, 2, 2), Some(2));
        let mut replacement = ReceivingLease::new(Arc::clone(&ceiling));
        replacement.progress(2);
        assert_eq!(ceiling.lock().expect("live leases").receiving, 3);
        assert_eq!(
            limit(&ceiling, 8),
            2,
            "a completed response syncing its file is not receiving"
        );
        drop((finished_but_syncing, peer, replacement));
        assert_eq!(ceiling.lock().expect("retired leases").receiving, 0);
    }
}
