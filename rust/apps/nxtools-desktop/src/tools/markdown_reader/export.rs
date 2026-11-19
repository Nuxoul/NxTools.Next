use super::*;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use gpui_kit::component::menu::{DropdownMenu as _, PopupMenuItem};
use nxtools_protocol::{MarkdownExportFormat, PickMarkdownExportCommand, PrintMarkdownPdfCommand};
use std::{
    io::{self, Read, Write},
    path::Path,
};

const MAX_IMAGE_BYTES: u64 = 8 * 1024 * 1024;
const MAX_EMBEDDED_BYTES: u64 = 32 * 1024 * 1024;

pub(super) fn standalone(path: &Path, text: &str) -> io::Result<String> {
    let body = markdown::to_html_with_options(text, &markdown::Options::gfm())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid Markdown"))?;
    let (headings, _) = document::project(path, text);
    let mut outline = String::new();
    let mut content = String::new();
    if !headings.is_empty() {
        outline.push_str("<nav aria-label=\"文档大纲\"><ul>");
    }
    let mut rest = body.as_str();
    for heading in headings {
        let open = format!("<h{}>", heading.level);
        let Some((prior, following)) = rest.split_once(&open) else {
            break;
        };
        let anchor = &heading.anchor;
        content.push_str(prior);
        content.push_str(&format!("<h{} id=\"{anchor}\">", heading.level));
        outline.push_str(&format!(
            "<li class=\"level-{}\"><a href=\"#{anchor}\">{}</a></li>",
            heading.level,
            escape(&heading.title)
        ));
        rest = following;
    }
    content.push_str(rest);
    if !outline.is_empty() {
        outline.push_str("</ul></nav>");
    }
    let content = inline_images(path, &content)?;
    let title = path
        .file_stem()
        .map(|name| name.to_string_lossy())
        .unwrap_or_default();
    Ok(format!(
        r#"<!doctype html>
<html lang="zh-CN"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<meta http-equiv="Content-Security-Policy" content="default-src 'none'; img-src data:; style-src 'unsafe-inline'; base-uri 'none'; form-action 'none'">
<title>{}</title><style>{STYLES}</style><main>{outline}<article>{content}</article></main></html>"#,
        escape(&title)
    ))
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn inline_images(document: &Path, html: &str) -> io::Result<String> {
    // The Markdown compiler, not document-supplied HTML, is the only producer of these tags.
    let mut html = html;
    let mut output = String::new();
    let mut remaining = MAX_EMBEDDED_BYTES;
    while let Some((prefix, after)) = html.split_once("<img src=\"") {
        output.push_str(prefix);
        let Some((source, attributes)) = after.split_once('"') else {
            break;
        };
        let Some((attributes, suffix)) = attributes.split_once('>') else {
            break;
        };
        html = suffix;
        let decoded = source.replace("&amp;", "&").replace("&quot;", "\"");
        let data = document::relative_path(document, &decoded)
            .and_then(|path| image_data(&path, &mut remaining).transpose())
            .transpose()?;
        if let Some(data) = data {
            output.push_str(&format!("<img src=\"{data}\"{attributes}>"));
        } else {
            let alt = attributes
                .split_once(" alt=\"")
                .and_then(|(_, alt)| alt.split_once('"'))
                .map_or("图片", |(alt, _)| alt);
            output.push_str(&format!(
                "<span class=\"unavailable\">[{alt}：未嵌入]</span>"
            ));
        }
    }
    output.push_str(html);
    Ok(output)
}

fn image_data(path: &Path, remaining: &mut u64) -> io::Result<Option<String>> {
    let Ok(metadata) = std::fs::metadata(path) else {
        return Ok(None);
    };
    if !metadata.is_file() {
        return Ok(None);
    }
    let mime = match path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("bmp") => "image/bmp",
        Some("svg") => "image/svg+xml",
        _ => return Ok(None),
    };
    let limit = (*remaining).min(MAX_IMAGE_BYTES);
    if metadata.len() > limit {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "embedded image budget exceeded",
        ));
    }
    let Ok(file) = std::fs::File::open(path) else {
        return Ok(None);
    };
    let mut bytes = Vec::new();
    file.take(limit + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "embedded image budget exceeded",
        ));
    }
    *remaining -= bytes.len() as u64;
    Ok(Some(format!(
        "data:{mime};base64,{}",
        STANDARD.encode(bytes)
    )))
}

