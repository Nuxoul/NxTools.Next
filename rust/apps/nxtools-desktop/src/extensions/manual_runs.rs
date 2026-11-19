//! Session-only manual invocation projections and host-confirmed effects.
use super::ExtensionApplication;
use gpui_kit::Context;
use nxtools_extensions::{InvocationIdentity, InvokeOutcome, RuntimeError};
use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, Mutex},
    time::Instant,
};
use uuid::Uuid;

#[derive(Clone)]
pub(crate) struct ManualEffects {
    runs: Arc<Mutex<HashMap<Uuid, EffectLease>>>,
    changed: tokio::sync::watch::Sender<u64>,
}
impl Default for ManualEffects {
    fn default() -> Self {
        Self {
            runs: Default::default(),
            changed: tokio::sync::watch::channel(0).0,
        }
    }
}
struct EffectLease {
    package: String,
    handler: String,
    hash: String,
    items: Vec<i64>,
}
impl ManualEffects {
    pub(crate) fn subscribe(&self) -> tokio::sync::watch::Receiver<u64> {
        self.changed.subscribe()
    }
    pub(crate) fn remove(&self, id: Uuid) {
        if let Ok(mut runs) = self.runs.lock() {
            runs.remove(&id);
        }
    }
    pub(crate) fn begin(&self, id: Uuid, package: &str, handler: &str, hash: &str) {
        if let Ok(mut runs) = self.runs.lock()
            && runs.len() < 36
        {
            runs.insert(
                id,
                EffectLease {
                    package: package.into(),
                    handler: handler.into(),
                    hash: hash.into(),
                    items: Vec::new(),
                },
            );
        }
    }
    pub(crate) fn committed(&self, identity: &InvocationIdentity, item: i64) {
        if item <= 0 {
            return;
        }
        if let Ok(mut runs) = self.runs.lock()
            && let Some(run) = runs.get_mut(&identity.correlation_id)
            && run.package == identity.package_id
            && run.handler == identity.handler_id
            && run.hash == identity.content_hash
            && run.items.len() < 64
            && !run.items.contains(&item)
        {
            run.items.push(item);
            self.changed
                .send_modify(|revision| *revision = revision.wrapping_add(1));
        }
    }
    pub(crate) fn accepts_progress(&self, identity: &InvocationIdentity) -> bool {
        self.runs.lock().is_ok_and(|runs| {
            runs.get(&identity.correlation_id).is_some_and(|run| {
                run.package == identity.package_id
                    && run.handler == identity.handler_id
                    && run.hash == identity.content_hash
            })
        })
    }
    pub(crate) fn items(&self, id: Uuid) -> Vec<i64> {
        self.runs
            .lock()
            .ok()
            .and_then(|runs| runs.get(&id).map(|run| run.items.clone()))
            .unwrap_or_default()
    }
    pub(crate) fn finish(&self, id: Uuid) -> Vec<i64> {
        self.items(id)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ManualState {
    Running,
    Cancelling,
    Succeeded,
    NotHandled,
    Failed(RuntimeError),
    Cancelled,
}
impl ManualState {
    pub(crate) fn pending(&self) -> bool {
        matches!(self, Self::Running | Self::Cancelling)
    }
    pub(crate) fn label(&self) -> &'static str {
        match self {
            Self::Running => "执行中",
            Self::Cancelling => "正在请求取消",
            Self::Succeeded => "插件报告完成",
            Self::NotHandled => "未处理 / 无需修改",
            Self::Cancelled => "调用已取消",
            Self::Failed(RuntimeError::Timeout) => "运行超时，结果未完全确认",
            Self::Failed(RuntimeError::Untrusted | RuntimeError::Stale) => {
                "信任或内容已变更，调用已停止"
            }
            Self::Failed(RuntimeError::QueueFull) => "调用繁忙，未能执行",
            Self::Failed(RuntimeError::NativeUnavailable) => "运行通道不可用，结果未完全确认",
            Self::Failed(RuntimeError::HandlerUnavailable) => "动作已不可用",
            Self::Failed(RuntimeError::InvalidSettings) => "插件设置未完成或无效，请打开设置后保存",
            Self::Failed(RuntimeError::UnsupportedTransport) => "插件请求的运行模式暂不可用",
            Self::Failed(_) => "调用失败，结果未完全确认",
        }
    }
}
#[derive(Clone)]
pub(crate) struct ManualRun {
    pub(crate) id: Uuid,
    pub(crate) package_id: String,
    pub(crate) package_name: String,
    pub(crate) action_name: String,
    pub(crate) state: ManualState,
    pub(crate) started: Instant,
    pub(crate) at_unix_ms: i64,
    pub(crate) elapsed_ms: Option<u128>,
    pub(crate) message: Option<String>,
    pub(crate) items: Vec<i64>,
    pub(crate) origin: Option<super::manual_actions::ClipboardOrigin>,
    pub(crate) progress: Option<nxtools_extensions::InvocationProgress>,
    pub(super) generation: u64,
}
#[cfg(debug_assertions)]
impl ExtensionApplication {
    pub(crate) fn seed_manual_feedback_preview(&mut self, running: bool) -> Uuid {
        let id = Uuid::new_v4();
        self.begin_manual_run(
            id,
            "synthetic.cleaner".into(),
            "剪贴板文本整理".into(),
            "整理文本并保留原文".into(),
        );
        if let Some(run) = self.manual_runs.back_mut() {
            run.state = if running {
                ManualState::Running
            } else {
                ManualState::Succeeded
            };
            run.elapsed_ms = (!running).then_some(142);
            run.message = (!running).then(|| "已生成整理后的文本，原内容保留。".into());
            if !running {
                run.items = vec![17];
            }
        }
        id
    }
}
#[cfg(test)]
impl ExtensionApplication {
    pub(crate) fn seed_manual_settings_failure(&mut self, package: &str) -> Uuid {
        let id = Uuid::new_v4();
        self.begin_manual_run(
            id,
            package.into(),
            "Settings fixture".into(),
            "Action".into(),
        );
        if let Some(run) = self.manual_runs.back_mut() {
            run.state = ManualState::Failed(RuntimeError::InvalidSettings);
        }
        id
    }
}

impl ExtensionApplication {
    pub(crate) fn attach_manual_origin(
        &mut self,
        id: Uuid,
        origin: Option<super::manual_actions::ClipboardOrigin>,
    ) {
        if let Some(run) = self.manual_runs.iter_mut().find(|run| run.id == id) {
            run.origin = origin;
        }
    }
    pub(crate) fn manual_run(&self, id: Uuid) -> Option<ManualRun> {
        let mut run = self.manual_runs.iter().find(|run| run.id == id)?.clone();
        if let Some(service) = &self.service {
            run.progress = run
                .state
                .pending()
                .then(|| service.runtime.progress(id))
                .flatten()
                .filter(|progress| {
                    progress.identity.generation == run.generation
                        && progress.identity.package_id == run.package_id
                        && service.manual_effects.accepts_progress(&progress.identity)
                });
            for item in service.manual_effects.items(id) {
                if !run.items.contains(&item) {
                    run.items.push(item);
                }
            }
        }
        Some(run)
    }
    pub(crate) fn recent_manual_runs(&self) -> Vec<ManualRun> {
        self.manual_runs
            .iter()
            .rev()
            .filter_map(|run| self.manual_run(run.id))
            .collect()
    }
    pub(crate) fn cancel_manual_run(&mut self, id: Uuid, cx: &mut Context<Self>) {
        let Some((_, cancellation)) = self.pending_manual.get(&id) else {
            return;
        };
        if *cancellation.borrow() {
            return;
        }
        let _ = cancellation.send(true);
        if let Some(run) = self.manual_runs.iter_mut().find(|run| run.id == id) {
            run.state = ManualState::Cancelling;
        }
        tracing::info!(event="extension.manual.cancel_requested", correlation_id=%id, "manual extension cancellation requested");
        self.notify_manual_run(id, cx);
        cx.notify();
    }
    pub(super) fn begin_manual_run(
        &mut self,
        id: Uuid,
        package_id: String,
        package_name: String,
        action_name: String,
    ) {
        // Completed records only are evicted. Active permits remain runtime-owned.
        while self
            .manual_runs
            .iter()
            .filter(|run| !run.state.pending())
            .count()
            >= 32
        {
            if let Some(index) = self.manual_runs.iter().position(|run| !run.state.pending()) {
                if let Some(run) = self.manual_runs.remove(index)
                    && let Some(service) = &self.service
                {
                    service.manual_effects.remove(run.id);
                }
            } else {
                break;
            }
        }
        self.manual_runs.push_back(ManualRun {
            id,
            package_id,
            package_name,
            action_name,
            state: ManualState::Running,
            started: Instant::now(),
            at_unix_ms: chrono::Utc::now().timestamp_millis(),
            elapsed_ms: None,
            message: None,
            items: Vec::new(),
            origin: None,
            progress: None,
            generation: self.generation,
        });
    }
    pub(super) fn finish_manual_run(
        &mut self,
        id: Uuid,
        outcome: Result<InvokeOutcome, RuntimeError>,
        cx: &mut Context<Self>,
    ) {
        if self.pending_manual.remove(&id).is_none() {
            return;
        }
        let items = self
            .service
            .as_ref()
            .map(|service| service.manual_effects.finish(id))
            .unwrap_or_default();
        let Some(run) = self.manual_runs.iter_mut().find(|run| run.id == id) else {
            cx.notify();
            return;
        };
        let (state, message) = match outcome {
            Ok(outcome) => {
                let state = match outcome.status.as_str() {
                    "success" | "handled" => ManualState::Succeeded,
                    "not_handled" => ManualState::NotHandled,
                    _ => ManualState::Failed(RuntimeError::HandlerFailed),
                };
                // Explicitly requested UI content, never diagnostics. No control sequences/markup.
                let message = outcome
                    .value
                    .get("text")
                    .and_then(serde_json::Value::as_str)
                    .filter(|text| {
                        text.len() <= 4096
                            && !text
                                .chars()
                                .any(|c| c.is_control() && !matches!(c, '\n' | '\t'))
                    })
                    .map(str::to_owned);
                (state, message)
            }
            Err(RuntimeError::Cancelled) => (ManualState::Cancelled, None),
            Err(error) => (ManualState::Failed(error), None),
        };
        run.state = state;
        run.progress = None;
        run.message = message;
        run.items = items;
        run.elapsed_ms = Some(run.started.elapsed().as_millis());
        run.at_unix_ms = chrono::Utc::now().timestamp_millis();
        // No plugin-provided text/name or clipboard content in diagnostics.
        tracing::info!(event="extension.manual.terminal", correlation_id=%id, elapsed_ms=run.elapsed_ms, result_count=run.items.len(), success=matches!(run.state, ManualState::Succeeded | ManualState::NotHandled), "manual extension invocation terminated");
        self.notify_manual_run(id, cx);
        while self
            .manual_runs
            .iter()
            .filter(|run| !run.state.pending())
            .count()
            > 32
        {
            let Some(index) = self.manual_runs.iter().position(|run| !run.state.pending()) else {
                break;
            };
            if let Some(retired) = self.manual_runs.remove(index)
                && let Some(service) = &self.service
            {
                service.manual_effects.remove(retired.id);
            }
        }
        cx.notify();
    }
}
pub(crate) fn progress_view(
    progress: &nxtools_extensions::InvocationProgress,
    id: impl Into<gpui_kit::ElementId>,
) -> impl gpui_kit::IntoElement {
    use gpui_kit::component::{Sizable as _, Size, progress::Progress};
    use gpui_kit::{InteractiveElement as _, ParentElement as _, Styled as _, div};
    let percentage = progress.percentage();
    let label = percentage.map_or_else(
        || "插件报告：处理中".to_owned(),
        |value| format!("插件报告进度：{value:.0}%"),
    );
    div()
        .w_full()
        .min_w_0()
        .flex()
        .flex_col()
        .gap_1()
        .child(
            div()
                .debug_selector(|| "extension-progress-track".into())
                .w_full()
                .flex_shrink_0()
                .child(
                    Progress::new(id)
                        .with_size(Size::XSmall)
                        .accessibility_label(label.clone())
                        .value(percentage.unwrap_or(0.0))
                        .loading(percentage.is_none()),
                ),
        )
        .child(
            progress
                .message
                .clone()
                .map_or(label.clone(), |message| format!("{label} · {message}")),
        )
}
pub(crate) type ManualRuns = VecDeque<ManualRun>;

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::{AppContext, TestAppContext, WeakEntity};

