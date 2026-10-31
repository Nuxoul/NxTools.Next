//! Current draft/request/input revision is authoritative; a picker never saves settings.
use super::*;
use nxtools_protocol::{PickExtensionSettingPathCommand, PlatformCommandResult};

#[derive(Clone)]
struct PathRequest {
    id: uuid::Uuid,
    draft: uuid::Uuid,
    package: String,
    hash: String,
    key: String,
    revision: u64,
    original: String,
}
impl ExtensionManagerView {
    pub(in crate::main_window::extensions_page) fn pick_setting_path(
        &mut self,
        key: String,
        kind: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(hwnd) = crate::ui::native_window_handle(window) else {
            self.draft_error = Some("当前窗口不可用，请重新打开设置".into());
            return;
        };
        self.begin_setting_path(key, kind, hwnd, window, cx);
    }
    pub(super) fn begin_setting_path(
        &mut self,
        key: String,
        kind: &str,
        hwnd: i64,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(draft) = self.settings_draft.as_mut() else {
            return;
        };
        if draft.submitted || draft.path_pending.contains_key(&key) {
            return;
        }
        let Some(input) = draft.inputs.get(&key) else {
            return;
        };
        let Some(revision) = draft.path_revisions.get(&key) else {
            return;
        };
        let request = PathRequest {
            id: uuid::Uuid::new_v4(),
            draft: draft.draft_id,
            package: draft.id.clone(),
            hash: draft.content_hash.clone(),
            key: key.clone(),
            revision: revision.get(),
            original: input.read(cx).value().to_string(),
        };
        draft.path_pending.insert(key, request.id);
        let command = PlatformCommand::PickExtensionSettingPath(PickExtensionSettingPathCommand {
            source_window_handle: hwnd,
            source_process_id: std::process::id(),
            kind: kind.into(),
            current_path: request.original.clone(),
        });
        let owner = self.application.downgrade();
        let view = cx.entity().downgrade();
        let source = window.window_handle();
        cx.defer(move |cx| {
            let receiver = owner
                .update(cx, |app, cx| {
                    app.request_setting_path(command, request.id, cx)
                })
                .ok()
                .flatten();
            let _ = view.update(cx, |_, cx| {
                cx.spawn(async move |this, cx| {
                    let result = if let Some(receiver) = receiver {
                        receiver.recv().await.ok()
                    } else {
                        None
                    };
                    let _ = source.update(cx, |_, window, cx| {
                        let _ = this.update(cx, |view, cx| {
                            view.finish_setting_path(request, result, window, cx);
                            cx.notify();
                        });
                    });
                })
                .detach();
            });
        });
        cx.notify();
    }
    fn finish_setting_path(
        &mut self,
        request: PathRequest,
        result: Option<PlatformCommandResult>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(draft) = self.settings_draft.as_mut().filter(|draft| {
            draft.draft_id == request.draft
                && draft.id == request.package
                && draft.content_hash == request.hash
        }) else {
            return;
        };
        if draft.path_pending.get(&request.key) != Some(&request.id) {
            return;
        }
        draft.path_pending.remove(&request.key);
        let current = self
            .application
            .read(cx)
            .packages
            .packages
            .iter()
            .any(|package| {
                package.manifest.id == request.package && package.content_hash == request.hash
            });
        let Some(input) = draft.inputs.get(&request.key) else {
            return;
        };
        if !current
            || draft.submitted
            || self.selected_package.as_ref() != Some(&request.package)
            || draft
                .path_revisions
                .get(&request.key)
                .is_none_or(|revision| revision.get() != request.revision)
            || input.read(cx).value().as_ref() != request.original
        {
            return;
        }
        match result {
            Some(result) if result.success => {
                if let Some(path) = result.data.filter(|path| !path.is_empty()) {
                    if PathBuf::from(&path).is_absolute()
                        && path.len() < 32767
                        && !path.contains('\0')
                    {
                        input.update(cx, |input, cx| input.set_value(path, window, cx));
                        self.draft_error = None;
                    } else {
                        self.draft_error = Some("选择器返回无效路径，请重试".into());
                    }
                }
            }
            Some(result) if result.native_error_code == Some(995) => {}
            _ => {
                self.draft_error =
                    Some("无法选择路径，请检查 Windows 适配器后重试；原值已保留".into())
            }
        }
    }
}
