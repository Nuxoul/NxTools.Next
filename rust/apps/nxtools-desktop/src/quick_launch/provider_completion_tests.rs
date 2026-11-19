use super::*;
use gpui_kit::TestAppContext;
use nxtools_protocol::{IndexSearchBatch, IndexSearchFailure};

#[gpui_kit::test]
fn normal_search_waits_for_every_current_provider_in_either_completion_order(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
    for terminal in ["complete", "disconnect", "failure"] {
        for index_first in [true, false] {
            let (_temporary, application, _platform) =
                super::super::tests::quick_launch_application_fixture(cx);
            let (panel, window) = super::super::tests::quick_launch_panel_fixture(
                cx,
                vec![super::super::tests::search_result(
                    "previous-query",
                    ApplicationSource::StartMenu,
                )],
            );
            panel.update_in(window, |panel, window, cx| {
                panel.application = application.downgrade();
                panel
                    .search_input
                    .update(cx, |input, cx| input.set_value("join-fixture", window, cx));
                panel.search_pending = true;
            });
            let (index_generation, identity) = window.update(|window, cx| {
                application.update(cx, |app, _| {
                    let manifest = br#"{"schema_version":1,"id":"fixture.join","name":"Join","version":"1","handlers":[{"id":"provider","title":"Provider","feature":"quick_launch","entry":"command","keywords":["join-fixture"],"action":{"type":"folder","path":"C:\\fixture"}}]}"#;
                    app.config.extensions.install(
                        nxtools_extensions::ImportPreview::from_json(manifest).expect("provider manifest"),
                        "fixture".into(), false,
                    ).expect("install provider fixture");
                    let package = &mut app.config.extensions.packages[0];
                    package.enabled = true;
                    let hash = package.content_hash.clone();
                    package.trust = Some(nxtools_extensions::TrustApproval {
                        content_hash: hash.clone(),
                        runtime: nxtools_extensions::RuntimeIdentity {
                            executable: PathBuf::from(r"C:\fixture.exe"),
                            sha256: "a".repeat(64),
                        },
                    });
                    app.extension_providers.generation = 21;
                    app.extension_providers.query = "join-fixture".into();
                    app.extension_providers.running = 2;
                    app.popup = Some(QuickLaunchWindow {
                        window: window.window_handle(), panel: panel.downgrade(), native_window_handle: 1,
                    });
                    let request = app.index_search.begin("join-fixture", INDEX_CANDIDATE_LIMIT);
                    (request.generation, InvocationIdentity {
                        run_id: uuid::Uuid::new_v4(), correlation_id: uuid::Uuid::new_v4(), generation: 21,
                        package_id: "fixture.join".into(), handler_id: "provider".into(), content_hash: hash,
                    })
                })
            });
            application.update(window, |app, cx| {
                app.apply_extension_batch(
                    "join-fixture",
                    21,
                    identity.clone(),
                    false,
                    vec![ProviderItem {
                        id: "current".into(),
                        title: "Current provider result".into(),
                        subtitle: String::new(),
                        action: ExtensionEffect::Copy {
                            text: "owned fixture".into(),
                        },
                    }],
                    cx,
                );
                app.finish_extension_provider("join-fixture", 20, cx);
                assert_eq!(
                    app.extension_providers.running, 2,
                    "retired completion cannot release a current provider"
                );
            });
            let index_terminal = |window: &mut gpui_kit::VisualTestContext| {
                application.update(window, |app, cx| {
                    let notification = match terminal {
                        "complete" => IndexerNotification::Event(IndexerEvent::SearchBatch(
                            IndexSearchBatch {
                                generation: index_generation,
                                items: vec![IndexSearchItem {
                                    name: "join-fixture.txt".into(),
                                    path: r"C:\synthetic\join-fixture.txt".into(),
                                    is_directory: false,
                                    score: 0,
                                    size_bytes: None,
                                    modified_at_unix_ms: None,
                                }],
                                complete: true,
                                elapsed_us: 20,
                            },
                        )),
                        "disconnect" => IndexerNotification::Disconnected,
                        "failure" => IndexerNotification::Event(IndexerEvent::SearchFailed(
                            IndexSearchFailure {
                                generation: index_generation,
                                reason: "recent_classifier_budget".into(),
                            },
                        )),
                        _ => unreachable!(),
                    };
                    app.handle_indexer_notification(notification, cx);
                });
            };
            if index_first {
                index_terminal(window);
            }
            application.update(window, |app, cx| {
                app.finish_extension_provider("join-fixture", 21, cx);
                assert_eq!(app.extension_providers.running, 1);
            });
            panel.read_with(window, |panel, _| {
                assert!(panel.search_pending);
                assert_eq!(panel.results[0].entry.id, "previous-query");
            });
            application.update(window, |app, cx| {
                app.finish_extension_provider("join-fixture", 21, cx);
            });
            if !index_first {
                panel.read_with(window, |panel, _| {
                    assert!(
                        panel.search_pending,
                        "extension completion alone cannot publish"
                    );
                    assert_eq!(panel.results[0].entry.id, "previous-query");
                });
                index_terminal(window);
            }
            panel.read_with(window, |panel, _| {
                assert!(!panel.search_pending, "last current completion must commit: {terminal}, index_first={index_first}");
                assert_eq!(panel.committed_query.as_ref(), "join-fixture");
                assert_eq!(panel.results.len(), if terminal == "complete" { 3 } else { 2 });
                assert_eq!(panel.results.iter().filter(|row| row.entry.source == ApplicationSource::IndexedFile).count(), usize::from(terminal == "complete"));
                assert!(panel.results.iter().any(|row| row.entry.id == "extension-script:fixture.join/provider/current"));
                assert!(!panel.results.iter().any(|row| row.entry.id == "previous-query"));
                assert_eq!(panel.status.as_deref(), (terminal == "failure").then_some(INDEX_SEARCH_FAILURE_MESSAGE));
            });
            application.update(window, |app, cx| {
                app.cancel_extension_providers();
                app.finish_extension_provider("join-fixture", 21, cx);
                assert_eq!(app.extension_providers.running, 0);
                app.index_search.begin("replacement", INDEX_CANDIDATE_LIMIT);
                assert!(
                    !app.index_search.failed,
                    "new query clears the preceding provider failure"
                );
            });
            window.run_until_parked();
        }
    }
}
