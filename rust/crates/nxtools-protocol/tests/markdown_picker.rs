use nxtools_protocol::{
    Envelope, MAX_FRAME_BYTES, MarkdownExportFormat, Message, PickMarkdownDocumentCommand,
    PickMarkdownDocumentResult, PickMarkdownExportCommand, PlatformCommand, PlatformCommandResult,
    PrintMarkdownPdfCommand,
};
use serde_json::json;

#[test]
fn owned_markdown_picker_command_roundtrips_snake_case_fields() {
    let command = PickMarkdownDocumentCommand {
        source_window_handle: 4_294_967_298,
        source_process_id: 42,
    };
    let request = Envelope::new(Message::PlatformCommand(
        PlatformCommand::PickMarkdownDocument(command),
    ));
    let line = request
        .encode_line()
        .expect("encode Markdown picker request");
    let wire: serde_json::Value = serde_json::from_str(&line).expect("decode request JSON");
    assert_eq!(
        wire["payload"],
        json!({"command": "pick_markdown_document", "data": {
            "source_window_handle": 4_294_967_298_u64,
            "source_process_id": 42,
        }})
    );
    assert_eq!(
        Envelope::decode_line(&line).expect("roundtrip request"),
        request
    );
}

#[test]
fn markdown_picker_reply_preserves_unicode_order_and_explicit_cancellation() {
    let request = Envelope::new(Message::PlatformCommand(
        PlatformCommand::PickMarkdownDocument(PickMarkdownDocumentCommand {
            source_window_handle: 7,
            source_process_id: 42,
        }),
    ));
    for paths in [vec![r"C:\owned\读我.MD", r"C:\owned\中文.markdown"], vec![]] {
        let reply = Envelope::reply_to(
            &request,
            Message::PlatformCommandResult(PlatformCommandResult {
                success: true,
                error: None,
                native_error_code: None,
                data: Some(json!({"paths": paths}).to_string()),
            }),
        );
        let line = reply.encode_line().expect("encode Markdown picker reply");
        assert!(line.len() < MAX_FRAME_BYTES);
        let decoded = Envelope::decode_line(&line).expect("roundtrip reply");
        assert_eq!(decoded.correlation_id, Some(request.id));
        let Message::PlatformCommandResult(result) = decoded.message else {
            panic!("expected existing platform command result");
        };
        let result: PickMarkdownDocumentResult =
            serde_json::from_str(&result.data.expect("successful reply has JSON data"))
                .expect("decode typed selection object");
        assert!(result.selection_valid());
        assert_eq!(result.paths, paths);
    }
}

#[test]
fn markdown_picker_selection_limits_count_and_total_utf8_bytes() {
    let mut selection = PickMarkdownDocumentResult {
        paths: vec![r"C:\owned\读我.md".into(); 32],
    };
    assert!(selection.selection_valid());
    selection.paths.push(r"C:\owned\overflow.md".into());
    assert!(!selection.selection_valid());

    let long = format!("C:\\{}.md", "界".repeat(10_919));
    assert_eq!(long.len(), 32_763);
    selection.paths = vec![long; 4];
    selection.paths.push(r"C:\xxxxxxxxxxxxxx.md".into());
    assert_eq!(
        selection.paths.iter().map(String::len).sum::<usize>(),
        131_072
    );
    assert!(selection.selection_valid());
    selection.paths[4].insert(3, 'x');
    assert!(!selection.selection_valid());
}

#[test]
fn markdown_export_uses_separate_picker_and_staged_print_commands() {
    let fields = [
        (
            PlatformCommand::PickMarkdownExport(PickMarkdownExportCommand {
                source_window_handle: 42,
                source_process_id: 12,
                format: MarkdownExportFormat::Pdf,
                suggested_name: "报告".into(),
            }),
            json!({"command": "pick_markdown_export", "data": {
                "source_window_handle": 42, "source_process_id": 12,
                "format": "pdf", "suggested_name": "报告",
            }}),
        ),
        (
            PlatformCommand::PrintMarkdownPdf(PrintMarkdownPdfCommand {
                html_path: r"C:\stage\document.html".into(),
                output_path: r"C:\stage\render.pdf".into(),
            }),
            json!({"command": "print_markdown_pdf", "data": {
                "html_path": r"C:\stage\document.html", "output_path": r"C:\stage\render.pdf",
            }}),
        ),
    ];
    for (command, payload) in fields {
        let envelope = Envelope::new(Message::PlatformCommand(command));
        let line = envelope.encode_line().expect("encode export command");
        let wire: serde_json::Value = serde_json::from_str(&line).expect("decode JSON wire fields");
        assert_eq!(wire["payload"], payload);
        assert_eq!(
            Envelope::decode_line(&line).expect("decode typed wire"),
            envelope
        );
    }
    assert_eq!(
        serde_json::to_value(MarkdownExportFormat::Html).expect("serialize HTML format"),
        json!("html")
    );
}
