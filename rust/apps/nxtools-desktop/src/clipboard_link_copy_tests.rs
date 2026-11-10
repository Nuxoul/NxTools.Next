use super::*;
use crate::tools::link_detector::DetectedLink;
use crate::tools::link_panel::{LINK_PANEL_WIDTH, LinkPanelView, link_panel_height};
use gpui_kit::base::Root;
use gpui_kit::{TestAppContext, VisualTestContext, px, size};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

type CopyEvents = Arc<Mutex<Vec<BTreeMap<String, String>>>>;

#[derive(Clone)]
struct CopyDiagnostics(CopyEvents);

#[derive(Default)]
struct CopyFields(BTreeMap<String, String>);

impl tracing::field::Visit for CopyFields {
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        self.0.insert(field.name().into(), value.into());
    }
    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        self.0.insert(field.name().into(), value.to_string());
    }
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        self.0.insert(field.name().into(), format!("{value:?}"));
    }
}

impl tracing::Subscriber for CopyDiagnostics {
    fn enabled(&self, _: &tracing::Metadata<'_>) -> bool {
        true
    }
    fn new_span(&self, _: &tracing::span::Attributes<'_>) -> tracing::span::Id {
        tracing::span::Id::from_u64(1)
    }
    fn record(&self, _: &tracing::span::Id, _: &tracing::span::Record<'_>) {}
    fn record_follows_from(&self, _: &tracing::span::Id, _: &tracing::span::Id) {}
    fn event(&self, event: &tracing::Event<'_>) {
        let mut fields = CopyFields::default();
        event.record(&mut fields);
        if fields
            .0
            .get("event")
            .is_some_and(|name| name.starts_with("tools.link_panel.copy_"))
        {
            self.0.lock().expect("copy diagnostics").push(fields.0);
        }
    }
    fn enter(&self, _: &tracing::span::Id) {}
    fn exit(&self, _: &tracing::span::Id) {}
}

fn links(target: &str) -> Vec<DetectedLink> {
    vec![DetectedLink {
        display: "synthetic target".into(),
        copy_value: target.into(),
        kind: DetectedLinkKind::Web,
    }]
}

#[gpui_kit::test]
fn ref01_explicit_copy_has_one_correlated_terminal_for_current_stale_and_ownerless_windows(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    cx.executor().allow_parking();
    let temporary = tempfile::tempdir().expect("synthetic copy state");
    let worker = crate::clipboard_worker::ClipboardCaptureWorkerHandle::spawn(
        temporary.path().join("history.db"),
        temporary.path().join("assets"),
        ClipboardRetentionPolicy {
            max_items: 1000,
            retention_days: 30,
        },
        1000,
    )
    .expect("synthetic clipboard worker");
    let (platform, endpoint) = nxtools_ipc::platform_bridge(32);
    endpoint.set_connected_for_contract_test(true);
    let application = cx.update(|cx| {
        let notifications = cx.new(|_| NotificationCenter::default());
        cx.new(|cx| {
            ClipboardApplication::new(
                worker.clone(),
                Rc::new(platform),
                AppConfig::default(),
                temporary.path().join("config.json"),
                notifications,
                cx,
            )
        })
    });
    let records = CopyEvents::default();
    tracing::subscriber::with_default(CopyDiagnostics(records.clone()), || {
        application.update(cx, |app, cx| {
            app.show_link_panel(links("https://normal.invalid/private"), cx)
        });
        let normal = *cx.windows().last().expect("normal panel");
        let mut surface = VisualTestContext::from_window(normal, cx);
        surface.refresh().expect("normal production Copy");
        let button = surface
            .debug_bounds("link-panel-copy-0")
            .expect("explicit Copy button");
        surface.simulate_click(button.center(), Default::default());
        cx.run_until_parked();
        assert!(cx.windows().is_empty());
        assert_eq!(
            cx.read_from_clipboard()
                .and_then(|item| item.text())
                .as_deref(),
            Some("https://normal.invalid/private")
        );
        while endpoint.try_receive_command_for_contract_test().is_ok() {}

        application.update(cx, |app, cx| {
            app.show_link_panel(links("https://retired.invalid/secret"), cx)
        });
        let retired = *cx.windows().last().expect("retired panel");
        let mut surface = VisualTestContext::from_window(retired, cx);
        surface.refresh().expect("retired production Copy");
        let button = surface
            .debug_bounds("link-panel-copy-0")
            .expect("explicit retired Copy");
        cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string("sentinel".into()));
        while endpoint.try_receive_command_for_contract_test().is_ok() {}
        let reopener = application.clone();
        let opened = Rc::new(std::cell::Cell::new(false));
        let replacement_opened = opened.clone();
        let retired_id = retired.window_id();
        let subscription = cx.update(|cx| {
            cx.on_window_closed(move |cx, closed| {
                if closed == retired_id {
                    assert_eq!(
                        cx.read_from_clipboard()
                            .and_then(|item| item.text())
                            .as_deref(),
                        Some("sentinel")
                    );
                    reopener.update(cx, |app, cx| {
                        app.show_link_panel(links("https://replacement.invalid/current"), cx)
                    });
                    replacement_opened.set(true);
                }
            })
        });
        surface.simulate_click(button.center(), Default::default());
        assert!(
            opened.get(),
            "native close observer establishes replacement before deferred Copy"
        );
        let current = *cx.windows().last().expect("replacement panel");
        assert_ne!(current, retired);
        cx.run_until_parked();
        assert_eq!(cx.windows(), vec![current]);
        application.read_with(cx, |app, _| {
            assert_eq!(app.link_panel.window, Some(current))
        });
        assert_eq!(
            cx.read_from_clipboard()
                .and_then(|item| item.text())
                .as_deref(),
            Some("sentinel")
        );
        assert!(
            endpoint.try_receive_command_for_contract_test().is_err(),
            "retired Copy emits no platform command"
        );
        drop(subscription);
        current
            .update(cx, |_, window, _| window.remove_window())
            .expect("close replacement");
        cx.run_until_parked();

        let (_root, surface) = cx.add_window_view(|window, cx| {
            window.resize(size(px(LINK_PANEL_WIDTH), px(link_panel_height(1, false))));
            let handle = window.window_handle();
            let view = cx.new(|_| LinkPanelView {
                links: links("https://ownerless.invalid/payload"),
                application: gpui_kit::WeakEntity::new_invalid(),
                expanded: false,
                auto_hide_ms: 6000,
                window_handle: Some(handle),
                auto_hide_task: None,
            });
            Root::new(view, window, cx)
        });
        surface.refresh().expect("ownerless production Copy");
        let button = surface
            .debug_bounds("link-panel-copy-0")
            .expect("ownerless explicit Copy");
        surface.simulate_click(button.center(), Default::default());
        cx.run_until_parked();
        assert!(cx.windows().is_empty());
        assert_eq!(
            cx.read_from_clipboard()
                .and_then(|item| item.text())
                .as_deref(),
            Some("https://ownerless.invalid/payload")
        );
        assert!(endpoint.try_receive_command_for_contract_test().is_err());
    });
    worker.shutdown().expect("flush synthetic clipboard worker");
    let records = records.lock().expect("captured copy diagnostics");
    let starts = records
        .iter()
        .filter(|fields| fields["event"] == "tools.link_panel.copy_started")
        .collect::<Vec<_>>();
    assert_eq!(starts.len(), 3);
    for (index, start) in starts.iter().enumerate() {
        let correlation = &start["correlation_id"];
        assert!(uuid::Uuid::parse_str(correlation).is_ok());
        let terminals = records
            .iter()
            .filter(|fields| {
                fields["event"] != "tools.link_panel.copy_started"
                    && fields.get("correlation_id") == Some(correlation)
            })
            .collect::<Vec<_>>();
        assert_eq!(
            terminals.len(),
            1,
            "each explicit Copy requires exactly one same-ID terminal (case {index})"
        );
        assert_eq!(
            terminals[0]["event"],
            if index == 1 {
                "tools.link_panel.copy_cancelled"
            } else {
                "tools.link_panel.copy_completed"
            }
        );
        assert!(terminals[0]["elapsed_ms"].parse::<u64>().is_ok());
        if index == 1 {
            assert_eq!(terminals[0]["reason"], "panel_replaced");
        }
    }
    let rendered = format!("{records:?}");
    for secret in [
        "https://",
        "sentinel",
        "private",
        "secret",
        "payload",
        "replacement.invalid",
    ] {
        assert!(
            !rendered.contains(secret),
            "diagnostics must not expose synthetic payload"
        );
    }
}
