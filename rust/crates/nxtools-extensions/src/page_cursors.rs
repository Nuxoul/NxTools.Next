//! Opaque, finite-lived read cursors; only bounded query/key metadata is retained.
use crate::InvocationIdentity;
use serde_json::Value;
use std::{
    collections::HashMap,
    time::{Duration, Instant},
};
use uuid::Uuid;

#[derive(Default)]
pub struct PageCursors {
    entries: HashMap<Uuid, Cursor>,
}
struct Cursor {
    identity: InvocationIdentity,
    method: &'static str,
    query: Value,
    state: Value,
    expires: Instant,
}
impl PageCursors {
    pub fn remember(
        &mut self,
        identity: &InvocationIdentity,
        method: &'static str,
        query: Value,
        state: Value,
    ) -> Result<Uuid, &'static str> {
        self.entries
            .retain(|_, value| value.expires > Instant::now());
        if self.entries.len() >= 128 {
            return Err("cursor_limit");
        }
        if serde_json::to_vec(&query)
            .map_err(|_| "invalid_params")?
            .len()
            > 8192
            || serde_json::to_vec(&state)
                .map_err(|_| "invalid_cursor")?
                .len()
                > 1024
        {
            return Err("invalid_params");
        }
        let token = Uuid::new_v4();
        self.entries.insert(
            token,
            Cursor {
                identity: identity.clone(),
                method,
                query,
                state,
                expires: Instant::now() + Duration::from_secs(60),
            },
        );
        Ok(token)
    }
    pub fn read(
        &mut self,
        token: Uuid,
        identity: &InvocationIdentity,
        method: &str,
        query: &Value,
    ) -> Result<Value, &'static str> {
        self.entries
            .retain(|_, value| value.expires > Instant::now());
        self.entries
            .get(&token)
            .filter(|value| {
                value.identity == *identity && value.method == method && value.query == *query
            })
            .map(|value| value.state.clone())
            .ok_or("invalid_cursor")
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cursor_binds_current_invocation_method_query_and_has_finite_budget() {
        let identity = InvocationIdentity {
            run_id: Uuid::new_v4(),
            correlation_id: Uuid::new_v4(),
            generation: 1,
            package_id: "fixture".into(),
            handler_id: "action".into(),
            content_hash: "a".repeat(64),
        };
        let mut cursors = PageCursors::default();
        let query = serde_json::json!({"query":"fixture"});
        let token = cursors
            .remember(
                &identity,
                "clipboard.query_page",
                query.clone(),
                serde_json::json!({"id":32}),
            )
            .expect("cursor");
        assert_eq!(
            cursors
                .read(token, &identity, "clipboard.query_page", &query)
                .expect("same")["id"],
            32
        );
        let mut foreign = identity.clone();
        foreign.run_id = Uuid::new_v4();
        assert_eq!(
            cursors.read(token, &foreign, "clipboard.query_page", &query),
            Err("invalid_cursor")
        );
        assert_eq!(
            cursors.read(token, &identity, "downloads.list_page", &query),
            Err("invalid_cursor")
        );
        assert_eq!(
            cursors.read(
                token,
                &identity,
                "clipboard.query_page",
                &serde_json::json!({})
            ),
            Err("invalid_cursor")
        );
        cursors
            .entries
            .get_mut(&token)
            .expect("expire owned token")
            .expires = Instant::now();
        assert_eq!(
            cursors.read(token, &identity, "clipboard.query_page", &query),
            Err("invalid_cursor")
        );
        for _ in 0..128 {
            cursors
                .remember(&identity, "downloads.list_page", Value::Null, Value::Null)
                .expect("bounded slot");
        }
        assert_eq!(
            cursors.remember(&identity, "downloads.list_page", Value::Null, Value::Null),
            Err("cursor_limit")
        );
    }
}
