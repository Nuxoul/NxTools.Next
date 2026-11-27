use super::*;
use gpui_kit::component::{
    WindowExt,
    checkbox::Checkbox,
    dialog::{DialogAction, DialogClose, DialogFooter},
};
use nxtools_download::{
    BrowserBatchSubmissionResult, BrowserDownloadSubmission, MAX_BROWSER_BATCH_ITEMS,
};

pub(crate) struct TemplatePreview {
    pub(crate) id: uuid::Uuid,
    pub(crate) entries: Vec<BrowserDownloadSubmission>,
    pub(crate) selected: Vec<bool>,
    pub(crate) results: Option<Vec<BrowserBatchSubmissionResult>>,
    submitted: bool,
    busy: bool,
    error: Option<String>,
    previous_size: gpui_kit::Size<gpui_kit::Pixels>,
    result_focus: gpui_kit::FocusHandle,
}

fn expand_template(template: &str) -> Result<Vec<String>, String> {
    let invalid =
        || "模板仅支持一个升序整数区间，例如 file_{001..020}.zip；步长固定为 1。".to_owned();
    let (prefix, rest) = template.split_once('{').ok_or_else(invalid)?;
    let (range, suffix) = rest.split_once('}').ok_or_else(invalid)?;
    if prefix.contains('}') || suffix.contains(['{', '}']) || range.contains('{') {
        return Err(invalid());
    }
    let (first, last) = range.split_once("..").ok_or_else(invalid)?;
    if [first, last].iter().any(|value| {
        value.is_empty() || value.len() > 20 || !value.bytes().all(|byte| byte.is_ascii_digit())
    }) {
        return Err(invalid());
    }
    let start = first.parse::<u64>().map_err(|_| invalid())?;
    let end = last.parse::<u64>().map_err(|_| invalid())?;
    let count = u128::from(end)
        .checked_sub(u128::from(start))
        .ok_or_else(invalid)?
        + 1;
    if count > MAX_BROWSER_BATCH_ITEMS as u128 {
        return Err(format!(
            "模板共 {count} 项，最多允许 {MAX_BROWSER_BATCH_ITEMS} 项；请缩小区间。"
        ));
    }
    let width = if first.starts_with('0') || last.starts_with('0') {
        first.len().max(last.len())
    } else {
        0
    };
    Ok((start..=end)
        .map(|value| format!("{prefix}{value:0width$}{suffix}"))
        .collect())
}

