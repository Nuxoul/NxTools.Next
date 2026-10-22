//! Open the same managed package root used by imports, without changing package state.
use super::*;

impl ExtensionApplication {
    pub(crate) fn opening_directory(&self) -> bool {
        self.pending_directory.is_some()
    }

    pub(crate) fn open_directory(&mut self, cx: &mut Context<Self>) {
        if self.opening_directory() {
            return;
        }
        let id = uuid::Uuid::new_v4();
        let started = Instant::now();
        self.pending_directory = Some((id, started));
        self.notify_directory(
            id,
            crate::notifications::NotificationLevel::Progress,
            "正在打开插件目录…",
            cx,
        );
        cx.notify();
        info!(event="extension.directory.accepted",correlation_id=%id,"managed directory open accepted");
        let root = self
            .config_path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .map(|parent| parent.join("extensions"));
        cx.spawn(async move |this, cx| {
            let prepared = cx.background_executor().spawn(async move {
                let root = root.ok_or_else(|| std::io::Error::other("configuration directory unavailable"))?;
                std::fs::create_dir_all(&root)?;
                Ok::<_, std::io::Error>(root)
            }).await;
            let receiver = this.update(cx, |app, cx| {
                if app.pending_directory.is_none_or(|(pending, _)| pending != id) {
                    return None;
                }
                let root = match prepared {
                    Ok(root) => root,
                    Err(failure) => {
                        info!(event="extension.directory.prepare_failed",correlation_id=%id,native_error_code=?failure.raw_os_error(),elapsed_ms=started.elapsed().as_millis(),"managed directory preparation failed");
                        app.finish_open_directory(id, false, "无法创建插件目录，请检查配置目录的访问权限后重试", cx);
                        return None;
                    }
                };
                let receiver = app.request_platform(
                    PlatformCommand::OpenShellTarget(nxtools_protocol::OpenShellTargetCommand {
                        target: root.to_string_lossy().into_owned(),
                        action: nxtools_protocol::ShellOpenAction::Open,
                    }),
                    id,
                    PlatformRequestOperation::OpenDirectory,
                    cx,
                );
                if receiver.is_none() {
                    app.finish_open_directory(id, false, "Windows 打开通道不可用或繁忙，请重试", cx);
                }
                receiver
            }).ok().flatten();
            if let Some(receiver) = receiver {
                let result = receiver.recv().await.ok();
                let _ = this.update(cx, |app, cx| {
                    let success = result.as_ref().is_some_and(|result| result.success);
                    let message = match result.as_ref() {
                        Some(result) if result.success => "已请求 Windows 打开插件目录",
                        Some(result) if result.native_error_code == Some(1460) => "打开插件目录超时，请确认后重试",
                        _ => "无法打开插件目录，请检查目录访问权限或 Windows 连接后重试",
                    };
                    app.finish_open_directory(id, success, message, cx);
                });
            }
        }).detach();
        // Also bounds preparation: a late filesystem reply cannot dispatch after retirement.
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(std::time::Duration::from_secs(30))
                .await;
            let _ = this.update(cx, |app, cx| {
                if app
                    .pending_directory
                    .is_some_and(|(pending, _)| pending == id)
                {
                    app.pending_runtime.remove(&id);
                    app.finish_open_directory(id, false, "打开插件目录超时，请确认后重试", cx);
                }
            });
        })
        .detach();
    }

    pub(super) fn finish_open_directory(
        &mut self,
        id: uuid::Uuid,
        success: bool,
        message: &'static str,
        cx: &mut Context<Self>,
    ) {
        let Some((pending, started)) = self.pending_directory else {
            return;
        };
        if pending != id {
            return;
        }
        self.pending_directory = None;
        info!(event="extension.directory.completed",correlation_id=%id,success,elapsed_ms=started.elapsed().as_millis(),"managed directory presentation completed");
        self.notify_directory(
            id,
            if success {
                crate::notifications::NotificationLevel::Success
            } else {
                crate::notifications::NotificationLevel::Error
            },
            message,
            cx,
        );
        cx.notify();
    }

    fn notify_directory(
        &self,
        id: uuid::Uuid,
        level: crate::notifications::NotificationLevel,
        message: &'static str,
        cx: &mut Context<Self>,
    ) {
        let Some(center) = self
            .notifications
            .clone()
            .or_else(|| crate::notifications::current(cx))
        else {
            return;
        };
        let owner = cx.entity().downgrade();
        cx.defer(move |cx| {
            if level == crate::notifications::NotificationLevel::Progress
                && !owner.upgrade().is_some_and(|owner| {
                    owner
                        .read(cx)
                        .pending_directory
                        .is_some_and(|(pending, _)| pending == id)
                })
            {
                return;
            }
            center.update(cx, |center, cx| {
                center.push(
                    crate::notifications::AppNotification::new(
                        format!("extension-directory-{id}"),
                        level,
                        "插件目录",
                        message,
                    ),
                    cx,
                );
            });
        });
    }
}

#[cfg(test)]
mod tests;
