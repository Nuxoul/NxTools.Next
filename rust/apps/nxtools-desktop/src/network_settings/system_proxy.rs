use super::*;
use nxtools_protocol::system_proxy::SystemProxySnapshot;

pub(super) struct PendingProxySnapshot {
    pub(super) id: uuid::Uuid,
    view: WeakEntity<NetworkSettingsView>,
    window: Option<AnyWindowHandle>,
    generation: u64,
    started: Instant,
}
impl NetworkSettingsApplication {
    pub(super) fn request_system_proxy(
        &mut self,
        view: WeakEntity<NetworkSettingsView>,
        generation: u64,
        cx: &mut Context<Self>,
    ) {
        if !view.upgrade().is_some_and(|view| {
            let view = view.read(cx);
            view.mode == NetworkProxyMode::System && view.proxy_generation == generation
        }) {
            return;
        }
        let id = uuid::Uuid::new_v4();
        self.pending_proxy_snapshot = Some(PendingProxySnapshot {
            id,
            view,
            window: self.window,
            generation,
            started: Instant::now(),
        });
        info!(event="system.proxy.snapshot.accepted", correlation_id=%id, generation);
        let queued = self.platform.as_ref().is_some_and(|platform| {
            platform
                .send_command_with_id(PlatformCommand::ReadSystemProxy, id)
                .is_ok()
        });
        if !queued {
            self.complete_system_proxy(id, None, cx);
            return;
        }
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(std::time::Duration::from_secs(5))
                .await;
            let _ = this.update(cx, |owner, cx| owner.complete_system_proxy(id, None, cx));
        })
        .detach();
    }
    pub(super) fn complete_system_proxy(
        &mut self,
        id: uuid::Uuid,
        snapshot: Option<SystemProxySnapshot>,
        cx: &mut Context<Self>,
    ) {
        if self
            .pending_proxy_snapshot
            .as_ref()
            .is_none_or(|pending| pending.id != id)
        {
            return;
        }
        let pending = self
            .pending_proxy_snapshot
            .take()
            .expect("matching snapshot");
        let snapshot = snapshot.filter(SystemProxySnapshot::is_bounded);
        info!(event="system.proxy.snapshot.terminal", correlation_id=%id, success=snapshot.is_some(), elapsed_ms=pending.started.elapsed().as_millis());
        if self.window != pending.window {
            return;
        }
        let Some(source) = pending.window else {
            return;
        };
        cx.defer(move |cx| {
            let _ = source.update(cx, |_, window, cx| {
                let _ = pending.view.update(cx, |view, cx| {
                    if view.mode != NetworkProxyMode::System
                        || view.proxy_generation != pending.generation
                        || !view.proxy_loading
                    {
                        return;
                    }
                    view.proxy_loading = false;
                    view.proxy_rows = snapshot
                        .as_ref()
                        .map(|snapshot| {
                            snapshot
                                .endpoints
                                .iter()
                                .map(|endpoint| {
                                    let host = cx.new(|cx| {
                                        InputState::new(window, cx)
                                            .default_value(endpoint.host.clone())
                                    });
                                    let port = cx.new(|cx| {
                                        InputState::new(window, cx)
                                            .default_value(endpoint.port.to_string())
                                    });
                                    (endpoint.protocol.clone(), host, port)
                                })
                                .collect()
                        })
                        .unwrap_or_default();
                    view.proxy_snapshot = snapshot;
                    view.proxy_read_failed = view.proxy_snapshot.is_none();
                    cx.notify();
                });
            });
        });
    }
}
impl NetworkSettingsView {
    pub(super) fn select_proxy_mode(&mut self, mode: NetworkProxyMode, cx: &mut Context<Self>) {
        self.mode = mode;
        self.error = None;
        self.proxy_generation = self.proxy_generation.wrapping_add(1);
        self.proxy_loading = mode == NetworkProxyMode::System;
        self.proxy_snapshot = None;
        self.proxy_rows.clear();
        self.proxy_read_failed = false;
        if self.proxy_loading {
            let application = self.application.clone();
            let view = cx.entity().downgrade();
            let generation = self.proxy_generation;
            cx.defer(move |cx| {
                application.update(cx, |owner, cx| {
                    owner.request_system_proxy(view, generation, cx)
                })
            });
        }
        cx.notify();
    }
    pub(super) fn render_system_proxy(&self) -> gpui_kit::AnyElement {
        let mut body = div()
            .debug_selector(|| "network-settings-system-proxy".into())
            .flex()
            .flex_col()
            .gap_2();
        let message = if self.proxy_loading {
            "正在读取系统代理…"
        } else if self.proxy_read_failed {
            "无法读取系统代理，请重新选择系统代理重试。"
        } else if let Some(snapshot) = &self.proxy_snapshot {
            if snapshot.pac_configured || snapshot.auto_detect {
                "系统启用了自动代理配置（PAC / 自动检测），实际地址可能随目标变化。"
            } else if snapshot.endpoints.is_empty() {
                "系统未设置静态代理。"
            } else {
                ""
            }
        } else {
            "系统代理尚未读取。"
        };
        if !message.is_empty() {
            body = body.child(gpui_kit::component::label::Label::new(message).text_sm());
        }
        for (protocol, host, port) in &self.proxy_rows {
            body = body.child(
                Form::vertical()
                    .small()
                    .columns(4)
                    .child(
                        Field::new()
                            .col_span(3)
                            .label(format!("代理地址（{protocol}）"))
                            .child(Input::new(host).disabled(true)),
                    )
                    .child(
                        Field::new()
                            .label("端口")
                            .child(Input::new(port).disabled(true)),
                    ),
            );
        }
        body.into_any_element()
    }
}