impl NewDownloadView {
    pub(super) fn try_template_preview(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let template = self.url.read(cx).value().trim().to_owned();
        if !template.contains(['{', '}']) {
            return false;
        }
        if self.template_preview.is_some()
            || self.submitting
            || self.probing
            || self.external_blob.is_some()
        {
            return true;
        }
        let sources = match expand_template(&template) {
            Ok(sources) => sources,
            Err(error) => {
                self.notify_validation_failure(error, cx);
                return true;
            }
        };
        let base = match self.request_for_source(sources[0].clone(), cx) {
            Ok(request) if request.media.is_none() => request,
            Ok(_) => {
                self.notify_validation_failure("URL 模板仅支持普通下载。".into(), cx);
                return true;
            }
            Err(error) => {
                self.notify_validation_failure(humanize_download_fault(&error), cx);
                return true;
            }
        };
        let entries = sources
            .into_iter()
            .map(|source_url| {
                let mut request = base.clone();
                request.source_url = source_url;
                BrowserDownloadSubmission {
                    handoff_id: uuid::Uuid::new_v4(),
                    request,
                }
            })
            .collect::<Vec<_>>();
        for entry in &entries {
            if let Err(error) = entry.request.validate() {
                self.notify_validation_failure(humanize_download_fault(&error.to_string()), cx);
                return true;
            }
        }
        let id = uuid::Uuid::new_v4();
        let count = entries.len();
        let previous_size = window.viewport_size();
        self.template_preview = Some(TemplatePreview {
            id,
            entries,
            selected: vec![true; count],
            results: None,
            submitted: false,
            busy: false,
            error: None,
            previous_size,
            result_focus: cx.focus_handle(),
        });
        self.resize_window_with_correlation(window, size(px(640.0), px(560.0)), id, cx);
        let weak = cx.entity().downgrade();
        window.defer(cx, move |window, cx| {
            window.open_dialog(cx, move |dialog, _, cx| {
                let dialog = dialog.bg(crate::theme::overlay_background(cx));
                let Some(view) = weak.upgrade() else {
                    return dialog;
                };
                let Some(preview) = view
                    .read(cx)
                    .template_preview
                    .as_ref()
                    .filter(|preview| preview.id == id)
                else {
                    return dialog;
                };
                let selected = preview
                    .selected
                    .iter()
                    .filter(|selected| **selected)
                    .count();
                let mut rows = div()
                    .id("download-template-preview")
                    .debug_selector(|| "download-template-preview".into())
                    .h(px(300.0))
                    .track_focus(&preview.result_focus)
                    .w_full()
                    .min_w_0()
                    .overflow_y_scrollbar()
                    .flex()
                    .flex_col()
                    .gap_2();
                for (index, entry) in preview.entries.iter().enumerate() {
                    let toggle = weak.clone();
                    let result = preview.results.as_ref().and_then(|results| {
                        results
                            .iter()
                            .find(|result| result.handoff_id == entry.handoff_id)
                    });
                    let status = result.map(|result| {
                        if result.task_id.is_some() {
                            if result.duplicate {
                                "已存在".to_owned()
                            } else {
                                "已加入队列".to_owned()
                            }
                        } else {
                            result
                                .fault
                                .as_ref()
                                .map(|fault| humanize_download_fault(&fault.message))
                                .unwrap_or_else(|| "未接受".into())
                        }
                    });
                    rows = rows.child(
                        div()
                            .id(("download-template-row", index))
                            .w_full()
                            .min_w_0()
                            .flex()
                            .items_start()
                            .gap_2()
                            .child(
                                Checkbox::new(("download-template-select", index))
                                    .debug_selector(move || {
                                        format!("download-template-select-{index}")
                                    })
                                    .small()
                                    .flex_none()
                                    .w_full()
                                    .label(format!("{} · {}", index + 1, entry.request.source_url))
                                    .accessibility_label(format!(
                                        "选择第 {} 项：{}",
                                        index + 1,
                                        entry.request.source_url
                                    ))
                                    .tooltip(entry.request.source_url.clone())
                                    .checked(preview.selected[index])
                                    .disabled(preview.submitted)
                                    .on_click(move |selected, window, cx| {
                                        let selected = *selected;
                                        let weak = toggle.clone();
                                        window.defer(cx, move |_, cx| {
                                            let _ = weak.update(cx, |view, cx| {
                                                if let Some(preview) = view
                                                    .template_preview
                                                    .as_mut()
                                                    .filter(|preview| {
                                                        preview.id == id && !preview.submitted
                                                    })
                                                {
                                                    preview.selected[index] = selected;
                                                    cx.notify();
                                                }
                                            });
                                        });
                                    })
                                    .when_some(status, |checkbox, status| checkbox.child(status)),
                            ),
                    );
                }
                let confirm = weak.clone();
                let cancel = weak.clone();
                let close = weak.clone();
                dialog
                    .title("批量 URL 模板")
                    .width(px(608.0))
                    .margin_top(px(12.0))
                    .overlay_closable(false)
                    .child(format!(
                        "共 {} 项 · 已选 {selected} 项 · 预览不发请求，确认后加入队列。",
                        preview.entries.len()
                    ))
                    .child(rows)
                    .when(preview.busy, |dialog| {
                        dialog.child("正在逐项准入，请勿重复提交。")
                    })
                    .when_some(preview.error.clone(), |dialog, error| dialog.child(error))
                    .footer(
                        DialogFooter::new()
                            .child(DialogClose::new().trigger(|button| {
                                button
                                    .label(if preview.submitted {
                                        "关闭"
                                    } else {
                                        "取消"
                                    })
                                    .outline()
                                    .debug_selector(|| "download-template-cancel".into())
                            }))
                            .child(
                                DialogAction::new().child(
                                    Button::new("download-template-confirm")
                                        .debug_selector(|| "download-template-confirm".into())
                                        .primary()
                                        .label(format!("加入队列 ({selected})"))
                                        .disabled(preview.submitted || selected == 0)
                                        .on_click(|event, window, cx| {
                                            if event.is_keyboard() {
                                                window.dispatch_action(
                                                    Box::new(
                                                        gpui_kit::component::dialog::Confirm {
                                                            secondary: false,
                                                        },
                                                    ),
                                                    cx,
                                                );
                                            }
                                        }),
                                ),
                            ),
                    )
                    .on_ok(move |_, window, cx| {
                        let weak = confirm.clone();
                        window.defer(cx, move |window, cx| {
                            let _ =
                                weak.update(cx, |view, cx| view.submit_template(id, window, cx));
                        });
                        false
                    })
                    .on_cancel(move |_, window, cx| {
                        Self::dismiss_template(cancel.clone(), id, window, cx);
                        true
                    })
                    .on_close(move |_, window, cx| {
                        Self::dismiss_template(close.clone(), id, window, cx)
                    })
            });
        });
        cx.notify();
        true
    }