const STYLES: &str = r#"
:root{color-scheme:light dark} body{margin:0;font:16px/1.65 system-ui,sans-serif}
main{width:min(90%,52rem);margin:2.5rem auto} nav{border-bottom:1px solid #8886;padding-bottom:1rem} nav ul{padding-left:1.2rem}
.level-2{margin-left:1rem}.level-3,.level-4,.level-5,.level-6{margin-left:2rem}
img{max-width:100%;height:auto} table{border-collapse:collapse;max-width:100%} td,th{border:1px solid #8888;padding:.25rem .6rem;overflow-wrap:anywhere}
pre,code{font-family:ui-monospace,Consolas,monospace;background:#8882} pre{padding:1rem;overflow:auto} blockquote{margin-left:0;padding-left:1rem;border-left:.2rem solid #8888}
.unavailable{font-style:italic;color:GrayText} a{color:LinkText}
@media print{:root{color-scheme:light} main{width:auto;margin:0} nav{display:none} pre{white-space:pre-wrap;overflow-wrap:anywhere} h1,h2,h3,h4,h5,h6{break-after:avoid} img,tr{break-inside:avoid} thead{display:table-header-group}}
"#;

pub(super) struct ExportSelection {
    id: Uuid,
    started: Instant,
    format: MarkdownExportFormat,
    document: PathBuf,
    text: String,
}

pub(super) struct Print {
    id: Uuid,
    request: ExportSelection,
    output: StagedOutput,
}

struct StagedOutput {
    directory: PathBuf,
    target: PathBuf,
    original: Option<document::Stamp>,
}

impl StagedOutput {
    fn new(target: PathBuf) -> io::Result<Self> {
        let original = target_stamp(&target)?;
        let directory = target.with_file_name(format!(".nxtools-export-{}", Uuid::new_v4()));
        std::fs::create_dir(&directory)?;
        Ok(Self {
            directory,
            target,
            original,
        })
    }

    fn html(&self) -> PathBuf {
        self.directory.join("document.html")
    }
    fn pdf(&self) -> PathBuf {
        self.directory.join("document.pdf")
    }

    fn publish(self, format: MarkdownExportFormat) -> io::Result<()> {
        let staged = match format {
            MarkdownExportFormat::Html => self.html(),
            MarkdownExportFormat::Pdf => {
                let pdf = self.pdf();
                let mut header = [0; 5];
                std::fs::File::open(&pdf)?.read_exact(&mut header)?;
                if &header != b"%PDF-" {
                    return Err(io::Error::new(io::ErrorKind::InvalidData, "PDF header"));
                }
                pdf
            }
        };
        if target_stamp(&self.target)? != self.original {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "export destination changed",
            ));
        }
        std::fs::rename(staged, &self.target)
    }
}

impl Drop for StagedOutput {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(self.html());
        let _ = std::fs::remove_file(self.pdf());
        let _ = std::fs::remove_dir(&self.directory);
    }
}

