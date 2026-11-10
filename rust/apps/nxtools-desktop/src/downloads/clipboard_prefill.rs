use super::*;

pub(super) struct PendingDownloadClipboard {
    command_id: uuid::Uuid,
    window: AnyWindowHandle,
    view: gpui_kit::WeakEntity<NewDownloadView>,
    started_at: Instant,
}

pub(super) fn valid_download_url(value: &str) -> bool {
    !value.chars().any(char::is_whitespace)
        && url::Url::parse(value)
            .is_ok_and(|url| matches!(url.scheme(), "http" | "https") && url.host_str().is_some())
}

fn single_download_url(text: &str) -> Option<String> {
    let lines: Vec<_> = text
        .lines()
        .map(str::trim)
        .filter(|line| valid_download_url(line))
        .collect();
    let candidates = if lines.is_empty() {
        nxtools_clipboard::extract_web_urls(text)
    } else {
        lines.into_iter().map(str::to_owned).collect()
    };
    let mut links = candidates
        .into_iter()
        .filter(|value| valid_download_url(value));
    let first = links.next()?;
    links.next().is_none().then_some(first)
}

impl DownloadApplication {
    pub(super) fn request_new_download_clipboard(
        &mut self,
        window: AnyWindowHandle,
        view: gpui_kit::WeakEntity<NewDownloadView>,
        cx: &mut Context<Self>,
    ) {
        if self.new_download_window != Some(window)
            || !view
                .upgrade()
                .is_some_and(|view| view.read(cx).clipboard_prefill_eligible(cx))
        {
            return;
        }
        self.retire_new_download_clipboard("replaced");
        let command_id = uuid::Uuid::new_v4();
        self.pending_new_download_clipboard = Some(PendingDownloadClipboard {
            command_id,
            window,
            view,
            started_at: Instant::now(),
        });
        info!(event = "download.clipboard_prefill.started", correlation_id = %command_id);
        if self
            .platform
            .send_command_with_id(PlatformCommand::ReadClipboardText, command_id)
            .is_err()
        {
            self.retire_new_download_clipboard("enqueue_failed");
            return;
        }
        cx.spawn(async move |application, cx| {
            cx.background_executor().timer(Duration::from_secs(3)).await;
            let _ = application.update(cx, |app, _| {
                if app
                    .pending_new_download_clipboard
                    .as_ref()
                    .is_some_and(|pending| pending.command_id == command_id)
                {
                    app.retire_new_download_clipboard("timeout");
                }
            });
        })
        .detach();
    }

    pub(super) fn retire_new_download_clipboard(&mut self, outcome: &'static str) {
        if let Some(pending) = self.pending_new_download_clipboard.take() {
            info!(event = "download.clipboard_prefill.finished", correlation_id = %pending.command_id,
                outcome, elapsed_ms = pending.started_at.elapsed().as_millis());
        }
    }

    pub(super) fn handle_new_download_clipboard(
        &mut self,
        notification: &PlatformNotification,
        cx: &mut Context<Self>,
    ) -> bool {
        if matches!(notification, PlatformNotification::Disconnected { .. }) {
            self.retire_new_download_clipboard("disconnected");
            return false;
        }
        let PlatformNotification::CommandResult {
            correlation_id,
            result,
        } = notification
        else {
            return false;
        };
        if self
            .pending_new_download_clipboard
            .as_ref()
            .is_none_or(|pending| pending.command_id != *correlation_id)
        {
            return false;
        }
        if !result.success {
            self.retire_new_download_clipboard("native_failure");
            return true;
        }
        if self
            .pending_new_download_clipboard
            .as_ref()
            .map(|pending| pending.window)
            != self.new_download_window
        {
            self.retire_new_download_clipboard("stale_window");
            return true;
        }
        let Some(url) = result.data.as_deref().and_then(single_download_url) else {
            self.retire_new_download_clipboard("no_single_link");
            return true;
        };
        let Some(pending) = self.pending_new_download_clipboard.take() else {
            return true;
        };
        cx.defer(move |cx| {
            let applied = pending.window.update(cx, |_, window, cx| {
                pending.view.update(cx, |view, cx| view.prefill_clipboard_url(url, window, cx))
            }).ok().and_then(Result::ok).unwrap_or(false);
            info!(event = "download.clipboard_prefill.finished", correlation_id = %pending.command_id,
                outcome = if applied { "filled" } else { "draft_retired" },
                elapsed_ms = pending.started_at.elapsed().as_millis());
        });
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn download_intake_url_selection_preserves_one_link_and_refuses_ambiguity() {
        for text in [
            "HTTP://example.test/file?x=1",
            "https://example.test/file?x=a%2Bb#frag",
            "下载链接： https://example.test/file.zip。",
        ] {
            assert!(single_download_url(text).is_some(), "single supported link");
        }
        for text in [
            "https://",
            "file:///C:/sample",
            "ftp://example.test/a",
            "https://a.test/a https://b.test/b",
            "https://a.test/a\nhttps://b.test/b",
        ] {
            assert!(
                single_download_url(text).is_none(),
                "unsupported or ambiguous input"
            );
        }
    }
}
