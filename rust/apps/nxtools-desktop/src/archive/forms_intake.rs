//! Local input intent over the original application's bounded native requests.
use super::*;
use nxtools_protocol::archive_clipboard::ArchiveClipboardCommand;

pub(super) fn valid_source_roots<'a>(
    paths: impl IntoIterator<Item = &'a str>,
    limit: usize,
    archive: Option<&str>,
) -> bool {
    let paths = paths.into_iter().collect::<Vec<_>>();
    if paths.is_empty()
        || paths.len() > limit
        || paths.iter().any(|path| !valid_job_path(path))
        || paths.iter().map(|path| path.len()).sum::<usize>() + archive.map_or(0, str::len)
            > 131_072
    {
        return false;
    }
    let normalize = |path: &str| path.replace('\\', "/").trim_end_matches('/').to_lowercase();
    let mut roots = Vec::<String>::with_capacity(paths.len() + usize::from(archive.is_some()));
    if let Some(archive) = archive {
        roots.push(normalize(archive));
    }
    for path in paths {
        let root = normalize(path);
        if roots.iter().any(|old| {
            old == &root
                || root
                    .strip_prefix(old)
                    .is_some_and(|rest| rest.starts_with('/'))
                || old
                    .strip_prefix(&root)
                    .is_some_and(|rest| rest.starts_with('/'))
        }) {
            return false;
        }
        roots.push(root);
    }
    true
}

impl OperationForm {
    fn accepts_file_paste(&self) -> bool {
        matches!(&self.draft, Draft::Create(_))
            || matches!(&self.draft, Draft::Modify(draft) if draft.action == ArchiveJobModifyAction::Add)
    }
    pub(in crate::archive) fn intake_target(
        &self,
    ) -> Option<(
        PathBuf,
        nxtools_protocol::archive_browser::ArchiveBrowserContext,
        String,
    )> {
        match &self.draft {
            Draft::Modify(draft) if draft.action == ArchiveJobModifyAction::Add => Some((
                draft.source.clone(),
                draft.browser.clone(),
                draft
                    .value
                    .clone()
                    .unwrap_or_default()
                    .trim_end_matches('/')
                    .to_owned(),
            )),
            _ => None,
        }
    }
    pub(in crate::archive) fn clipboard_current(&self, command: &ArchiveClipboardCommand) -> bool {
        !self.retired
            && !self.submitted
            && self.id == command.session_id
            && self.generation == command.generation
            && self.pending_clipboard.as_ref() == Some(command)
            && self.accepts_file_paste()
    }
    fn paste_files(&mut self, cx: &mut Context<Self>) {
        if self.busy() || !self.accepts_file_paste() {
            return;
        }
        let command = ArchiveClipboardCommand {
            version: 1,
            operation_id: Uuid::new_v4(),
            session_id: self.id,
            generation: self.generation,
        };
        self.pending_clipboard = Some(command.clone());
        self.error = None;
        let owner = self.owner.clone();
        let form = cx.entity().downgrade();
        cx.defer(move |cx| {
            if !form
                .read_with(cx, |form, _| form.clipboard_current(&command))
                .unwrap_or(false)
            {
                return;
            }
            if owner
                .update(cx, |app, cx| app.read_files_for_form(command.clone(), cx))
                .is_err()
            {
                let _ = form.update(cx, |form, cx| {
                    form.clipboard_result(&command, Err("文件读取所有者已关闭。".into()), cx)
                });
            }
        });
        cx.notify();
    }
    pub(in crate::archive) fn clipboard_result(
        &mut self,
        command: &ArchiveClipboardCommand,
        result: Result<Vec<String>, String>,
        cx: &mut Context<Self>,
    ) {
        if !self.clipboard_current(command) {
            return;
        }
        self.pending_clipboard = None;
        match result {
            Ok(paths) if paths.is_empty() => self.error = Some("剪贴板中没有文件来源。".into()),
            Ok(paths) => self.accept_sources(paths, cx),
            Err(message) => self.error = Some(message),
        }
        cx.notify();
    }
    pub(super) fn accept_sources(&mut self, paths: Vec<String>, cx: &mut Context<Self>) {
        if !valid_source_roots(paths.iter().map(String::as_str), 32, None) {
            self.error = Some("来源列表无效或超出上限；本次未添加任何来源。".into());
            return;
        }
        match &mut self.draft {
            Draft::Create(_) => self.enumerate_sources(paths, cx),
            Draft::Modify(draft) if draft.action == ArchiveJobModifyAction::Add => {
                if !valid_source_roots(
                    draft.added.iter().chain(&paths).map(String::as_str),
                    31,
                    Some(&draft.source.to_string_lossy()),
                ) {
                    self.error =
                        Some("来源重复、重叠、自包含或超出预算；本次未添加任何输入。".into());
                } else {
                    draft.added.extend(paths);
                    self.revalidate(cx);
                }
            }
            _ => self.error = Some("此操作不接受文件来源。".into()),
        }
    }
    pub(super) fn paste_button(&self, cx: &Context<Self>) -> Button {
        Button::new("archive-paste-files")
            .debug_selector(|| "archive-paste-files".into())
            .label("粘贴文件")
            .outline()
            .with_size(Size::Medium)
            .disabled(self.busy())
            .on_click(cx.listener(|this, _, _, cx| this.paste_files(cx)))
    }
    pub(super) fn file_paste_key(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let modifiers = event.keystroke.modifiers;
        if event.keystroke.key != "v"
            || !modifiers.control
            || modifiers.alt
            || modifiers.shift
            || modifiers.platform
            || event.is_held
            || window.has_active_dialog(cx)
            || self.busy()
            || !self.accepts_file_paste()
        {
            return;
        }
        if [
            &self.source,
            &self.destination,
            &self.name,
            &self.password,
            &self.confirm,
            &self.recovery_id,
            &self.volume,
        ]
        .into_iter()
        .any(|input| input.focus_handle(cx).contains_focused(window, cx))
            || self
                .exclusions
                .focus_handle(cx)
                .contains_focused(window, cx)
            || self
                .create_exclusions
                .iter()
                .any(|input| input.focus_handle(cx).contains_focused(window, cx))
        {
            return;
        }
        cx.stop_propagation();
        self.paste_files(cx);
    }
}