    #[gpui_kit::test]
    fn remediation_manual_completion_status_matrix_keeps_safe_failure_explanation(
        cx: &mut TestAppContext,
    ) {
        let owner = cx.new(|_| {
            ExtensionApplication::new(
                Default::default(),
                Default::default(),
                None,
                WeakEntity::new_invalid(),
            )
        });
        for (status, expected) in [
            ("handled", ManualState::Succeeded),
            ("success", ManualState::Succeeded),
            ("not_handled", ManualState::NotHandled),
            ("error", ManualState::Failed(RuntimeError::HandlerFailed)),
        ] {
            owner.update(cx, |app, cx| {
                let id = Uuid::new_v4();
                let (cancel, _receiver) = tokio::sync::watch::channel(false);
                app.pending_manual.insert(id, ("fixture".into(), cancel));
                app.begin_manual_run(id, "fixture".into(), "Fixture".into(), "Action".into());
                app.finish_manual_run(
                    id,
                    Ok(InvokeOutcome {
                        identity: InvocationIdentity {
                            run_id: Uuid::new_v4(),
                            correlation_id: id,
                            generation: 1,
                            package_id: "fixture".into(),
                            handler_id: "run".into(),
                            content_hash: "a".repeat(64),
                        },
                        status: status.into(),
                        value: serde_json::json!({"text":"请检查输入和设置"}),
                    }),
                    cx,
                );
                let run = app.manual_run(id).expect("terminal run");
                assert_eq!(run.state, expected, "{status}");
                assert_eq!(run.message.as_deref(), Some("请检查输入和设置"));
                assert!(!app.pending_manual.contains_key(&id));
            });
        }
    }
    #[gpui_kit::test]
    fn remediation_real_script_error_reaches_manual_feedback_without_becoming_native_failure(
        cx: &mut TestAppContext,
    ) {
        let root = tempfile::tempdir().expect("owned runtime feedback");
        let source = root.path().join("source");
        std::fs::create_dir(&source).expect("source");
        let manifest = serde_json::json!({"schema_version":1,"id":"fixture.feedback","name":"Feedback","version":"1","runtime":{"program":"python"},"files":["entry.py"],"handlers":[{"id":"run","title":"Run","feature":"tools","entry":"action","action":{"type":"script","entry_file":"entry.py"}}]});
        std::fs::write(
            source.join("extension.json"),
            serde_json::to_vec(&manifest).expect("manifest"),
        )
        .expect("manifest write");
        std::fs::write(source.join("entry.py"), "import json,sys\nf=json.loads(sys.stdin.readline())\nprint(json.dumps({'protocol_version':1,'op':'complete','run_id':f['run_id'],'status':'error','value':{'text':'Check input settings'}}),flush=True)\n").expect("script");
        let preview = nxtools_extensions::ImportPreview::from_directory(&source)
            .expect("preview")
            .stage_into(&root.path().join("packages"))
            .expect("stage");
        let hash = preview.content_hash.clone();
        let mut config = nxtools_core::AppConfig::default();
        config
            .extensions
            .install(preview, "owned fixture".into(), false)
            .expect("install");
        let python = std::env::split_paths(&std::env::var_os("PATH").expect("PATH"))
            .map(|directory| directory.join("python.exe"))
            .find(|path| path.is_file())
            .expect("installed interpreter");
        config
            .extensions
            .approve(
                "fixture.feedback",
                &hash,
                nxtools_extensions::RuntimeIdentity::inspect(&python).expect("identity"),
            )
            .expect("approve");
        config
            .extensions
            .set_enabled("fixture.feedback", true, &[])
            .expect("enable");
        let engine = nxtools_download::DownloadEngine::open(root.path().join("downloads.db"))
            .expect("engine");
        let (service, _) = crate::extensions::service::ExtensionService::new(
            &engine,
            config.extensions.clone(),
            None,
            false,
        )
        .expect("service");
        let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).ancestors().nth(3).expect("workspace").join("csharp/NxTools.PlatformHost.Tests/bin/Debug/net8.0-windows/NxTools.PlatformHost.Tests.exe");
        {
            let executor = service.executor();
            let _entered = executor.enter();
            service
                .runtime
                .bind_owned_fixture_broker(&fixture)
                .expect("isolated broker");
        }
        cx.executor().allow_parking();
        let owner = cx.new(|_| {
            let mut app = ExtensionApplication::new(
                config,
                root.path().join("unused.json"),
                None,
                WeakEntity::new_invalid(),
            );
            app.service = Some(service.clone());
            app
        });
        let id = owner.update(cx, |app, cx| {
            app.start_manual(
                "fixture.feedback".into(),
                "run".into(),
                "tools",
                serde_json::json!({}),
                cx,
            )
            .expect("admit")
        });
        let deadline = Instant::now() + std::time::Duration::from_secs(10);
        loop {
            cx.run_until_parked();
            if !owner.read_with(cx, |app, _| app.has_pending_manual()) {
                break;
            }
            assert!(Instant::now() < deadline, "actual run completion");
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let run = owner.read_with(cx, |app, _| app.manual_run(id).expect("run"));
        assert_eq!(run.state, ManualState::Failed(RuntimeError::HandlerFailed));
        assert_eq!(run.message.as_deref(), Some("Check input settings"));
        service.runtime.shutdown();
    }
    #[gpui_kit::test]
    fn manual_runs_cancel_only_the_selected_run_and_terminal_is_once(cx: &mut TestAppContext) {
        let owner = cx.new(|_| {
            ExtensionApplication::new(
                Default::default(),
                Default::default(),
                None,
                WeakEntity::new_invalid(),
            )
        });
        let first = Uuid::new_v4();
        let second = Uuid::new_v4();
        let (first_cancel, first_rx) = tokio::sync::watch::channel(false);
        let (second_cancel, second_rx) = tokio::sync::watch::channel(false);
        owner.update(cx, |app, cx| {
            app.pending_manual
                .insert(first, ("fixture.one".into(), first_cancel));
            app.pending_manual
                .insert(second, ("fixture.two".into(), second_cancel));
            app.begin_manual_run(first, "fixture.one".into(), "One".into(), "Clean".into());
            app.begin_manual_run(second, "fixture.two".into(), "Two".into(), "Other".into());
            app.cancel_manual_run(first, cx);
            assert!(*first_rx.borrow());
            assert!(!*second_rx.borrow());
            assert_eq!(app.pending_manual.len(), 2);
            assert_eq!(
                app.manual_run(first).expect("first").state,
                ManualState::Cancelling
            );
            assert_eq!(
                app.manual_run(second).expect("second").state,
                ManualState::Running
            );
            app.finish_manual_run(first, Err(RuntimeError::Cancelled), cx);
            assert_eq!(app.pending_manual.len(), 1);
            app.finish_manual_run(first, Err(RuntimeError::Protocol), cx);
            assert_eq!(
                app.manual_run(first).expect("once").state,
                ManualState::Cancelled
            );
            let identity = InvocationIdentity {
                run_id: Uuid::new_v4(),
                correlation_id: second,
                generation: 0,
                package_id: "fixture.two".into(),
                handler_id: "other".into(),
                content_hash: "a".repeat(64),
            };
            app.finish_manual_run(
                second,
                Ok(InvokeOutcome {
                    identity,
                    status: "not_handled".into(),
                    value: serde_json::json!({"text":"文本无需整理。","item_id":999}),
                }),
                cx,
            );
            let run = app.manual_run(second).expect("complete");
            assert_eq!(run.state, ManualState::NotHandled);
            assert!(
                run.items.is_empty(),
                "script-reported ID is not a host commit"
            );
            assert_eq!(run.message.as_deref(), Some("文本无需整理。"));
            assert!(!app.has_pending_manual());
        });
    }
    #[gpui_kit::test]
    fn recent_manual_records_are_bounded_and_never_evict_pending(cx: &mut TestAppContext) {
        let owner = cx.new(|_| {
            ExtensionApplication::new(
                Default::default(),
                Default::default(),
                None,
                WeakEntity::new_invalid(),
            )
        });
        owner.update(cx, |app, cx| {
            let active = Uuid::new_v4();
            app.begin_manual_run(active, "active".into(), "Active".into(), "Wait".into());
            for _ in 0..40 {
                let id = Uuid::new_v4();
                let (cancel, _rx) = tokio::sync::watch::channel(false);
                app.pending_manual.insert(id, ("done".into(), cancel));
                app.begin_manual_run(id, "done".into(), "Done".into(), "Done".into());
                app.finish_manual_run(id, Err(RuntimeError::Timeout), cx);
            }
            assert!(app.manual_run(active).is_some());
            assert!(app.recent_manual_runs().len() <= 33);
            let mut concurrent = Vec::new();
            for _ in 0..4 {
                let id = Uuid::new_v4();
                let (cancel, _rx) = tokio::sync::watch::channel(false);
                app.pending_manual.insert(id, ("parallel".into(), cancel));
                app.begin_manual_run(id, "parallel".into(), "Parallel".into(), "Wait".into());
                concurrent.push(id);
            }
            for id in concurrent {
                app.finish_manual_run(id, Err(RuntimeError::Timeout), cx);
            }
            assert_eq!(
                app.recent_manual_runs()
                    .iter()
                    .filter(|run| !run.state.pending())
                    .count(),
                32
            );
            assert!(app.manual_run(active).is_some());
        });
    }
    #[test]
    fn effects_require_registered_identity_and_do_not_accept_script_result_ids() {
        let effects = ManualEffects::default();
        let id = Uuid::new_v4();
        let mut identity = InvocationIdentity {
            run_id: Uuid::new_v4(),
            correlation_id: id,
            generation: 1,
            package_id: "fixture".into(),
            handler_id: "clean".into(),
            content_hash: "a".repeat(64),
        };
        effects.committed(&identity, 90);
        assert!(effects.items(id).is_empty());
        effects.begin(id, "fixture", "clean", &"a".repeat(64));
        identity.content_hash = "b".repeat(64);
        effects.committed(&identity, 90);
        assert!(effects.items(id).is_empty());
        identity.content_hash = "a".repeat(64);
        effects.committed(&identity, 90);
        effects.committed(&identity, 90);
        assert_eq!(effects.finish(id), vec![90]);
        effects.committed(&identity, 91);
        assert_eq!(effects.items(id), vec![90, 91]);
    }
}
