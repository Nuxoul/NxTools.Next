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
                        Some(
                            app.paths
                                .get(&path)
                                .is_some_and(|id| app.windows.contains_key(id)),
                        )
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
            let Ok(sources)=sources else{completion.finish(Err(LaunchFailure::OpenFailed));return};
            let _=owner.update(cx,|app,cx|{
                if !completion.is_pending(){return}
                match intent.action{
                    ArchiveIntentAction::Compress=>{if sources.len()>32{completion.finish(Err(LaunchFailure::InvalidRequest));}else{let before=app.jobs.forms.len();app.open_create_form(sources,cx);completion.finish(if app.jobs.forms.len()>before{Ok(())}else{Err(LaunchFailure::OpenFailed)});}},
                    ArchiveIntentAction::Extract=>{
                        if sources.iter().any(|s|s.directory)||sources.len()>32{completion.finish(Err(LaunchFailure::InvalidRequest));return}
                        if intent.mode==Some(ArchiveExtractMode::Dialog){for source in sources{app.open_extract_form(source.path,None,cx);}completion.finish(Ok(()));}
                        else{
                            let strategy=match intent.mode{Some(ArchiveExtractMode::Smart)=>ArchiveJobStrategy::Smart,Some(ArchiveExtractMode::Name)=>ArchiveJobStrategy::Name,_=>ArchiveJobStrategy::Here};
                            let requests=sources.into_iter().map(|source|ArchiveJobRequest{kind:ArchiveJobKind::Extract,destination:source.path.parent().map(|p|p.to_string_lossy().into_owned()),paths:vec![source.path.to_string_lossy().into_owned()],codepage:default_codepage(app.settings.filename_encoding),limits:Default::default(),extract:Some(ArchiveJobExtract{strategy,single_file_folder:false,conflict:match app.settings.conflict_policy{nxtools_core::archive::ArchiveConflictPolicy::Ask=>ArchiveJobConflict::Ask,nxtools_core::archive::ArchiveConflictPolicy::Overwrite=>ArchiveJobConflict::Overwrite,nxtools_core::archive::ArchiveConflictPolicy::Skip=>ArchiveJobConflict::Skip,nxtools_core::archive::ArchiveConflictPolicy::AutoRename=>ArchiveJobConflict::AutoRename},copy_motw:app.settings.copy_motw,selection:None}),create:None,readmission:None,modify:None}).collect::<Vec<_>>();
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