fn target_stamp(path: &Path) -> io::Result<Option<document::Stamp>> {
    match document::stamp(path) {
        Ok(stamp) => Ok(Some(stamp)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

impl Reader {
    pub(super) fn export_busy(&self) -> bool {
        self.export_selection.is_some() || self.export_worker.is_some() || self.pdf.is_some()
    }

    pub(super) fn export_menu(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let owner = cx.weak_entity();
        Button::new("markdown-export")
            .label(if self.export_busy() {
                "正在导出…"
            } else {
                "导出"
            })
            .small()
            .outline()
            .disabled(self.active_tab().is_none() || self.export_busy() || self.picker.is_some())
            .dropdown_menu(move |menu, _, _| {
                [
                    ("单文件 HTML", MarkdownExportFormat::Html),
                    ("PDF", MarkdownExportFormat::Pdf),
                ]
                .into_iter()
                .fold(menu, |menu, (title, format)| {
                    let owner = owner.clone();
                    menu.item(PopupMenuItem::new(title).on_click(move |_, window, cx| {
                        let owner = owner.clone();
                        window.defer(cx, move |window, cx| {
                            let _ =
                                owner.update(cx, |this, cx| this.begin_export(format, window, cx));
                        });
                    }))
                })
            })
    }

    fn begin_export(
        &mut self,
        format: MarkdownExportFormat,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.export_busy() || self.picker.is_some() {
            return;
        }
        let Some(tab) = self.active_tab() else {
            return;
        };
        let id = Uuid::new_v4();
        let selection = ExportSelection {
            id,
            started: Instant::now(),
            format,
            document: tab.document.path.clone(),
            text: tab.draft.text(&tab.document.text).to_owned(),
        };
        let suggested_name = selection
            .document
            .file_stem()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Markdown".into());
        if let Some((platform, hwnd)) = self
            .platform
            .as_ref()
            .zip(crate::ui::native_window_handle(window))
            && platform
                .send_command_with_id(
                    PlatformCommand::PickMarkdownExport(PickMarkdownExportCommand {
                        source_window_handle: hwnd as u64,
                        source_process_id: std::process::id(),
                        format,
                        suggested_name,
                    }),
                    id,
                )
                .is_ok()
        {
            self.export_selection = Some(selection);
            self.problem = None;
            tracing::info!(event = "markdown.export.started", correlation_id = %id);
        } else {
            self.finish_export(&selection, "picker_unavailable", None, cx);
        }
        cx.notify();
    }

    pub(super) fn export_receipt(
        &mut self,
        event: &PlatformNotification,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let (id, response) = match event {
            PlatformNotification::CommandResult {
                correlation_id,
                result,
            } => (*correlation_id, Some(result)),
            PlatformNotification::Disconnected {
                pending_command_ids,
                ..
            } => {
                let id = self
                    .export_selection
                    .as_ref()
                    .map(|selection| selection.id)
                    .or_else(|| self.pdf.as_ref().map(|print| print.id));
                let Some(id) = id.filter(|id| pending_command_ids.contains(id)) else {
                    return;
                };
                (id, None)
            }
            _ => return,
        };
        if self
            .export_selection
            .as_ref()
            .is_some_and(|selection| selection.id == id)
        {
            let request = self.export_selection.take().expect("matching request");
            match response {
                Some(result) if result.success => {
                    if let Some(target) = result.data.as_deref().filter(|path| !path.is_empty()) {
                        self.prepare_export(request, PathBuf::from(target), window, cx);
                    } else {
                        self.finish_export(&request, "cancelled", None, cx);
                    }
                }
                result => self.finish_export(
                    &request,
                    "picker_failed",
                    result.and_then(|result| result.native_error_code),
                    cx,
                ),
            }
        } else if self.pdf.as_ref().is_some_and(|print| print.id == id) {
            let Print {
                request, output, ..
            } = self.pdf.take().expect("matching print");
            match response {
                Some(result) if result.success => self.publish_export(request, output, window, cx),
                result => {
                    let missing = result
                        .and_then(|result| result.error.as_deref())
                        .is_some_and(|error| {
                            error == "webview2_runtime_missing" || error == "runtime_missing"
                        });
                    self.finish_export(
                        &request,
                        if missing {
                            "runtime_missing"
                        } else {
                            "pdf_failed"
                        },
                        result.and_then(|result| result.native_error_code),
                        cx,
                    );
                }
            }
        }
    }

    fn prepare_export(
        &mut self,
        request: ExportSelection,
        target: PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let path = request.document.clone();
        let text = request.text.clone();
        let format = request.format;
        let worker = cx.background_spawn(async move {
            let ext = if format == MarkdownExportFormat::Pdf {
                "pdf"
            } else {
                "html"
            };
            if !target.is_absolute()
                || !target
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .is_some_and(|suffix| suffix.eq_ignore_ascii_case(ext))
            {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "unsupported output",
                ));
            }
            let html = standalone(&path, &text)?;
            let stage = StagedOutput::new(target)?;
            let mut file = std::fs::File::options()
                .create_new(true)
                .write(true)
                .open(stage.html())?;
            file.write_all(html.as_bytes())?;
            file.sync_all()?;
            Ok::<_, io::Error>(stage)
        });
        self.export_worker = Some(cx.spawn_in(window, async move |owner, cx| {
            let result = worker.await;
            let _ = owner.update_in(cx, |this, window, cx| {
                this.export_worker = None;
                match result {
                    Err(error) => {
                        this.finish_export(&request, "prepare_failed", error.raw_os_error(), cx)
                    }
                    Ok(stage) if request.format == MarkdownExportFormat::Html => {
                        this.publish_export(request, stage, window, cx)
                    }
                    Ok(stage) => {
                        let id = Uuid::new_v4();
                        let sent = this.platform.as_ref().is_some_and(|platform| {
                            platform
                                .send_command_with_id_correlated(
                                    PlatformCommand::PrintMarkdownPdf(PrintMarkdownPdfCommand {
                                        html_path: stage.html().to_string_lossy().into_owned(),
                                        output_path: stage.pdf().to_string_lossy().into_owned(),
                                    }),
                                    id,
                                    request.id,
                                )
                                .is_ok()
                        });
                        if sent {
                            this.pdf = Some(Print {
                                id,
                                request,
                                output: stage,
                            });
                        } else {
                            this.finish_export(&request, "pdf_failed", None, cx);
                        }
                    }
                }
                cx.notify();
            });
        }));
    }

    fn publish_export(
        &mut self,
        request: ExportSelection,
        output: StagedOutput,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let format = request.format;
        let worker = cx.background_spawn(async move { output.publish(format) });
        self.export_worker = Some(cx.spawn_in(window, async move |owner, cx| {
            let result = worker.await;
            let _ = owner.update_in(cx, |this, _, cx| {
                this.export_worker = None;
                let (outcome, code) = match result {
                    Ok(()) => ("exported", None),
                    Err(error) => ("publish_failed", error.raw_os_error()),
                };
                this.finish_export(&request, outcome, code, cx);
            });
        }));
    }

    fn finish_export(
        &mut self,
        request: &ExportSelection,
        outcome: &'static str,
        native_error: Option<i32>,
        cx: &mut Context<Self>,
    ) {
        tracing::info!(event = "markdown.export.finished", correlation_id = %request.id, outcome, ?native_error, elapsed_ms = request.started.elapsed().as_millis());
        self.problem = match outcome {
            "exported" => {
                self.notice = Some("导出完成。");
                None
            }
            "cancelled" => None,
            "runtime_missing" => {
                Some("无法导出 PDF：请安装 Microsoft Edge WebView2 运行时；仍可导出 HTML。")
            }
            "picker_unavailable" | "picker_failed" => Some("无法打开保存对话框，请重试。"),
            "publish_failed" => Some("无法写入导出文件，请检查保存位置或换一个文件名。"),
            _ => Some("导出未完成。请检查文档中的图片和目标位置后重试。"),
        };
        cx.notify();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markdown_reader_never_publishes_failed_or_superseded_export() {
        let folder = tempfile::tempdir().unwrap();
        let target = folder.path().join("document.pdf");
        std::fs::write(&target, b"original PDF").unwrap();
        let failed = StagedOutput::new(target.clone()).unwrap();
        std::fs::write(failed.pdf(), b"incomplete file").unwrap();
        assert!(failed.publish(MarkdownExportFormat::Pdf).is_err());
        assert_eq!(std::fs::read(&target).unwrap(), b"original PDF");
        let superseded = StagedOutput::new(target.clone()).unwrap();
        std::fs::write(superseded.pdf(), b"%PDF-1.7\n").unwrap();
        std::fs::write(&target, b"another writer's new PDF").unwrap();
        assert!(superseded.publish(MarkdownExportFormat::Pdf).is_err());
        assert_eq!(std::fs::read(&target).unwrap(), b"another writer's new PDF");
        assert_eq!(std::fs::read_dir(folder.path()).unwrap().count(), 1);
    }
}