    fn dismiss_template(
        weak: gpui_kit::WeakEntity<Self>,
        id: uuid::Uuid,
        window: &mut Window,
        cx: &mut App,
    ) {
        window.defer(cx, move |window, cx| {
            let _ = weak.update(cx, |view, cx| {
                if view
                    .template_preview
                    .as_ref()
                    .is_some_and(|preview| preview.id == id)
                {
                    let previous = view
                        .template_preview
                        .take()
                        .expect("matching preview")
                        .previous_size;
                    view.resize_window_with_correlation(window, previous, id, cx);
                    view.url.read(cx).focus_handle(cx).focus(window, cx);
                    cx.notify();
                }
            });
        });
    }

    fn submit_template(&mut self, id: uuid::Uuid, window: &mut Window, cx: &mut Context<Self>) {
        let Some(preview) = self
            .template_preview
            .as_mut()
            .filter(|preview| preview.id == id && !preview.submitted)
        else {
            return;
        };
        let submissions = preview
            .entries
            .iter()
            .zip(&preview.selected)
            .filter(|(_, selected)| **selected)
            .map(|(entry, _)| entry.clone())
            .collect::<Vec<_>>();
        if submissions.is_empty() {
            return;
        }
        preview.submitted = true;
        preview.busy = true;
        let engine = self.application.read(cx).engine.clone();
        let application = self.application.downgrade();
        preview.result_focus.focus(window, cx);
        let source = window.window_handle();
        let background = cx.background_executor().clone();
        cx.spawn(async move |view, cx| {
            let result = background
                .spawn(async move {
                    engine.handle_command(DownloadCommandEnvelope {
                        protocol_version: nxtools_download::DOWNLOAD_COMMAND_PROTOCOL_VERSION,
                        request_id: id,
                        origin: RequestOrigin::Ui,
                        command: DownloadCommand::BrowserBatchSubmit { submissions },
                    })
                })
                .await;
            let _ = source.update(cx, |_, _, cx| {
                let _ = view.update(cx, |view, cx| {
                    let Some(preview) = view
                        .template_preview
                        .as_mut()
                        .filter(|preview| preview.id == id)
                    else {
                        return;
                    };
                    preview.busy = false;
                    match result.outcome {
                        DownloadCommandOutcome::BrowserBatchAccepted { results } => {
                            preview.results = Some(results)
                        }
                        DownloadCommandOutcome::Rejected { fault } => {
                            preview.error = Some(humanize_download_fault(&fault.message))
                        }
                        _ => {
                            preview.error = Some("批量准入未返回逐项结果，请查看任务列表。".into())
                        }
                    }
                    cx.notify();
                });
            });
            cx.update(|cx| {
                cx.defer(move |cx| {
                    let _ = application.update(cx, |app, cx| app.refresh(cx));
                })
            });
        })
        .detach();
        cx.notify();
    }
}
