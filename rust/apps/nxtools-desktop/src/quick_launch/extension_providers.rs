use super::*;
use nxtools_extensions::{
    ExtensionEffect, InvocationIdentity, InvocationNotice, InvokeRequest, ProviderItem, ResultMerge,
};

#[derive(Default)]
pub(super) struct ExtensionProviders {
    generation: u64,
    query: String,
    keyword: bool,
    cancels: Vec<tokio::sync::watch::Sender<bool>>,
    rows: HashMap<String, (InvocationIdentity, ProviderItem, SearchResult)>,
    order: Vec<String>,
}
impl QuickLaunchApplication {
    pub(crate) fn cancel_extension_providers(&mut self) {
        for cancel in self.extension_providers.cancels.drain(..) {
            let _ = cancel.send(true);
        }
        self.extension_providers.rows.clear();
        self.extension_providers.order.clear();
        self.extension_providers.generation =
            self.extension_providers.generation.wrapping_add(1).max(1);
    }
    pub(super) fn begin_extension_providers(&mut self, query: &str, cx: &mut Context<Self>) {
        self.cancel_extension_providers();
        self.extension_providers.query = query.into();
        self.extension_providers.keyword = false;
        if query.trim().is_empty() || recent_modified_filter(query).is_some() {
            return;
        }
        let Some(service) = cx
            .try_global::<crate::extensions::ui_host::ExtensionServiceGlobal>()
            .map(|global| global.0.clone())
        else {
            return;
        };
        let mut selected = Vec::new();
        let mut ordinary = Vec::new();
        for package in self
            .config
            .extensions
            .packages
            .iter()
            .filter(|package| package.enabled && package.trust.is_some())
        {
            for handler in package.manifest.handlers.iter().filter(|handler| {
                handler.feature == "quick_launch"
                    && matches!(
                        handler.action,
                        nxtools_extensions::DeclarativeAction::Script { .. }
                    )
            }) {
                let keyword = package.keywords(handler).iter().find(|keyword| {
                    query.eq_ignore_ascii_case(keyword)
                        || query
                            .get(..keyword.len())
                            .is_some_and(|prefix| prefix.eq_ignore_ascii_case(keyword))
                            && query.as_bytes().get(keyword.len()) == Some(&b' ')
                });
                if let Some(keyword) = keyword {
                    selected.push((
                        package.manifest.id.clone(),
                        handler.id.clone(),
                        query[keyword.len()..].trim_start().to_owned(),
                    ));
                } else if handler.entry == "provider"
                    && package.enabled_providers.contains(&handler.id)
                {
                    ordinary.push((
                        package.manifest.id.clone(),
                        handler.id.clone(),
                        query.into(),
                    ));
                }
            }
        }
        let keyword = !selected.is_empty();
        self.extension_providers.keyword = keyword;
        let handlers = if keyword {
            selected.into_iter().take(1).collect::<Vec<_>>()
        } else {
            ordinary
        };
        let generation = self.extension_providers.generation;
        let query = query.to_owned();
        for (package_id, handler_id, input) in handlers {
            let (cancel, cancellation) = tokio::sync::watch::channel(false);
            self.extension_providers.cancels.push(cancel);
            let (batches, mut receiver) = tokio::sync::mpsc::channel(16);
            let service = service.clone();
            let query = query.clone();
            let request = InvokeRequest {
                package_id,
                handler_id,
                correlation_id: uuid::Uuid::new_v4(),
                generation,
                context: serde_json::json!({"query":input,"keyword":keyword}),
                causality: nxtools_extensions::Causality::root(uuid::Uuid::new_v4()),
                cancellation,
                batches: Some(batches),
            };
            let runtime = service.runtime.clone();
            let request_cancel = request.cancellation.clone();
            let task = service.executor().spawn(async move {
                tokio::time::sleep(std::time::Duration::from_millis(150)).await;
                if *request_cancel.borrow() {
                    return Err(nxtools_extensions::RuntimeError::Cancelled);
                }
                runtime.invoke(request).await
            });
            cx.spawn(async move |this, cx| {
                while let Some((identity, notice)) = receiver.recv().await {
                    if let InvocationNotice::ResultBatch { replace, items, .. } = notice {
                        let _ = this.update(cx, |app, cx| {
                            app.apply_extension_batch(
                                &query, generation, identity, replace, items, cx,
                            )
                        });
                    }
                }
                let _ = task.await;
            })
            .detach();
        }
    }
    pub(super) fn apply_extension_batch(
        &mut self,
        query: &str,
        generation: u64,
        identity: InvocationIdentity,
        replace: bool,
        items: Vec<ProviderItem>,
        cx: &mut Context<Self>,
    ) {
        if self.extension_providers.generation != generation
            || self.extension_providers.query != query
            || identity.generation != generation
        {
            return;
        }
        let Some(package) = self.config.extensions.packages.iter().find(|package| {
            package.enabled
                && package.trust.is_some()
                && package.manifest.id == identity.package_id
                && package.content_hash == identity.content_hash
        }) else {
            return;
        };
        let source = package.manifest.name.clone();
        let Some(popup) = self.popup.clone() else {
            return;
        };
        let state = popup
            .panel
            .update(cx, |panel, cx| {
                if !panel.is_open
                    || panel.action_mode.is_some()
                    || panel.search_input.read(cx).value().as_ref() != query
                {
                    return None;
                }
                Some((panel.results.clone(), panel.selected, panel.search_pending))
            })
            .ok()
            .flatten();
        let Some((existing, selected, pending)) = state else {
            return;
        };
        let mut merge = ResultMerge::new(generation, self.extension_providers.keyword);
        let owners = self
            .extension_providers
            .rows
            .values()
            .map(|(identity, _, _)| identity.clone())
            .collect::<Vec<_>>();
        for owner in owners.iter().chain(std::iter::once(&identity)) {
            merge.register(owner.clone());
        }
        if !self.extension_providers.keyword && !pending {
            merge.set_normal(
                existing
                    .iter()
                    .filter(|row| !row.entry.id.starts_with("extension-script:"))
                    .map(|row| (row.entry.id.clone(), row.clone()))
                    .collect(),
            );
        }
        let old_items = self
            .extension_providers
            .order
            .iter()
            .filter_map(|id| self.extension_providers.rows.get(id))
            .map(|(owner, item, _)| (owner.clone(), item.clone()))
            .collect::<Vec<_>>();
        for (owner, item) in old_items {
            merge.apply(&owner, false, &[item], |item, id| {
                prepare_item(item, id, &source)
            });
        }
        if let Some(selected) = existing.get(selected) {
            let _ = merge.select(&selected.entry.id.replace("extension-script:", "extension:"));
        }
        let previous_ids = items
            .iter()
            .map(|item| item.id.clone())
            .collect::<std::collections::HashSet<_>>();
        if replace {
            self.extension_providers.rows.retain(|_, (owner, item, _)| {
                owner.package_id != identity.package_id
                    || owner.handler_id != identity.handler_id
                    || previous_ids.contains(&item.id)
            });
        }
        merge.apply(&identity, replace, &items, |item, id| {
            prepare_item(item, id, &source)
        });
        let results = merge
            .rows()
            .iter()
            .map(|row| row.value.clone())
            .collect::<Vec<_>>();
        let retained = results
            .iter()
            .map(|row| row.entry.id.clone())
            .collect::<std::collections::HashSet<_>>();
        self.extension_providers
            .rows
            .retain(|id, _| retained.contains(id));
        for item in items {
            let id = format!(
                "extension-script:{}/{}/{}",
                identity.package_id, identity.handler_id, item.id
            );
            if let Some(row) = results.iter().find(|row| row.entry.id == id) {
                self.extension_providers
                    .rows
                    .insert(id, (identity.clone(), item, row.clone()));
            }
        }
        self.extension_providers
            .order
            .retain(|id| self.extension_providers.rows.contains_key(id));
        for row in &results {
            if row.entry.id.starts_with("extension-script:")
                && !self.extension_providers.order.contains(&row.entry.id)
            {
                self.extension_providers.order.push(row.entry.id.clone());
            }
        }
        let selected_id = merge.selected_id().map(|id| {
            if results.iter().any(|row| row.entry.id == id) {
                id.to_owned()
            } else {
                id.replace("extension:", "extension-script:")
            }
        });
        if pending && results.is_empty() {
            return;
        }
        self.request_icons(&results[..results.len().min(ICON_PREFETCH_LIMIT)], cx);
        let _ = popup.panel.update(cx, |panel, cx| {
            panel.replace_results(results, cx);
            panel.search_pending = false;
            if let Some(id) = selected_id
                && let Some(index) = panel.results.iter().position(|row| row.entry.id == id)
            {
                panel.selected = index;
            }
            cx.notify();
        });
    }
    pub(super) fn merge_extension_rows(
        &self,
        query: &str,
        normal: Vec<SearchResult>,
    ) -> Vec<SearchResult> {
        if self.extension_providers.query != query {
            return normal;
        }
        let mut rows = if self.extension_providers.keyword {
            Vec::new()
        } else {
            normal
        };
        rows.extend(
            self.extension_providers
                .order
                .iter()
                .filter_map(|id| self.extension_providers.rows.get(id))
                .map(|(_, _, row)| row.clone()),
        );
        rows
    }
    pub(super) fn extension_result_current(&self, result: &SearchResult, query: &str) -> bool {
        self.extension_providers.query == query
            && self
                .extension_providers
                .rows
                .get(&result.entry.id)
                .is_some_and(|(identity, _, row)| {
                    row == result
                        && identity.generation == self.extension_providers.generation
                        && self.config.extensions.packages.iter().any(|package| {
                            package.enabled
                                && package.trust.is_some()
                                && package.manifest.id == identity.package_id
                                && package.content_hash == identity.content_hash
                        })
                })
    }
    pub(super) fn verify_and_execute_extension(
        &mut self,
        result: SearchResult,
        action: QuickLaunchAction,
        query: String,
        trigger: QuickLaunchActionTrigger,
        cx: &mut Context<Self>,
    ) {
        let Some((identity, item, _)) =
            self.extension_providers.rows.get(&result.entry.id).cloned()
        else {
            return;
        };
        let Some(package) = self
            .config
            .extensions
            .packages
            .iter()
            .find(|package| package.manifest.id == identity.package_id)
            .cloned()
        else {
            return;
        };
        let generation = self.extension_providers.generation;
        let background = cx.background_executor().clone();
        cx.spawn(async move|this,cx|{let verified=background.spawn(async move{package.verify_approval().is_ok()}).await;let _=this.update(cx,|app,cx|{
            if !verified||app.extension_providers.generation!=generation||!app.extension_result_current(&result,&query){return;}
            let selected=app.popup.as_ref().is_some_and(|popup|popup.panel.upgrade().is_some_and(|panel|{let panel=panel.read(cx);panel.is_open&&!panel.search_pending&&panel.search_input.read(cx).value().as_ref()==query&&panel.results.get(panel.selected).is_some_and(|selected|selected==&result)}));if !selected{return;}
            if action==QuickLaunchAction::Open{match item.action{
                ExtensionEffect::Query{text}=>{let(_cancel,cancellation)=tokio::sync::watch::channel(false);let(reply,_)=async_channel::bounded(1);app.extension_set_query(text,cancellation,reply,cx);return;}
                ExtensionEffect::Invoke{action_id}=>{
                    let Some(service)=cx.try_global::<crate::extensions::ui_host::ExtensionServiceGlobal>().map(|global|global.0.clone())else{return;};
                    let(cancel,cancellation)=tokio::sync::watch::channel(false);app.extension_providers.cancels.push(cancel);let task=service.invoke(InvokeRequest{package_id:identity.package_id,handler_id:identity.handler_id,correlation_id:uuid::Uuid::new_v4(),generation,context:serde_json::json!({"query":query,"item_id":item.id,"action_id":action_id}),causality:nxtools_extensions::Causality::root(uuid::Uuid::new_v4()),cancellation,batches:None});
                    cx.spawn(async move|_,_|{let _=task.await;}).detach();return;
                }
                _=>{}
            }}
            app.execute_native_result(result,action,query,trigger,cx);
        });}).detach();
    }
}
fn prepare_item(item: &ProviderItem, id: &str, source: &str) -> SearchResult {
    let id = id.replace("extension:", "extension-script:");
    let (target, action) = match &item.action {
        ExtensionEffect::Open { target } => (target.clone(), SearchResultAction::OpenTarget),
        ExtensionEffect::Copy { text } => (
            String::new(),
            SearchResultAction::CopyText { text: text.clone() },
        ),
        ExtensionEffect::Program {
            executable,
            arguments,
            working_directory,
        } => (
            executable.clone(),
            SearchResultAction::RunCommand {
                command: nxtools_finder::PreparedCustomCommand {
                    executable: executable.clone(),
                    arguments: arguments.clone(),
                    working_directory: (!working_directory.is_empty())
                        .then_some(working_directory.clone()),
                    run_as_admin: false,
                    run_silently: false,
                },
            },
        ),
        ExtensionEffect::Query { text } => (
            text.clone(),
            SearchResultAction::CopyText { text: text.clone() },
        ),
        ExtensionEffect::Invoke { action_id } => (
            action_id.clone(),
            SearchResultAction::CopyText {
                text: action_id.clone(),
            },
        ),
    };
    SearchResult {
        entry: nxtools_finder::ApplicationEntry {
            id,
            name: item.title.clone(),
            target,
            source: ApplicationSource::CustomCommand,
        },
        action,
        subtitle: Some(format!("{source} · {}", item.subtitle)),
        score: 0,
        activation_count: 0,
        last_activated_at_unix_ms: None,
        query_activation_count: 0,
        last_query_activation_at_unix_ms: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::{AppContext, TestAppContext};
    fn normal() -> SearchResult {
        prepare_item(
            &ProviderItem {
                id: "normal".into(),
                title: "Normal".into(),
                subtitle: String::new(),
                action: ExtensionEffect::Open {
                    target: "C:\\fixture".into(),
                },
            },
            "normal",
            "fixture",
        )
    }
    #[gpui_kit::test]
    fn expansion_storage_metadata_projection_keeps_current_provider_generation_and_rows(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_kit::init);
        let (_root, application, _endpoint) =
            super::super::tests::quick_launch_application_fixture(cx);
        let (cancel, cancellation) = tokio::sync::watch::channel(false);
        application.update(cx, |app, cx| {
            let manifest = br#"{"schema_version":1,"id":"fixture.metadata","name":"Metadata","version":"1","handlers":[{"id":"run","title":"Run","feature":"tools","entry":"action","action":{"type":"website","url_template":"https://fixture.test/"}}]}"#;
            app.config.extensions.install(nxtools_extensions::ImportPreview::from_json(manifest).expect("preview"), "owned".into(), false).expect("install");
            app.extension_providers.generation = 31;
            app.extension_providers.query = "unchanged".into();
            app.extension_providers.cancels.push(cancel);
            app.extension_providers.order.push("existing".into());
            let mut bound = app.config.extensions.clone();
            bound.packages[0].data_uuid = Some(uuid::Uuid::new_v4());
            app.refresh_extension_storage_metadata(&bound, cx);
            assert_eq!(app.config.extensions, bound);
            assert_eq!(app.extension_providers.generation, 31);
            assert_eq!(app.extension_providers.query, "unchanged");
            assert_eq!(app.extension_providers.order, vec!["existing".to_owned()]);
            assert_eq!(app.extension_providers.cancels.len(), 1);
            assert!(!*cancellation.borrow(), "metadata must not cancel a running provider");
        });
    }
    #[gpui_kit::test]
    fn real_panel_late_batches_keep_identity_and_current_input_retires_actions(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_kit::init);
        let (_root, application, endpoint) =
            super::super::tests::quick_launch_application_fixture(cx);
        let slot = Rc::new(RefCell::new(None));
        let retained = slot.clone();
        let owner = application.downgrade();
        let (_view, window) = cx.add_window_view(move |window, cx| {
            let panel = cx.new(|cx| {
                QuickLaunchPanel::new(
                    vec![normal()],
                    nxtools_protocol::ForegroundContext {
                        window_handle: 1,
                        focus_window_handle: 1,
                        process_id: 1,
                        process_name: "fixture".into(),
                        window_title: String::new(),
                    },
                    None,
                    Vec::new(),
                    owner,
                    window,
                    cx,
                )
            });
            *retained.borrow_mut() = Some(panel.clone());
            Root::new(panel, window, cx)
        });
        let panel = slot.borrow().clone().expect("panel");
        panel.update_in(window, |panel, window, cx| {
            panel.is_open = true;
            panel
                .search_input
                .update(cx, |input, cx| input.set_value("query", window, cx));
        });
        window.run_until_parked();
        let manifest=br#"{"schema_version":1,"id":"fixture.provider","name":"Provider","version":"1","handlers":[{"id":"provider","title":"Provider","feature":"quick_launch","entry":"command","keywords":["query"],"action":{"type":"folder","path":"C:\\fixture"}}]}"#;
        let identity = window.update(|window, cx| {
            application.update(cx, |app, _| {
                app.config
                    .extensions
                    .install(
                        nxtools_extensions::ImportPreview::from_json(manifest).expect("preview"),
                        "fixture".into(),
                        false,
                    )
                    .expect("install");
                let package = &mut app.config.extensions.packages[0];
                package.enabled = true;
                let hash = package.content_hash.clone();
                package.trust = Some(nxtools_extensions::TrustApproval {
                    content_hash: hash.clone(),
                    runtime: nxtools_extensions::RuntimeIdentity {
                        executable: PathBuf::from("C:\\fixture.exe"),
                        sha256: "a".repeat(64),
                    },
                });
                app.extension_providers.generation = 17;
                app.extension_providers.query = "query".into();
                app.popup = Some(QuickLaunchWindow {
                    window: window.window_handle(),
                    panel: panel.downgrade(),
                    native_window_handle: 1,
                });
                InvocationIdentity {
                    run_id: uuid::Uuid::new_v4(),
                    correlation_id: uuid::Uuid::new_v4(),
                    generation: 17,
                    package_id: "fixture.provider".into(),
                    handler_id: "provider".into(),
                    content_hash: hash,
                }
            })
        });
        panel.update_in(window, |panel, _, _| {
            panel.results = vec![normal()].into();
            panel.selected = 0;
            panel.search_pending = false;
        });
        let item = ProviderItem {
            id: "one".into(),
            title: "Original".into(),
            subtitle: String::new(),
            action: ExtensionEffect::Copy {
                text: "owned fixture".into(),
            },
        };
        window.update(|_, cx| {
            application.update(cx, |app, cx| {
                app.apply_extension_batch(
                    "query",
                    17,
                    identity.clone(),
                    false,
                    vec![item.clone()],
                    cx,
                )
            })
        });
        panel.update_in(window, |panel, _, _| {
            assert_eq!(panel.results[panel.selected].entry.id, "normal");
            assert_eq!(panel.results.len(), 2);
            panel.selected = 1;
        });
        let mut updated = item.clone();
        updated.title = "Updated".into();
        let another = ProviderItem {
            id: "two".into(),
            title: "New".into(),
            subtitle: String::new(),
            action: ExtensionEffect::Copy {
                text: "other".into(),
            },
        };
        window.update(|_, cx| {
            application.update(cx, |app, cx| {
                app.apply_extension_batch(
                    "query",
                    17,
                    identity.clone(),
                    false,
                    vec![updated, another],
                    cx,
                )
            })
        });
        let result = panel.update_in(window, |panel, _, _| {
            assert_eq!(panel.selected, 1);
            assert_eq!(panel.results[1].entry.name, "Updated");
            panel.results[1].clone()
        });
        window.update(|_, cx| {
            application.update(cx, |app, _| {
                assert!(app.extension_result_current(&result, "query"));
                assert!(!app.extension_result_current(&result, "new input"));
            })
        });
        let mut stale = identity.clone();
        stale.content_hash = "b".repeat(64);
        window.update(|_, cx| {
            application.update(cx, |app, cx| {
                app.apply_extension_batch("query", 17, stale, true, Vec::new(), cx)
            })
        });
        panel.update_in(window, |panel, _, _| assert_eq!(panel.results.len(), 3));
        panel.update_in(window, |panel, _, _| {
            panel.results = vec![normal()].into();
            panel.selected = 0;
            panel.search_pending = true;
        });
        window.update(|_, cx| {
            application.update(cx, |app, cx| {
                app.extension_providers.rows.clear();
                app.extension_providers.order.clear();
                app.apply_extension_batch("query", 17, identity.clone(), false, Vec::new(), cx);
            })
        });
        panel.read_with(window, |panel, _| {
            assert!(panel.search_pending);
            assert_eq!(panel.results.len(), 1);
            assert_eq!(panel.results[0].entry.id, "normal");
        });
        window.update(|_, cx| {
            application.update(cx, |app, cx| {
                app.apply_extension_batch(
                    "query",
                    17,
                    identity.clone(),
                    false,
                    vec![item.clone()],
                    cx,
                );
            })
        });
        panel.read_with(window, |panel, _| {
            assert!(
                !panel.search_pending,
                "current extension results cannot wait for the indexer terminal"
            );
            assert_eq!(
                panel.results.len(),
                1,
                "old ordinary rows must not become actionable with fresh extension rows"
            );
            assert!(panel.results[0].entry.id.starts_with("extension-script:"));
        });
        panel.update_in(window, |panel, window, cx| {
            panel
                .search_input
                .update(cx, |input, cx| input.set_value("new input", window, cx))
        });
        window.run_until_parked();
        while endpoint.try_receive_command_for_contract_test().is_ok() {}
        window.update(|_, cx| {
            application.update(cx, |app, cx| {
                app.execute(
                    result,
                    QuickLaunchAction::Open,
                    "query".into(),
                    QuickLaunchActionTrigger::Enter,
                    cx,
                )
            })
        });
        assert!(
            endpoint.try_receive_command_for_contract_test().is_err(),
            "retired result cannot dispatch"
        );
    }
}
