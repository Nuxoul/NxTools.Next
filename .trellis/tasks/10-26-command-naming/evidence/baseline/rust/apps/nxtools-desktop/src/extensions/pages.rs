//! Additive page APIs. Legacy arrays and original feature owners remain unchanged.
use super::*;
use nxtools_extensions::InvocationIdentity;
use serde::Deserialize;
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DownloadPage {
    #[serde(default = "page_limit")]
    limit: usize,
    #[serde(default)]
    cursor: Option<Uuid>,
    #[serde(default)]
    statuses: Vec<nxtools_download::DownloadStatus>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ClipboardPage {
    query: String,
    #[serde(default = "page_limit")]
    limit: usize,
    #[serde(default)]
    cursor: Option<Uuid>,
}
fn page_limit() -> usize {
    32
}

impl FeatureAdapters {
    pub(super) async fn clipboard_page(&self, context: &HostContext, params: Value) -> HostReply {
        let Ok(input) = serde_json::from_value::<ClipboardPage>(params) else {
            return HostReply::error("invalid_params");
        };
        if !(1..=32).contains(&input.limit)
            || input.query.len() > 4096
            || input.query.contains('\0')
        {
            return HostReply::error("invalid_params");
        }
        let Some(clipboard) = &self.clipboard else {
            return HostReply::error("ui_unavailable");
        };
        let query = json!({"query":input.query,"limit":input.limit});
        let state = match cursor_state(
            &self.cursors,
            input.cursor,
            &context.identity,
            "clipboard.query_page",
            &query,
        ) {
            Ok(state) => state,
            Err(code) => return HostReply::error(code),
        };
        let mut page = match clipboard.query_page(&input.query, input.limit, state).await {
            Ok(page) => page,
            Err(code) => return HostReply::error(code),
        };
        if *context.cancellation.borrow() {
            return HostReply::error("cancelled");
        }
        finish_page(
            &self.cursors,
            &context.identity,
            "clipboard.query_page",
            query,
            &mut page,
        )
    }
}
pub(super) fn download_page(
    engine: &DownloadEngine,
    cursors: &Arc<Mutex<nxtools_extensions::PageCursors>>,
    identity: &InvocationIdentity,
    params: Value,
) -> HostReply {
    let Ok(input) = serde_json::from_value::<DownloadPage>(params) else {
        return HostReply::error("invalid_params");
    };
    if !(1..=32).contains(&input.limit) || input.statuses.len() > 16 {
        return HostReply::error("invalid_params");
    }
    let query = json!({"limit":input.limit,"statuses":input.statuses,"session":engine.session_generation()});
    let state = match cursor_state(
        cursors,
        input.cursor,
        identity,
        "downloads.list_page",
        &query,
    ) {
        Ok(state) => state,
        Err(code) => return HostReply::error(code),
    };
    let before = match state {
        Some(state) => match serde_json::from_value::<(i64, DownloadTaskId)>(state) {
            Ok(key) => Some(key),
            Err(_) => return HostReply::error("invalid_cursor"),
        },
        None => None,
    };
    let mut tasks = match engine.tasks_page_after(before, &input.statuses, input.limit + 1) {
        Ok(tasks) => tasks,
        Err(_) => return HostReply::error("download_unavailable"),
    };
    let more = tasks.len() > input.limit;
    tasks.truncate(input.limit);
    let state = tasks
        .last()
        .filter(|_| more)
        .map(|task| json!([task.task.created_at_unix_ms(), task.task.id()]));
    let items=tasks.iter().map(|task|json!({"task_id":task.task.id(),"status":task.task.status(),"request":task.task.request(),"resolution":task.task.resolution(),"downloaded_bytes":task.downloaded_bytes,"total_bytes":task.total_bytes})).collect::<Vec<_>>();
    let mut page = json!({"items":items,"has_more":more,"state":state,"consistency":"live_stable_created_key","order":"created_desc_id_asc"});
    finish_page(cursors, identity, "downloads.list_page", query, &mut page)
}
fn cursor_state(
    cursors: &Arc<Mutex<nxtools_extensions::PageCursors>>,
    token: Option<Uuid>,
    identity: &InvocationIdentity,
    method: &str,
    query: &Value,
) -> Result<Option<Value>, &'static str> {
    token
        .map(|token| {
            cursors
                .lock()
                .map_err(|_| "unavailable")?
                .read(token, identity, method, query)
        })
        .transpose()
}
fn finish_page(
    cursors: &Arc<Mutex<nxtools_extensions::PageCursors>>,
    identity: &InvocationIdentity,
    method: &'static str,
    query: Value,
    page: &mut Value,
) -> HostReply {
    let state = page
        .as_object_mut()
        .and_then(|page| page.remove("state"))
        .filter(|state| !state.is_null());
    if !serde_json::to_vec(page).is_ok_and(|bytes| bytes.len() <= 128 * 1024 - 128) {
        return HostReply::error("content_too_large");
    }
    let token = match state {
        Some(state) => match cursors
            .lock()
            .map_err(|_| "unavailable")
            .and_then(|mut cursors| cursors.remember(identity, method, query, state))
        {
            Ok(token) => Some(token),
            Err(code) => return HostReply::error(code),
        },
        None => None,
    };
    page["next_cursor"] = json!(token);
    page["cursor_ttl_ms"] = json!(60_000);
    HostReply {
        success: true,
        value: page.take(),
    }
}
