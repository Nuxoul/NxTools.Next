//! Launch/Quick Find handoff into the one ArchiveApplication; no shell fallback on rejection.
use super::*;
use crate::archive::{draft::DraftSource, forms::FormCompletion};
use crate::archive_launch::LaunchCompletion;
use nxtools_protocol::{
    archive_job::*,
    desktop_archive_intent::{
        ArchiveExtractMode, ArchiveIntentAction, DesktopArchiveIntent, LaunchFailure,
    },
};
type OpenCompletion = Rc<dyn Fn(Result<(), LaunchFailure>, &mut App)>;

pub(crate) fn handle_launch_intent(
    intent: DesktopArchiveIntent,
    completion: LaunchCompletion,
    cx: &mut App,
) {
    let Some(owner) = cx.try_global::<ArchiveGlobal>().map(|g| g.0.downgrade()) else {
        completion.finish(Err(LaunchFailure::OwnerUnavailable));
        return;
    };
    cx.defer(move |cx| {
        if completion.is_pending() {
            let failure = completion.clone();
            if owner
                .update(cx, |app, cx| app.launch_intent(intent, completion, cx))
                .is_err()
            {
                failure.finish(Err(LaunchFailure::OwnerUnavailable));
            }
        }
    });
}
pub(crate) fn open_internal(
    path: PathBuf,
    complete: impl Fn(Result<(), LaunchFailure>, &mut App) + 'static,
    cx: &mut App,
) -> bool {
    let Some(owner) = cx.try_global::<ArchiveGlobal>().map(|g| g.0.downgrade()) else {
        return false;
    };
    if !owner
        .read_with(cx, |app, _| app.settings_open_internal())
        .unwrap_or(false)
    {
        return false;
    }
    let complete: OpenCompletion = Rc::new(complete);
    cx.defer(move |cx| {
        let failed = complete.clone();
        if owner
            .update(cx, |app, cx| app.open_with_completion(path, complete, cx))
            .is_err()
        {
            failed(Err(LaunchFailure::OwnerUnavailable), cx)
        }
    });
    true
}
pub(crate) fn new_from_window(cx: &mut App) {
    if let Some(owner) = cx.try_global::<ArchiveGlobal>().map(|g| g.0.downgrade()) {
        cx.defer(move |cx| {
            let _ = owner.update(cx, |app, cx| app.open_create_form(vec![], cx));
        });
    }
}
pub(crate) fn extract_from_window(cx: &mut App) {
    if let Some(owner) = cx.try_global::<ArchiveGlobal>().map(|g| g.0.downgrade()) {
        cx.defer(move |cx| {
            let _ = owner.update(cx, |app, cx| {
                app.open_extract_form(PathBuf::new(), None, cx)
            });
        });
    }
}
impl ArchiveApplication {
    fn settings_open_internal(&self) -> bool {
        self.settings.open_in_nxtools
    }
    pub(super) fn open_with_completion(
        &mut self,
        path: PathBuf,
        complete: OpenCompletion,
        cx: &mut Context<Self>,
    ) {
        let owner = cx.entity().downgrade();
        let background = cx.background_executor().clone();
        cx.spawn(async move |_, cx| {
            let normalized = background
                .spawn(async move {
                    std::fs::canonicalize(path).map(|path| {
                        let value = path.to_string_lossy();
                        value
                            .strip_prefix(r"\\?\")
                            .map(PathBuf::from)
                            .unwrap_or(path)
                    })
                })
                .await;
            let Ok(path) = normalized else {
                cx.update(|cx| complete(Err(LaunchFailure::OpenFailed), cx));
                return;
            };
            let started = Instant::now();
            loop {
                let path = path.clone();
                let ready = owner
                    .update(cx, |app, cx| {
                        if app.ui_state.as_ref().is_some_and(|store| {
                            matches!(store.load_status(), UiStateLoad::Loading)
                        }) {
                            return None;
                        }
                        app.open_canonical(path.clone(), 0, vec![], cx);
                        Some(app.browser_id(&path).is_some())
                    })
                    .ok()
                    .flatten();
                if let Some(ready) = ready {
                    cx.update(|cx| {
                        complete(
                            if ready {
                                Ok(())
                            } else {
                                Err(LaunchFailure::OpenFailed)
                            },
                            cx,
                        )
                    });
                    return;
                }
                if started.elapsed() > std::time::Duration::from_secs(2) {
                    cx.update(|cx| complete(Err(LaunchFailure::OpenFailed), cx));
                    return;
                }
                cx.background_executor()
                    .timer(std::time::Duration::from_millis(20))
                    .await;
            }
        })
        .detach();
    }
    fn launch_intent(
        &mut self,
        intent: DesktopArchiveIntent,
        completion: LaunchCompletion,
        cx: &mut Context<Self>,
    ) {
        if !completion.is_pending() {
            return;
        }
        if intent.validate().is_err() {
            completion.finish(Err(LaunchFailure::InvalidRequest));
            return;
        }
        if intent.action == ArchiveIntentAction::Open {
            let complete = completion.clone();
            self.open_with_completion(
                intent.paths[0].clone().into(),
                Rc::new(move |result, _| {
                    complete.finish(result);
                }),
                cx,
            );
            return;
        }
        let owner = cx.entity().downgrade();
        let background = cx.background_executor().clone();
        cx.spawn(async move|_,cx|{
            let paths=intent.paths.clone();
            let sources=background.spawn(async move{paths.into_iter().map(|path|{let normalized=std::fs::canonicalize(path).map_err(|_|LaunchFailure::OpenFailed)?;let metadata=std::fs::symlink_metadata(&normalized).map_err(|_|LaunchFailure::OpenFailed)?;if metadata.file_type().is_symlink(){return Err(LaunchFailure::InvalidRequest)};let value=normalized.to_string_lossy();let path=value.strip_prefix(r"\\?\").map(PathBuf::from).unwrap_or(normalized);Ok(DraftSource{path,directory:metadata.is_dir(),bytes:metadata.is_file().then_some(metadata.len())})}).collect::<Result<Vec<_>,LaunchFailure>>()}).await;
            if !completion.is_pending(){return}
            let Ok(mut sources)=sources else{completion.finish(Err(LaunchFailure::OpenFailed));return};
            if intent.action==ArchiveIntentAction::Extract && sources.len()>1 {
                let mut observed=Vec::new();
                for source in &sources {
                    let (send,receive)=async_channel::bounded(1);
                    let path=source.path.to_string_lossy().into_owned();
                    let destination=source.path.parent().map(|p|p.to_string_lossy().into_owned()).unwrap_or_default();
                    let command=nxtools_protocol::archive_space::ArchiveSpaceCommand{version:2,operation_id:Uuid::new_v4(),session_id:Uuid::new_v4(),generation:1,source:path,destination,expected_source:None,expected_target:None};
                    if owner.update(cx,|app,cx|app.start_job_observation(command,None,Some(send),cx)).is_err(){completion.finish(Err(LaunchFailure::OwnerUnavailable));return;}
                    observed.push(receive.recv().await.unwrap_or_else(|_|Err("observation_closed".into())));
                    if !completion.is_pending(){return;}
                }
                match deduplicate_source_sets(sources,observed){Ok(unique)=>sources=unique,Err(())=>{completion.finish(Err(LaunchFailure::OpenFailed));return;}}
            }
            let _=owner.update(cx,|app,cx|{
                if !completion.is_pending(){return}
                match intent.action{
                    ArchiveIntentAction::Compress=>{if sources.len()>32{completion.finish(Err(LaunchFailure::InvalidRequest));}else{let before=app.jobs.forms.len();app.open_create_form(sources,cx);completion.finish(if app.jobs.forms.len()>before{Ok(())}else{Err(LaunchFailure::OpenFailed)});}},
                    ArchiveIntentAction::Extract=>{
                        if sources.iter().any(|s|s.directory)||sources.len()>32{completion.finish(Err(LaunchFailure::InvalidRequest));return}
                        if matches!(intent.mode, Some(ArchiveExtractMode::Dialog | ArchiveExtractMode::SmartDialog | ArchiveExtractMode::NameDialog)) {
                            let strategy=match intent.mode { Some(ArchiveExtractMode::SmartDialog)=>nxtools_core::archive::ArchiveDestination::Smart,Some(ArchiveExtractMode::NameDialog)=>nxtools_core::archive::ArchiveDestination::Name,_=>app.settings.default_destination };
                            let before=app.jobs.forms.keys().copied().collect::<std::collections::HashSet<_>>();
                            for source in sources { app.open_extract_confirmation(source.path,strategy,cx); }
                            let forms=app.jobs.forms.keys().filter(|id|!before.contains(id)).copied().collect();app.bind_confirmation_forms(forms,cx);
                            completion.finish(Ok(()));
                        }
                        else{
                            let strategy=match intent.mode{Some(ArchiveExtractMode::Smart)=>ArchiveJobStrategy::Smart,Some(ArchiveExtractMode::Name)=>ArchiveJobStrategy::Name,_=>ArchiveJobStrategy::Here};
                            let requests=sources.into_iter().map(|source|ArchiveJobRequest{kind:ArchiveJobKind::Extract,destination:source.path.parent().map(|p|p.to_string_lossy().into_owned()),paths:vec![source.path.to_string_lossy().into_owned()],codepage:default_codepage(app.settings.filename_encoding),limits:Default::default(),extract:Some(ArchiveJobExtract{strategy,single_file_folder:strategy==ArchiveJobStrategy::Smart&&app.settings.smart_single_file_folder,conflict:match app.settings.conflict_policy{nxtools_core::archive::ArchiveConflictPolicy::Ask=>ArchiveJobConflict::Ask,nxtools_core::archive::ArchiveConflictPolicy::Overwrite=>ArchiveJobConflict::Overwrite,nxtools_core::archive::ArchiveConflictPolicy::Skip=>ArchiveJobConflict::Skip,nxtools_core::archive::ArchiveConflictPolicy::AutoRename=>ArchiveJobConflict::AutoRename},copy_motw:app.settings.copy_motw,selection:None,reclamation:None}),create:None,readmission:None,modify:None}).collect::<Vec<_>>();
                            completion.finish(app.submit_batch(requests,cx).map(|_|()).map_err(|_|LaunchFailure::QueueFull));
                        }
                    },
                    ArchiveIntentAction::Test=>{let source=&sources[0];let request=ArchiveJobRequest{kind:ArchiveJobKind::Test,paths:vec![source.path.to_string_lossy().into_owned()],destination:None,codepage:default_codepage(app.settings.filename_encoding),limits:Default::default(),extract:None,create:None,readmission:None,modify:None};completion.finish(app.submit_job(request,FormCompletion::default(),cx).map(|_|()).map_err(|_|LaunchFailure::QueueFull));},
                    ArchiveIntentAction::Open=>{},
                }
            });
        }).detach();
    }
}

/// Only native validated source sets establish membership; no basename/case/suffix inference.
fn deduplicate_source_sets(
    sources: Vec<DraftSource>,
    observed: Vec<Result<nxtools_protocol::archive_space::ArchiveSpaceObservation, String>>,
) -> Result<Vec<DraftSource>, ()> {
    if sources.len() != observed.len()
        || observed
            .iter()
            .any(|o| o.as_ref().is_ok_and(|o| o.source_set.is_none()))
    {
        return Err(());
    }
    let sets = observed
        .iter()
        .filter_map(|o| o.as_ref().ok()?.source_set.as_ref())
        .collect::<Vec<_>>();
    for (source, observation) in sources.iter().zip(&observed) {
        if observation.is_err()
            && !sets.iter().any(|set| {
                set.members
                    .iter()
                    .any(|p| std::path::Path::new(p) == source.path)
            })
        {
            return Err(());
        }
    }
    let mut seen = std::collections::HashSet::new();
    let mut result = Vec::new();
    for set in sets {
        if seen.insert(set.identity.clone()) {
            result.push(DraftSource {
                path: set.primary.clone().into(),
                directory: false,
                bytes: None,
            });
        }
    }
    if result.is_empty() {
        Err(())
    } else {
        Ok(result)
    }
}
#[cfg(test)]
mod settings_completion_tests {
    #[gpui_kit::test]
    fn shell_settings_double_click_launch_opens_original_form_without_starting_jobs(
        cx: &mut gpui_kit::TestAppContext,
    ) {
        for argument in ["smart-dialog", "name-dialog"] {
            let (owner, _native) = super::super::tests::setup(cx);
            let directory = tempfile::tempdir().expect("owned source");
            let source = directory.path().join("fixture.zip");
            std::fs::write(&source, b"synthetic").expect("fixture file");
            let intent = crate::archive_launch::parse_arguments(
                [
                    "--archive-extract".into(),
                    argument.into(),
                    source.clone().into_os_string(),
                ],
                directory.path(),
            )
            .expect("fixed command parse")
            .expect("intent");
            let (ingress, receiver) = nxtools_ipc::desktop_archive_intent::archive_intent_channel();
            ingress.submit(intent);
            let (intent, completion) = receiver.try_next().expect("original bounded ingress");
            owner.update(cx, |app, cx| app.launch_intent(intent, completion, cx));
            cx.run_until_parked();
            let window = owner.read_with(cx, |app, cx| {
                assert_eq!(app.jobs.forms.len(), 1, "original confirmation form");
                assert!(
                    app.jobs.records.is_empty(),
                    "double click must not start extraction"
                );
                let expected = if argument == "smart-dialog" {
                    nxtools_core::archive::ArchiveDestination::Smart
                } else {
                    nxtools_core::archive::ArchiveDestination::Name
                };
                assert_eq!(
                    app.jobs
                        .forms
                        .values()
                        .next()
                        .expect("form")
                        .view
                        .read_with(cx, |form, _| form.confirmation_strategy_for_test())
                        .expect("live form"),
                    Some(expected)
                );
                app.jobs.forms.values().next().expect("form").window
            });
            cx.update(|cx| {
                window
                    .update(cx, |_, window, cx| {
                        window.draw(cx).clear(cx);
                        let labels = gpui_kit::base::test_support::snapshots(window)
                            .into_iter()
                            .filter_map(|node| node.label().map(str::to_owned))
                            .collect::<Vec<_>>();
                        assert!(
                            labels.iter().any(|label| label == "解压目标策略"),
                            "original strategy control mounted"
                        );
                    })
                    .expect("original window")
            });
        }
    }

    #[cfg(test)]
    mod hz3_tests {
        use super::super::*;
        fn facts(
            source: &str,
            identity: &str,
            members: &[&str],
        ) -> nxtools_protocol::archive_space::ArchiveSpaceObservation {
            serde_json::from_value(serde_json::json!({"version":2,"operation_id":Uuid::new_v4(),"session_id":Uuid::new_v4(),"generation":1,"available_bytes":0,"allocation_bytes":4096,"target_identity":"v1:target","volume_identity":"v1:volume","destination_missing":false,"source_matches":true,"source_set":{"identity":identity,"primary":source,"members":members}})).expect("native source facts")
        }
        #[test]
        fn hz3_native_source_sets_deduplicate_volumes_not_same_basename() {
            let source = |path: &str| DraftSource {
                path: path.into(),
                directory: false,
                bytes: None,
            };
            let family = facts(
                r"C:\a\one.7z.001",
                "v1:family",
                &[r"C:\a\one.7z.001", r"C:\a\one.7z.002"],
            );
            let same_name = facts(r"C:\b\one.7z.001", "v1:other", &[r"C:\b\one.7z.001"]);
            let unique = deduplicate_source_sets(
                vec![
                    source(r"C:\a\one.7z.002"),
                    source(r"C:\a\one.7z.001"),
                    source(r"C:\b\one.7z.001"),
                ],
                vec![Err("missing_volume".into()), Ok(family), Ok(same_name)],
            )
            .expect("recognized family");
            assert_eq!(unique.len(), 2);
            assert_eq!(unique[0].path, PathBuf::from(r"C:\a\one.7z.001"));
            assert!(
                deduplicate_source_sets(
                    vec![source(r"C:\missing\one.7z.002")],
                    vec![Err("missing_volume".into())]
                )
                .is_err()
            );
        }
    }
}
