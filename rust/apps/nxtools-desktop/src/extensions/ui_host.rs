use super::{
    ExtensionApplication, ExtensionApplicationGlobal, feature_adapters::UiHostCall,
    service::ExtensionService,
};
use gpui_kit::{App, Context, Entity, Global, WeakEntity};
use nxtools_extensions::HostReply;
use nxtools_protocol::{LaunchProcessCommand, OpenShellTargetCommand, PlatformCommand};
use serde::Deserialize;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

pub(crate) struct ExtensionServiceGlobal(pub ExtensionService);
impl Global for ExtensionServiceGlobal {}
impl ExtensionApplication {
    pub(crate) fn bind_ui(
        &mut self,
        clipboard: WeakEntity<crate::clipboard::ClipboardApplication>,
        notifications: Entity<crate::notifications::NotificationCenter>,
        cx: &mut Context<Self>,
    ) {
        self.clipboard = Some(clipboard);
        self.notifications = Some(notifications);
        self.service = cx
            .try_global::<ExtensionServiceGlobal>()
            .map(|global| global.0.clone());
        self.event_control = self.service.as_ref().map(|service| service.events.clone());
        if let Some(service) = &self.service {
            let mut changed = service.manual_effects.subscribe();
            cx.spawn(async move |this, cx| {
                while changed.changed().await.is_ok() {
                    if this
                        .update(cx, |app, cx| {
                            app.refresh_manual_result_notifications(cx);
                            cx.notify();
                        })
                        .is_err()
                    {
                        break;
                    }
                }
            })
            .detach();
        }
        if let Some(control) = self.event_control.as_ref() {
            let mut changed = control.subscribe();
            cx.spawn(async move |this, cx| {
                while changed.changed().await.is_ok() {
                    if this.update(cx, |_, cx| cx.notify()).is_err() {
                        break;
                    }
                }
            })
            .detach();
        }
    }
    pub(crate) fn handle_host_call(&mut self, call: UiHostCall, cx: &mut Context<Self>) {
        let current = self.packages.packages.iter().any(|package| {
            package.enabled
                && package.trust.is_some()
                && package.manifest.id == call.identity.package_id
                && package.content_hash == call.identity.content_hash
        });
        if !current || *call.cancellation.borrow() {
            respond(call.reply, HostReply::error("stale_or_cancelled"), cx);
            return;
        }
        let call_id = uuid::Uuid::new_v4();
        match call.method.as_str() {
            "common.open" | "common.program" => {
                let command = if call.method == "common.open" {
                    let Ok(input) = serde_json::from_value::<Open>(call.params) else {
                        respond(call.reply, HostReply::error("invalid_params"), cx);
                        return;
                    };
                    if input.target.is_empty()
                        || input.target.len() > 32768
                        || input.target.contains('\0')
                    {
                        respond(call.reply, HostReply::error("invalid_params"), cx);
                        return;
                    }
                    PlatformCommand::OpenShellTarget(OpenShellTargetCommand {
                        target: input.target,
                        action: nxtools_protocol::ShellOpenAction::Open,
                    })
                } else {
                    let Ok(input) = serde_json::from_value::<Program>(call.params) else {
                        respond(call.reply, HostReply::error("invalid_params"), cx);
                        return;
                    };
                    if input.executable.is_empty()
                        || input.executable.len() > 32768
                        || input.executable.contains('\0')
                        || input.arguments.len() > 128
                        || input
                            .arguments
                            .iter()
                            .any(|value| value.len() > 32768 || value.contains('\0'))
                    {
                        respond(call.reply, HostReply::error("invalid_params"), cx);
                        return;
                    }
                    PlatformCommand::LaunchProcess(LaunchProcessCommand {
                        executable: input.executable,
                        arguments: input.arguments,
                        working_directory: input.working_directory,
                        run_as_admin: false,
                        run_silently: false,
                    })
                };
                let Some(platform) = self.platform.as_ref() else {
                    respond(call.reply, HostReply::error("ui_unavailable"), cx);
                    return;
                };
                if self.pending_host.len() >= 8 {
                    respond(call.reply, HostReply::error("busy"), cx);
                    return;
                }
                self.pending_host.insert(call_id, call.reply);
                if platform.send_command_with_id(command, call_id).is_err() {
                    if let Some(reply) = self.pending_host.remove(&call_id) {
                        respond(reply, HostReply::error("disconnected"), cx);
                    }
                } else {
                    cx.spawn(async move |this, cx| {
                        cx.background_executor()
                            .timer(std::time::Duration::from_secs(15))
                            .await;
                        let _ = this.update(cx, |app, cx| {
                            if let Some(reply) = app.pending_host.remove(&call_id) {
                                respond(reply, HostReply::error("native_timeout"), cx);
                            }
                        });
                    })
                    .detach();
                }
            }
            "common.copy" | "clipboard.copy" => {
                let Ok(input) = serde_json::from_value::<Copy>(call.params) else {
                    respond(call.reply, HostReply::error("invalid_params"), cx);
                    return;
                };
                let content = match (input.text, input.paths) {
                    (Some(text), None) => {
                        crate::clipboard::extension_adapter::ClipboardExtensionContent::Text(text)
                    }
                    (None, Some(paths)) => {
                        crate::clipboard::extension_adapter::ClipboardExtensionContent::Files(paths)
                    }
                    _ => {
                        respond(call.reply, HostReply::error("invalid_params"), cx);
                        return;
                    }
                };
                let Some(clipboard) = self.clipboard.clone() else {
                    respond(call.reply, HostReply::error("ui_unavailable"), cx);
                    return;
                };
                let causality = crate::clipboard_worker::ClipboardCommitCausality {
                    root_operation_id: call.causality.root_id,
                    origin_plugin: call.causality.origin_plugin,
                    depth: call.causality.handlers.len() as u8,
                    invoked_handlers: call.causality.handlers,
                    derived_from: None,
                };
                let cancelled = Arc::new(AtomicBool::new(*call.cancellation.borrow()));
                let cancel_flag = cancelled.clone();
                let mut cancellation = call.cancellation;
                let monitor = self.service.as_ref().map(|service| {
                    let mut cancellation = cancellation.clone();
                    let cancelled = cancelled.clone();
                    service.executor().spawn(async move {
                        if cancellation.changed().await.is_ok() {
                            cancelled.store(true, Ordering::Release);
                        }
                    })
                });
                let receiver =
                    crate::clipboard::extension_adapter::ClipboardExtensionAdapter::copy_deferred(
                        clipboard, content, causality, cancelled, cx,
                    );
                cx.spawn(async move |_, cx| {
                    let outcome = futures_copy(receiver, &mut cancellation, cancel_flag).await;
                    if let Some(monitor) = monitor {
                        monitor.abort();
                    }
                    cx.update(|cx| {
                        cx.defer(move |_| {
                            let _ = call.reply.try_send(outcome);
                        })
                    });
                })
                .detach();
            }
            "common.notify" => {
                let Ok(input) = serde_json::from_value::<Notice>(call.params) else {
                    respond(call.reply, HostReply::error("invalid_params"), cx);
                    return;
                };
                if input.title.len() > 256 || input.message.len() > 4096 {
                    respond(call.reply, HostReply::error("invalid_params"), cx);
                    return;
                }
                let Some(notifications) = self.notifications.clone() else {
                    respond(call.reply, HostReply::error("ui_unavailable"), cx);
                    return;
                };
                cx.defer(move |cx| {
                    if *call.cancellation.borrow() {
                        let _ = call.reply.try_send(HostReply::error("cancelled"));
                        return;
                    }
                    notifications.update(cx, |owner, cx| {
                        owner.show_info(
                            format!("extension:{call_id}"),
                            input.title,
                            input.message,
                            cx,
                        )
                    });
                    let _ = call.reply.try_send(HostReply {
                        success: true,
                        value: serde_json::json!({}),
                    });
                });
            }
            "quick_launch.search" => {
                let Ok(input) = serde_json::from_value::<Query>(call.params) else {
                    respond(call.reply, HostReply::error("invalid_params"), cx);
                    return;
                };
                if input.query.len() > 4096 {
                    respond(call.reply, HostReply::error("invalid_params"), cx);
                    return;
                }
                let quick = self.quick_launch.clone();
                cx.defer(move |cx| {
                    if *call.cancellation.borrow() {
                        let _ = call.reply.try_send(HostReply::error("cancelled"));
                        return;
                    }
                    let reply = quick
                        .update(cx, |quick, _| quick.extension_search(&input.query))
                        .unwrap_or_else(|_| HostReply::error("ui_unavailable"));
                    let _ = call.reply.try_send(reply);
                });
            }
            "quick_launch.query" => {
                let Ok(input) = serde_json::from_value::<Query>(call.params) else {
                    respond(call.reply, HostReply::error("invalid_params"), cx);
                    return;
                };
                let quick = self.quick_launch.clone();
                cx.defer(move |cx| {
                    let _ = quick.update(cx, |quick, cx| {
                        quick.extension_set_query(input.query, call.cancellation, call.reply, cx)
                    });
                });
            }
            _ => respond(call.reply, HostReply::error("unsupported"), cx),
        }
    }
}
async fn futures_copy(
    receiver: async_channel::Receiver<
        anyhow::Result<crate::clipboard::extension_adapter::ClipboardExtensionCopyCompleted>,
    >,
    cancellation: &mut tokio::sync::watch::Receiver<bool>,
    cancelled: Arc<AtomicBool>,
) -> HostReply {
    // GPUI executor is not Tokio; use its async channel as the completion future.
    let outcome = receiver.recv().await;
    if *cancellation.borrow() {
        cancelled.store(true, Ordering::Release);
        return HostReply::error("cancelled");
    }
    match outcome {
        Ok(Ok(result)) => HostReply {
            success: true,
            value: serde_json::json!({"operation_id":result.operation_id}),
        },
        _ => HostReply::error("clipboard_copy_failed"),
    }
}
fn respond(
    reply: async_channel::Sender<HostReply>,
    outcome: HostReply,
    cx: &mut Context<ExtensionApplication>,
) {
    cx.defer(move |_| {
        let _ = reply.try_send(outcome);
    });
}
pub(crate) fn start_ui_host(receiver: async_channel::Receiver<UiHostCall>, cx: &mut App) {
    cx.spawn(async move |cx| {
        while let Ok(call) = receiver.recv().await {
            cx.update(|cx| {
                let owner = cx
                    .try_global::<ExtensionApplicationGlobal>()
                    .map(|global| global.0.clone());
                cx.defer(move |cx| {
                    if let Some(owner) = owner {
                        let reply = call.reply.clone();
                        if owner
                            .update(cx, |owner, cx| owner.handle_host_call(call, cx))
                            .is_err()
                        {
                            let _ = reply.try_send(HostReply::error("ui_unavailable"));
                        }
                    } else {
                        let _ = call.reply.try_send(HostReply::error("ui_unavailable"));
                    }
                });
            });
        }
    })
    .detach();
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Open {
    target: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Program {
    executable: String,
    #[serde(default)]
    arguments: Vec<String>,
    #[serde(default)]
    working_directory: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Copy {
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    paths: Option<Vec<String>>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Notice {
    title: String,
    message: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Query {
    query: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::{AppContext, TestAppContext};
    use std::{path::PathBuf, rc::Rc};
    #[gpui_kit::test]
    fn actual_ui_host_queue_rejects_stale_and_elevation_and_waits_for_native_terminal(
        cx: &mut TestAppContext,
    ) {
        let (platform, endpoint) = nxtools_ipc::platform_bridge(8);
        endpoint.set_connected_for_contract_test(true);
        let mut config = nxtools_core::AppConfig::default();
        let manifest=br#"{"schema_version":1,"id":"fixture.host","name":"Host","version":"1","handlers":[{"id":"action","title":"Action","feature":"tools","entry":"action","action":{"type":"folder","path":"C:\\fixture"}}]}"#;
        config
            .extensions
            .install(
                nxtools_extensions::ImportPreview::from_json(manifest).expect("preview"),
                "fixture".into(),
                false,
            )
            .expect("install");
        config.extensions.packages[0].enabled = true;
        let hash = config.extensions.packages[0].content_hash.clone();
        config.extensions.packages[0].trust = Some(nxtools_extensions::TrustApproval {
            content_hash: hash.clone(),
            runtime: nxtools_extensions::RuntimeIdentity {
                executable: PathBuf::from("C:\\fixture.exe"),
                sha256: "a".repeat(64),
            },
        }); // isolated fixture state; never serialized or executed.
        let app = cx.new(|_| {
            ExtensionApplication::new(
                config,
                PathBuf::new(),
                Some(Rc::new(platform)),
                WeakEntity::new_invalid(),
            )
        });
        cx.update(|cx| super::super::register_global(&app, cx));
        let (sender, receiver) = async_channel::bounded(4);
        cx.update(|cx| start_ui_host(receiver, cx));
        let (cancel, cancellation) = tokio::sync::watch::channel(false);
        let identity = nxtools_extensions::InvocationIdentity {
            run_id: uuid::Uuid::new_v4(),
            correlation_id: uuid::Uuid::new_v4(),
            generation: 1,
            package_id: "fixture.host".into(),
            handler_id: "action".into(),
            content_hash: hash,
        };
        let (reply, response) = async_channel::bounded(1);
        sender
            .try_send(UiHostCall {
                identity: identity.clone(),
                method: "common.program".into(),
                params: serde_json::json!({"executable":"C:\\fixture.exe","arguments":["one"]}),
                causality: nxtools_extensions::Causality::root(identity.correlation_id),
                cancellation: cancellation.clone(),
                reply,
            })
            .expect("queue actual host call");
        assert!(
            endpoint.try_receive_command_for_contract_test().is_err(),
            "dispatch is deferred"
        );
        cx.run_until_parked();
        let command = endpoint
            .try_receive_command_for_contract_test()
            .expect("deferred command");
        assert!(
            matches!(&command.message,nxtools_protocol::Message::PlatformCommand(PlatformCommand::LaunchProcess(command))if !command.run_as_admin&&command.arguments==["one"])
        );
        assert!(
            response.try_recv().is_err(),
            "admission isn't native success"
        );
        app.update(cx, |app, cx| {
            app.handle_platform_notification(
                nxtools_ipc::PlatformNotification::CommandResult {
                    correlation_id: command.id,
                    result: nxtools_protocol::PlatformCommandResult {
                        success: true,
                        error: None,
                        native_error_code: None,
                        data: None,
                    },
                },
                cx,
            );
            assert!(
                response.try_recv().is_err(),
                "terminal callback delivery deferred"
            );
        });
        cx.run_until_parked();
        assert!(response.try_recv().expect("native terminal").success);
        for cancelled in [false, true] {
            let (reply, response) = async_channel::bounded(1);
            let mut stale = identity.clone();
            if !cancelled {
                stale.content_hash = "b".repeat(64);
            } else {
                cancel.send(true).expect("cancel");
            }
            sender
                .try_send(UiHostCall {
                    identity: stale,
                    method: "common.open".into(),
                    params: serde_json::json!({"target":"https://fixture.test"}),
                    causality: nxtools_extensions::Causality::root(identity.correlation_id),
                    cancellation: cancellation.clone(),
                    reply,
                })
                .expect("queue retired");
            cx.run_until_parked();
            assert!(!response.try_recv().expect("retired reply").success);
            assert!(endpoint.try_receive_command_for_contract_test().is_err());
        }
    }
}
