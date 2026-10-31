use super::*;

#[derive(Clone, Copy)]
pub(super) enum PlatformRequestOperation {
    Environment,
    OpenDirectory,
    SettingPath,
}
impl PlatformRequestOperation {
    pub(super) fn terminal_event(self) -> &'static str {
        match self {
            Self::Environment => "extension.environment.terminal",
            Self::OpenDirectory => "extension.directory.terminal",
            Self::SettingPath => "extension.setting_path.terminal",
        }
    }
    fn accepted_event(self) -> &'static str {
        match self {
            Self::Environment => "extension.environment.accepted",
            Self::OpenDirectory => "extension.directory.dispatch",
            Self::SettingPath => "extension.setting_path.accepted",
        }
    }
}

impl ExtensionApplication {
    pub(crate) fn save_runtime_defaults(
        &mut self,
        paths: std::collections::BTreeMap<String, PathBuf>,
        cx: &mut Context<Self>,
    ) {
        self.commit(
            "runtime_defaults",
            move |path| AppConfig::save_extension_runtime_defaults(path, paths),
            cx,
        );
    }
    pub(crate) fn save_runtime_override(
        &mut self,
        id: String,
        executable: Option<PathBuf>,
        cx: &mut Context<Self>,
    ) {
        self.commit(
            "runtime_override",
            move |path| AppConfig::set_extension_runtime_override(path, &id, executable),
            cx,
        );
    }
    pub(crate) fn request_setting_path(
        &mut self,
        command: PlatformCommand,
        id: uuid::Uuid,
        cx: &mut Context<Self>,
    ) -> Option<async_channel::Receiver<nxtools_protocol::PlatformCommandResult>> {
        self.request_platform(command, id, PlatformRequestOperation::SettingPath, cx)
    }
    pub(crate) fn request_runtime(
        &mut self,
        command: PlatformCommand,
        cx: &mut Context<Self>,
    ) -> Option<async_channel::Receiver<nxtools_protocol::PlatformCommandResult>> {
        self.request_platform(
            command,
            uuid::Uuid::new_v4(),
            PlatformRequestOperation::Environment,
            cx,
        )
    }
    pub(super) fn request_platform(
        &mut self,
        command: PlatformCommand,
        id: uuid::Uuid,
        operation: PlatformRequestOperation,
        cx: &mut Context<Self>,
    ) -> Option<async_channel::Receiver<nxtools_protocol::PlatformCommandResult>> {
        if self.pending_runtime.len() >= 4 || self.platform.is_none() {
            info!(event=operation.terminal_event(),correlation_id=%id,success=false,reason="unavailable",elapsed_ms=0,"extension platform operation rejected");
            return None;
        }
        let platform = self.platform.as_ref()?;
        let (reply, receiver) = async_channel::bounded(1);
        let timeout = if matches!(&command, PlatformCommand::PickExtensionRuntime(_)) {
            300
        } else {
            30
        };
        self.pending_runtime
            .insert(id, (Instant::now(), reply, operation));
        if platform.send_command_with_id(command, id).is_err() {
            self.pending_runtime.remove(&id);
            info!(event=operation.terminal_event(),correlation_id=%id,success=false,reason="dispatch_failed",elapsed_ms=0,"extension platform operation terminated");
            return None;
        }
        info!(event=operation.accepted_event(),correlation_id=%id,"extension platform operation admitted");
        if matches!(operation, PlatformRequestOperation::SettingPath) {
            return Some(receiver);
        }
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(std::time::Duration::from_secs(timeout))
                .await;
            let _ = this.update(cx, |app, _| {
                if let Some((started, reply, operation)) = app.pending_runtime.remove(&id) {
                    info!(event=operation.terminal_event(),correlation_id=%id,success=false,reason="timeout",native_error_code=1460,elapsed_ms=started.elapsed().as_millis(),"extension platform operation terminated");
                    let message = match operation {
                        PlatformRequestOperation::Environment => "运行环境操作超时，请重试",
                        PlatformRequestOperation::OpenDirectory => "打开插件目录超时，请确认后重试",
                        PlatformRequestOperation::SettingPath => "路径选择尚未完成",
                    };
                    let _ = reply.try_send(nxtools_protocol::PlatformCommandResult {
                        success: false,
                        error: Some(message.into()),
                        native_error_code: Some(1460),
                        data: None,
                    });
                }
            });
        })
        .detach();
        Some(receiver)
    }
}
